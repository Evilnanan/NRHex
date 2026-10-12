"""Isolated original greatshield comparison and attacker recoil, no game writes."""
import ctypes as c, hashlib, json, re, struct, tomllib
from pathlib import Path
import pefile
from local_paths import GAME,ROOT

work=ROOT/'.work/crystal-rebound-01730'
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
weapons=json.loads((work/'native-weapons.json').read_text())
shield=next(r for r in weapons if r['ID']==32000000)
assert shield['fields']['wepType']==69 and shield['fields']['guardBaseRepel']==70
pe=pefile.PE(data=raw)
# Independently bind the Rust lookup to current native code and RTTI. The old
# test passed a hand-made damage object directly and missed the wrong slot.
source=(ROOT/'src/lib.rs').read_text()
slot=int(re.search(r'const DAMAGE_MODULE_SLOT: usize = (0x[0-9a-f]+);',source)[1],16)
damage_vtable_rva=int(re.search(r'const DAMAGE_MODULE_VTABLE_RVA: usize = (0x[0-9a-f]+);',source)[1],16)
assert pe.get_data(0x3f5f85,7)==bytes.fromhex('48 8b 88 b8 01 00 00')
assert pe.get_data(0x3f5f8c,7)==b'\x48\x8b\x89'+struct.pack('<I',slot)
assert 0x3f5f98+struct.unpack('<i',pe.get_data(0x3f5f94,4))[0]==0x431550
def module_name(vt):
 col=struct.unpack('<Q',pe.get_data(vt-8,8))[0]-pe.OPTIONAL_HEADER.ImageBase
 td=struct.unpack('<I',pe.get_data(col+12,4))[0]
 return pe.get_data(td+16,100).split(b'\0')[0].decode()
assert module_name(damage_vtable_rva)=='.?AVCSChrDamageModule@CS@@'
assert module_name(0x2c12320)=='.?AVCSChrHitStopModule@CS@@'
# Native hit-stop deleting destructor frees a 0x30-byte object.
assert pe.get_data(0x444378,5)==bytes.fromhex('ba 30 00 00 00')
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
memory=k.VirtualAlloc(None,pe.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert memory
callbacks=[];motion=[];notifications=[];boost=0
def ptr(b):return c.addressof(b)
def callback(rva,result,args,fn):
 f=c.WINFUNCTYPE(result,*args)(fn);callbacks.append(f)
 c.memmove(memory+rva,b'\x48\xb8'+struct.pack('<Q',c.cast(f,c.c_void_p).value)+b'\xff\xe0',12)
 return f
def vfun(result,args,fn):
 f=c.WINFUNCTYPE(result,*args)(fn);callbacks.append(f);return c.cast(f,c.c_void_p).value
def put(b,off,fmt,value):struct.pack_into(fmt,b,off,value)
try:
 c.memmove(memory,raw[:pe.OPTIONAL_HEADER.SizeOfHeaders],pe.OPTIONAL_HEADER.SizeOfHeaders)
 for section in pe.sections:c.memmove(memory+section.VirtualAddress,section.get_data(),section.SizeOfRawData)
 delta=memory-pe.OPTIONAL_HEADER.ImageBase
 for block in pe.DIRECTORY_ENTRY_BASERELOC:
  for rel in block.entries:
   if rel.type==10:
    p=c.c_uint64.from_address(memory+rel.rva);p.value=(p.value+delta)&0xffffffffffffffff
   else:assert rel.type==0
 # External dependencies are synthetic identity/effect queries and notification
 # sinks. Original 439D50 comparison and 436E50 reaction selection remain intact.
 actor=c.create_string_buffer(0x200);vtable=c.create_string_buffer(0x400)
 manager=c.create_string_buffer(0x80);damage=c.create_string_buffer(0x100)
 modules=c.create_string_buffer(0x200);reaction=c.create_string_buffer(0x100)
 reaction_vtable=c.create_string_buffer(0x100)
 put(actor,0,'<Q',ptr(vtable));put(actor,0x190,'<Q',ptr(manager));put(actor,0x1b8,'<Q',ptr(modules))
 put(damage,8,'<Q',ptr(actor));put(damage,0,'<Q',ptr(vtable))
 put(modules,0x38,'<Q',1);put(reaction,0,'<Q',ptr(reaction_vtable))
 put(vtable,0x390,'<Q',vfun(c.c_int,[c.c_void_p],lambda _:70))
 put(vtable,0x388,'<Q',vfun(c.c_int,[c.c_void_p,c.c_int],lambda *_:70))
 put(vtable,0x28,'<Q',vfun(c.c_float,[c.c_void_p],lambda _:1.0))
 put(reaction_vtable,0x88,'<Q',vfun(None,[c.c_void_p,c.c_void_p],lambda *a:notifications.append('native attacker notification')))
 callback(0x42c120,c.c_void_p,[c.c_void_p],lambda m:c.c_uint64.from_address(m+8).value)
 callback(0x5079a0,c.c_int,[c.c_void_p],lambda _:0)
 callback(0x507950,c.c_int,[c.c_void_p],lambda _:boost)
 callback(0x424d80,None,[c.c_void_p,c.c_void_p,c.c_void_p],lambda *_:None)
 def direction(out,kind):c.c_uint64.from_address(out).value=kind;return out
 callback(0x3e6960,c.c_void_p,[c.c_void_p,c.c_int],direction)
 callback(0x3e6780,None,[c.c_void_p],lambda _:None)
 callback(0x4381f0,None,[c.c_void_p,c.c_void_p,c.c_uint,c.c_uint64],lambda module,hit,kind,direction:motion.append(kind))
 callback(0x3d0100,c.c_void_p,[c.c_void_p],lambda _:ptr(reaction))
 callback(0x3d4180,c.c_ubyte,[c.c_void_p],lambda _:0)
 compare=c.WINFUNCTYPE(c.c_float,c.c_void_p,c.c_void_p,c.c_void_p)(memory+0x439d50)
 dispatch=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p,c.c_void_p)(memory+0x436e50)
 comparison=[];dispatch_cases=[]
 for raw_power in [0,30,50,69,70,71,80,99]:
  for boost_value in [0,50,80,99]:
   boost=boost_value;hit=c.create_string_buffer(0x2c8)
   put(hit,0x2c,'<i',raw_power);compare(ptr(damage),ptr(actor),ptr(hit))
   attack=struct.unpack_from('<i',hit,0x2a8)[0]
   assert attack==max(raw_power,boost)
   assert struct.unpack_from('<i',hit,0x2ac)[0]==70
   assert hit.raw[0x2be]==int(attack<=70)
   comparison.append(dict(raw=raw_power,effect=boost,attack=attack,rebound=attack<=70))
 for weapon,expected in [(0,1000),(2,1003),(6,1008),(7,1009),(8,1010)]:
  for gate in ['valid','nullified_hp','no_damage','guard_break','cancelled','not_repelled']:
   hit=c.create_string_buffer(0x2c8);put(hit,0,'<f',100.0);hit[0x122]=weapon
   hit[0x2be]=1;put(hit,0x2ac,'<i',70);put(hit,0x2a8,'<i',30)
   if gate=='nullified_hp':hit[0x2c3]=2
   if gate=='no_damage':put(hit,0,'<f',0.0)
   if gate=='guard_break':hit[0x2b5]=1
   if gate=='cancelled':hit[0x2c1]=2
   if gate=='not_repelled':hit[0x2be]=0
   motion.clear();notifications.clear();dispatch(ptr(damage),ptr(actor),ptr(hit))
   assert motion==([expected] if gate in ['valid','nullified_hp'] else []),(weapon,gate,motion)
   assert len(notifications)==1
   dispatch_cases.append(dict(weapon=weapon,gate=gate,motion=motion.copy()))
 # Model the old 0x30-byte object with a generously allocated, checked tail.
 # Execute the real recoil writer: it must demonstrate the out-of-object
 # +74 write, without corrupting this test process or any game memory.
 wrong=c.create_string_buffer(bytes([0xa5])*0x100)
 put(wrong,8,'<Q',ptr(actor));before=wrong.raw
 hit=c.create_string_buffer(0x2c8);put(hit,0,'<f',100.0)
 hit[0x2be]=1;put(hit,0x2a8,'<i',60);put(hit,0x2ac,'<i',70)
 dispatch(ptr(wrong),ptr(actor),ptr(hit))
 overwritten=[i for i,(a,b) in enumerate(zip(before,wrong.raw)) if a!=b]
 assert overwritten==list(range(0x74,0x78)),overwritten
 wrong_rate=struct.unpack_from('<f',wrong,0x74)[0]
 assert wrong_rate==0.25 # same high 32 bits seen in the invalid crash pointer
finally:assert k.VirtualFree(memory,0,0x8000)
version=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version']
report=dict(version=version,nativeGreatshield=shield,nativeHardnessRva='439d50',nativeRecoilRva='436e50',
 damageModuleSlot=slot,damageModuleVtable=hex(damage_vtable_rva),damageModuleRtti=module_name(damage_vtable_rva),
 wrongModuleSize=0x30,oldWrongModuleOverwrite=overwritten,oldWrongModuleWrittenFloat=wrong_rate,
 comparisonCases=comparison,recoilCases=dispatch_cases,isolatedNativeCases=len(comparison)+len(dispatch_cases),
 nativeHpNullifyAndRecoilIndependent=True,dependenciesStubbed=['recipient identity','effect hardness override','module lookup','direction handle','motion and notification sinks'],
 originalExeSha256=hashlib.sha256(raw).hexdigest(),gameMemoryWritten=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2))
print(f'PASS: {len(comparison)} native hardness comparisons; {len(dispatch_cases)} native recoil/weapon/cancellation cases; no game writes.')
print(f'PASS: native damage slot {slot:#x}, RTTI identity, old wrong-module overwrite at +0x74 reproduced ({wrong_rate}).')
