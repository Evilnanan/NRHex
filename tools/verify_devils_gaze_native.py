"""Execute original NPC damage and network validation in an isolated PE copy."""
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
 row=c.create_string_buffer(0x200)
 lookups=[]
 def lookup(out,table,attack):
  lookups.append((table,attack));c.c_uint64.from_address(out+16).value=c.addressof(row);return out
 stub(0xe6d630,c.c_void_p,[c.c_void_p,c.c_int,c.c_int],lookup)
 # Original damage dispatch resolves Bullet's owner through the Chr manager.
 # An environment handle cannot resolve to a Chr, hence raw NPC damage.
 resolved=[]
 def resolve(world,handle):resolved.append(c.c_uint64.from_address(handle).value);return 0
 stub(0x518f30,c.c_void_p,[c.c_void_p,c.c_void_p],resolve)
 c.c_uint64.from_address(memory+0x3c0f0a8).value=1
 vt=c.create_string_buffer(0x368);bullet=c.create_string_buffer(0x100)
 kind=c.WINFUNCTYPE(c.c_int,c.c_void_p)(lambda _:3);callbacks.append(kind)
 put(vt,0x10,'<Q',c.cast(kind,c.c_void_p).value);put(bullet,0,'<Q',c.addressof(vt))
 handle=0x3c00000060000001;put(bullet,0xb0,'<Q',handle)
 # Validate the exact loaded-map tree / static asset array layout used by
 # the runtime against the game's original environment-owner resolver.
 manager=c.create_string_buffer(0x40);head=c.create_string_buffer(0x30)
 node=c.create_string_buffer(0x30);loaded=c.create_string_buffer(0x310)
 asset=c.create_string_buffer(0x280);slots=(c.c_void_p*2)(0,c.addressof(asset))
 put(manager,0x20,'<Q',c.addressof(head));put(head,8,'<Q',c.addressof(node));head[0x19]=1
 put(node,0,'<Q',c.addressof(head));put(node,0x10,'<Q',c.addressof(head))
 put(node,0x20,'<I',handle>>32);put(node,0x28,'<Q',c.addressof(loaded))
 loaded[0x185]=1;put(loaded,0x278,'<Q',c.addressof(slots));put(loaded,0x280,'<Q',c.addressof(slots)+16)
 find_asset=c.WINFUNCTYPE(c.c_void_p,c.c_void_p,c.c_void_p)(memory+0x70ca60)
 owner=c.c_uint64(handle)
 assert find_asset(c.addressof(manager),c.addressof(owner))==c.addressof(asset)
 loaded[0x185]=0;assert not find_asset(c.addressof(manager),c.addressof(owner))
 loaded[0x185]=1;owner.value=0x3d00000060000001
 assert not find_asset(c.addressof(manager),c.addressof(owner))
 init=c.WINFUNCTYPE(c.c_void_p,c.c_void_p)(memory+0x33e360)
 damage=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p)(memory+0x6c2dd0)
 validation=json.loads((ROOT/'.work/devils-gaze/staged/validation.json').read_text())
 if '--tuned' in sys.argv:
  tuning=json.loads((ROOT/'.work/devils-gaze-tuning/parameter-validation.json').read_text())
  validation['holyDamage']=[dict(ID=r['id'],damage=r['next']) for r in tuning['changes']]
 cases=[]
 for item in validation['holyDamage']:
  ctx=c.create_string_buffer(0x150);out=c.create_string_buffer(0x40)
  init(c.addressof(ctx));put(ctx,0,'<Q',c.addressof(bullet));put(ctx,8,'<i',item['ID']);put(ctx,12,'<i',0)
  put(row,0x188,'<H',item['damage']);damage(c.addressof(out),c.addressof(ctx))
  result=struct.unpack_from('<5f',out)
  assert result==(0.,0.,0.,0.,float(item['damage'])),result
  assert lookups[-1]==(0,item['ID']) and resolved[-1]==handle
  cases.append(result[-1])
 # Exercise the native protocol's field validation, using valid fixtures for
 # external map/target lookups. The source-kind and range checks run unchanged.
 for address,result in [(0xf560e0,0),(0x3493e0,1),(0x4e10c0,0),(0x50ee40,1),(0x6343a0,1)]:
  stub(address,c.c_ubyte,[c.c_void_p],lambda _,value=result:value)
 stub(0x2ff820,c.c_void_p,[c.c_void_p,c.c_void_p],lambda source,out:source)
 packet=c.create_string_buffer(0xc0)
 put(packet,0x80,'<Q',handle);put(packet,0x88,'<i',1)
 for off in [0x90,0x98,0x9c,0xa0]:put(packet,off,'<i',-1)
 for off in [0xa4,0xa8]:put(packet,off,'<i',16)
 validate=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p)(memory+0x33fd10)
 assert validate(c.addressof(packet))==1
 for off,value in [(0x80,0xffffffffffffffff),(0x94,0xffffffff),(0xac,0xffffffff),(0xb0,0xffffffff)]:
  count=8 if off==0x80 else 4;old=packet.raw[off:off+count]
  put(packet,off,'<Q' if count==8 else '<I',value);assert validate(c.addressof(packet))==0
  c.memmove(c.addressof(packet)+off,old,count)
 assert validate(c.addressof(packet))==1
finally:assert k.VirtualFree(memory,0,0x8000)
report=dict(nativeNpcDamageCases=cases,nativeEnvironmentRegistryVerified=True,environmentOwnerBypassesCharacterDamage=True,nativeNetworkFieldChecksPassed=True,
 dependenciesStubbed=['parameter row lookup','environment handle absent from Chr manager','valid external map/target fixtures'],gameMemoryWritten=False,gameplayVerified=False,multiplayerVerified=False)
(ROOT/('.work/devils-gaze-tuning/native-validation.json' if '--tuned' in sys.argv else '.work/devils-gaze/native-validation.json')).write_text(json.dumps(report,indent=2))
print('PASS: 6 native holy damage cases; environment owner cannot inherit PC damage; native packet field acceptance/rejection')
