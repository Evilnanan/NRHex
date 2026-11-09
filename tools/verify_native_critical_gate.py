"""Read-only research: execute the native throw-only damage gate in an EXE copy.

No running process is opened. Calculation/notification sinks and object services
are fixtures; this does not verify actual collision, animation, or multiplayer.
"""
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
pe.parse_data_directories(directories=[5])
assert pe.get_data(0x436638, 7) == bytes.fromhex('80 bf 21 01 00 00 02')
assert pe.get_data(0xe70700, 10) == bytes.fromhex('0f b6 40 7d 88 86 21 01 00 00')
k = c.WinDLL('kernel32', use_last_error=True)
k.VirtualAlloc.argtypes = [c.c_void_p, c.c_size_t, c.c_uint, c.c_uint]
k.VirtualAlloc.restype = c.c_void_p
k.VirtualFree.argtypes = [c.c_void_p, c.c_size_t, c.c_uint]
k.VirtualFree.restype = c.c_int
memory = k.VirtualAlloc(None, pe.OPTIONAL_HEADER.SizeOfImage, 0x3000, 0x40)
assert memory
callbacks = []
events = []


def stub(rva, result, args, fn):
    callback = c.WINFUNCTYPE(result, *args)(fn)
    callbacks.append(callback)
    address = c.cast(callback, c.c_void_p).value
    c.memmove(memory + rva, b'\x48\xb8' + struct.pack('<Q', address) + b'\xff\xe0', 12)


def put(buf, offset, fmt, value):
    struct.pack_into(fmt, buf, offset, value)


try:
    c.memmove(memory, raw[:pe.OPTIONAL_HEADER.SizeOfHeaders], pe.OPTIONAL_HEADER.SizeOfHeaders)
    for section in pe.sections:
        c.memmove(memory + section.VirtualAddress, section.get_data(), section.SizeOfRawData)
    delta = memory - pe.OPTIONAL_HEADER.ImageBase
    for block in pe.DIRECTORY_ENTRY_BASERELOC:
        for rel in block.entries:
            assert rel.type in (0, 10)
            if rel.type == 10:
                cell = c.c_uint64.from_address(memory + rel.rva)
                cell.value = (cell.value + delta) & ((1 << 64) - 1)
    actor = c.create_string_buffer(0x800)
    modules = c.create_string_buffer(0x200)
    behavior = c.create_string_buffer(0x300)
    damage_module = c.create_string_buffer(0x100)
    hit = c.create_string_buffer(0x2d0)
    put(actor, 0x1b8, '<Q', c.addressof(modules))
    put(modules, 8, '<Q', c.addressof(behavior))
    put(damage_module, 8, '<Q', c.addressof(actor))
    p = c.c_void_p
    stub(0x42c120, p, [p], lambda _: c.addressof(actor))
    stub(0x427260, c.c_ubyte, [p], lambda _: 1)  # Skip unrelated post-hit network notifications.
    stub(0x6d1b60, c.c_ubyte, [p, p], lambda *_: 0)
    stub(0x39bd20, c.c_int, [p, c.c_int], lambda *_: 0)
    stub(0x33ba30, p, [p, p], lambda out, src: (c.memmove(out, src, 0x2d0), out)[1])
    stub(0x490910, c.c_ubyte, [p], lambda _: 0)
    stub(0x438610, None, [p, p, p, c.c_ubyte, c.c_ubyte], lambda *args: events.append('reaction'))
    stub(0x43d260, None, [p, p, p, c.c_ubyte, c.c_ubyte], lambda *args: events.append('damage_dispatch'))
    stub(0x53b520, None, [p], lambda _: None)
    stub(0x26909a0, None, [p], lambda _: None)
    native = c.WINFUNCTYPE(None, p, p, p, c.c_ubyte)(memory + 0x436530)
    cases = []
    for flags in (0, 8):
        for throw_flag in (0, 1, 2):
            c.memset(c.addressof(hit), 0, len(hit))
            put(behavior, 0x58, '<Q', flags)
            put(hit, 0x121, '<B', throw_flag)
            events.clear()
            native(c.addressof(damage_module), None, c.addressof(hit), 0)
            expected = ['reaction', 'damage_dispatch'] if flags == 0 or throw_flag == 2 else []
            assert events == expected, (flags, throw_flag, events)
            cases.append(dict(behaviorFlags=flags, throwFlag=throw_flag, events=events.copy()))
finally:
    assert k.VirtualFree(memory, 0, 0x8000)

report = dict(exeSha256=digest, nativeGate='0x436530', throwFlagCopy='0xE70700',
              cases=cases, gameMemoryWritten=False, gameplayVerified=False,
              fixtures=['actor/module lookup', 'reaction and damage sinks',
                        'network/other throw status', 'hit copy/destruction', 'security cookie'])
out = ROOT / 'work/soul-critical/native-gate'
out.mkdir(parents=True, exist_ok=True)
(out / 'validation.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
print('PASS: 6 native gate cases; throw-only protection rejects flags 0/1 and accepts 2.')
