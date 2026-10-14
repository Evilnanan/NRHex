"""Execute untouched native rotation setter/getters in private memory, never the game."""
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
def put(obj, off, fmt, value):
    value=struct.pack(fmt,value)
    c.memmove(c.addressof(obj)+off,value,len(value))

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


    physics=c.create_string_buffer(0x300)
    actor=c.create_string_buffer(0x1c0); coord=c.create_string_buffer(0x30); modules=c.create_string_buffer(0x70)
    put(actor,0x60,'<Q',c.addressof(coord));put(actor,0x1b8,'<Q',c.addressof(modules))
    put(coord,0x10,'<Q',c.addressof(actor));put(modules,0x68,'<Q',c.addressof(physics))
    setter=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p)(m+0x467c90)
    forward=c.WINFUNCTYPE(c.c_void_p,c.c_void_p,c.c_void_p)(m+0x3cfc20)
    import math
    assert p.get_data(0x467c90,16)==bytes.fromhex('48 89 5c 24 10 57 48 83 ec 60 f3 0f 10 0a 48 8b')
    # Native source type check and the resolved source passed to 439E80.
    assert p.get_data(0x43d4be,5)==bytes.fromhex('e8 bd c9 ff ff')
    getter=c.WINFUNCTYPE(c.c_void_p,c.c_void_p,c.c_void_p)(m+0x3ceb20)
    for x,z in [(0,1),(1,0),(-1,0),(0,-1),(2,3),(-3,2),(2,-3),(-2,-3)]:
        yaw=math.atan2(-x,-z)
        rotation=(c.c_float*4)(0,yaw,0,0);out=(c.c_float*4)()
        before=physics.raw
        setter(c.addressof(physics),c.addressof(rotation))
        assert physics.raw[:0x50]==before[:0x50] and physics.raw[0x70:]==before[0x70:]
        forward(c.addressof(actor),c.addressof(out))
        norm=math.hypot(x,z)
        assert abs(out[0]-x/norm)<0.00001 and abs(out[2]-z/norm)<0.00001 and abs(out[1])<0.00001
        angles=(c.c_float*4)();getter(c.addressof(actor),c.addressof(angles))
        assert abs(math.sin(angles[1])-math.sin(yaw))<0.00001
        cases.append(dict(target=[x,z],yaw=yaw,nativeForward=list(out),nativeEuler=list(angles)))
finally:
    assert k.VirtualFree(m,0,0x8000)

report=dict(exeSha256=digest,nativeSetterAndGettersExecuted=True,positionAndOtherPhysicsFieldsUnchanged=True,cases=cases,gameMemoryWritten=False,gameplayVerified=False,multiplayerVerified=False)
(ROOT/'.work/parry-fix/turn-native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print('PASS: 8 native turning directions, Euler roundtrip, position/other physics fields unchanged; no game writes')
