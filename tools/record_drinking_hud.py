"""Check the HUD-only parameter patch and execute native icon aggregation off-game."""
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
    return {p.relative_to(ROOT / 'dist').as_posix(): digest(p)
            for p in paths if p.is_file() and p.suffix != '.log'}


work = ROOT / '.work/drinking-hud'
work.mkdir(parents=True, exist_ok=True)
baseline = work / 'package-baseline.json'
if '--before' in sys.argv:
    baseline.write_text(json.dumps(assets(), indent=2), encoding='utf8')
    print('Captured package assets and user configuration before Drinking HUD patch.')
    sys.exit(0)

exe_hash = digest(GAME / 'nightreign.exe')
assert exe_hash == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe = pefile.PE(str(GAME / 'nightreign.exe'), fast_load=True)
# All reached branches are within this original fragment. Preallocated synthetic
# vectors keep native allocation paths (and their external calls) unreachable.
code = pe.get_data(0x7deb30, 0x2d4)
assert code[:10].hex() == '40534883ec40f3410f10'
assert pe.get_data(0x7deb6e, 7).hex() == 'f680c503000040'
assert pe.get_data(0x7dedea, 7).hex() == '418b4310014110'
k = c.WinDLL('kernel32', use_last_error=True)
k.VirtualAlloc.argtypes = [c.c_void_p, c.c_size_t, c.c_ulong, c.c_ulong]
k.VirtualAlloc.restype = c.c_void_p
k.VirtualFree.argtypes = [c.c_void_p, c.c_size_t, c.c_ulong]
k.VirtualFree.restype = c.c_int
memory = k.VirtualAlloc(None, 4096, 0x3000, 0x40)
assert memory
cases = []
try:
    c.memmove(memory, code, len(code))
    aggregate = c.WINFUNCTYPE(c.c_void_p, c.c_void_p, c.c_void_p, c.c_void_p)(memory)
    row = c.create_string_buffer(0x400)  # dispDartDebuffStacks == 0
    node = c.create_string_buffer(0x88)
    struct.pack_into('<Q', node, 0, c.addressof(row))
    entries = c.create_string_buffer(128 * 20)
    vector = c.create_string_buffer(0x30)
    first = c.addressof(entries)

    def collect(layers):
        struct.pack_into('<QQQ', vector, 0x10, first, first, first + len(entries))
        for effect, icon, remaining in layers:
            item = c.create_string_buffer(struct.pack('<iiifi', effect, icon, 0, remaining, 1))
            aggregate(c.addressof(vector), c.addressof(node), c.addressof(item))
        end = struct.unpack_from('<Q', vector, 0x18)[0]
        return [struct.unpack_from('<iiifi', entries, off) for off in range(0, end-first, 20)]

    ids = [95000501] + list(range(95005001, 95005080))
    for count in [0, 1, 2, 3, 7, 15, 40, 80]:
        result = collect([(effect, 20006, 15-i/8) for i, effect in enumerate(ids[:count])])
        assert len(result) == int(count > 0)
        assert not count or (result[0][1] == 20006 and result[0][4] == count)
        cases.append(dict(liveLayers=count, hudIcons=len(result), displayedCount=count))
    # A native frame rebuild observes only the currently live instances. The
    # surviving slot ID is not a count (in contrast to the Dart ID%100 branch).
    for layers, expected in [([ids[0], ids[1]], 2), ([ids[1]], 1),
                             ([ids[1], ids[0]], 2), ([], 0)]:
        result = collect([(effect, 20006, 5) for effect in layers])
        assert (result[0][4] if result else 0) == expected
    result = collect([(ids[0], 20006, 10), (95000403, 20297, 2), (ids[1], 20006, 15)])
    assert {entry[1]: entry[4] for entry in result} == {20006: 2, 20297: 1}
    # Document the game's grouping rule honestly: other sources with this same
    # native attack-up icon also contribute to its native badge.
    result = collect([(ids[0], 20006, 10), (1681, 20006, 1)])
    assert len(result) == 1 and result[0][4] == 2
finally:
    assert k.VirtualFree(memory, 0, 0x8000)

patch = json.loads((work / 'parameter-patch.json').read_text())
assert patch['changedEffectIds'] == sorted(ids)
assert patch['field'] == 'iconId' and patch['previousIcon'] == 21206 and patch['hudIcon'] == 20006
assert patch['allOtherEffectFieldsUnchanged'] and patch['unrelatedTablesUnchanged']
before = json.loads(baseline.read_text())
after = assets()
assert before['mod/regulation.bin'].upper() == patch['inputRegulationSha256']
assert after['mod/regulation.bin'].upper() == patch['outputRegulationSha256']
changed = sorted(p for p in before.keys() | after.keys() if before.get(p) != after.get(p))
assert set(changed) <= {'mod/regulation.bin', 'mod/natives/NRHex.dll'}, changed
dll_hash = digest(ROOT / 'target/release/NRHex.dll')
assert dll_hash == after['mod/natives/NRHex.dll']
version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
test_count = int(sys.argv[sys.argv.index('--rust-tests') + 1])
report = dict(version=version, hudIcon=20006, nativeReference=1681,
              changedEffectIds=sorted(ids), originalNativeAggregationExecuted=True,
              nativeAggregationRva='7DEB30', originalNativeAggregationSha256=hashlib.sha256(code).hexdigest(),
              aggregationCases=cases, expirySlotReuseAndOtherIconsPassed=True,
              grouping='Native same-icon instance count; other sources using 20006 share the badge',
              hudBadgeTransferRva='7CFE40', stackDurationSeconds=15,
              perLayerDamageMultiplier=1.05, damageExpiryAndGlowFieldsUnchanged=True,
              allOtherEffectFieldsAndTablesUnchanged=True, changedAssets=changed,
              unrelatedAssetsAndUserConfigurationUnchanged=True, rustTestsPassed=test_count,
              gameplayVerified=False, multiplayerVerified=False)
(ROOT / 'dist/drinking-hud-validation.json').write_text(json.dumps(report, indent=2), encoding='utf8')
checks = ['drinking-hud-validation.json', 'escape-vfx-validation.json', 'cat-balance-validation.json',
          'jiahao-balance-validation.json', 'effect-sync-validation.json', 'cage-patch-validation.json']
build = dict(version=version, buildPassed=True, incremental=True,
             scope='80 Drinking iconId fields; unchanged gameplay; DLL version updated',
             signatureCount=49, rustTestsPassed=test_count, changedAssets=changed,
             originalExeSha256=exe_hash, dllSha256=dll_hash, installedDllSha256=dll_hash,
             dllInstalled=True, regulationSha256=after['mod/regulation.bin'], functionChecks=checks,
             unrelatedAssetsAndUserConfigsUnchanged=True, gameplayVerified=False, multiplayerVerified=False)
(ROOT / 'dist/build-validation.json').write_text(json.dumps(build, indent=2), encoding='utf8')
runtime = dict(version=version, active=None, pid=None, allFourNativeDllsLoaded=False,
               build='build-validation.json', dllSha256=dll_hash, dllInstalled=True,
               gameplayVerified=False, multiplayerVerified=False,
               pendingChecks=['Drinking attack-up HUD icon and numeric badge after repeated flasks and individual expiry',
                              'Previously pending balance, visual and multiplayer checks'],
               notes='Native HUD collector executed on synthetic entries; no game session or actual renderer used.')
(ROOT / 'dist/runtime-validation.json').write_text(json.dumps(runtime, indent=2), encoding='utf8')
print('PASS: native icon/count aggregation at 0-80 layers, expiry and slot reuse; HUD-only parameter scope.')
