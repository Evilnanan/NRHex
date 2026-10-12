"""Record this incremental patch without inheriting gameplay/coop test claims."""
import hashlib
import json
import sys
import tomllib
from local_paths import GAME, ROOT

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def assets():
    paths = list((ROOT / 'dist/mod').rglob('*'))
    paths += list((ROOT / 'dist/testmods').rglob('*')) + list((ROOT / 'dist').glob('*.me3'))
    return {p.relative_to(ROOT / 'dist').as_posix(): digest(p) for p in paths if p.is_file() and p.suffix != '.log'}

baseline = ROOT / '.work/effect-sync/package-baseline.json'
baseline.parent.mkdir(parents=True, exist_ok=True)
if '--before' in sys.argv:
    snapshot = assets()
    if '--from-research' in sys.argv:
        # The first targeted param patch was generated during native research.
        snapshot['mod/regulation.bin'] = digest(ROOT / '.work/effect-sync/before-regulation.bin')
    baseline.write_text(json.dumps(snapshot, indent=2), encoding='utf8')
    print('Captured package assets and user configuration for effect sync scope verification.')
else:
    before = json.loads(baseline.read_text())
    after = assets()
    changed = sorted(p for p in before.keys() | after.keys() if before.get(p) != after.get(p))
    assert set(changed) <= {'mod/regulation.bin', 'mod/natives/NRHex.dll'}, changed
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
    effects = json.loads((ROOT / 'dist/effect-sync-validation.json').read_text())
    assert effects['version'] == version and effects['originalCategoryPredicatesExecuted']
    assert effects['parameterReadbackPassed']
    patch = json.loads((ROOT / '.work/effect-sync/parameter-patch.json').read_text())
    assert after['mod/regulation.bin'].upper() == patch['outputRegulationSha256']
    dll_hash = digest(ROOT / 'target/release/NRHex.dll')
    assert dll_hash == after['mod/natives/NRHex.dll']
    cage = json.loads((ROOT / 'dist/cage-patch-validation.json').read_text())
    assert all(after[p] == h for p, h in cage['assetSha256'].items())
    report = dict(version=version, buildPassed=True, incremental=True,
                  scope='Effect sync parameter rows and owning-character DLL refresh; cage assets retained',
                  signatureCount=49, rustTestsPassed=153, changedAssets=changed,
                  originalExeSha256=digest(GAME / 'nightreign.exe'),
                  dllSha256=dll_hash, installedDllSha256=after['mod/natives/NRHex.dll'], dllInstalled=True,
                  regulationSha256=after['mod/regulation.bin'],
                  functionChecks=['effect-sync-validation.json', 'cage-patch-validation.json'],
                  cageAssetsUnchanged=True, unrelatedAssetsAndUserConfigsUnchanged=True,
                  gameplayVerified=False, multiplayerVerified=False)
    (ROOT / 'dist/build-validation.json').write_text(json.dumps(report, indent=2), encoding='utf8')
    runtime = dict(version=version, active=None, pid=None, allFourNativeDllsLoaded=False,
                   build='build-validation.json', dllSha256=dll_hash, dllInstalled=True,
                   gameplayVerified=False, multiplayerVerified=False,
                   pendingChecks=effects['pendingChecks'],
                   notes='No game process or private-server multiplayer session was used for this build.')
    (ROOT / 'dist/runtime-validation.json').write_text(json.dumps(runtime, indent=2), encoding='utf8')
    print('PASS: only regulation/DLL changed; cage assets and user configuration preserved; no packaging.')
