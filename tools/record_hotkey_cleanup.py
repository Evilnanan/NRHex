"""Verify the single random-reward shortcut and preserve unrelated package assets."""
import argparse
import hashlib
import json
import re
import tomllib
from pathlib import Path
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


parser = argparse.ArgumentParser()
parser.add_argument('--before', action='store_true')
parser.add_argument('--allow-text-updates', action='store_true')
parser.add_argument('--rust-tests', type=int)
parser.add_argument('--dll', type=Path)
args = parser.parse_args()
work = ROOT / '.work/hotkeys'
work.mkdir(parents=True, exist_ok=True)
baseline = work / 'package-baseline.json'
ini_path = ROOT / 'dist/mod/natives/NRHex.ini'
csv_path = ROOT / 'data/balance/sword.csv'
if args.before:
    baseline.write_text(json.dumps(dict(assets=assets(), csvSha256=digest(csv_path),
                                       ini=ini_path.read_text(encoding='utf8')), indent=2), encoding='utf8')
    print('Captured package, current settings and user balance CSV before hotkey cleanup.')
    raise SystemExit(0)

before = json.loads(baseline.read_text())
after = assets()
changed = sorted(p for p in before['assets'].keys() | after.keys()
                 if before['assets'].get(p) != after.get(p))
allowed = {'mod/natives/NRHex.dll', 'mod/natives/NRHex.ini'}
if args.allow_text_updates:
    allowed.update(f'mod/msg/zhocn/{kind}_dlc01.msgbnd.dcx' for kind in ['item', 'menu'])
assert set(changed) <= allowed, changed
assert digest(csv_path) == before['csvSha256']
old = settings(before['ini'])
current = settings(ini_path.read_text(encoding='utf8'))
assert current.keys() == {'VirtualKey', 'RequireControl', 'CooldownMs'}
assert all(current[key] == old[key] for key in current)
assert int(current['VirtualKey'], 0) == 0x75 and current['RequireControl'] == 'true'
source = (ROOT / 'src/lib.rs').read_text(encoding='utf8')
update = source[source.index('unsafe fn update_test_hotkeys'):source.index('\n#[cfg(test)]\nmod tests {')]
assert re.findall(r'GetAsyncKeyState\(cfg\.([a-z_]+)\)', update) == ['key']
assert re.findall(r'GetAsyncKeyState\(([^)]*)\)', update) == ['cfg.key', '0x11']
assert re.findall(r'static ([A-Z_]*KEY_WAS_DOWN):', source) == ['KEY_WAS_DOWN']
for retired in ['TEST_REWARD', 'NEW_HEX_REWARD', 'CHANT_REWARD', 'GAZE_REWARD', 'CONTRACT_REWARD',
                'cfg.test_key', 'cfg.new_hex_key', 'cfg.chant_key', 'cfg.gaze_key', 'cfg.contract_key']:
    assert retired not in source, retired
for guard in ['if !pressed{return;}', 'cfg.cooldown', 'cfg.ctrl', 'GetForegroundWindow()',
              'pid!=win::GetCurrentProcessId()', 'stats+0x140', 'unclaimed_hex_ball(base)',
              'CALLBACK_LOCK.swap(true', 'choose_reward(player,base,None)',
              'reward_cycle_runtime::commit(player,base,next_seen)']:
    assert guard in update, guard
assert '#[cfg(feature="test-hotkeys")]\nunsafe fn update_test_hotkeys' in source
version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
dll = args.dll or ROOT / 'dist/mod/natives/NRHex.dll'
assert digest(dll) == after['mod/natives/NRHex.dll']
assert args.rust_tests is not None and args.rust_tests > 0
report = dict(version=version, activeHotkeys={'Ctrl+F6': 'random reward'},
              removedDedicatedKeySettings=sorted(old.keys() - current.keys()),
              chineseTextUpdatesAuthorized=args.allow_text_updates,
              retiredIniKeysIgnoredWithoutPolling=True,
              existingCooldownFocusOwnershipTerrainAndPendingBallGuardsPreserved=True,
              unrelatedAssetsAndUserBalanceUnchanged=True, rustTestsPassed=args.rust_tests,
              changedAssets=changed, dllSha256=digest(dll),
              sourceHashes={p: digest(ROOT / p) for p in [
                  'src/lib.rs', 'tools/record_hotkey_cleanup.py', 'tools/package_no_hotkeys.py',
                  'dist/mod/natives/NRHex.ini']},
              gameplayVerified=False, multiplayerVerified=False)
(ROOT / 'dist/reports/hotkey-cleanup-validation.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf8')
print('PASS: only Ctrl+F6 remains; original spawn guards/settings retained; package changes limited to DLL/INI and explicitly allowed text updates.')
