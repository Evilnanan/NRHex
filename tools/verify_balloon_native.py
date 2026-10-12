"""Run original base-attack calculation in an isolated PE; never touch the game."""
import ctypes as c
import hashlib
import json
import struct
from pathlib import Path
import pefile
from local_paths import GAME, ROOT

raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
pe.parse_data_directories(directories=[5])
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
size=pe.OPTIONAL_HEADER.SizeOfImage
memory=k.VirtualAlloc(None,size+4096,0x3000,0x40);assert memory
def put(b,off,fmt,value):struct.pack_into(fmt,b,off,value)
def jump(at,to):c.memmove(at,b'\xe9'+struct.pack('<i',to-at-5),5)
results=[]
try:
 c.memmove(memory,raw[:pe.OPTIONAL_HEADER.SizeOfHeaders],pe.OPTIONAL_HEADER.SizeOfHeaders)
 for section in pe.sections:c.memmove(memory+section.VirtualAddress,section.get_data(),section.SizeOfRawData)
 delta=memory-pe.OPTIONAL_HEADER.ImageBase
 for block in pe.DIRECTORY_ENTRY_BASERELOC:
  for rel in block.entries:
   if rel.type==10:
    p=c.c_uint64.from_address(memory+rel.rva);p.value=(p.value+delta)&0xffffffffffffffff
   else:assert rel.type==0
 row=c.create_string_buffer(0x200)
 @c.WINFUNCTYPE(c.c_void_p,c.c_void_p,c.c_int,c.c_int)
 def lookup(out,table,attack):
  c.c_uint64.from_address(out+16).value=c.addressof(row)
  return out
 c.memmove(memory+0xe6d630,b'\x48\xb8'+struct.pack('<Q',c.cast(lookup,c.c_void_p).value)+b'\xff\xe0',12)
 init=c.WINFUNCTYPE(c.c_void_p,c.c_void_p)(memory+0x33e360)
 callbacks=[]
 def stub(rva,result,args,fn):
  f=c.WINFUNCTYPE(result,*args)(fn);callbacks.append(f)
  c.memmove(memory+rva,b'\x48\xb8'+struct.pack('<Q',c.cast(f,c.c_void_p).value)+b'\xff\xe0',12)
 def empty_ref(out,*_):c.c_uint64.from_address(out+8).value=0;return out
 for address in [0xea9a60,0xea7fc0]:stub(address,c.c_void_p,[c.c_void_p,c.c_int],empty_ref)
 stub(0xe6f150,c.c_void_p,[c.c_void_p,c.c_int],empty_ref)
 stub(0xe6f8e0,None,[c.c_void_p,c.c_void_p,c.c_int],lambda ref,out,kind:c.memset(out,0,12))
 for address in [0x6d1cc0,0x6d2370]:stub(address,c.c_float,[c.c_void_p,c.c_void_p,c.c_void_p,c.c_int],lambda *_:1.)
 pc_calculate=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p,c.c_int,c.c_int,c.c_float,c.c_void_p,c.c_ubyte,c.c_void_p,c.c_ubyte)(memory+0x6c3200)
 stats=c.create_string_buffer(0x1000)
 def calculate(out,ctx):pc_calculate(out,c.addressof(stats),-1,-1,1.,ctx,0,0,1)
 rates=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p)(memory+0xe70270)
 site=memory+0x6c49bb;stolen=pe.get_data(0x6c49bb,5)
 assert stolen==bytes.fromhex('66 44 0f 6e c1')
 # The real Rust wrapper is covered by its ABI regression. Here substitute
 # ECX only, then execute the original modifier instructions unchanged.
 for hp in [1,1500,2345,70000]:
  for power,flat in [(1.,0.),(1.5,0.),(.5,50.),(0.,0.)]:
   ctx=c.create_string_buffer(0x120);out=c.create_string_buffer(0x40)
   init(c.addressof(ctx));put(ctx,8,'<i',95001760);put(ctx,12,'<i',1);ctx[0xf8]=1
   put(ctx,0x6c,'<f',power);put(ctx,0x48,'<f',flat)
   put(row,0x50,'<H',1)
   stub=memory+size
   code=b'\xb9'+struct.pack('<I',hp)+stolen
   c.memmove(stub,code,len(code));jump(stub+len(code),site+5);jump(site,stub)
   calculate(c.addressof(out),c.addressof(ctx))
   actual=struct.unpack_from('<f',out,0)[0]
   assert actual==hp*power+flat,(hp,power,flat,actual)
   assert struct.unpack_from('<4f',out,4)==(0.,0.,0.,0.)
   if hp<=65535:
    c.memmove(site,stolen,5);put(row,0x50,'<H',hp)
    native=c.create_string_buffer(0x40);calculate(c.addressof(native),c.addressof(ctx))
    assert native.raw==out.raw # identical to a native row with that base attack
   results.append(dict(hp=hp,power=power,flat=flat,nativePhysicalAttack=actual))
 # Native final multipliers are still created separately by E70270.
 put(ctx,0x90,'<f',1.5);put(ctx,0xa0,'<f',1.2)
 hit=c.create_string_buffer(0x2c8);hit[0x25]=3
 rates(c.addressof(hit),c.addressof(ctx))
 assert struct.unpack_from('<h',hit,0x14)[0]==180
finally:assert k.VirtualFree(memory,0,0x8000)
source=(ROOT/'src/ability_damage.rs').read_text(encoding='utf-8')
assert 'PhysicalBuffs' not in source and 'AttackPowerRate' not in source
lib=(ROOT/'src/lib.rs').read_text(encoding='utf-8')
assert 'BALLOON_DAMAGE' not in lib and 'read_i32(hit+0x40)==balloon::ATTACK' not in lib
work=ROOT/'.work/balloon-native-01766';work.mkdir(exist_ok=True)
(work/'native-validation.json').write_text(json.dumps(dict(cases=results,nativeCasesPassed=len(results),nativeFinalRatePercent=180,
 nativePlayerDamageRoutine="6c3200",dependencyStubs=["parameter lookup", "empty weapon references", "neutral stat scaling"],onlyBaseRegisterReplaced=True,oldFinalHpOverwriteRemoved=True,gameMemoryWritten=False,gameplayVerified=False,multiplayerVerified=False),indent=2))
print(f'PASS: {len(results)} native base-attack cases, native final-rate encoding, removed final-HP overwrite; no game writes')
