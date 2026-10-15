"""Execute native regain acquisition, queueing, decay and HP settlement in isolation."""
from pathlib import Path
import ctypes as native
import hashlib,json,struct
import pefile
from local_paths import ROOT,GAME

raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
for site,expected in [(0x33c4b0,'4C 8B C6 8B D0 48 8B CF E8 53 E0 09 00'),(0x43f3e5,'4C 8B C6 8B D0 49 8B CE E8 1E B1 F9 FF')]:
 assert pe.get_data(site,len(bytes.fromhex(expected)))==bytes.fromhex(expected)
 assert site+13+struct.unpack('<i',pe.get_data(site+9,4))[0]==0x3da510
kernel=native.WinDLL('kernel32')
kernel.VirtualAlloc.argtypes=[native.c_void_p,native.c_size_t,native.c_uint,native.c_uint];kernel.VirtualAlloc.restype=native.c_void_p
kernel.VirtualFree.argtypes=[native.c_void_p,native.c_size_t,native.c_uint]
memory=kernel.VirtualAlloc(None,pe.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert memory
cases=[];tier_cases=[]
enabled=True
@native.WINFUNCTYPE(native.c_ubyte,native.c_void_p,native.c_ushort)
def has_state(manager,state):
 assert state==449
 return enabled
def put(buffer,offset,format,value):struct.pack_into(format,buffer,offset,value)
def integer(buffer,offset):return struct.unpack_from('<i',buffer,offset)[0]
def floating(buffer,offset):return struct.unpack_from('<f',buffer,offset)[0]
def stats(hp=1000):
 buffer=native.create_string_buffer(0x200);put(buffer,0x140,'<i',hp);put(buffer,0x144,'<i',1000);return buffer
try:
 for section in pe.sections:
  if section.SizeOfRawData:native.memmove(memory+section.VirtualAddress,raw[section.PointerToRawData:section.PointerToRawData+section.SizeOfRawData],section.SizeOfRawData)
 native.memmove(memory+0xe6d630,bytes.fromhex('48 8B C1 C3'),4)
 describe=native.WINFUNCTYPE(native.c_void_p,native.c_void_p,native.c_void_p)(memory+0x50f230)
 calculate=native.WINFUNCTYPE(native.c_float,native.c_void_p,native.c_void_p)(memory+0x507bc0)
 def rate_for(effect_ids,descriptor):
  manager=native.create_string_buffer(0x80);parameters=[];nodes=[]
  for effect in effect_ids:
   fixture=(ROOT/f'.work/sword-regain/effect-{effect}.param').read_bytes()
   assert struct.unpack_from('<i',fixture,0x40)[0]==effect
   offset=struct.unpack_from('<Q',fixture,0x48)[0]
   parameter=native.create_string_buffer(fixture[offset:offset+0x3a8],0x3a8);node=native.create_string_buffer(0x80)
   put(node,0,'<Q',native.addressof(parameter));put(node,8,'<i',effect);parameters.append(parameter);nodes.append(node)
  if nodes:put(manager,8,'<Q',native.addressof(nodes[0]))
  for previous,following in zip(nodes,nodes[1:]):put(previous,0x38,'<Q',native.addressof(following))
  return calculate(manager,descriptor)
 for address,code in [(0x6ada30,'31 C0 C3'),(0x42c120,'48 8B C1 C3'),(0x42c130,'48 8B C1 C3'),(0x3d4180,'B0 01 C3')]:native.memmove(memory+address,bytes.fromhex(code),len(bytes.fromhex(code)))
 branch=b'\x48\xb8'+struct.pack('<Q',native.cast(has_state,native.c_void_p).value)+b'\xff\xe0';native.memmove(memory+0x508e20,branch,len(branch))
 set_hp=native.WINFUNCTYPE(None,native.c_void_p,native.c_int,native.c_ubyte,native.c_float,native.c_float,native.c_ubyte)(memory+0x426b00)
 queue=native.WINFUNCTYPE(None,native.c_void_p,native.c_float)(memory+0x4275a0)
 update=native.WINFUNCTYPE(None,native.c_void_p,native.c_float)(memory+0x4289c0)
 for effect,expected_rate in [(7005600,1.),(7005601,1.25),(7005602,1.35)]:
  for spell_type in [3,4]:
   hit=native.create_string_buffer(0x2c8);hit[0x122]=spell_type;put(hit,0xf8,'<i',-1);put(hit,0x40,'<i',40000);put(hit,0x44,'<i',1)
   before_hit=hit.raw;descriptor=native.create_string_buffer(0x3c);describe(descriptor,hit);before_descriptor=descriptor.raw
   assert rate_for([effect],descriptor)==1.
   descriptor[0]=1;rate=rate_for([effect],descriptor)
   assert abs(rate-expected_rate)<1e-6 and descriptor.raw[1:]==before_descriptor[1:] and hit.raw==before_hit
   requested=int(180*rate)
   for loss in [100,500]:
    buffer=stats();set_hp(buffer,1000-loss,0,1.,1.,1);queue(buffer,requested);update(buffer,.016)
    recovered=min(loss,int(180*expected_rate));assert integer(buffer,0x140)==1000-loss+recovered
    assert floating(buffer,0x168)==loss-recovered
    tier_cases.append(dict(effect=effect,spellType=spell_type,rate=rate,requested=requested,loss=loss,recovered=recovered))
 descriptor=native.create_string_buffer(0x3c);descriptor[0]=1;put(descriptor,0x30,'<i',-1);put(descriptor,0x34,'<i',40000);put(descriptor,0x38,'<i',1)
 assert rate_for([],descriptor)==1.
 assert abs(rate_for([7005601,7005602],descriptor)-1.6875)<1e-6
 for loss in [1,20,200,700]:
  for amount in [0,1,20,180,65535]:
   buffer=stats();set_hp(buffer,1000-loss,0,1.,1.,1)
   assert floating(buffer,0x168)==loss and floating(buffer,0x170)==4.
   queue(buffer,amount);assert integer(buffer,0x140)==1000-loss
   expected=min(loss,amount);assert floating(buffer,0x174)==expected
   update(buffer,.016);assert integer(buffer,0x140)==1000-loss+expected
   assert floating(buffer,0x168)==loss-expected
   cases.append(dict(loss=loss,requested=amount,recovered=expected))
 buffer=stats(700);set_hp(buffer,600,0,1.,1.,1);queue(buffer,999);update(buffer,.016)
 assert integer(buffer,0x140)==700;cases.append(dict(historicalLossNotRecovered=True))
 buffer=stats();queue(buffer,999);update(buffer,.016);assert integer(buffer,0x140)==1000 and floating(buffer,0x174)==0
 cases.append(dict(noDamageNoHealing=True))
 buffer=stats();set_hp(buffer,800,0,1.,1.,1);queue(buffer,180);queue(buffer,180);update(buffer,.016)
 assert integer(buffer,0x140)==1000 and floating(buffer,0x168)==0;cases.append(dict(multipleHitsSharePool=True))
 buffer=stats();set_hp(buffer,800,0,1.,1.,1);update(buffer,3.99);assert floating(buffer,0x168)==200
 update(buffer,.02);assert abs(floating(buffer,0x168)-180)<.01
 update(buffer,.2);queue(buffer,999);update(buffer,.016);assert integer(buffer,0x140)==800
 cases.append(dict(nativeFourSecondWindowAndDecay=True))
 buffer=stats();set_hp(buffer,800,0,1.,1.,1);queue(buffer,200);update(buffer,5.)
 assert integer(buffer,0x140)==800 and floating(buffer,0x174)==0;cases.append(dict(expiryClampsPendingRecovery=True))
 buffer=stats();set_hp(buffer,800,0,1.,1.,1);set_hp(buffer,950,0,0.,1.,1);queue(buffer,999);update(buffer,.016)
 assert integer(buffer,0x140)==1000;cases.append(dict(otherHealingClampsNativePool=True))
 buffer=stats();set_hp(buffer,800,0,1.,1.,1);queue(buffer,200);put(buffer,0x140,'<i',0);update(buffer,.016)
 assert integer(buffer,0x140)==0;cases.append(dict(noResurrection=True))
 buffer=stats();enabled=False;set_hp(buffer,800,0,1.,1.,1);queue(buffer,200);update(buffer,.016)
 assert integer(buffer,0x140)==800 and floating(buffer,0x168)==0;cases.append(dict(nativeStateRequired=True))
finally:kernel.VirtualFree(memory,0,0x8000)
source=(ROOT/'src/lib.rs').read_text()
assert 'sword::Healing' not in source and 'sword::HEAL_BASE' not in source and 'sword::HEAL_VISUAL' not in source
regain=(ROOT/'src/sword_regain.rs').read_text()
assert 'describe(descriptor.as_mut_ptr(),hit);descriptor[0]=1;' in regain and 'calculate(manager,descriptor.as_ptr())' in regain
assert '||native_rate(source,hit,base)' in regain
report=dict(nativeCases=len(cases),cases=cases,nativeState=449,nativeQueue='0x4275a0',nativeSettlement='0x4289c0',nativeHpSetter='0x426b00',nativeAward='0x3da510',sharedPool=True,customHpWrites=False,fixtureStubs=['network authority','actor lookup','local player predicate','state 449 presence'],gameplayVerified=False,multiplayerVerified=False)
report.update(nativeTierCases=len(tier_cases),tierCases=tier_cases,nativeRate='0x507bc0',nativeDescriptor='0x50f230',privateWeaponDescriptor=True,originalHitUnchanged=True,nativeAggregationPreserved=True)
report['fixtureStubs'].append('E6D630 PARAM lookup for exported original tier rows')
directory=ROOT/'.work/sword-regain';directory.mkdir(parents=True,exist_ok=True)
(directory/'native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print(f'PASS: {len(cases)} isolated native regain cases; acquisition, shared cap, four-second window, expiry, death and native settlement verified')
print(f'PASS: {len(tier_cases)} native tier/settlement cases; 1.0/1.25/1.35, shared pool caps, unchanged spell hit and native aggregation verified')
