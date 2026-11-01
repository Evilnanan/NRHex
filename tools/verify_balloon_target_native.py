"""Execute native attack-effect delivery, guard handling and HP pulse aggregation.
World/network delivery is captured at 3C80A0, not simulated as gameplay.
Run with Windows Python, ROOT and GAME configured via tools/local_paths.py.
"""
import ctypes as c, hashlib, json, struct
from pathlib import Path
import pefile
from local_paths import ROOT,GAME
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True);work=ROOT/'.work/balloon7-independent'
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p;k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
m=k.VirtualAlloc(None,pe.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert m
callbacks=[];cases=[];pulse=None

def put(buf,off,fmt,v):struct.pack_into(fmt,buf,off,v)
def patch(rva,code):c.memmove(m+rva,code,len(code))
def jump(rva):return b'\x48\xb8'+struct.pack('<Q',m+rva)+b'\xff\xe0'
def stub(rva,restype,args,fn):
 f=c.WINFUNCTYPE(restype,*args)(fn);callbacks.append(f);patch(rva,b'\x48\xb8'+struct.pack('<Q',c.cast(f,c.c_void_p).value)+b'\xff\xe0')
def row(data,id):
 for i in range(struct.unpack_from('<H',data,10)[0]):
  at=64+24*i
  if struct.unpack_from('<i',data,at)[0]==id:
   off=struct.unpack_from('<Q',data,at+8)[0];return c.create_string_buffer(data[off:off+1024],1024)
 raise AssertionError(id)
try:
 for s in pe.sections:c.memmove(m+s.VirtualAddress,s.get_data(),s.SizeOfRawData)
 patch(0x26909a0,b'\xc3')
 # Preserve nonvolatile registers, including XMM6 used by the native fragment.
 wrapper=bytes.fromhex('53 55 56 57 41 54 41 55 41 56 41 57 48 81 ec 88 02 00 00 48 8d ac 24 00 01 00 00 0f 29 b5 80 00 00 00 49 89 cc 49 89 d6 4c 89 45 50 c6 85 18 01 00 00 00')
 end=bytes.fromhex('0f 28 b5 80 00 00 00 48 81 c4 88 02 00 00 41 5f 41 5e 41 5d 41 5c 5f 5e 5d 5b c3')
 patch(0x1000,wrapper+jump(0x43c5c5));patch(0x43c78b,end);patch(0x43ca84,end)
 patch(0x42c120,bytes.fromhex('48 89 c8 c3'))
 stub(0x50edf0,c.c_void_p,[c.c_void_p,c.c_void_p],lambda out,hit:c.memset(out,0,16))
 def lookup(out,id):
  assert id==95001702;c.c_void_p.from_address(out).value=c.addressof(pulse);return out
 stub(0xea17e0,c.c_void_p,[c.c_void_p,c.c_int],lookup)
 delivered=[]
 def apply(target,id,source,force,context,info,*flags):
  delivered.append((target,id,source,c.c_ubyte.from_address(info+0x40).value));return 1
 stub(0x3c80a0,c.c_ubyte,[c.c_void_p,c.c_int,c.c_void_p,c.c_ubyte,c.c_void_p,c.c_void_p,*([c.c_ubyte]*6),c.c_int],apply)
 deliver=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p,c.c_void_p)(m+0x1000)
 # Full original state-0 dispatch returns without applying ailment/guard scaling.
 guard=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p,c.c_void_p,c.c_void_p)(m+0x42d110)
 hp=c.WINFUNCTYPE(None,c.c_void_p,c.c_int,c.c_void_p,c.c_void_p,c.c_void_p,c.c_ubyte)(m+0x506ec0)
 for package in ['standalone','mmv']:
  fixture=(work/package/'effects.param').read_bytes();pulse=row(fixture,95001702)
  node=c.create_string_buffer(0x88);manager=c.create_string_buffer(0x80)
  put(node,0,'<Q',c.addressof(pulse));put(node,8,'<i',95001702);put(manager,8,'<Q',c.addressof(node))
  for off in [0x58,0x5c,0x60,0x64]:put(node,off,'<f',1.)
  target=c.create_string_buffer(0x200);source=c.create_string_buffer(0x200)
  for blocked in [0,1]:
   for original_damage in [0,100,10000]:
    for cached in [0,8]:
     hit=c.create_string_buffer(0x300);put(hit,0x70,'<i',95001702);put(hit,0x280,'<i',original_damage);put(hit,0x2b4,'<B',blocked);put(hit,0x2c1,'<B',cached)
     for off in [0x29c,0xd4,0x254]:put(hit,off,'<f',1.)
     before=hit.raw;delivered.clear();deliver(target,hit,source)
     assert hit.raw==before
     assert delivered==([]if cached else[(c.addressof(target),95001702,c.addressof(source),blocked)]),delivered
     info=c.create_string_buffer(0x48);put(info,0x40,'<B',blocked);before_info=info.raw;ref=c.c_void_p(c.addressof(pulse))
     assert guard(None,c.byref(ref),info)==0 and info.raw==before_info
     cases.append(dict(package=package,guard=blocked,originalDamage=original_damage,cached=bool(cached),separateEffects=len(delivered)))
  for maximum in [1000,10001,50000]:
   for flags in [0,2,4,8]:
    put(node,0x68,'<I',flags);out=c.create_string_buffer(0x20);attacker=c.c_uint64(0xffffffffffffffff);attribute=c.c_ubyte(0xfe)
    hp(manager,maximum,c.byref(attacker),c.byref(attribute),out,0)
    lanes=struct.unpack_from('<5i',out);actual=sum(lanes[i]for i in [0,1,3]);expected=-(maximum*7//100)if flags==4 else 0
    assert actual==expected,(package,maximum,flags,lanes,expected)
    cases.append(dict(package=package,maxHp=maximum,nodeFlags=flags,hpDelta=actual))
finally:assert k.VirtualFree(m,0,0x8000)
report=dict(cases=cases,nativeCases=len(cases),gameplayVerified=False,multiplayerVerified=False,stubs=['security cookie','damage-module actor lookup','hit category context initialization','effect row lookup','final network-aware effect submission captured at 3C80A0'],nativePaths=['43C5C5 attack-effect list including zero-damage guard and cache skip','42D110 state-0 bypasses ailment guard scaling','506EC0/50B860 one-shot maximum-HP pulse'])
(work/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n');print('PASS:',len(cases),'native Balloon delivery/guard/HP cases across standalone and MMV')
