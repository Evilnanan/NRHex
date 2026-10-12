"""Guard native anchors and record the DLL/text-only Jiahao balance patch."""
import hashlib
import json
import re
import struct
import sys
import tomllib
import pefile
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

work = ROOT / '.work/jiahao'
work.mkdir(parents=True, exist_ok=True)
baseline = work / 'package-baseline.json'
if '--before' in sys.argv:
    baseline.write_text(json.dumps(assets(), indent=2), encoding='utf8')
    print('Captured package assets and configuration before Jiahao DLL/text patch.')
    sys.exit(0)

raw = (GAME / 'nightreign.exe').read_bytes()
exe_hash = hashlib.sha256(raw).hexdigest()
assert exe_hash == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe = pefile.PE(data=raw, fast_load=True)
def check(rva, expected):
    code = bytes.fromhex(expected)
    assert pe.get_data(rva, len(code)) == code, hex(rva)
def call(rva):
    assert pe.get_data(rva, 1) == b'\xe8'
    return rva + 5 + struct.unpack('<i', pe.get_data(rva + 1, 4))[0]

# Native current/max gauge ratio is converted to a 0..100 percentage.
check(0x48b904, 'f30f104314')
check(0x48b90e, 'f30f5ec1f30f59c6')
constant = 0x48b87e + 8 + struct.unpack('<i', pe.get_data(0x48b882, 4))[0]
assert struct.unpack('<f', pe.get_data(constant, 4))[0] == 100.0
# Calculator writes HP damage before normal NPC bypass and the hit callback.
assert call(0x43d4be) == 0x439e80
check(0x439ef3, '4d8bf04c8be24c8bf9')
check(0x439efc, '41f680c102000008')
check(0x43ab95, 'f30f2cc041898680020000')
check(0x43d534, 'f680f9010000010f8409010000')
assert call(0x43e296) == 0x43ced0
assert 0x43d4be < 0x43d534 < 0x43e296

attacks = json.loads((ROOT / 'data/reference/actions/attack-throw.json').read_text())['attacks']
backstep_rows = [r for r in attacks if 129 in [r['fields']['subCategory' + str(i)] for i in range(1, 6)]]
assert len(backstep_rows) == 173
before = json.loads(baseline.read_text())
after = assets()
changed = sorted(p for p in before.keys() | after.keys() if before.get(p) != after.get(p))
allowed = {'mod/natives/NRHex.dll', 'mod/msg/zhocn/item_dlc01.msgbnd.dcx', 'mod/msg/engus/item_dlc01.msgbnd.dcx'}
assert set(changed) <= allowed, changed
assert before['mod/regulation.bin'] == after['mod/regulation.bin']
text = json.loads((work / 'text-validation.json').read_text())
assert len(text) == 2 and {r['language'] for r in text} == {'zhocn', 'engus'}
for row in text:
    assert row['unrelatedEntriesUnchanged'] and row['unrelatedMembersUnchanged']
    assert row['outputSha256'].lower() == after[f"mod/msg/{row['language']}/item_dlc01.msgbnd.dcx"]
    assert row['effectId'] == 95001300
zh = next(r['description'] for r in text if r['language'] == 'zhocn')
assert '后跃攻击的伤害增幅当前绝招量表百分比的两倍' in zh
assert '后跃攻击命中时，回复25%绝招量表' in zh
assert '正面处决敌人时，回复25%绝招量表' in zh
assert '非后跃闪避时，损失25%绝招量表' in zh
assert zh.endswith('后跃攻击的伤害增幅当前绝招量表百分比的两倍')
en = next(r['description'] for r in text if r['language'] == 'engus')
assert 'Other dodges cost 25% Art gauge.' in en
assert 'Frontal critical hits restore 25%.' in en
assert en.endswith('Backstep attacks gain bonus damage equal to twice your current Art gauge percentage.')
dll_hash = digest(ROOT / 'target/release/NRHex.dll')
assert dll_hash == after['mod/natives/NRHex.dll']
version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
test_count = int(sys.argv[sys.argv.index('--rust-tests') + 1])
checks = dict(version=version, exeSha256=exe_hash, nativeAnchorsPassed=True,
              backstepCategory=129, nativeBackstepAttackRows=len(backstep_rows),
              backstepGaugeRestorePercent=25, frontalCriticalGaugeRestorePercent=25,
              nonBackstepDodgeGaugeLossPercent=25, backstepDamageDescriptionLast=True,
              bonusFormula='1 + 2 * clamp(current Art gauge percentage, 0, 100) / 100',
              gaugeSample='Per resolved hit, before that hit queues its gauge reward',
              fullGaugeBonusPercent=200, fullGaugeDamageMultiplier=3,
              rustTestsPassed=test_count, descriptionReadback=text,
              changedAssets=changed, regulationUnchanged=True,
              previousSyncCageAssetsAndUserConfigurationUnchanged=True,
              gameplayVerified=False, multiplayerVerified=False)
(ROOT / 'dist/jiahao-balance-validation.json').write_text(json.dumps(checks, ensure_ascii=False, indent=2), encoding='utf8')
build = dict(version=version, buildPassed=True, incremental=True,
             scope='Jiahao owning-player DLL balance and Chinese/English description only',
             signatureCount=int(re.search(r'fn signatures\(\)->\[\(&\x27static str,u32\);(\d+)\]', (ROOT / 'src/lib.rs').read_text()).group(1)), rustTestsPassed=test_count, changedAssets=changed,
             originalExeSha256=exe_hash, dllSha256=dll_hash, installedDllSha256=dll_hash,
             dllInstalled=True, regulationSha256=after['mod/regulation.bin'],
             functionChecks=['jiahao-balance-validation.json', 'sword-balance-validation.json', 'drinking-hud-validation.json', 'escape-vfx-validation.json', 'cat-balance-validation.json', 'effect-sync-validation.json', 'cage-patch-validation.json'],
             unrelatedAssetsAndUserConfigsUnchanged=True, gameplayVerified=False, multiplayerVerified=False)
(ROOT / 'dist/build-validation.json').write_text(json.dumps(build, indent=2), encoding='utf8')
runtime = dict(version=version, active=None, pid=None, allFourNativeDllsLoaded=False,
               build='build-validation.json', dllSha256=dll_hash, dllInstalled=True,
               gameplayVerified=False, multiplayerVerified=False,
               pendingChecks=['Jiahao backstep attacks at empty/half/full Art gauge',
                              '25% dodge loss, 25% backstep and 25% frontal critical recovery, dual-hit deduplication',
                              'Previously pending Cat/Escape visuals and coop cage/effect-list checks'],
               notes='No game session was used for this DLL/text-only balance build.')
(ROOT / 'dist/runtime-validation.json').write_text(json.dumps(runtime, indent=2), encoding='utf8')
print('PASS: current native gauge/calculator anchors; 173 backstep rows; DLL/text-only patch; previous sync/cage assets preserved.')
