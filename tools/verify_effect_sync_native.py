"""Execute original category/query predicates on synthetic nodes, never the game."""
import ctypes as c
import hashlib
import json
import struct
import tomllib
import pefile
from local_paths import GAME, ROOT

raw = (GAME / 'nightreign.exe').read_bytes()
digest = hashlib.sha256(raw).hexdigest()
assert digest == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe = pefile.PE(data=raw, fast_load=True)
code = pe.get_data(0x510b20, 0x63)
assert code[:16] == bytes.fromhex('8b4168c1e81ff6d0a8017434f6417c02')
category = pe.get_data(0x511450, 0x1a)
assert category.hex() == '4532d2450fb6d86683fa14750d44394908410f94c2410fb6c2c3'
has_code = pe.get_data(0x508df0, 0x21)
assert has_code.hex() == '488b41084885c07417f7406803000c8075053950087406488b4038ebe7b001c3c3'

k = c.WinDLL('kernel32', use_last_error=True)
k.VirtualAlloc.argtypes = [c.c_void_p, c.c_size_t, c.c_ulong, c.c_ulong]
k.VirtualAlloc.restype = c.c_void_p
k.VirtualFree.argtypes = [c.c_void_p, c.c_size_t, c.c_ulong]
k.VirtualFree.restype = c.c_int
memory = k.VirtualAlloc(None, 4096, 0x3000, 0x40)
assert memory
cases = []
try:
    for offset, fragment in [(0, code), (256, category), (512, has_code)]:
        c.memmove(memory + offset, fragment, len(fragment))
    matches = c.WINFUNCTYPE(c.c_ubyte, c.c_void_p, c.c_int)(memory)
    refresh = c.WINFUNCTYPE(c.c_ubyte, c.c_void_p, c.c_ushort, c.c_ubyte, c.c_int)(memory + 256)
    has = c.WINFUNCTYPE(c.c_ubyte, c.c_void_p, c.c_int)(memory + 512)

    def node(effect, category_id=20, flags=0):
        row = c.create_string_buffer(0x400)
        struct.pack_into('<h', row, 0x13e, category_id)
        entry = c.create_string_buffer(0x88)
        struct.pack_into('<Q', entry, 0, c.addressof(row))
        struct.pack_into('<i', entry, 8, effect)
        struct.pack_into('<I', entry, 0x68, flags)
        return row, entry

    for cat in [10, 20]:
        row, entry = node(95001203, cat)
        result = bool(matches(c.addressof(entry), 95001203))
        assert result == (cat == 20)
        assert not matches(c.addressof(entry), 95000403)
        cases.append(dict(category=cat, sameIdReused=result, differentIdReused=False))
    row, entry = node(95001203)
    assert refresh(c.addressof(entry), 20, 0, 95001203)
    assert not refresh(c.addressof(entry), 20, 0, 95000403)
    manager = c.create_string_buffer(0x80)
    struct.pack_into('<Q', manager, 8, c.addressof(entry))
    for flags in [0, 2, 0x40000, 0x80000, 0x80000000]:
        struct.pack_into('<I', entry, 0x68, flags)
        assert bool(has(c.addressof(manager), 95001203)) == (flags == 0)

    # Model native receive refreshes with original matching machine code.
    # Repeated announcements and a viewer's rebuilt replica are independent
    # of the owner's scheduler; no mocked deduplication by effect ID is used.
    replicas = []
    for announcement in range(600):
        for effect in [95000100, 95000700, 95000403, 95000701, 95001203]:
            existing = next((e for _, e in replicas if matches(c.addressof(e), effect)), None)
            if existing is None:
                replicas.append(node(effect))
            else:
                assert refresh(c.addressof(existing), 20, 0, effect)
        assert len(replicas) == 5
        if announcement == 299:
            replicas.clear()  # Viewer resurrection: sender keeps its ownership.
    assert len(replicas) == 5
finally:
    assert k.VirtualFree(memory, 0, 0x8000)

patch = json.loads((ROOT / '.work/effect-sync/parameter-patch.json').read_text())
assert len(patch['changes']) == 21
assert patch['unrelatedTablesUnchanged'] and patch['unrelatedFieldsUnchanged']
assert hashlib.sha256((ROOT / 'dist/mod/regulation.bin').read_bytes()).hexdigest().upper() == patch['outputRegulationSha256']
rows = {r['ID']: r['fields'] for r in json.loads((ROOT / '.work/effect-sync/parameters.json').read_text())['current']}
owners = [95000100,95000200,95000300,95000400,95000500,95000600,95000700,95000800,95000900,95001100,95001200,95001300,95001400,95001500,95001600]
for effect in owners:
    assert rows[effect]['spCategory'] == 20 and rows[effect]['saveCategory'] == 10 and rows[effect]['effectEndurance'] == -1
for effect in [95000403, 95000701, 95001203]:
    assert rows[effect]['spCategory'] == 20 and rows[effect]['effectEndurance'] == 2 and rows[effect]['saveCategory'] == -1
source = (ROOT / 'src/lib.rs').read_text()
assert 'clean_replica_crystal' not in source
version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
report = dict(version=version, exeSha256=digest, originalCategoryPredicatesExecuted=True,
              originalGameplayQueryExecuted=True, nativeMatchingCases=cases,
              repeatedNativeReceiveModelAnnouncements=600, viewerReplicaRebuildModelPassed=True,
              parameterReadbackPassed=True, changes=patch['changes'],
              ownerRefreshSeconds=5, readyRefreshSeconds=0.75, readyLeaseSeconds=2,
              gameplayVerified=False, multiplayerVerified=False,
              pendingChecks=['Both teammate detail list and HUD after owner and viewer revival',
                             'Repeated Escape and Ultimate cooldown transitions',
                             'Crystal one-hit consumption, 40-second cooldown and aura continuity'])
(ROOT / 'dist/effect-sync-validation.json').write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf8')
print('PASS: original native matching/query execution, 600 receive refreshes, replica rebuild and parameter scope; coop remains unverified.')
