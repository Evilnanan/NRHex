"""Run the real local popup enqueue function in an isolated, preallocated deque.

No game process is opened; UI rendering and multiplayer remain in-game checks.
"""
import ctypes as c
import hashlib
import json
import struct
from pathlib import Path
import pefile
from local_paths import ROOT, GAME

raw = (GAME / 'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest() == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe = pefile.PE(data=raw, fast_load=True)
def read(rva, size):
    off = pe.get_offset_from_rva(rva)
    return raw[off:off+size]
def target(rva):
    assert read(rva, 1) == b'\xe8'
    return rva + 5 + struct.unpack('<i', read(rva+1, 4))[0]

# Same API as ordinary latent-power pickup, with its presentation flag = 0.
assert read(0x57a233, 3) == bytes.fromhex('45 33 c0')
assert target(0x57a239) == 0x7c5080
assert read(0x59011e, 3) == bytes.fromhex('41 b0 01')
assert target(0x590123) == 0x7c5080
assert struct.unpack('<I', read(0x5901e0 + (124-3)*4, 4))[0] == 0x5900fb
# Both callers use the same native menu singleton.
assert 0x57a22e + struct.unpack('<i', read(0x57a22a, 4))[0] == 0x3c14a70
assert read(0x7c508c, 3) == bytes.fromhex('48 8b 05')
assert 0x7c5093 + struct.unpack('<i', read(0x7c508f, 4))[0] == 0x3c078d0

k = c.WinDLL('kernel32', use_last_error=True)
k.VirtualAlloc.argtypes = [c.c_void_p, c.c_size_t, c.c_uint, c.c_uint]
k.VirtualAlloc.restype = c.c_void_p
k.VirtualFree.argtypes = [c.c_void_p, c.c_size_t, c.c_uint]
k.FlushInstructionCache.argtypes = [c.c_void_p, c.c_void_p, c.c_size_t]
memory = k.VirtualAlloc(None, 0x4000, 0x3000, 0x40)
assert memory
try:
    code = bytearray(read(0x7c5080, 0x25b))
    # Relocate only the GameDataMan RIP load. ID > 200 bypasses its record
    # calls; preallocated pages bypass allocator/growth/error branches.
    struct.pack_into('<i', code, 0xf, 0x400-0x13)
    c.memmove(memory, bytes(code), len(code))
    c.c_uint64.from_address(memory+0x400).value = memory+0x500
    assert k.FlushInstructionCache(c.c_void_p(-1), memory, 0x4000)
    queue = c.WINFUNCTYPE(None, c.c_void_p, c.c_int, c.c_ubyte)(memory)
    menu = c.create_string_buffer(0xd920)
    address = c.addressof(menu)
    pages = [c.create_string_buffer(16) for _ in range(16)]
    table = (c.c_uint64 * 16)(*[c.addressof(page) for page in pages])
    def put(off, value):
        struct.pack_into('<Q', menu, 0xd8d8+off, value)
    def count():
        return struct.unpack_from('<Q', menu, 0xd8d8+0x28)[0]
    cases = []
    for offset in [0, 1, 3, 4, 15, 60, 63]:
        put(0x10, c.addressof(table)); put(0x18, 16); put(0x20, offset); put(0x28, 0)
        ids = [95000300, 95004600]
        for identity in ids:
            queue(address, identity, 1)
        assert count() == 0, 'Legacy record-only path unexpectedly enqueued'
        for identity in ids:
            queue(address, identity, 0)
        assert count() == 2
        actual = [struct.unpack_from('<i', pages[((offset+i)//4) & 15], ((offset+i) & 3)*4)[0] for i in range(2)]
        assert actual == ids, (offset, actual)
        cases.append(dict(offset=offset, legacyQueued=0, pickupQueued=actual))
finally:
    assert k.VirtualFree(memory, 0, 0x8000)

report = dict(exeSha256=hashlib.sha256(raw).hexdigest(), nativeFunction='0x7c5080',
              nativePickupFlag=0, legacyEventFlag=1, realNativeDequeCases=cases,
              gameProcessOpened=False, renderingVerified=False, multiplayerVerified=False)
output = ROOT/'work/double-dip-popup/native-validation.json'
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps(report, indent=2)+'\n', encoding='utf8')
print('PASS: actual native queue keeps both gifts in order; legacy opcode enqueues neither; seven deque offsets including wraparound')
