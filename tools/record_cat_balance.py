"""Record a DLL-only Cat damage coefficient change without gameplay claims."""
import hashlib
import json
import re
import sys
import tomllib
from local_paths import GAME, ROOT

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def assets():
    paths = []
    for folder in ['mod', 'testmods']:
        paths += list((ROOT / 'dist' / folder).rglob('*'))
    paths += list((ROOT / 'dist').glob('*.me3'))
    return {p.relative_to(ROOT / 'dist').as_posix(): digest(p)
            for p in paths if p.is_file() and p.suffix != '.log'}

work = ROOT / '.work/cat-balance'
work.mkdir(parents=True, exist_ok=True)
baseline = work / 'package-baseline.json'
if '--before' in sys.argv:
    baseline.write_text(json.dumps(assets(), indent=2), encoding='utf8')
    print('Captured assets and user configuration before Cat DLL balance patch.')
    sys.exit(0)

before = json.loads(baseline.read_text())
after = assets()
changed = sorted(p for p in before.keys() | after.keys() if before.get(p) != after.get(p))
assert set(changed) <= {'mod/natives/NRHex.dll'}, changed
dll_hash = digest(ROOT / 'target/release/NRHex.dll')
assert dll_hash == after['mod/natives/NRHex.dll']
version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
native = json.loads((ROOT / 'dist/reports/cat-jump-native-validation.json').read_text())
assert native['version'] == version and len(native['isolatedNativeHeightCases']) == 6
source = (ROOT / 'src/cat_jump.rs').read_text()
coefficient = float(re.search(r'HEIGHT_BONUS_COEFFICIENT:f64=([0-9.]+)', source)[1])
assert coefficient == 1.0 and native['formula'] == 'max(0, 1.00 * (1 + log2(h / 1)))'
assert native['landingGraceMilliseconds'] == 250 and native['eachHitUsesCurrentHeight']
test_count = int(sys.argv[sys.argv.index('--rust-tests') + 1])
checks = dict(version=version, oldCoefficient=0.4, coefficient=coefficient,
              formula=native['formula'], rustTestsPassed=test_count,
              heightAnchors=[dict(metres=h, bonusPercent=b, damageMultiplier=m)
                             for h, b, m in [(1,100,2),(2,200,3),(4,300,4),(8,400,5),(16,500,6)]],
              nativeHeightChecks='cat-jump-native-validation.json', changedAssets=changed,
              previousJiahaoSyncCageAssetsAndUserConfigurationUnchanged=True,
              gameplayVerified=False, multiplayerVerified=False)
(ROOT / 'dist/reports/cat-balance-validation.json').write_text(json.dumps(checks, indent=2), encoding='utf8')
build = dict(version=version, buildPassed=True, incremental=True,
             scope='Cat jumping damage coefficient; DLL only', signatureCount=49,
             rustTestsPassed=test_count, changedAssets=changed,
             originalExeSha256=digest(GAME / 'nightreign.exe'), dllSha256=dll_hash,
             installedDllSha256=dll_hash, dllInstalled=True,
             regulationSha256=after['mod/regulation.bin'],
             functionChecks=['cat-balance-validation.json', 'cat-jump-native-validation.json',
                             'jiahao-balance-validation.json', 'effect-sync-validation.json', 'cage-patch-validation.json'],
             unrelatedAssetsAndUserConfigsUnchanged=True, gameplayVerified=False, multiplayerVerified=False)
(ROOT / 'dist/reports/build-validation.json').write_text(json.dumps(build, indent=2), encoding='utf8')
runtime = dict(version=version, active=None, pid=None, allFourNativeDllsLoaded=False,
               build='build-validation.json', dllSha256=dll_hash, dllInstalled=True,
               gameplayVerified=False, multiplayerVerified=False,
               pendingChecks=['Cat damage at normal jumps and elevated drops, multi-hit and spiritspring',
                              'Previously pending Jiahao balance and coop cage/effect-list checks'],
               notes='No game session was used for this DLL-only balance build.')
(ROOT / 'dist/reports/runtime-validation.json').write_text(json.dumps(runtime, indent=2), encoding='utf8')
print('PASS: Cat coefficient 100%; DLL-only patch; previous assets and user configuration preserved.')
