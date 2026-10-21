"""Run the native on-hit selector/category filter with charged-ward fixtures."""
import ctypes as c,hashlib,json,struct
import pefile
from local_paths import ROOT,GAME
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True);pe.parse_data_directories(directories=[5])
assert pe.get_data(0x505e90,27)==bytes.fromhex('40 53 55 56 57 48 81 EC 88 00 00 00 48 8B 05 C5 00 5E 03 48 33 C4 48 89 44 24 78')
assert pe.get_data(0x505ee9,4)==bytes.fromhex('4C 8B 45 10') # selector's actual owner
data=json.loads((ROOT/'.work/sword-charged/native.json').read_text())
rows={r['ID']:r['fields']for r in data['SpEffectParam']}
rows.update({r['ID']:r['fields']for r in data['Contract']})
assert all(rows[95002600][f]==0 for f in ['magParamChange','miracleParamChange','shamanParamChange'])
for tier,rate in [(8320400,.76),(8320410,.68),(8320420,.6)]:
 r=rows[tier];assert r['stateInfo']==2103 and r['atkOccurrenceSpEffectId']==tier+1
 assert r['magicSubCategoryChange1']==100 and not r['magParamChange'] and not r['miracleParamChange']
 assert rows[tier+1]['effectEndurance']==10 and rows[tier+1]['neutralDamageCutRate']==rate
for id in [8330200,8330300]:assert rows[id]['magicSubCategoryChange1']==110
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint];k.VirtualFree.restype=c.c_int
m=k.VirtualAlloc(None,pe.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert m
callbacks=[];cases=[]
def put(buf,off,fmt,v):struct.pack_into(fmt,buf,off,v)
def stub(rva,ret,args,fn):
 f=c.WINFUNCTYPE(ret,*args)(fn);callbacks.append(f)
 code=b'\x48\xb8'+struct.pack('<Q',c.cast(f,c.c_void_p).value)+b'\xff\xe0';c.memmove(m+rva,code,len(code))
try:
 for s in pe.sections:c.memmove(m+s.VirtualAddress,s.get_data(),s.SizeOfRawData)
 for block in pe.DIRECTORY_ENTRY_BASERELOC:
  for r in block.entries:
   if r.type==10:
    v=c.c_uint64.from_address(m+r.rva);v.value=(v.value+m-pe.OPTIONAL_HEADER.ImageBase)&0xffffffffffffffff
   else:assert r.type==0
 c.memmove(m+0x26909a0,b'\xc3',1) # cookie helper must preserve the native return register
 hit_ok=[True];near=[False];sword=[False]
 stub(0xea0790,c.c_ubyte,[c.c_void_p,c.c_void_p],lambda *_:hit_ok[0])
 select=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p,c.c_void_p)(m+0x505e90)
 for tier in [8320400,8320410,8320420,8600200,95002600]:
  fixture=(ROOT/f'.work/sword-charged/effect-{tier}.param').read_bytes();offset=struct.unpack_from('<Q',fixture,0x48)[0]
  row=c.create_string_buffer(fixture[offset:offset+0x3a8],0x3a8);node=c.create_string_buffer(0x80)
  put(node,0,'<Q',c.addressof(row));put(node,8,'<i',tier)
  manager=c.create_string_buffer(0x20);put(manager,8,'<Q',c.addressof(node))
  for kind,tag in [(1,100),(1,110),(3,110),(4,110),(3,100),(4,100),(3,0),(4,0),(10,110)]:
   for owns in [False,True]:
    for close in [False,True]:
     for valid in [False,True]:
      sword[0]=owns;near[0]=close;hit_ok[0]=valid
      hit=c.create_string_buffer(0x2c8);put(hit,0x122,'<B',kind)
      if tag:put(hit,0x124+tag//8,'<B',1<<(tag%8))
      ids=(c.c_int*4)();out=c.create_string_buffer(0x20);begin=c.addressof(ids)
      for off,v in [(8,begin),(16,begin),(24,begin+16)]:put(out,off,'<Q',v)
      before=(bytes(row),bytes(node),bytes(hit))
      copied=c.create_string_buffer(bytes(hit),0x2c8)
      if kind in [3,4] and owns:
       put(copied,0x122,'<B',1)
       off=0x124+130//8;put(copied,off,'<B',copied.raw[off]|(1<<(130%8)))
       if tag==110:
        off=0x124+100//8;put(copied,off,'<B',copied.raw[off]|(1<<(100%8)))
      select(c.addressof(manager),c.addressof(out),c.addressof(copied))
      count=(c.c_uint64.from_buffer(out,16).value-begin)//4
      expected=valid and ((kind==1 and tag==100)or(kind in [3,4]and tag in [100,110] and owns))
      if tier in [8600200,95002600]:expected=valid and (kind==1 or(kind in [3,4] and owns))
      assert count==int(expected),(tier,kind,tag,owns,close,valid,count)
      if count:assert ids[0]==rows[tier]['atkOccurrenceSpEffectId']
      assert before==(bytes(row),bytes(node),bytes(hit))
      cases.append(dict(tier=tier,kind=kind,category=tag,sword=owns,near=close,valid=valid,count=count))
finally:
 assert k.VirtualFree(m,0,0x8000)
source=(ROOT/'src/sword_hit_rules.rs').read_text()
assert 'sword_hit_distance' not in source and 'dispatch(hit,enabled' in source and 'hit[0x122]=1' in source
report=dict(nativeCases=len(cases),nativeSelector='0x505e90',effectFilter='0x511010',weaponChargeCategory=100,spellChargeCategory=110,tiers=[8320400,8320410,8320420],durationSeconds=10,negation=[.24,.32,.4],spellCasterDistanceGate=False,additionalOnHitReference=8600200,privateWeaponHitDescriptor=True,sharedRowsAndHitsUnchanged=True,gameplayVerified=False,multiplayerVerified=False)
report.update(contractNativeFlagsUnchanged=True,contractNoSeparateSpellFilter=True,externalHitEligibilityStubbed=True)
assert 'spell_filter' not in (ROOT/'src/ominous_pact.rs').read_text()
(ROOT/'dist/sword-hit-rules-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f'PASS: {len(cases)} native selector/filter cases; three ward tiers plus native FP-on-hit, charged/uncharged spells, ownership and native eligibility gates and original weapon triggers')
