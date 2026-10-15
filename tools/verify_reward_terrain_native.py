"""Execute the original read-only flag getter against isolated flag storage."""
from pathlib import Path
import ctypes as c,hashlib,json,struct,pefile
from local_paths import ROOT,GAME

raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True);code=pe.get_data(0x60ce40,0xa4)
assert code[:21]==bytes.fromhex('44 8b 41 1c 44 8b da 33 d2 41 8b c3 41 f7 f0 4c 8b d1 45 33 c9') and code[-1]==0xc3
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p;k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
image=k.VirtualAlloc(None,4096,0x3000,0x40);assert image
cases=[]
def put(buf,off,fmt,value):struct.pack_into(fmt,buf,off,value)
try:
 c.memmove(image,code,len(code));get=c.WINFUNCTYPE(c.c_uint,c.c_void_p,c.c_uint)(image)
 manager=c.create_string_buffer(0x48);head=c.create_string_buffer(0x38);node=c.create_string_buffer(0x38);bits=c.create_string_buffer(125)
 put(manager,0x1c,'<I',1000);put(manager,0x20,'<I',125);put(manager,0x38,'<Q',c.addressof(head))
 put(head,8,'<Q',c.addressof(node));put(head,0x19,'<B',1)
 for off in [0,8,0x10]:put(node,off,'<Q',c.addressof(head))
 put(node,0x20,'<I',7)
 def read(flag):
  before=[v.raw for v in [manager,head,node,bits]]
  result=get(c.addressof(manager),flag)
  assert before==[v.raw for v in [manager,head,node,bits]],'Flag getter wrote storage'
  return result
 for storage in [1,2]:
  put(node,0x28,'<I',storage)
  if storage==1:put(manager,0x28,'<Q',c.addressof(bits));put(node,0x30,'<I',0)
  else:put(node,0x30,'<Q',c.addressof(bits))
  for terrain in range(7600,7606):
   c.memset(c.addressof(bits),0,125);index=terrain%1000;put(bits,index//8,'<B',1<<(7-index%8))
   actual=bool(read(7604));assert actual==(terrain==7604)
   cases.append(dict(storage=storage,activeTerrain=terrain,greatHollow=actual))
  for active in [True,False,True,False]:
   c.memset(c.addressof(bits),0,125)
   if active:put(bits,604//8,'<B',1<<(7-604%8))
   assert bool(read(7604))==active
   cases.append(dict(storage=storage,liveTransition=active))
  for flag in [6004,8604]:assert read(flag)==0;cases.append(dict(storage=storage,unallocatedFlag=flag,value=0))
 put(node,0x28,'<I',3);assert read(7604)==0
 put(node,0x28,'<I',2);put(node,0x30,'<Q',0);assert read(7604)==0
finally:k.VirtualFree(image,0,0x8000)
report=dict(nativeGetterRva='0x60ce40',managerRva='0x3c115a8',currentTerrainFlag=7604,nativeCases=len(cases)+2,cases=cases,originalNativeInstructionsExecuted=True,allReadsLeftStorageUnchanged=True,bothNativeStorageKindsVerified=True,gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
out=ROOT/'.work/cage-terrain-126/native-validation.json';out.parent.mkdir(parents=True,exist_ok=True);out.write_text(json.dumps(report,indent=2),encoding='utf8')
print(f'PASS: {report["nativeCases"]} isolated native flag reads; current Great Hollow state and both storage kinds verified without writes')
