"""Execute native buildup threshold code privately; no live game writes.

The threshold-growth helpers are stubbed. This proves the zero-buildup
failure and proc gate, not live frost damage, immunity, visuals or networking.
"""
import ctypes as c
import hashlib
import struct
import pefile
from local_paths import GAME

raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
k=c.WinDLL('kernel32')
k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint]
k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
base=k.VirtualAlloc(None,0xe83000,0x3000,0x40)
assert base
try:
 code=pe.get_data(0x42cd70,0xa2)
 assert code[:5]==bytes.fromhex('48895c2408') and code[-1]==0xc3
 c.memmove(base+0x42cd70,code,len(code))
 # Only helpers for max proc count and recomputing the next threshold.
 c.memmove(base+0xe82990,b'\xb8\x64\0\0\0\xc3',6)
 c.memmove(base+0x42e450,b'\xc3',1)
 apply=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p,c.c_uint32,c.c_int32)(base+0x42cd70)
 count=0
 for threshold in [100,250,999,2000]:
  for buildup in [0,30,threshold,99999]:
   module=c.create_string_buffer(0xc0)
   for offset in [0x20,0x3c]:struct.pack_into('<i',module,offset,threshold)
   result=apply(c.addressof(module),4,buildup)
   proc=buildup>=threshold
   assert result==int(not proc)
   assert struct.unpack_from('<I',module,0xb8)[0]==(16 if proc else 0)
   assert struct.unpack_from('<i',module,0x20)[0]==(threshold if proc else threshold-buildup)
   assert struct.unpack_from('<i',module,0x74)[0]==int(proc)
   count+=1
 print(f'PASS: {count} native frost threshold cases; zero buildup never procs, 99999 crosses tested thresholds; no game access')
finally:
 k.VirtualFree(base,0,0x8000)
