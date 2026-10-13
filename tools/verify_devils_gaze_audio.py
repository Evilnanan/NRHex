"""Check Libra event ownership and execute native character-bank request ABI.

Runs in a private PE copy; formatter/allocation/request endpoints use fixtures.
No game process memory is changed and no audio bank is copied into the mod.
"""
import ctypes as c
import hashlib,json,struct
from pathlib import Path
import pefile
from local_paths import ROOT,GAME

def fnv(name):
 h=2166136261
 for b in name.lower().encode('ascii'):h=((h*16777619)^b)&0xffffffff
 return h
def events(path):
 data=path.read_bytes();offset=0;result=set()
 while offset<len(data):
  kind=data[offset:offset+4];size=struct.unpack_from('<I',data,offset+4)[0]
  body=data[offset+8:offset+8+size];offset+=8+size
  assert len(body)==size
  if kind==b'HIRC':
   pos=4
   for _ in range(struct.unpack_from('<I',body)[0]):
    typ=body[pos];length,ident=struct.unpack_from('<II',body,pos+1)
    if typ==4:result.add(ident)
    pos+=5+length
   assert pos==len(body)
 assert offset==len(data)
 return result

bank=GAME/'sd/enus/cs_c7560.bnk';ids=[675683,675685,675687]
native=events(bank)
for n in ids:
 assert fnv(f'Play_s{n:09}') in native
 assert all(fnv(f'Play_s{n:09}') not in events(GAME/'sd'/name) for name in ['cs_main.bnk','cs_smain.bnk'])
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True);pe.parse_data_directories(directories=[5])
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p;k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
memory=k.VirtualAlloc(None,pe.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert memory
callbacks=[];calls=[];models=[];frees=[]
def put(b,off,fmt,value):struct.pack_into(fmt,b,off,value)
def callback(result,args,fn):
 f=c.WINFUNCTYPE(result,*args)(fn);callbacks.append(f);return c.cast(f,c.c_void_p).value
def stub(rva,result,args,fn):
 p=callback(result,args,fn);c.memmove(memory+rva,b'\x48\xb8'+struct.pack('<Q',p)+b'\xff\xe0',12)
try:
 c.memmove(memory,raw[:pe.OPTIONAL_HEADER.SizeOfHeaders],pe.OPTIONAL_HEADER.SizeOfHeaders)
 for s in pe.sections:c.memmove(memory+s.VirtualAddress,s.get_data(),s.SizeOfRawData)
 for block in pe.DIRECTORY_ENTRY_BASERELOC:
  for rel in block.entries:
   if rel.type==10:
    p=c.c_uint64.from_address(memory+rel.rva);p.value=(p.value+memory-pe.OPTIONAL_HEADER.ImageBase)&0xffffffffffffffff
   else:assert rel.type==0
 vtable=c.create_string_buffer(0x70);allocator=c.create_string_buffer(8)
 put(vtable,0x68,'<Q',callback(None,[c.c_void_p,c.c_void_p],lambda a,p:frees.append(p)))
 put(allocator,0,'<Q',c.addressof(vtable));name=c.create_unicode_buffer('cs_c7560')
 def format_model(model,out):
  models.append(c.c_int.from_address(model).value)
  c.memset(out,0,40);c.c_uint64.from_address(out).value=c.addressof(allocator)
  c.c_uint64.from_address(out+8).value=c.addressof(name)
  c.c_uint64.from_address(out+24).value=8;c.c_uint64.from_address(out+32).value=8
  return out
 stub(0x3c5d50,c.c_void_p,[c.c_void_p,c.c_void_p],format_model)
 stub(0xf14360,None,[c.c_void_p,c.c_void_p,c.c_void_p,c.c_ubyte],lambda manager,key,path,flag:calls.append((manager,c.wstring_at(key),c.wstring_at(path),flag)))
 c.c_uint64.from_address(memory+0x3c2a8a8).value=0x12345678
 request=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p)(memory+0xeedfb0)
 model=c.c_int(7560);request(1,c.addressof(model))
 assert models==[7560] and calls==[(0x12345678,'cs_c7560','cs_c7560',0)] and frees==[c.addressof(name)]
 for invalid in [0,-1]:model.value=invalid;request(1,c.addressof(model))
 assert len(calls)==1
finally:assert k.VirtualFree(memory,0,0x8000)
out=ROOT/'.work/devils-gaze-tuning';out.mkdir(exist_ok=True)
(out/'audio-validation.json').write_text(json.dumps(dict(nativeBank='cs_c7560',bankSha256=hashlib.sha256(bank.read_bytes()).hexdigest(),events=ids,absentFromSharedBanks=True,nativeRequestAbiVerified=True,dependencyFixtures=['model string formatter','allocator free','bank request capture'],newSoundFiles=0,gameplayVerified=False),indent=2))
print('PASS: all three native Libra audio events located; native character-bank request ABI and invalid-ID handling verified')
