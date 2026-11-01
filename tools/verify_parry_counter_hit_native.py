"""Isolated native guard-tag storage and self-effect submission ABI.

The effect dispatcher is a fixture; gameplay/network delivery is not exercised.
"""
import ctypes as c
import hashlib
import json
import struct
import pefile
from local_paths import ROOT, GAME

raw = (GAME / 'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest() == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe = pefile.PE(data=raw, fast_load=True)
k = c.WinDLL('kernel32')
k.VirtualAlloc.argtypes = [c.c_void_p, c.c_size_t, c.c_uint, c.c_uint]
k.VirtualAlloc.restype = c.c_void_p
k.VirtualFree.argtypes = [c.c_void_p, c.c_size_t, c.c_uint]
m = k.VirtualAlloc(None, pe.OPTIONAL_HEADER.SizeOfImage, 0x3000, 0x40)
assert m
callbacks = []
def patch(rva, code): c.memmove(m + rva, code, len(code))
def stub(rva, result, args, fn):
    callback = c.WINFUNCTYPE(result, *args)(fn)
    callbacks.append(callback)
    patch(rva, b'\x48\xb8' + struct.pack('<Q', c.cast(callback, c.c_void_p).value) + b'\xff\xe0')
try:
    for section in pe.sections:
        c.memmove(m + section.VirtualAddress, section.get_data(), section.SizeOfRawData)
    code = pe.get_data(0x40d443, 16)
    assert code == bytes.fromhex('41 8b 44 24 08 48 83 4b 58 20 89 83 4c 02 00 00')
    # Native type-0/guard branch: payload +8 -> action +24C, 32 bits intact.
    patch(0x1000, bytes.fromhex('53 41 54 48 89 cb 49 89 d4') + code + bytes.fromhex('41 5c 5b c3'))
    store = c.WINFUNCTYPE(None, c.c_void_p, c.c_void_p)(m + 0x1000)
    action = c.create_string_buffer(0x260)
    payload = c.create_string_buffer(16)
    for tag in [95002400, 96014000, 96017761, 96023999]:
        c.c_int.from_buffer(payload, 8).value = tag
        c.c_uint64.from_buffer(action, 0x58).value = 0x80
        store(action, payload)
        assert c.c_int.from_buffer(action, 0x24c).value == tag
        assert c.c_uint64.from_buffer(action, 0x58).value == 0xa0
    # Actual APPLY_SELF wrapper keeps recipient/source the same actor and
    # forwards to the ordinary native dispatcher used by other self effects.
    for site, target in [(0x3c8bbd, 0x3c80a0), (0x3c8885, 0x3db410)]:
        call = pe.get_data(site, 5)
        assert call[0] == 0xe8 and site + 5 + struct.unpack('<i', call[1:])[0] == target
    stub(0x50ee20, c.c_void_p, [c.c_void_p], lambda p: c.memset(p, 0, 16) or p)
    submitted = []
    def dispatch(actor, effect, source, flag, *args):
        submitted.append((actor, effect, source, flag))
        return 0x7b
    stub(0x3c80a0, c.c_ubyte, [c.c_void_p, c.c_int, c.c_void_p, c.c_ubyte] + [c.c_size_t] * 9, dispatch)
    apply = c.WINFUNCTYPE(c.c_ubyte, c.c_void_p, c.c_int, c.c_ubyte)(m + 0x3c8b10)
    actor = c.create_string_buffer(0x200)
    assert apply(actor, 95002406, 0) == 0x7b
    assert submitted == [(c.addressof(actor), 95002406, c.addressof(actor), 0)]
finally:
    assert k.VirtualFree(m, 0, 0x8000)
out = ROOT / '.work/parry-counter-hit/native-validation.json'
out.write_text(json.dumps(dict(nativeGuardTags=4, selfEffectAbi=True,
    ordinaryDispatcherCallsSharedHook=True, dispatcherStubbed=True,
    gameplayVerified=False, multiplayerVerified=False), indent=2) + '\n')
print('PASS: native TAE retains full guard tags; self recovery uses ordinary effect dispatch')
