"""Execute native animation selection against separate equipment snapshots.

Parameter repositories and instance ownership use fixtures; this proves the
consumer and copy ordering, not packet delivery or a real multiplayer failure.
Run with Windows Python (or Wine) and NRHEX_GAME pointing at the audited EXE.
"""
import ctypes as c
import hashlib
import json
import struct
from pathlib import Path
import pefile
from local_paths import GAME, ROOT

raw = (GAME / 'nightreign.exe').read_bytes()
sha = hashlib.sha256(raw).hexdigest()
assert sha == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe = pefile.PE(data=raw, fast_load=True)
pe.parse_data_directories(directories=[5])
kernel = c.WinDLL('kernel32', use_last_error=True)
kernel.VirtualAlloc.argtypes = [c.c_void_p, c.c_size_t, c.c_uint, c.c_uint]
kernel.VirtualAlloc.restype = c.c_void_p
kernel.VirtualFree.argtypes = [c.c_void_p, c.c_size_t, c.c_uint]
mem = kernel.VirtualAlloc(None, pe.OPTIONAL_HEADER.SizeOfImage, 0x3000, 0x40)
assert mem
callbacks = []
def stub(rva, result, args, fn):
    callback = c.WINFUNCTYPE(result, *args)(fn)
    callbacks.append(callback)
    jump = b'\x48\xb8' + struct.pack('<Q', c.cast(callback, c.c_void_p).value) + b'\xff\xe0'
    c.memmove(mem + rva, jump, len(jump))
def put(p, off, fmt, value):
    c.memmove(p + off, struct.pack(fmt, value), struct.calcsize(fmt))
def get(p, off, fmt):
    return struct.unpack(fmt, c.string_at(p + off, struct.calcsize(fmt)))[0]
def buffer(n):
    b = c.create_string_buffer(n)
    return b, c.addressof(b)
try:
    c.memmove(mem, raw[:pe.OPTIONAL_HEADER.SizeOfHeaders], pe.OPTIONAL_HEADER.SizeOfHeaders)
    for section in pe.sections:
        c.memmove(mem + section.VirtualAddress, section.get_data(), section.SizeOfRawData)
    for block in pe.DIRECTORY_ENTRY_BASERELOC:
        for rel in block.entries:
            if rel.type == 10:
                p = c.c_uint64.from_address(mem + rel.rva)
                p.value = (p.value + mem - pe.OPTIONAL_HEADER.ImageBase) & 0xffffffffffffffff
            else:
                assert rel.type == 0
    types = {x['id']: x['animationType'] for x in json.loads((ROOT / 'dist/reports/arts-pool.json').read_text(encoding='utf8'))['skills']}
    ids = [503, 504, 1173, 1047]
    rows = {id: buffer(0x20) for id in ids}
    for id, (_, p) in rows.items():
        put(p, 8, '<H', types[id])
    weapon, wp = buffer(0x200)
    put(wp, 0x198, '<i', 503)
    def weapon_lookup(out, id):
        put(out, 0, '<i', id); put(out, 8, '<Q', wp)
        return out
    def arts_lookup(out, id):
        assert id in rows, id
        put(out, 0, '<i', id); put(out, 8, '<Q', rows[id][1])
        return out
    stub(0xea7fc0, c.c_void_p, [c.c_void_p, c.c_int], weapon_lookup)
    stub(0xea2340, c.c_void_p, [c.c_void_p, c.c_int], arts_lookup)
    stub(0xea9a60, None, [c.c_void_p, c.c_int], lambda *_: None)
    stub(0xea2370, None, [c.c_void_p], lambda *_: None)
    instances = {0x80800010 + i: buffer(0x40) for i in range(len(ids))}
    for (handle, (_, p)), id in zip(instances.items(), ids):
        put(p, 8, '<I', handle); put(p, 0x10, '<i', 110000 if id in (503, 504) else 2000000); put(p, 0x1c, '<i', id)
    def reference(out, hp):
        handle = get(hp, 0, '<I')
        put(out, 0, '<I', handle); put(out, 8, '<Q', instances[handle][1]); put(out, 16, '<i', get(instances[handle][1], 0x10, '<i'))
        return out
    stub(0x6a1d70, c.c_void_p, [c.c_void_p, c.c_void_p], reference)
    stub(0x6a1d90, None, [c.c_void_p], lambda *_: None)
    # Refcounts are fixture-owned; native 3820F0 still copies all 32 slots.
    stub(0x6c1d80, c.c_void_p, [c.c_void_p, c.c_void_p], lambda out, src: (c.memmove(out, src, 4), out)[1])
    style = [2]
    style_callback = c.WINFUNCTYPE(c.c_int, c.c_void_p)(lambda _: style[0]); callbacks.append(style_callback)
    vt, vp = buffer(0x280); actor, ap = buffer(0x700); modules, mp = buffer(0x180)
    action, actionp = buffer(0x20); artmodule, artp = buffer(0x20)
    live, lp = buffer(0x130); snapshot, sp = buffer(0x130)
    put(vp, 0x278, '<Q', c.cast(style_callback, c.c_void_p).value)
    put(vp, 0x258, '<Q', mem + 0x66cda0)
    put(ap, 0, '<Q', vp); put(ap, 0x1b8, '<Q', mp); put(ap, 0x670, '<Q', lp)
    put(mp, 0x28, '<Q', actionp); put(mp, 0x110, '<Q', artp)
    put(actionp, 8, '<Q', ap); put(artp, 8, '<Q', ap)
    animate = c.WINFUNCTYPE(c.c_int, c.c_int, c.c_int, c.c_void_p)(mem + 0x402830)
    copy = c.WINFUNCTYPE(None, c.c_void_p, c.c_void_p)(mem + 0x3820f0)
    different = c.WINFUNCTYPE(c.c_ubyte, c.c_void_p, c.c_void_p)(mem + 0x6c0c70)
    cases = []
    for old, new in [(503, 504), (1173, 1047)]:
        h0 = 0x80800010 + ids.index(old); h1 = 0x80800010 + ids.index(new)
        a, a0 = buffer(0x48); b, b0 = buffer(0x48)
        for p, id in [(a0, old), (b0, new)]:
            put(p, 0, '<i', 110000 if old == 503 else 2000000); put(p, 4, '<i', id)
        assert different(a0, b0) == 1
        for hand_style in [2, 3]:
            style[0] = hand_style
            for i in range(6):
                put(lp, 0x1c + i*4, '<I', h0); put(sp, 0x1c + i*4, '<I', h1)
            before = animate(18, 40000, ap)
            assert before == (600 + types[old])*1000000
            copy(lp, sp)
            after = animate(18, 40000, ap)
            assert after == (600 + types[new])*1000000
            cases.append(dict(oldSkill=old, newSkill=new, style=hand_style, beforeActorCopy=before, afterActorCopy=after))
    # Execute the original 675D20 guard only, ending at either native branch.
    # A tiny private harness supplies the function's saved-register frame.
    player_data, dp = buffer(0x300)
    put(ap, 0x5a0, '<Q', dp)
    put(vp, 0x130, '<Q', mem + 0x66d630)
    put(vp, 0x190, '<Q', mem + 0x66c6e0)
    harness_rva = 0x1000
    code = bytes.fromhex('53 56 41 55 41 57 48 83 EC 68 48 89 CE')
    code += b'\xe9' + struct.pack('<i', 0x675d5f - (harness_rva + len(code) + 5))
    c.memmove(mem + harness_rva, code, len(code))
    # No function calls occur between each stop point and this private return.
    epilogue = bytes.fromhex('48 83 C4 68 41 5F 41 5D 5E 5B C3')
    for stop, result in [(0x675daa, 1), (0x676083, 0)]:
        code = b'\xb8' + struct.pack('<I', result) + epilogue
        c.memmove(mem + stop, code, len(code))
    gate = c.WINFUNCTYPE(c.c_int, c.c_void_p)(mem + harness_rva)
    gates = []
    for actor_kind in [0, 1, 2, 3, 4]:
        for player_slot in [1, 2, 3, 10000]:
            put(ap, 0x70, '<i', actor_kind); put(dp, 8, '<i', player_slot)
            actual = bool(gate(ap))
            assert actual == (actor_kind in (0, 1, 2) or player_slot == 10000), (actor_kind, player_slot, actual)
            gates.append(dict(actorKind=actor_kind, playerDataId=player_slot, copiesPlayerData=actual))
    report = dict(exeSha256=sha, nativeAnimationSelection=True, nativeEquipmentCopy=True,
                  nativeActorCopyGateCases=gates, actorCopyGateExecutedAsBoundedFragment=True,

                  nativeSkillDifferenceDetected=True, cases=cases,
                  parameterRepositoriesAndInstanceResolutionMocked=True,
                  networkReceiveExecuted=False, currentModHooksExecuted=False,
                  gameplayVerified=False, multiplayerVerified=False)
    out = ROOT / 'work/arts-peer-animation/native-validation.json'
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
finally:
    assert kernel.VirtualFree(mem, 0, 0x8000)
