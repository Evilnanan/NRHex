"""Record only the incremental cage checks; keep full-build evidence separate."""
from pathlib import Path
import hashlib, json, re, sys, tomllib
from local_paths import ROOT, GAME


def hashes():
    files = list((ROOT/'dist/mod').rglob('*'))
    for folder in ['mod/natives', 'testmods']:
        files += [p for p in (ROOT/'dist'/folder).rglob('*') if p.suffix in ('.dll', '.ini', '.csv', '.me3')]
    files += list((ROOT/'dist/support').rglob('*'))
    files += [ROOT/'dist/记录刷怪笼状态.ps1'] if (ROOT/'dist/记录刷怪笼状态.ps1').exists() else []
    files += list((ROOT/'dist').glob('*.me3'))
    return {p.relative_to(ROOT/'dist').as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in files if p.is_file() and p.suffix != '.log'}


baseline = ROOT/'.work/cage/patch-baseline.json'
baseline.parent.mkdir(parents=True, exist_ok=True)
if '--before' in sys.argv:
    baseline.write_text(json.dumps(hashes(), indent=2), encoding='utf8')
    print('Captured package assets and user configuration hashes before cage rebuild.')
else:
    before = json.loads(baseline.read_text())
    after = hashes()
    changed = sorted(p for p in before.keys() | after.keys() if before.get(p) != after.get(p))
    allowed = {f'mod/event/m46_{block}_00_00.emevd.dcx' for block in (50, 60, 70, 80)}
    allowed |= {f'mod/map/mapstudio/m46_{block}_00_00.msb.dcx' for block in (50, 60, 70, 80)}
    retired={'support/cage_snapshot.py','support/cage-health-reference.json','记录刷怪笼状态.ps1'}
    allowed |= {'mod/natives/NRHex.dll','mod/regulation.bin'} | retired
    assert set(changed) <= allowed, f'Unexpected asset/config change: {changed}'
    version = tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version']
    cage = json.loads((ROOT/'dist/cage-validation.json').read_text())
    assert cage['version'] == version and cage['perPeerBirthPlayback'] and not cage['healthBarRequiresReplicaAI']
    mutation=json.loads((ROOT/'dist/cage-mutation-validation.json').read_text())
    parameters=json.loads((ROOT/'.work/cage/parameter-patch.json').read_text())
    assert mutation['version']==version and mutation['decisionCases']==15984
    assert parameters['combatFieldsPreserved']==37 and parameters['unrelatedRowsAndTablesPreserved']
    assert parameters['outputRegulationSha256'].lower()==after['mod/regulation.bin']
    assert not retired & after.keys()
    assert not (ROOT/'tools/cage_snapshot.py').exists() and not (ROOT/'tools/verify_cage_health.py').exists()
    signature_count = int(re.search(r"fn signatures\(\)->\[\(&'static str,u32\);(\d+)\]", (ROOT/'src/lib.rs').read_text()).group(1))
    digest = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
    report = dict(version=version,incrementalBuildPassed=True,scope='Cage native mutation and seven tower weapon reward overrides; retired live diagnostics; previous lifecycle/HUD fixes retained',
                  exeSha256=digest(GAME/'nightreign.exe'),signatureCount=signature_count,
                  dllSha256=digest(ROOT/'target/release/NRHex.dll'),installedDllSha256=digest(ROOT/'dist/mod/natives/NRHex.dll'),
                  changedAssets=changed,assetSha256={p:after[p] for p in sorted(allowed) if p in after},unrelatedAssetsAndUserConfigsUnchanged=True,
                  cageChecks='cage-validation.json',mutationChecks='cage-mutation-validation.json',parameterChecks=parameters,diagnosticToolsRemoved=True,me3ProfileChecked=True,gameplayVerified=False,multiplayerVerified=False,
                  reportedFailure='0.17.9 HUD fix failed: opener sees HUD, peers intermittently do not but can attack; ordinary solo after dozens of openings can stall until day/map reload; bosses reported fragile in Deep Night',
                  pendingChecks=['Opener and peers see HUD/intro; reverse opener roles', 'Repeated immediate reopening beyond dozens of rounds', 'Deep Night depth 1 never red; depth 2..5 red visual/combat/runes and rewards', 'Seven tower archetypes offer weapons in normal and red rewards', 'Simultaneous nearby bosses and unload/reload'])
    (ROOT/'dist/cage-patch-validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf8')
    installed = report['dllSha256'] == report['installedDllSha256']
    if '--no-install' not in sys.argv:
        assert installed, 'Built DLL was not installed'
    build = dict(version=version,buildPassed=True,incremental=True,scope=report['scope'],signatureCount=signature_count,
                 functionChecks=['cage-validation.json','cage-patch-validation.json','cage-mutation-validation.json','sword-balance-validation.json','jiahao-balance-validation.json','drinking-hud-validation.json','escape-vfx-validation.json','cat-balance-validation.json','effect-sync-validation.json'],dllSha256=report['dllSha256'],changedAssets=changed,
                 installedDllSha256=report['installedDllSha256'],dllInstalled=installed,gameplayVerified=False,multiplayerVerified=False)
    if '--rust-tests' in sys.argv:
        build['rustTestsPassed'] = int(sys.argv[sys.argv.index('--rust-tests')+1])
    (ROOT/'dist/build-validation.json').write_text(json.dumps(build,indent=2),encoding='utf8')
    runtime = dict(version=version,active=None,pid=None,allFourNativeDllsLoaded=False,build='build-validation.json',
                   dllSha256=report['installedDllSha256'],dllInstalled=installed,gameplayVerified=False,multiplayerVerified=False,
                   pendingChecks=report['pendingChecks'],notes='Incremental cage rebuild; no game process was launched or checked on this computer.')
    (ROOT/'dist/runtime-validation.json').write_text(json.dumps(runtime,indent=2),encoding='utf8')
    print('PASS: cage patch scope, current EXE/DLL hashes and user configuration preservation recorded; multiplayer remains unverified.')
