"""Check regeneration pulse parameters and the matching runtime FP gate."""
import hashlib
import json
import sys
import tomllib
from local_paths import ROOT

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def assets():
    paths = [p for folder in ['mod', 'testmods']
             for p in (ROOT / 'dist' / folder).rglob('*')]
    paths += list((ROOT / 'dist').glob('*.me3'))
    return {p.relative_to(ROOT / 'dist').as_posix(): digest(p)
            for p in paths if p.is_file() and p.suffix != '.log'}

work = ROOT / '.work/regen-balance'
work.mkdir(parents=True, exist_ok=True)
baseline = work / 'package-baseline.json'
if '--before' in sys.argv:
    baseline.write_text(json.dumps(assets(), indent=2), encoding='utf8')
    print('Captured package assets and user configuration before regeneration patch.')
    sys.exit(0)

before = json.loads(baseline.read_text())
after = assets()
changed = sorted(p for p in before.keys() | after.keys() if before.get(p) != after.get(p))
assert set(changed) <= {'mod/regulation.bin', 'natives/NRHex.dll'}, changed
patch = json.loads((work / 'parameter-patch.json').read_text())
assert patch['outputRegulationSha256'].lower() == after['mod/regulation.bin']
assert patch['effectId'] == 95000601
assert {c['field']: c['after'] for c in patch['changes']} == {'changeHpPoint': -16, 'changeMpPoint': 4}
assert patch['fpCost'] == 4 and patch['hpRestore'] == 16
assert all(patch[k] for k in ['unrelatedRowsAndFieldsUnchanged', 'unrelatedTablesUnchanged', 'encryptedRegulationReadbackPassed'])
version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
runtime_source = (ROOT / 'src/runtime.rs').read_text(encoding='utf8')
assert 's.fp>=4' in runtime_source and 'REGEN_INTERVAL: f32 = 0.25' in runtime_source
dll_hash = digest(ROOT / 'target/release/NRHex.dll')
assert dll_hash == after['natives/NRHex.dll']
test_count = int(sys.argv[sys.argv.index('--rust-tests') + 1])
report = dict(version=version, **patch, intervalSeconds=0.25, changedAssets=changed,
              runtimeMinimumFp=4, rustTestsPassed=test_count,
              eventsTextsAndUserConfigurationUnchanged=True,
              gameplayVerified=False, multiplayerVerified=False)
(ROOT / 'dist/reports/regen-balance-validation.json').write_text(json.dumps(report, indent=2), encoding='utf8')
build_path = ROOT / 'dist/reports/build-validation.json'
build = json.loads(build_path.read_text())
build.update(version=version, scope='Regeneration spends 4 FP to heal 16 HP per 0.25 second; parameters and runtime FP gate',
             changedAssets=changed, regulationSha256=after['mod/regulation.bin'],
             dllSha256=dll_hash, installedDllSha256=dll_hash, dllInstalled=True,
             rustTestsPassed=test_count,
             unrelatedAssetsAndUserConfigsUnchanged=True,
             gameplayVerified=False, multiplayerVerified=False)
build['functionChecks'] = list(dict.fromkeys(build.get('functionChecks', []) + ['regen-balance-validation.json']))
build_path.write_text(json.dumps(build, indent=2), encoding='utf8')
runtime_path = ROOT / 'dist/reports/runtime-validation.json'
runtime = json.loads(runtime_path.read_text())
runtime.update(version=version, active=None, pid=None, allFourNativeDllsLoaded=False,
               dllSha256=dll_hash, dllInstalled=True, gameplayVerified=False, multiplayerVerified=False,
               notes='Regeneration pulse and FP gate updated; current game loading and gameplay remain unverified.')
runtime['pendingChecks'] = list(dict.fromkeys(runtime.get('pendingChecks', []) + ['Regeneration at exactly 4 FP and below, 16 HP per pulse']))
runtime_path.write_text(json.dumps(runtime, indent=2), encoding='utf8')
print('PASS: 4 FP / 16 HP per pulse; runtime gate 4 FP; events, texts and user configuration preserved.')
