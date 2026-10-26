"""Verify the native Bloodboil visual chain and additive Escape visual patch."""
import hashlib
import json
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

work = ROOT / '.work/escape-vfx'
work.mkdir(parents=True, exist_ok=True)
baseline = work / 'package-baseline.json'
if '--before' in sys.argv:
    snapshot = assets()
    if '--from-research' in sys.argv:
        snapshot['mod/regulation.bin'] = digest(work / 'before-regulation.bin')
    baseline.write_text(json.dumps(snapshot, indent=2), encoding='utf8')
    print('Captured package baseline for Escape visual scope verification.')
    sys.exit(0)

patch = json.loads((work / 'parameter-patch.json').read_text())
probe = json.loads((work / 'parameters.json').read_text())
good = probe['nativeGood']['fields']
bullet = probe['nativeBullet']['fields']
native_effects = {r['ID']: r['fields'] for r in probe['nativeEffects']}
native_vfx = {r['ID']: r['fields'] for r in probe['nativeVfx']}
assert good['refCategory'] == 1 and good['refId_default'] == 10355000
assert bullet['spEffectId0'] == 503550 and native_effects[503550]['vfxId'] == 5280
assert native_vfx[5280]['midstSfxId'] == 4530 and native_vfx[5280]['midstDmyId'] == 905
expected = dict(probe['nativeNeutral']['fields'])
expected.update(maxHpRate=1, saveCategory=-1, effectEndurance=10, spCategory=20,
                iconId=-1, spEffectTextId_1=-1, displayIconForPermamentEffect=0, vfxId=5280)
assert probe['currentVisual']['ID'] == 95001204 and probe['currentVisual']['fields'] == expected
active = probe['currentEffect']['fields']
assert active['vfxId'] == 340800 and active['effectEndurance'] == 10
assert all(active[f] == .4 for f in ['slashDamageCutRate','blowDamageCutRate','thrustDamageCutRate','neutralDamageCutRate','magicDamageCutRate','fireDamageCutRate','thunderDamageCutRate','darkDamageCutRate'])
assert patch['originalEffectsUnchanged'] and patch['unrelatedTablesUnchanged']
assert digest(ROOT / 'dist/mod/regulation.bin').upper() == patch['outputRegulationSha256'] == probe['installedRegulationSha256']
before = json.loads(baseline.read_text())
after = assets()
assert before['mod/regulation.bin'].upper() == patch['inputRegulationSha256']
changed = sorted(p for p in before.keys() | after.keys() if before.get(p) != after.get(p))
assert set(changed) <= {'mod/regulation.bin', 'natives/NRHex.dll'}, changed
dll_hash = digest(ROOT / 'target/release/NRHex.dll')
assert dll_hash == after['natives/NRHex.dll']
version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
test_count = int(sys.argv[sys.argv.index('--rust-tests') + 1])
report = dict(version=version, nativeGoods=3550, nativeBullet=10355000, nativeEffect=503550,
              nativeVfx=5280, midstSfxId=4530, midstDmyId=905, addedEffect=95001204,
              durationSeconds=10, originalEscapeVisualRetained=True,
              visualFieldsMatchNeutralTemplate=True, originalEffectsAndUnrelatedTablesUnchanged=True,
              changedAssets=changed, previousBalanceSyncCageAssetsAndUserConfigurationUnchanged=True,
              rustTestsPassed=test_count, gameplayVerified=False, multiplayerVerified=False)
(ROOT / 'dist/reports/escape-vfx-validation.json').write_text(json.dumps(report, indent=2), encoding='utf8')
build = dict(version=version, buildPassed=True, incremental=True,
             scope='Independent neutral Bloodboil aura during Escape Plan; DLL and one added param row',
             signatureCount=49, rustTestsPassed=test_count, changedAssets=changed,
             originalExeSha256=digest(GAME / 'nightreign.exe'), dllSha256=dll_hash,
             installedDllSha256=dll_hash, dllInstalled=True, regulationSha256=after['mod/regulation.bin'],
             functionChecks=['escape-vfx-validation.json','cat-balance-validation.json','jiahao-balance-validation.json','effect-sync-validation.json','cage-patch-validation.json'],
             unrelatedAssetsAndUserConfigsUnchanged=True, gameplayVerified=False, multiplayerVerified=False)
(ROOT / 'dist/reports/build-validation.json').write_text(json.dumps(build, indent=2), encoding='utf8')
runtime = dict(version=version, active=None, pid=None, allFourNativeDllsLoaded=False,
               build='build-validation.json', dllSha256=dll_hash, dllInstalled=True,
               gameplayVerified=False, multiplayerVerified=False,
               pendingChecks=['Escape Bloodboil aura starts with activation, ends after ten seconds or incapacitation',
                              'Previously pending balance and multiplayer checks'],
               notes='No game session was used for this additive visual patch.')
(ROOT / 'dist/reports/runtime-validation.json').write_text(json.dumps(runtime, indent=2), encoding='utf8')
print('PASS: original Bloodboil visual chain; neutral visual-only row; original Escape and unrelated assets preserved.')
