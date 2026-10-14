"""Exercise actual model routing and dummy-count ABI without opening the game.

The model's dummy table is a controlled fixture; count lookup and model table
accessors are stubbed. Player dispatch, model selection, ID splitting and the
native count wrapper execute unchanged from the current EXE.
"""
import ctypes as c
import hashlib
import json
import struct
import pefile
from local_paths import ROOT, GAME

raw = (GAME / 'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest() == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
p = pefile.PE(data=raw, fast_load=True)
p.parse_data_directories(directories=[5])
k = c.WinDLL('kernel32')
k.VirtualAlloc.argtypes = [c.c_void_p, c.c_size_t, c.c_uint, c.c_uint]
k.VirtualAlloc.restype = c.c_void_p
k.VirtualFree.argtypes = [c.c_void_p, c.c_size_t, c.c_uint]
m = k.VirtualAlloc(None, p.OPTIONAL_HEADER.SizeOfImage, 0x3000, 0x40)
assert m
ptr = c.c_void_p
callbacks, cases, calls, errors = [], [], [], []

def put(obj, off, fmt, v):
    b = struct.pack(fmt, v)
    c.memmove(c.addressof(obj) + off, b, len(b))

try:
    for s in p.sections:
        c.memmove(m + s.VirtualAddress, s.get_data(), s.SizeOfRawData)
    for block in p.DIRECTORY_ENTRY_BASERELOC:
        for r in block.entries:
            if r.type == 10:
                q = c.c_uint64.from_address(m + r.rva)
                q.value = (q.value + m - p.OPTIONAL_HEADER.ImageBase) & ((1 << 64) - 1)
            else:
                assert r.type == 0
    for slot, rva in [(0x4a0, 0x66c400), (0x4d0, 0x3c9660)]:
        assert p.get_data(0x2c307f8 + slot, 8) == struct.pack('<Q', p.OPTIONAL_HEADER.ImageBase + rva)
    actor = c.create_string_buffer(0x688)
    vt = c.create_string_buffer(0x4d8)
    models = c.create_string_buffer(0x3c0)
    model_vt = c.create_string_buffer(0x80)
    model_objs = [c.create_string_buffer(16) for _ in range(3)]
    banks = [c.c_int(1), c.c_int(2), c.c_int(3)]

    def bank(model):
        return c.c_void_p.from_address(model + 8).value

    get_bank = c.WINFUNCTYPE(ptr, ptr)(bank)
    callbacks.append(get_bank)
    put(model_vt, 0x78, '<Q', c.cast(get_bank, ptr).value)
    for slot, obj, table in zip([7, 11, 15], model_objs, banks):
        put(obj, 0, '<Q', c.addressof(model_vt))
        put(obj, 8, '<Q', c.addressof(table))
        put(models, 0x120 + slot * 8, '<Q', c.addressof(obj))
    put(actor, 0, '<Q', c.addressof(vt))
    put(actor, 0x680, '<Q', c.addressof(models))
    put(vt, 0x4a0, '<Q', m + 0x66c400)

    def count(bank_ptr, output, first, last, limit):
        lo, hi = c.c_int.from_address(first).value, c.c_int.from_address(last).value
        if lo != 110 or hi != 110 or limit != 0x7fffffff:
            errors.append((lo, hi, limit))
        value = c.c_int.from_address(bank_ptr).value
        c.c_int.from_address(output + 8).value = value
        calls.append(bank_ptr)
        return output

    callback = c.WINFUNCTYPE(ptr, ptr, ptr, ptr, ptr, c.c_int)(count)
    callbacks.append(callback)
    jump = b'\x48\xb8' + struct.pack('<Q', c.cast(callback, ptr).value) + b'\xff\xe0'
    c.memmove(m + 0xcac980, jump, len(jump))
    query = c.WINFUNCTYPE(c.c_int, ptr, c.POINTER(c.c_int), c.POINTER(c.c_int))(m + 0x3c9660)
    for style in [0, 1, 2, 3]:
        put(models, 0x3b8, '<i', style)
        for dummy, index in [(20110, 0), (10110, 0 if style == 2 else 1), (5110, 2)]:
            d = c.c_int(dummy)
            for available in [True, False]:
                bank_obj = banks[index]
                old = bank_obj.value
                if not available:
                    bank_obj.value = 0
                calls.clear()
                actual = query(c.addressof(actor), c.byref(d), c.byref(d))
                assert actual == bank_obj.value and calls == [c.addressof(bank_obj)] and not errors
                cases.append(dict(style=style, dummy=dummy, pointPresent=available, count=actual))
                bank_obj.value = old
    # An unloaded weapon must return zero; do not mistake body dummy 110 for it.
    for dummy, slot in [(20110, 7), (10110, 11)]:
        put(models, 0x3b8, '<i', 3)
        put(models, 0x120 + slot * 8, '<Q', 0)
        d = c.c_int(dummy)
        calls.clear()
        assert query(c.addressof(actor), c.byref(d), c.byref(d)) == 0 and not calls
        cases.append(dict(dummy=dummy, unloadedModel=True, count=0))
finally:
    assert k.VirtualFree(m, 0, 0x8000)

report = dict(nativeCases=len(cases), cases=cases, actualModelRoutingExecuted=True,
              dummyTableStubbed=True, gameMemoryWritten=False, gameplayVerified=False)
out = ROOT / '.work/parry-attachment/native-validation.json'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(report, indent=2), encoding='utf8')
print(f'PASS: {len(cases)} native attachment cases; model routing, point absence and unloaded models')
