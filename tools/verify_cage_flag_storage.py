"""Execute the exact native flag setter in fake storage; never opens the game."""
from pathlib import Path
import ctypes as c,hashlib,json,struct,pefile
from local_paths import GAME
root=Path(__file__).resolve().parents[1]
registrations=json.loads((root/'data/reference/world/map-flag-storage.json').read_text())
assert registrations['flagGroupSize']==1000
assert all(6 not in categories and 7 in categories for kind in ['legacyMapGroups','openMapGroups'] for categories in registrations[kind].values())
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
code=raw[pe.get_offset_from_rva(0x60d330):][:0xb3]
assert code[:9]==bytes.fromhex('48 89 5C 24 08 44 8B 49 1C') and code[-1]==0xc3
k=c.WinDLL('kernel32',use_last_error=True)
k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
k.FlushInstructionCache.argtypes=[c.c_void_p,c.c_void_p,c.c_size_t]
address=k.VirtualAlloc(None,4096,0x3000,0x40);assert address
try:
    c.memmove(address,code,len(code));assert k.FlushInstructionCache(c.c_void_p(-1),address,4096)
    manager=c.create_string_buffer(0x80);head=c.create_string_buffer(0x38);node=c.create_string_buffer(0x38);bits=c.create_string_buffer(125)
    struct.pack_into('<I',manager,0x1c,1000)
    struct.pack_into('<Q',manager,0x38,c.addressof(head))
    struct.pack_into('<Q',head,8,c.addressof(node));head[0x19]=b'\x01'
    struct.pack_into('<3Q',node,0,c.addressof(head),c.addressof(head),c.addressof(head))
    struct.pack_into('<I',node,0x20,46507);struct.pack_into('<I',node,0x28,2);struct.pack_into('<Q',node,0x30,c.addressof(bits))
    setter=c.WINFUNCTYPE(None,c.c_void_p,c.c_uint,c.c_uint)(address)
    def flag(id):return bool(bits.raw[(id%1000)//8]&(1<<(7-id%8)))
    before=bits.raw
    setter(c.addressof(manager),46506400,1)
    assert bits.raw==before,'Missing category must silently discard the old request'
    for id in [46507400,46507401,46507402,46507403,46507404,46507500,46507536,46507550,46507586,46507650,46507686,46507700,46507736]:
        setter(c.addressof(manager),id,1);assert flag(id)
        setter(c.addressof(manager),id,0);assert not flag(id)
    assert bits.raw==before
finally:k.VirtualFree(address,0,0x8000)
(root/'.work/cage/flag-storage-test.json').write_text(json.dumps(dict(nativeSetterRva='0x60d330',isolatedExactNativeCode=True,old6000RequestSilentlyIgnored=True,new7000StatesSetAndClear=True,gameProcessAccessed=False),indent=2),encoding='utf8')
print('PASS: exact native setter discards unallocated 6000-group flags; cage state/choice/cleanup/mutation flags set and clear in isolated storage.')
