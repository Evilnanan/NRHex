"""Record the two-key reward interface and preserve unrelated package assets."""
import hashlib
import json
import re
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

def settings(text):
    return {key.strip(): value.strip() for line in text.splitlines()
            if '=' in line and not line.strip().startswith((';', '#'))
            for key, value in [line.split('=', 1)]}

work = ROOT / '.work/hotkeys'
work.mkdir(parents=True, exist_ok=True)
baseline = work / 'package-baseline.json'
ini_path = ROOT / 'dist/mod/natives/NRHex.ini'
csv_path = ROOT / 'data/balance/sword.csv'
if '--before' in sys.argv:
    baseline.write_text(json.dumps(dict(assets=assets(), csvSha256=digest(csv_path),
                                       ini=ini_path.read_text(encoding='utf8')), indent=2), encoding='utf8')
    print('Captured package, current settings and user balance CSV before hotkey cleanup.')
    sys.exit(0)

before = json.loads(baseline.read_text())
after = assets()
changed = sorted(p for p in before['assets'].keys() | after.keys()
                 if before['assets'].get(p) != after.get(p))
assert set(changed) <= {'mod/natives/NRHex.dll', 'mod/natives/NRHex.ini'}, changed
assert digest(csv_path) == before['csvSha256']
old = settings(before['ini'])
current = settings(ini_path.read_text(encoding='utf8'))
expected_settings = {'VirtualKey', 'TestVirtualKey', 'NewHexVirtualKey', 'ChantVirtualKey', 'RequireControl', 'CooldownMs'}
assert current.keys() == expected_settings
assert all(current[key] == old[key] for key in expected_settings)
assert int(current['VirtualKey'], 0) == 0x75 and int(current['TestVirtualKey'], 0) == 0x76
assert current['RequireControl'] == 'true'
source = (ROOT / 'src/lib.rs').read_text(encoding='utf8')
update = source[source.index('unsafe extern "system" fn on_update'):source.index('\n#[cfg(test)]\nmod tests {')]
assert re.findall(r'GetAsyncKeyState\(cfg\.([a-z_]+)\)', update) == ['key', 'test_key']
assert re.findall(r'static ([A-Z_]+KEY_WAS_DOWN):', source) == ['TEST_KEY_WAS_DOWN']
assert 'static KEY_WAS_DOWN:' in source
assert 'const TEST_REWARD: [usize;3] = [14,6,12];' in source
assert 'if test_pressed {Some(TEST_REWARD)}else{None}' in update
assert all(token in update for token in ['cfg.cooldown', 'cfg.ctrl', 'GetForegroundWindow()',
                                        'unclaimed_hex_ball(base)', 'unavailable_hexes(player,base)',
                                        'reward_choices::preferred(o,owned)', 'random_order(owned)'])
version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
dll_hash = digest(ROOT / 'target/release/NRHex.dll')
assert dll_hash == after['mod/natives/NRHex.dll']
tests = int(sys.argv[sys.argv.index('--rust-tests') + 1])
report = dict(version=version, activeHotkeys={'Ctrl+F6': 'random reward',
               'Ctrl+F7': ['Monster Cage', 'Crystal Shell', 'Versatile Arts']},
              preferredOwnerIds=[95001600, 95000700, 95001400],
              removedDedicatedKeySettings=sorted(old.keys() - current.keys()),
              existingCooldownFocusOwnershipTerrainAndPendingBallGuardsPreserved=True,
              unrelatedAssetsAndUserBalanceUnchanged=True, rustTestsPassed=tests,
              changedAssets=changed, gameplayVerified=False, multiplayerVerified=False)
(ROOT / 'dist/reports/hotkey-cleanup-validation.json').write_text(json.dumps(report, indent=2), encoding='utf8')
build_path = ROOT / 'dist/reports/build-validation.json'
build = json.loads(build_path.read_text())
build.update(version=version, scope='Keep random F6 and cage/crystal/arts F7; remove dedicated debug keys',
             changedAssets=changed, dllSha256=dll_hash, installedDllSha256=dll_hash,
             dllInstalled=True, rustTestsPassed=tests, regulationSha256=after['mod/regulation.bin'],
             unrelatedAssetsAndUserConfigsUnchanged=True, gameplayVerified=False, multiplayerVerified=False)
build['functionChecks'] = list(dict.fromkeys(build.get('functionChecks', []) + ['hotkey-cleanup-validation.json']))
build_path.write_text(json.dumps(build, indent=2), encoding='utf8')
runtime_path = ROOT / 'dist/reports/runtime-validation.json'
runtime = json.loads(runtime_path.read_text())
runtime.update(version=version, active=None, pid=None, allFourNativeDllsLoaded=False,
               dllSha256=dll_hash, dllInstalled=True, gameplayVerified=False, multiplayerVerified=False,
               notes='Debug key cleanup built and installed; actual game keypresses remain unverified.')
runtime['pendingChecks'] = [check.replace('Ctrl+F10', 'random reward') for check in runtime.get('pendingChecks', [])]
runtime_path.write_text(json.dumps(runtime, indent=2), encoding='utf8')
integration_path = ROOT / 'dist/reports/integration-validation.json'
integration = json.loads(integration_path.read_text())
for name in ['mod/natives/NRHex.dll', 'mod/natives/NRHex.ini']:
    integration['nativeFiles'][name] = after[name]
integration.update(gameplayVerified=False, loadingVerified=False)
integration_path.write_text(json.dumps(integration, indent=2), encoding='utf8')
print('PASS: only Ctrl+F6/F7 polled; focus choices and existing gates verified; only DLL/INI changed.')
