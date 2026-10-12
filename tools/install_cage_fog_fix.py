"""Install only the four verified birth-event updates and versioned DLL."""
from pathlib import Path
import hashlib,json,shutil,tomllib
root=Path(__file__).resolve().parents[1];dist=root/'dist';work=root/'.work/cage-fog-01729'
digest=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
write=lambda p,x:p.write_text(json.dumps(x,ensure_ascii=False,indent=2),encoding='utf8')
baseline=json.loads((work/'baseline.json').read_text());patch=json.loads((work/'patch.json').read_text())
proof=json.loads((work/'validation.json').read_text())
assert tomllib.loads((root/'Cargo.toml').read_text())['package']['version']=='0.17.29'
assert proof['oldPendingSecondEncounterSkipReproduced']==336 and proof['freshDelayRepeatAndReconnectCases']==2352
assert (root/'data/balance/sword.csv').read_bytes()==(work/'before/data/balance/sword.csv').read_bytes()
for name,sha in baseline.items():
 if name not in ['README.md','runtime-validation.json']:assert digest(dist/name)==sha,('input package changed',name)
assets={}
for report in patch['reports']:
 name=f"mod/event/{report['mapName']}.emevd.dcx";source=work/'events'/f"{report['mapName']}.emevd.dcx"
 assert report['inputSha256'].lower()==baseline[name] and report['outputSha256'].lower()==digest(source)
 assert report['unrelatedEventsAndConstructorUnchanged'] and len(report['events'])==84
 assets[name]=source
assets['mod/natives/NRHex.dll']=root/'target/release/NRHex.dll'
assert digest(assets['mod/natives/NRHex.dll'])!=baseline['mod/natives/NRHex.dll']
for name,source in assets.items():
 backup=work/'before/dist'/name;backup.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(dist/name,backup)
for name in ['scope-validation.json','build-validation.json','runtime-validation.json','integration-validation.json','icon-validation.json']:
 backup=work/'before/reports'/name;backup.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(dist/name,backup)
for name,source in assets.items():shutil.copy2(source,dist/name)
write(dist/'cage-fog-validation.json',proof)
scope=dict(version='0.17.29',assets={n:dict(before=baseline[n],after=digest(p)) for n,p in assets.items()},
           onlyBirthFreshOngoingBranchChanged=True,allOtherEventsAndMetadataUnchanged=True,
           regulationMapsHksAndUserConfigurationsUnchanged=True,handEditedSwordCsvUnchanged=True,
           gameplayVerified=False,multiplayerVerified=False)
write(dist/'scope-validation.json',scope);write(work/'installed-assets.json',scope)
allow=set(assets)|{'README.md','scope-validation.json','runtime-validation.json'}
for name,sha in baseline.items():
 if name not in allow:assert digest(dist/name)==sha,('unrelated delivered file changed',name)
assert all(digest(dist/name)==digest(source) for name,source in assets.items())
print('PASS: installed four birth-only event patches and versioned DLL; other events, maps, regulation, HKS and configs unchanged.')
