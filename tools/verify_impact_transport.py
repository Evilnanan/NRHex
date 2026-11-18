"""Execute original impact pack/rebuild fragments in a private process image.

No game process access. This verifies one native hit-message path, not live
transport delivery, actor authority or final animation selection.
"""
from pathlib import Path
import ctypes as c
import hashlib
import json
import struct
import pefile
from local_paths import GAME, ROOT

raw = (GAME / 'nightreign.exe').read_bytes()
digest = hashlib.sha256(raw).hexdigest()
assert digest == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe = pefile.PE(data=raw, fast_load=True)
kernel = c.WinDLL('kernel32', use_last_error=True)
kernel.VirtualAlloc.argtypes = [c.c_void_p, c.c_size_t, c.c_ulong, c.c_ulong]
kernel.VirtualAlloc.restype = c.c_void_p
kernel.VirtualFree.argtypes = [c.c_void_p, c.c_size_t, c.c_ulong]
base = kernel.VirtualAlloc(None, pe.OPTIONAL_HEADER.SizeOfImage, 0x3000, 0x40)
arena = kernel.VirtualAlloc(None, 0x1000, 0x3000, 0x40)
assert base and arena

def original(rva, count):
    return pe.get_data(rva, count)

def check(rva, hex_bytes):
    value = bytes.fromhex(hex_bytes)
    assert original(rva, len(value)) == value, hex(rva)
    return value

def jump(address):
    return b'\x48\xb8' + struct.pack('<Q', address) + b'\xff\xe0'

try:
    for section in pe.sections:
        c.memmove(base + section.VirtualAddress,
                  raw[section.PointerToRawData:section.PointerToRawData + section.SizeOfRawData],
                  section.SizeOfRawData)
    # 435780 writes the explicit override to hit message +8C. It does not
    # serialize the local resolved impact at hit+24.
    pack_bytes = check(0x4358d2, '41 0f b6 40 1e 88 87 8c 00 00 00')
    pack_code = bytes.fromhex('57 49 89 c8 48 89 d7') + pack_bytes + bytes.fromhex('5f c3')
    c.memmove(arena, pack_code, len(pack_code))
    pack = c.WINFUNCTYPE(None, c.c_void_p, c.c_void_p)(arena)
    # Native 434FA0: RBP = entry RSP-2B8, context = current RSP+40 =
    # entry RSP-378. Thus RBP+8 is context+C8.
    decode_bytes = check(0x4350e6, '0f b6 83 8c 00 00 00 88 45 08')
    decode_code = bytes.fromhex('53 55 48 89 cb 48 8d aa c0 00 00 00') + decode_bytes + bytes.fromhex('5d 5b c3')
    c.memmove(arena + 0x100, decode_code, len(decode_code))
    decode = c.WINFUNCTYPE(None, c.c_void_p, c.c_void_p)(arena + 0x100)
    check(0xe709bd, '41 0f b6 8c 24 c8 00 00 00 84 c9 75 14')
    check(0xe709e8, '88 46 24 41 0f b6 84 24 c8 00 00 00 88 46 1e')
    # Execute the original constructor's entire override/default branch,
    # including its real E6D510 lookup. Other constructor work is not run.
    prologue = bytes.fromhex('41 54 56 48 83 ec 58 48 89 ce 49 89 d4 4c 89 44 24 48')
    prologue += jump(base + 0xe709bd)
    c.memmove(arena + 0x200, prologue, len(prologue))
    epilogue = bytes.fromhex('48 83 c4 58 5e 41 5c c3')
    c.memmove(base + 0xe709f7, epilogue, len(epilogue))
    rebuild = c.WINFUNCTYPE(None, c.c_void_p, c.c_void_p, c.c_void_p)(arena + 0x200)
    lookup = c.WINFUNCTYPE(c.c_int, c.c_ubyte)(base + 0xe6d510)
    mapping = {i: lookup(i) for i in range(1, 13)}
    cases = []
    for name, native, wanted in [('mighty0', 0, 1), ('mighty1', 1, 2),
                                  ('mighty2', 2, 3), ('mighty3', 3, 7),
                                  ('mighty8', 8, 2), ('slideGround', 1, 9),
                                  ('slideAir', 1, 3)]:
        override = next(i for i, level in mapping.items() if level == wanted)
        outcomes = []
        for wire_override in [0, override]:
            hit = (c.c_ubyte * 0x2c8)()
            hit[0x24], hit[0x1e] = wanted, wire_override
            message = (c.c_ubyte * 0x150)()
            context = (c.c_ubyte * 0x110)()
            row = (c.c_ubyte * 0x200)()
            row[0x72] = native
            rebuilt = (c.c_ubyte * 0x2c8)()
            pack(hit, message)
            decode(message, context)
            rebuild(rebuilt, context, row)
            assert message[0x8c] == context[0xc8] == wire_override
            assert rebuilt[0x24] == (native if wire_override == 0 else wanted)
            assert rebuilt[0x1e] == wire_override
            outcomes.append(dict(override=wire_override, receiverImpact=rebuilt[0x24]))
        cases.append(dict(name=name, nativeImpact=native, senderImpact=wanted, outcomes=outcomes))
    report = dict(exeSha256=digest, overrideMapping=mapping, cases=cases,
                  nativeFragmentCases=14, gameProcessAccessed=False,
                  liveMultiplayerVerified=False,
                  path=['435780 hit+1E to message+8C', 'DDFB20 sends type 0F, size 150',
                        '434FA0 message+8C to context+C8', 'E70350 override/default reconstruction'])
    output = ROOT / 'work/impact-transport'
    output.mkdir(parents=True, exist_ok=True)
    (output / 'native-roundtrip.json').write_text(json.dumps(report, indent=2), encoding='utf8')
    print(json.dumps(report))
finally:
    kernel.VirtualFree(arena, 0, 0x8000)
    kernel.VirtualFree(base, 0, 0x8000)
