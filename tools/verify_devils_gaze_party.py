"""Execute original expedition party lookup in an isolated PE copy."""
import ctypes as c
import hashlib,json,struct,sys
from pathlib import Path
import pefile
from local_paths import GAME,ROOT

raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True);pe.parse_data_directories(directories=[5])
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
memory=k.VirtualAlloc(None,pe.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert memory
callbacks=[]
def put(b,off,fmt,value):struct.pack_into(fmt,b,off,value)
def stub(rva,result,args,fn):
 f=c.WINFUNCTYPE(result,*args)(fn);callbacks.append(f)
 c.memmove(memory+rva,b'\x48\xb8'+struct.pack('<Q',c.cast(f,c.c_void_p).value)+b'\xff\xe0',12)
try:
 c.memmove(memory,raw[:pe.OPTIONAL_HEADER.SizeOfHeaders],pe.OPTIONAL_HEADER.SizeOfHeaders)
 for s in pe.sections:c.memmove(memory+s.VirtualAddress,s.get_data(),s.SizeOfRawData)
 for block in pe.DIRECTORY_ENTRY_BASERELOC:
  for rel in block.entries:
   if rel.type==10:
    p=c.c_uint64.from_address(memory+rel.rva);p.value=(p.value+memory-pe.OPTIONAL_HEADER.ImageBase)&0xffffffffffffffff
   else:assert rel.type==0
 # Execute the original registry and slot getter; no engine callback stubs.
 world=c.create_string_buffer(0x9a60);entries=(c.c_uint64*6)()
 vt=(c.c_uint64*3)(memory+0x4ac780,0,memory+0x4ac770)
 actors=[c.create_string_buffer(0x5a8) for _ in range(3)]
 data=[c.create_string_buffer(16) for _ in range(3)]
 for i in range(3):
  put(data[i],8,'<i',i+1);put(actors[i],0x5a0,'<Q',c.addressof(data[i]));entries[2*i]=c.addressof(actors[i])
 put(world,0x9a40,'<Q',c.addressof(vt));put(world,0x9a50,'<i',3);put(world,0x9a58,'<Q',c.addressof(entries))
 get=c.WINFUNCTYPE(c.c_void_p,c.c_void_p,c.c_int)(memory+0x5197e0)
 cases=[]
 for slot in [0,1,2,3,4]:
  result=get(c.addressof(world),slot) or 0
  assert result==(c.addressof(actors[slot-1]) if 1<=slot<=3 else 0)
  cases.append(dict(slot=slot,found=bool(result)))
 entries[2]=0
 assert not get(c.addressof(world),2)
 entries[2]=c.addressof(actors[1])
 assert get(c.addressof(world),2)==c.addressof(actors[1])
 report=dict(exeSha256=hashlib.sha256(raw).hexdigest(),nativeRegistry='0x5197e0',nativeSlotGetter='0x66c6f0',nativeCases=7,cases=cases,emptyAndRejoinedSlotPassed=True,engineCallbacksStubbed=False,gameMemoryWritten=False,gameplayVerified=False,multiplayerVerified=False)
 (ROOT/'.work/devils-gaze-faction/native-validation.json').write_text(json.dumps(report,indent=2))
 print('PASS: original party registry and slot getter; 7 cases including absent and rejoined slots')
finally:k.VirtualFree(memory,0,0x8000)
