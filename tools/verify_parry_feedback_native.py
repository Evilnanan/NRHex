"""Execute native FFX descriptor/instance selection in a private PE mapping.

No game process is opened. Allocation and rendering are controlled stubs;
the original descriptor conversion and start/reuse branches execute unchanged,
including the relocated out-of-line branch at 533571B.
"""
import ctypes as c
import hashlib
import json
import struct
from pathlib import Path

import pefile
from local_paths import ROOT, GAME

raw = (GAME / 'nightreign.exe').read_bytes()
digest = hashlib.sha256(raw).hexdigest()
assert digest == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
p = pefile.PE(data=raw, fast_load=True)
p.parse_data_directories(directories=[5])
k = c.WinDLL('kernel32')
k.VirtualAlloc.argtypes = [c.c_void_p, c.c_size_t, c.c_uint, c.c_uint]
k.VirtualAlloc.restype = c.c_void_p
k.VirtualFree.argtypes = [c.c_void_p, c.c_size_t, c.c_uint]
m = k.VirtualAlloc(None, p.OPTIONAL_HEADER.SizeOfImage, 0x3000, 0x40)
assert m
cases = []
callbacks = []

def put(obj, off, fmt, value):
    value = struct.pack(fmt, value)
    c.memmove(c.addressof(obj) + off, value, len(value))

def patch_callback(rva, signature, fn):
    cb = signature(fn)
    callbacks.append(cb)
    jump = b'\x48\xb8' + struct.pack('<Q', c.cast(cb, c.c_void_p).value) + b'\xff\xe0'
    c.memmove(m + rva, jump, len(jump))

try:
    c.memmove(m, raw[:p.OPTIONAL_HEADER.SizeOfHeaders], p.OPTIONAL_HEADER.SizeOfHeaders)
    for s in p.sections:
        c.memmove(m + s.VirtualAddress, s.get_data(), s.SizeOfRawData)
    for block in p.DIRECTORY_ENTRY_BASERELOC:
        for r in block.entries:
            if r.type == 10:
                q = c.c_uint64.from_address(m + r.rva)
                q.value = (q.value + m - p.OPTIONAL_HEADER.ImageBase) & 0xffffffffffffffff
            else:
                assert r.type == 0

    # Original start edge branches on converted flag +0C, then either
    # allocates a new one-shot entry or searches by TAE parameter address.
    assert p.get_data(0x533571b, 4) == bytes.fromhex('80 7b 0c 00')
    assert p.get_data(0x4b3284, 4) == bytes.fromhex('4c 3b 40 08')
    assert p.get_data(0x4b32b4, 4) == bytes.fromhex('c6 42 19 01')

    ptr = c.c_void_p
    actor = c.create_string_buffer(0x1c0)
    modules = c.create_string_buffer(0x90)
    vtable = c.create_string_buffer(0x180)
    put(actor, 0, '<Q', c.addressof(vtable))
    put(actor, 0x1b8, '<Q', c.addressof(modules))
    # Actor classification/cosmetic predicates are irrelevant to reuse.
    patch_callback(0x3d4230, c.WINFUNCTYPE(c.c_ubyte, ptr), lambda _: 0)
    classify = c.WINFUNCTYPE(c.c_ubyte, ptr)(lambda _: 0)
    callbacks.append(classify)
    put(vtable, 0x178, '<Q', c.cast(classify, ptr).value)
    convert = c.WINFUNCTYPE(ptr, ptr, ptr, ptr)(m + 0x4c1a80)
    select = c.WINFUNCTYPE(None, ptr, ptr, ptr, c.c_ubyte)(m + 0x4b3250)

    manager = c.create_string_buffer(0x40)
    entries = c.create_string_buffer(24 * 32)
    nodes, configured, errors = [], [], []

    def allocate(mgr, key):
        if mgr != c.addressof(manager) or len(nodes) >= 32:
            errors.append('unexpected allocation')
            return 0
        node = c.create_string_buffer(0x50)
        index = len(nodes)
        put(node, 8, '<i', 30000 + index)
        nodes.append(node)
        put(entries, index * 24, '<i', 30000 + index)
        put(entries, index * 24 + 8, '<Q', key)
        put(entries, index * 24 + 16, '<Q', c.addressof(node))
        put(manager, 0x28, '<Q', c.addressof(entries) + len(nodes) * 24)
        return c.addressof(node)

    def configure(node, descriptor):
        configured.append((node, c.string_at(descriptor, 24)))

    patch_callback(0x4b0820, c.WINFUNCTYPE(ptr, ptr, ptr), allocate)
    patch_callback(0x4b0d70, c.WINFUNCTYPE(ptr, ptr, c.c_int),
                   lambda *_: errors.append('unexpected fixed slot') or 0)
    patch_callback(0x4c1e60, c.WINFUNCTYPE(None, ptr, ptr), configure)

    for dummy in [20, 21]:
        for native_flag in [1, 0]:
            nodes.clear()
            configured.clear()
            c.memset(c.addressof(entries), 0, len(entries))
            put(manager, 0x20, '<Q', c.addressof(entries))
            put(manager, 0x28, '<Q', c.addressof(entries))
            params = c.create_string_buffer(bytes.fromhex(
                'D00F0700 F6130000 FFFFFFFF 00010100 00000000 00000000 00000000 00000000'))
            put(params, 4, '<i', dummy)
            put(params, 14, '<B', native_flag)
            descriptor = c.create_string_buffer(24)
            assert convert(c.addressof(descriptor), c.addressof(params), c.addressof(actor)) == c.addressof(descriptor)
            assert struct.unpack_from('<3i', descriptor.raw) == (-1, 462800, dummy)
            assert descriptor.raw[12] == 1 - native_flag
            starts = []
            for _ in range(6):
                before = len(nodes)
                select(c.addressof(manager), c.addressof(descriptor), c.addressof(params), 1)
                starts.append(configured[-1][0])
                assert len(nodes) == before + int(native_flag == 0 or before == 0)
                # Event remaining active must not allocate more instances.
                before = len(nodes)
                for _ in range(4):
                    select(c.addressof(manager), c.addressof(descriptor), c.addressof(params), 0)
                assert len(nodes) == before
            wanted = 1 if native_flag else 6
            assert len(nodes) == len(set(starts)) == wanted
            assert len(configured) == 30 and not errors
            cases.append(dict(dummy=dummy,taeByte14=native_flag,starts=6,
                              heldUpdates=24,allocatedInstances=len(nodes)))
finally:
    assert k.VirtualFree(m, 0, 0x8000)

report = dict(exeSha256=digest, cases=cases, nativeBranchExecuted=True,
              allocatorAndRendererStubbed=True, gameMemoryWritten=False,
              gameplayVerified=False, multiplayerVerified=False)
output = ROOT / '.work/parry-fix/spark-native-validation.json'
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps(report, indent=2), encoding='utf8')
print('PASS: native left/right spark starts allocate 6 instances; old flag only 1; held updates allocate none')
