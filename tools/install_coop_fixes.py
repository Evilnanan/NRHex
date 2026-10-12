"""Install the verified 0.17.26 -> 0.17.27 patch with an exact asset allowlist.

Run after closing the game; all inputs are checked before any package writes.
"""
from pathlib import Path
import hashlib,json,shutil,tomllib

root = Path(__file__).resolve().parents[1]
work = root/'.work/fixes-01727'
dist = root/'dist'
digest = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
read = lambda p: json.loads(p.read_text(encoding='utf-8'))
write = lambda p, x: p.write_text(json.dumps(x,ensure_ascii=False,indent=2),encoding='utf-8')
assert tomllib.loads((root/'Cargo.toml').read_text())['package']['version'] == '0.17.27'
baseline = read(work/'baseline.json')
assert (root/'data/balance/sword.csv').read_bytes() == (work/'sword-before.csv').read_bytes()
expected = {'mod/regulation.bin','mod/action/script/c0000.hks','mod/event/common_func.emevd.dcx'}
expected |= {f'mod/event/m46_{block}_00_00.emevd.dcx' for block in [50,60,70,80]}
changed = {}
for p in (work/'staged/mod').rglob('*'):
    if not p.is_file():continue
    k = 'mod/'+p.relative_to(work/'staged/mod').as_posix()
    if digest(p) != baseline.get(k):changed[k] = p
assert set(changed) == expected
for k,h in baseline.items():
    if k == 'README.md':continue
    assert digest(dist/k) == h, ('installed input changed',k)
parameters = read(work/'parameter-patch.json')
assert parameters['inputSha256'].lower() == baseline['mod/regulation.bin']
assert parameters['outputSha256'].lower() == digest(changed['mod/regulation.bin'])
assert parameters['originalRowsAndOtherTablesPreserved'] and len(parameters['corrections']) == 84
arts = read(work/'arts-stance-validation.json')
assert arts['independentDecompilePassed'] and arts['inverseRestoresInput']
assert arts['sourceSha256'] == baseline['mod/action/script/c0000.hks']
assert arts['outputSha256'] == digest(changed['mod/action/script/c0000.hks'])
groups = read(root/'.work/cage-groups/built.json')
for r in groups['reports']:
    assert r['eventSha256'] == digest(changed[f"mod/event/{r['mapName']}.emevd.dcx"])
    assert r['mapSha256'] == baseline[f"mod/map/mapstudio/{r['mapName']}.msb.dcx"]
coop = read(work/'coop-fixes-validation.json')
assert coop['singleAuthorityAndLifecycleCases'] == 296 and coop['groupEventCases'] == 960
assert not coop['multiplayerVerified'] and not coop['gameplayVerified']
native = root/'target/release/NRHex.dll'
assert native.is_file() and digest(native) != baseline['mod/natives/NRHex.dll']
reports = {'coop-fixes-validation.json':coop,'coop-parameter-validation.json':parameters,
           'arts-stance-validation.json':arts,
           'cage-groups-validation.json':read(root/'.work/cage-groups/validation.json'),
           'cage-validation.json':read(root/'.work/cage-groups/cage-validation.json'),
           'cage-mutation-validation.json':read(root/'.work/cage-groups/cage-mutation-validation.json')}
reports['readiness-sort-validation.json'] = dict(version='0.17.27',nativeComparatorRva='82ba40',
    nativeFixtureSha256=digest(root/'data/reference/ui/status-sort.bin'),nativeEntryBytes=20,
    readyEffects=[95000403,95000701,95001203],canonicalSortDuration=2,
    isolatedOriginalComparatorAndTrampolinePassed=True,originalSwapReproduced=True,
    callerEntriesAndActualLeasesUnchanged=True,gameplayVerified=False)
for name in [*reports,'arts-hks-validation.json','build-validation.json','runtime-validation.json',
             'integration-validation.json','icon-validation.json','scope-validation.json']:
    p = dist/name
    if p.exists():
        destination = work/'before/reports'/name;destination.parent.mkdir(parents=True,exist_ok=True)
        shutil.copy2(p,destination)
for k,p in changed.items():shutil.copy2(p,dist/k)
shutil.copy2(native,dist/'mod/natives/NRHex.dll')
for name,report in reports.items():write(dist/name,report)
old_arts = read(dist/'arts-hks-validation.json')
old_arts.update(version='0.17.27',stanceReentryFix='arts-stance-validation.json',
                gameplayVerified=False,multiplayerVerified=False)
write(dist/'arts-hks-validation.json',old_arts)
allow = expected | {'mod/natives/NRHex.dll','README.md',*reports,'arts-hks-validation.json'}
for k,h in baseline.items():
    if k not in allow:assert digest(dist/k) == h, ('unrelated delivered file changed',k)
for k,p in changed.items():assert digest(dist/k) == digest(p)
assert digest(dist/'mod/natives/NRHex.dll') == digest(native)
scope = dict(version='0.17.27',assets={k:dict(before=baseline[k],after=digest(dist/k))
             for k in sorted(expected|{'mod/natives/NRHex.dll'})},originalMapsUnchanged=True,
             unrelatedPackageFilesUnchanged=True,handEditedSwordCsvUnchanged=True,
             originalParameterRowsAndOtherTablesUnchanged=True,gameplayVerified=False,multiplayerVerified=False)
write(work/'installed-assets.json',scope);write(dist/'scope-validation.json',scope)
print('PASS: installed seven verified game assets and DLL; maps, configs, CSV and unrelated files preserved.')
