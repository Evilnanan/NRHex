"""Execute native selection/Leave eligibility/item reads with isolated storage.

Only item-parameter lookup and UID lookup are fixture callbacks. The real
selector, list getters, Leave predicate and ItemData copy run unchanged.
No live process is opened and no game files are modified.
"""
from pathlib import Path
import ctypes as c,hashlib,json,struct,pefile
from local_paths import ROOT,GAME

raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p;k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
image=k.VirtualAlloc(None,0x1000000,0x3000,0x40);assert image
def put(buf,off,fmt,value):struct.pack_into(fmt,buf,off,value)
def install(rva,code):c.memmove(image+rva,code,len(code))
def jump(rva,callback):install(rva,b'\x48\xb8'+struct.pack('<Q',c.cast(callback,c.c_void_p).value)+b'\xff\xe0')
cases=[]
try:
 vt_data=struct.unpack('<8Q',pe.get_data(0x2c5afe8,64));count_rva=vt_data[1]-pe.OPTIONAL_HEADER.ImageBase
 # Count method is a leaf; selectors return pointers into an aligned array.
 for rva,size in [(0x9c8450,0x5f),(0x781a50,7),(0x9c6f20,0x1e),(count_rva,16),(0x9c7fc0,0xbd),(0x570fc0,0x53)]:install(rva,pe.get_data(rva,size))
 selected=c.WINFUNCTYPE(c.c_void_p,c.c_void_p)(image+0x9c8450)
 enabled=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p)(image+0x9c7fc0)
 read_item=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p,c.c_void_p,c.c_uint)(image+0x570fc0)
 vt=(c.c_uint64*8)();vt[1]=image+count_rva;vt[4]=vt[5]=image+0x9c6f20
 dialog=c.create_string_buffer(0x1d28);dialog_ptr=c.addressof(dialog);first=(dialog_ptr+0x1770+15)&~7
 put(dialog,0x1770,'<Q',c.addressof(vt));put(dialog,0x1770+0x358,'<Q',3)
 functor=(c.c_uint64*2)(0,dialog_ptr);param=c.create_string_buffer(0x100)
 hex_good=0x40000000|95000400
 @c.WINFUNCTYPE(None,c.c_void_p,c.c_uint)
 def resolve(out,id):
  c.c_void_p.from_address(out+8).value=c.addressof(param) if id in [95000400,100] else None
 jump(0xe8d840,resolve)
 for i in range(3):
  put(dialog,first-dialog_ptr+i*0x118+8,'<I',0x2002+i*0x20)
  put(dialog,first-dialog_ptr+i*0x118+0x64,'<I',hex_good)
 for index in [-1,0,1,2,3,7]:
  put(dialog,0xe20+0xd4,'<i',index)
  wanted=first+index*0x118 if 0<=index<3 else None
  before=dialog.raw;assert selected(dialog_ptr)==wanted;assert dialog.raw==before
  cases.append(dict(nativeSelector=index,valid=wanted is not None))
 for index in range(3):
  put(dialog,0xe20+0xd4,'<i',index)
  for good,kind,wanted in [(hex_good,11,False),(0x40000000|100,11,False),(0x40000000|100,0,True),(100,11,True)]:
   put(dialog,first-dialog_ptr+index*0x118+0x64,'<I',good);put(param,0x3e,'<B',kind)
   assert bool(enabled(c.addressof(functor)))==wanted
   cases.append(dict(originalLeave=index,good=good,goodsType=kind,enabled=wanted))
 node=c.create_string_buffer(0x100);node_ptr=c.addressof(node);uid=0x2022
 @c.WINFUNCTYPE(c.c_void_p,c.c_void_p,c.c_uint)
 def lookup(manager,requested):return node_ptr if requested==uid else None
 jump(0x582680,lookup)
 put(node,0x20,'<B',1);put(node,0x78,'<I',uid)
 for good in [hex_good,0x40000000|95002300,0x40000000|95002600]:
  put(node,0x28,'<I',good);out=c.create_string_buffer(0x60);before=node.raw
  assert read_item(1,c.addressof(out),uid)==1 and struct.unpack_from('<I',out.raw)[0]==good
  assert node.raw==before and out.raw[:0x50]==node.raw[0x28:0x78]
  cases.append(dict(nativeItemRead=good,uid=uid))
 put(node,0x20,'<B',0);assert read_item(1,c.addressof(out),uid)==0;assert read_item(1,c.addressof(out),uid+1)==0
 # Registration still uses native action 209 and native Leave text 120600.
 from capstone import Cs,CS_ARCH_X86,CS_MODE_64
 instructions=list(Cs(CS_ARCH_X86,CS_MODE_64).disasm(pe.get_data(0x7a3210,0xc0),0x7a3210))
 assert any(i.mnemonic=='mov' and i.op_str=='edx, 0x1d718' for i in instructions)
 assert any(i.mnemonic=='mov' and i.op_str=='edx, 0xd1' for i in instructions)
finally:k.VirtualFree(image,0,0x8000)
report=dict(nativeCases=len(cases)+2,cases=cases,originalNativeInstructionsExecuted=True,onlyLookupDependenciesStubbed=True,nativeLeaveAction=209,nativeLeaveText=120600,slotStride=0x118,goodsOffset=0x64,nodeGoodsOffset=0x28,gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
out=ROOT/'.work/hex-reroll/native-validation.json';out.write_text(json.dumps(report,indent=2),encoding='utf8')
print(f'PASS: {report["nativeCases"]} isolated native menu/Leave/item-data cases; original weapon Leave and action 209 verified')
