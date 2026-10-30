"""Execute Clash's installed native category/on-hit/recovery paths in a private EXE image."""
import ctypes as c, hashlib, json, struct
from pathlib import Path
import pefile
from local_paths import ROOT,GAME
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
work=ROOT/'work/parry-recovery';fixture=(work/'installed-effects.param').read_bytes()
def row(id):
 for i in range(struct.unpack_from('<H',fixture,10)[0]):
  at=64+24*i
  if struct.unpack_from('<i',fixture,at)[0]==id:
   off=struct.unpack_from('<Q',fixture,at+8)[0];return c.create_string_buffer(fixture[off:off+1024],1024)
 raise AssertionError(id)
def put(buf,off,fmt,value):struct.pack_into(fmt,buf,off,value)
def integer(buf,off):return struct.unpack_from('<i',buf,off)[0]
def check(rva,hex):assert pe.get_data(rva,len(bytes.fromhex(hex)))==bytes.fromhex(hex)
check(0x43c921,'41 83 BE 80 02 00 00 00 0F 8E 0B 01 00 00')
check(0x43c996,'49 8B 8F 90 01 00 00 E8 EE 94 0C 00')
check(0x43ca08,'8B D6 49 8B CF E8 2E B5 F8 FF')
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p;k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
m=k.VirtualAlloc(None,pe.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert m
callbacks=[];cases=[]
def patch(rva,code):c.memmove(m+rva,code,len(code))
def stub(rva,restype,args,fn):
 f=c.WINFUNCTYPE(restype,*args)(fn);callbacks.append(f);patch(rva,b'\x48\xb8'+struct.pack('<Q',c.cast(f,c.c_void_p).value)+b'\xff\xe0')
def jump(rva):return b'\x48\xb8'+struct.pack('<Q',m+rva)+b'\xff\xe0'
try:
 for s in pe.sections:c.memmove(m+s.VirtualAddress,s.get_data(),s.SizeOfRawData)
 patch(0x26909a0,b'\xc3') # Security cookie only; category and hit eligibility execute untouched.
 counter=row(95002404);pulse=row(95002406)
 node=c.create_string_buffer(0x88);manager=c.create_string_buffer(0x80)
 put(node,0,'<Q',c.addressof(counter));put(node,8,'<i',95002404);put(manager,8,'<Q',c.addressof(node))
 select=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p,c.c_void_p)(m+0x505e90)
 # Native caller only runs selection for positive damage. Execute that gate.
 patch(0x1000,bytes.fromhex('41 56 48 83 ec 20 49 89 ce')+jump(0x43c921))
 epilogue=bytes.fromhex('48 83 c4 20 41 5e c3')
 patch(0x43c92f,bytes.fromhex('b8 01 00 00 00')+epilogue);patch(0x43ca3a,bytes.fromhex('31 c0')+epilogue)
 gate=c.WINFUNCTYPE(c.c_int,c.c_void_p)(m+0x1000)
 for active in [False,True]:
  for kind in [1,3,4,10]:
   for tag in [0,100,103,130]:
    for damage in [0,100]:
     hit=c.create_string_buffer(0x300);put(hit,0x122,'<B',kind);put(hit,0x280,'<i',damage)
     if tag:put(hit,0x124+tag//8,'<B',1<<(tag%8))
     put(node,0x68,'<I',0 if active else 2)
     ids=(c.c_int*4)();out=c.create_string_buffer(0x20);start=c.addressof(ids)
     for off,v in [(8,start),(16,start),(24,start+16)]:put(out,off,'<Q',v)
     if gate(hit):select(manager,out,hit)
     count=(struct.unpack_from('<Q',out,16)[0]-start)//4
     expected=active and kind==1 and tag==103 and damage>0
     assert count==int(expected),(active,kind,tag,damage,count)
     if count:assert ids[0]==95002406
     cases.append(dict(active=active,kind=kind,tag=tag,damage=damage,recoveryEvents=count))
 # Adding the on-hit state must retain the original five-element x2 multiplier.
 patch(0xe6d630,bytes.fromhex('48 8b c1 c3')) # Param repository reference lookup only.
 calculate=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p,c.c_int,c.c_int)(m+0x502830)
 for tag in [0,103,130]:
  context=c.create_string_buffer(0x180);put(context,8,'<i',40000);put(context,12,'<i',1)
  if tag:put(context,0x14+tag//8,'<B',1<<(tag%8))
  put(node,0x68,'<I',0);calculate(manager,context,-1,-1)
  for off in [0x90,0xa4,0xa8,0xac,0xb0]:assert abs(struct.unpack_from('<f',context,off)[0]-(2 if tag==103 else 1))<1e-6,(tag,off)
 # Run the actual selected-effect delivery loop. Recipient is the attacker,
 # not the struck enemy. Only row lookup / final apply call are fixtures.
 applied=[]
 def lookup(out,id):
  assert id==95002406;c.c_void_p.from_address(out).value=c.addressof(pulse);return out
 stub(0xea17e0,c.c_void_p,[c.c_void_p,c.c_int],lookup)
 stub(0x3c7f40,c.c_ubyte,[c.c_void_p,c.c_int,c.c_void_p,c.c_ubyte],lambda actor,id,*_:applied.append((actor,id))or 1)
 wrapper=bytes.fromhex('53 55 56 57 41 55 41 57 48 81 ec 08 02 00 00 48 8d ac 24 00 01 00 00 49 89 cf 48 89 d3 48 8d 7a 04 45 31 ed')
 patch(0x1200,wrapper+jump(0x43c9b0));patch(0x43ca1b,bytes.fromhex('48 81 c4 08 02 00 00 41 5f 41 5d 5f 5e 5d 5b c3'))
 deliver=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p)(m+0x1200)
 actor=c.create_string_buffer(0x200);ids=(c.c_int*1)(95002406)
 deliver(actor,ids);deliver(actor,ids);assert applied==[(c.addressof(actor),95002406)]*2
 # Native percentage aggregation with the installed immediate pulse.
 put(node,0,'<Q',c.addressof(pulse));put(node,8,'<i',95002406)
 for off in [0x58,0x5c,0x60,0x64]:put(node,off,'<f',1.)
 hp=c.WINFUNCTYPE(None,c.c_void_p,c.c_int,c.c_void_p,c.c_void_p,c.c_void_p,c.c_ubyte)(m+0x506ec0)
 stamina=c.WINFUNCTYPE(c.c_void_p,c.c_void_p,c.c_void_p)(m+0x5085a0)
 recovery=[]
 for max_hp in [1000,1200,2000]:
  for flags in [0,2,4]:
   put(node,0x68,'<I',flags);source=c.c_uint64(0xffffffffffffffff);flag=c.c_ubyte(0xfe);out=c.create_string_buffer(0x20)
   hp(manager,max_hp,c.byref(source),c.byref(flag),out,0)
   # 431C47/431C70/431E01 apply lanes +4/+12/+0; +8 tracks attribution.
   gains=[integer(out,i*4) for i in range(5)];gain=gains[0]+gains[1]+gains[3]
   assert gain==(max_hp*35//100 if flags==4 else 0),(max_hp,flags,gains)
   rates=(c.c_float*2)();stamina(manager,rates);assert list(rates)==([-65.,0.]if flags==4 else [0.,0.])
   recovery.append(dict(maxHp=max_hp,flags=flags,hpGains=gains,staminaRate=rates[0]))
 # Execute the native stamina application math plus original AddStamina/clamp.
 put(node,0x68,'<I',4);stats=c.create_string_buffer(0x200);modules=c.create_string_buffer(0x180)
 put(modules,0,'<Q',c.addressof(stats));put(actor,0x1b8,'<Q',c.addressof(modules));put(actor,0x190,'<Q',c.addressof(manager))
 patch(0x1400,bytes.fromhex('56 48 83 ec 30 48 89 ce')+jump(0x3daeb4));patch(0x3daf00,bytes.fromhex('48 83 c4 30 5e c3'))
 restore=c.WINFUNCTYPE(None,c.c_void_p)(m+0x1400)
 for maximum in [100,120,200]:
  for current in [0,maximum//2,maximum]:
   put(stats,0x15c,'<i',current);put(stats,0x160,'<i',maximum);restore(actor)
   assert integer(stats,0x15c)==min(maximum,current+maximum*65//100),(maximum,current,integer(stats,0x15c))
 # Execute the original HP setter's clamp; stop its post-write authority path.
 patch(0x6ada30,bytes.fromhex('b0 01 c3'))
 set_hp=c.WINFUNCTYPE(None,c.c_void_p,c.c_int,c.c_ubyte,c.c_float,c.c_float,c.c_ubyte)(m+0x426b00)
 for maximum in [1000,1200,2000]:
  for current in [1,maximum//2,maximum]:
   put(stats,0x140,'<i',current);put(stats,0x144,'<i',maximum)
   out=c.create_string_buffer(0x20);hp(manager,maximum,c.byref(source),c.byref(flag),out,0)
   gain=sum(integer(out,i*4) for i in [0,1,3])
   set_hp(stats,current+gain,0,1.,1.,1)
   assert integer(stats,0x140)==min(maximum,current+maximum*35//100)
 # The removed hook cannot waive a missed counter's ordinary attack cost.
 assert 'COUNTER_COST' not in (ROOT/'src/lib.rs').read_text()
 assert 'counter_cost_detour' not in (ROOT/'src/parry.rs').read_text()
finally:assert k.VirtualFree(m,0,0x8000)
report=dict(nativeSelectorCases=len(cases),cases=cases,nativeRecoveryCases=recovery,damageMultiplierPreserved=True,nativeAttackerDelivery=True,prayerfulStrikeVfx=8630,prayerfulStrikeSfx=525033,counterCostHookRemoved=True,stubs=['security-cookie helper','HP setter post-write authority predicate','attack-rate parameter reference lookup','selected-effect row lookup and final effect submission; native caller recipient verified'],fixtureSha256=hashlib.sha256(fixture).hexdigest(),gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n')
print(f'PASS: {len(cases)} native Clash on-hit/category cases; x2 damage, attacker-only delivery, 35% HP and 65% stamina recovery')
