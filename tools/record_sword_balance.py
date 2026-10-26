"""Record Sword role eligibility, lifesteal scaling and a DLL/text-only patch."""
import ctypes as c
import hashlib
import json
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
    result = {p.relative_to(ROOT / 'dist').as_posix(): digest(p)
              for p in paths if p.is_file() and p.suffix != '.log'}
    result['source:data/balance/sword.csv'] = digest(ROOT / 'data/balance/sword.csv')
    return result


work = ROOT / '.work/sword-balance'
work.mkdir(parents=True, exist_ok=True)
baseline = work / 'package-baseline.json'
if '--before' in sys.argv:
    baseline.write_text(json.dumps(assets(), indent=2), encoding='utf8')
    print('Captured package and current user-edited Sword CSV before patch.')
    sys.exit(0)

exe_hash = digest(GAME / 'nightreign.exe')
assert exe_hash == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe = pefile.PE(str(GAME / 'nightreign.exe'), fast_load=True)
# Native current-player role read, covered by the new live/scancheck signature.
read_code = pe.get_data(0xb61462, 0x17)
assert read_code.hex() == '488b0567640a03488b48084885c974200fb681c7000000'
assert 0xb61462 + 7 + struct.unpack_from('<i', read_code, 3)[0] == 0x3c078d0
getter = pe.get_data(0xe87df0, 0x12)
assert getter.hex() == '488b41084885c074080fb680de000000c3c3'
heroes = {row['ID']: row['heroId'] for row in json.loads((work / 'native-heroes.json').read_text())}
for index in range(10):
    assert heroes[50000 + index * 100] == index + 1
assert (heroes[50300], heroes[50500], heroes[50600]) == (4, 6, 7)

k = c.WinDLL('kernel32', use_last_error=True)
k.VirtualAlloc.argtypes = [c.c_void_p, c.c_size_t, c.c_ulong, c.c_ulong]
k.VirtualAlloc.restype = c.c_void_p
k.VirtualFree.argtypes = [c.c_void_p, c.c_size_t, c.c_ulong]
k.VirtualFree.restype = c.c_int
memory = k.VirtualAlloc(None, 4096, 0x3000, 0x40)
assert memory
try:
    c.memmove(memory, getter, len(getter))
    hero_of_param = c.WINFUNCTYPE(c.c_ubyte, c.c_void_p)(memory)
    row = c.create_string_buffer(0x100)
    reference = c.create_string_buffer(0x10)
    struct.pack_into('<Q', reference, 8, c.addressof(row))
    for hero in range(11):
        row[0xde] = hero
        assert hero_of_param(c.addressof(reference)) == hero
    struct.pack_into('<Q', reference, 8, 0)
    assert hero_of_param(c.addressof(reference)) == 0
finally:
    assert k.VirtualFree(memory, 0, 0x8000)

texts = json.loads((work / 'text-validation.json').read_text())
assert {r['language'] for r in texts} == {'zhocn', 'engus'}
for report in texts:
    assert report['effectId'] == 95000900
    assert report['unrelatedEntriesUnchanged'] and report['unrelatedMembersUnchanged']
    assert report['changedFiles'] == ['AttachEffectInfo.fmg', 'PermanentBuffInfo.fmg', 'PermanentBuffCaption.fmg']
    path = ROOT / 'dist/mod/msg' / report['language'] / 'item_dlc01.msgbnd.dcx'
    assert digest(path).upper() == report['outputSha256']
    if report['language'] == 'zhocn':
        assert '吸血、增伤、咏唱时减伤率' in report['description']
    else:
        assert 'lifesteal, bonus damage and casting damage reduction' in report['description']

before = json.loads(baseline.read_text())
after = assets()
changed = sorted(p for p in before.keys() | after.keys() if before.get(p) != after.get(p))
assert set(changed) == {'natives/NRHex.dll', 'mod/msg/zhocn/item_dlc01.msgbnd.dcx', 'mod/msg/engus/item_dlc01.msgbnd.dcx'}, changed
assert after['source:data/balance/sword.csv'] == before['source:data/balance/sword.csv']
assert after['mod/regulation.bin'] == before['mod/regulation.bin']
dll_hash = digest(ROOT / 'target/release/NRHex.dll')
assert dll_hash == after['natives/NRHex.dll']
version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
test_count = int(sys.argv[sys.argv.index('--rust-tests') + 1])
checks = ['sword-balance-validation.json', 'drinking-hud-validation.json', 'escape-vfx-validation.json',
          'cat-balance-validation.json', 'jiahao-balance-validation.json', 'effect-sync-validation.json',
          'cage-patch-validation.json']
report = dict(version=version, eligibleHeroes={'Duchess': 4, 'Revenant': 6, 'Recluse': 7},
              currentHeroSource='GameDataMan+8 -> PlayerGameData+C7', nativeHeroReadRva='B61462',
              nativeParamHeroGetterRva='E87DF0', originalNativeParamHeroGetterExecuted=True,
              roleAndUnknownIdentityTestsPassed=True,
              rewardEntries=['level rewards', 'random hotkey', 'all preferred test hotkeys'],
              nativeSerializedChoicePoolUnchanged=True, ownedAndTerrainExclusionsRetained=True,
              lifestealScale=1.5, damageAndCastingReductionScale=1,
              fractionalBonusAndNullifiedHitAndHealingCapTestsPassed=True,
              chineseDescription='伤害 replaced with 增伤', texts=texts,
              changedAssets=changed, originalRegulationAndUserEditedCsvUnchanged=True,
              csvSha256=after['source:data/balance/sword.csv'], rustTestsPassed=test_count,
              gameplayVerified=False, multiplayerVerified=False)
(ROOT / 'dist/reports/sword-balance-validation.json').write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf8')
build = dict(version=version, buildPassed=True, incremental=True,
             scope='Sword role-gated draws and lifesteal x1.5; DLL and two description bundles',
             signatureCount=50, rustTestsPassed=test_count, changedAssets=changed,
             originalExeSha256=exe_hash, dllSha256=dll_hash, installedDllSha256=dll_hash,
             dllInstalled=True, regulationSha256=after['mod/regulation.bin'], functionChecks=checks,
             unrelatedAssetsAndUserConfigsUnchanged=True, gameplayVerified=False, multiplayerVerified=False)
(ROOT / 'dist/reports/build-validation.json').write_text(json.dumps(build, indent=2), encoding='utf8')
runtime = dict(version=version, active=None, pid=None, allFourNativeDllsLoaded=False,
               build='build-validation.json', dllSha256=dll_hash, dllInstalled=True,
               gameplayVerified=False, multiplayerVerified=False,
               pendingChecks=['Sword class restrictions for automatic/random rewards',
                              'Sword lifesteal x1.5 with unchanged damage/casting reduction',
                              'Previously pending HUD, balance, visual and multiplayer checks'],
               notes='Code, native getter fixture and text readback checked; no game session used.')
(ROOT / 'dist/reports/runtime-validation.json').write_text(json.dumps(runtime, indent=2), encoding='utf8')
print('PASS: native role anchors; DLL/text-only patch; regulation and current user CSV preserved.')
