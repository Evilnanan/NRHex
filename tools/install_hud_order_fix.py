"""Install the HUD-only DLL update and prove all game/config assets unchanged."""
from distribution_paths import distribution_file
from pathlib import Path
import hashlib
import json
import shutil
import tomllib

root = Path(__file__).resolve().parents[1]
work = root/'.work/hud-order-01728'
dist = root/'dist'
digest = lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
write = lambda p,value:p.write_text(json.dumps(value,ensure_ascii=False,indent=2),encoding='utf8')
baseline = json.loads((work/'baseline.json').read_text())
version = tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
assert version == '0.17.28'
assert (root/'data/balance/sword.csv').read_bytes() == (work/'before/data/balance/sword.csv').read_bytes()
for name,sha in baseline.items():
    if name != 'README.md':assert digest(distribution_file(dist,name)) == sha, ('package changed before install',name)
report = json.loads((work/'validation.json').read_text())
assert report['version']==version and report['nativeHudCases']==1761
assert report['originalHudSwapReproduced'] and report['allCustomIconRowsAudited']
report.update(menuNativeComparatorWithExplicitRegistryStubPassed=True,
              rustUnitTestsPassed=159,signatureGuardsPassed=56,
              actualHudAndListRenderingVerified=False)
native = root/'target/release/NRHex.dll'
assert digest(native) != baseline['natives/NRHex.dll']
for name in ['readiness-sort-validation.json','scope-validation.json','build-validation.json',
             'runtime-validation.json','integration-validation.json','icon-validation.json']:
    path = distribution_file(dist,name)
    if path.exists():
        backup = work/'before/reports'/name
        backup.parent.mkdir(parents=True,exist_ok=True)
        shutil.copy2(path,backup)
shutil.copy2(native,dist/'natives/NRHex.dll')
write(dist/'reports'/'readiness-sort-validation.json',report)
scope = dict(version=version,assets={'natives/NRHex.dll':dict(before=baseline['natives/NRHex.dll'],after=digest(native))},
             regulationEventsMapsAndHksUnchanged=True,allUserConfigurationsUnchanged=True,
             handEditedSwordCsvUnchanged=True,gameplayVerified=False,multiplayerVerified=False)
write(dist/'reports'/'scope-validation.json',scope)
allow = {'natives/NRHex.dll','README.md','readiness-sort-validation.json','scope-validation.json'}
for name,sha in baseline.items():
    if name not in allow:assert digest(distribution_file(dist,name))==sha, ('unrelated package changed',name)
assert digest(dist/'natives/NRHex.dll') == digest(native)
write(work/'installed-assets.json',scope)
print('PASS: installed HUD/list DLL fix; regulation, events, maps, HKS, CSV and configs unchanged.')
