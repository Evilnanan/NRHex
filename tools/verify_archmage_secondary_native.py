"""Execute the original selection and FP-cost routines in an isolated PE image."""
import ctypes as c,hashlib,json,struct,math
import pefile
from local_paths import GAME,ROOT
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw);k=c.WinDLL('kernel32')
k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
mem=k.VirtualAlloc(None,pe.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert mem
callbacks=[]
def address(b):return c.addressof(b)
def put(b,off,fmt,v):struct.pack_into(fmt,b,off,v)
def callback(result,args,fn):
 f=c.WINFUNCTYPE(result,*args)(fn);callbacks.append(f);return c.cast(f,c.c_void_p).value
def stub(rva,result,args,fn):c.memmove(mem+rva,b'\x48\xb8'+struct.pack('<Q',callback(result,args,fn))+b'\xff\xe0',12)
def direct(ret,target):
 code=pe.get_data(ret-5,5);assert code[0]==0xe8 and ret+struct.unpack('<i',code[1:])[0]==target
try:
 c.memmove(mem,raw[:pe.OPTIONAL_HEADER.SizeOfHeaders],pe.OPTIONAL_HEADER.SizeOfHeaders)
 for sec in pe.sections:c.memmove(mem+sec.VirtualAddress,sec.get_data(),sec.SizeOfRawData)
 for block in pe.DIRECTORY_ENTRY_BASERELOC:
  for rel in block.entries:
   if rel.type==10:
    v=c.c_uint64.from_address(mem+rel.rva);v.value=(v.value+mem-pe.OPTIONAL_HEADER.ImageBase)&0xffffffffffffffff
   else:assert rel.type==0
 for ret in [0x6d15b6,0x38499e,0x6c5eab]:direct(ret,0x6c5ec0)
 direct(0x3477a5,0x6c5e80);direct(0x66ac24,0x6a1e10)
 actor=c.create_string_buffer(0x800);modules=c.create_string_buffer(0x200);magic=c.create_string_buffer(0x40);vt=c.create_string_buffer(0x40);network=c.create_string_buffer(0x10);nvt=c.create_string_buffer(0x90)
 put(actor,0x1b8,'<Q',address(modules));put(modules,0x60,'<Q',address(magic));put(modules,0x180,'<Q',address(magic));put(magic,0,'<Q',address(vt));put(actor,0x760,'<Q',address(network));put(network,0,'<Q',address(nvt))
 def get_magic(module,out):c.c_int.from_address(out).value=c.c_int.from_address(module+8).value;return out
 def set_magic(module,spell):c.c_int.from_address(module+8).value=spell
 put(vt,0x18,'<Q',callback(c.c_void_p,[c.c_void_p,c.c_void_p],get_magic));put(vt,0x20,'<Q',callback(None,[c.c_void_p,c.c_int],set_magic));put(vt,0x30,'<Q',callback(c.c_ubyte,[c.c_void_p],lambda *_:0));put(nvt,0x88,'<Q',callback(None,[c.c_void_p],lambda *_:None))
 selection=[]
 def hand(player,out,index):selection.append(('hand',index));c.c_uint.from_address(out).value=index+1;return out
 def reference(out,handle):c.c_uint.from_address(out).value=c.c_uint.from_address(handle).value;return out
 def spell(ref,slot):selection.append(('slot',slot));return 4000+100*c.c_uint.from_address(ref).value+slot
 stub(0x3cefe0,c.c_void_p,[c.c_void_p,c.c_void_p,c.c_uint],hand)
 stub(0x6a1d70,c.c_void_p,[c.c_void_p,c.c_void_p],reference)
 stub(0x6a1d90,None,[c.c_void_p],lambda *_:None)
 stub(0x6a1e10,c.c_uint64,[c.c_void_p,c.c_uint],spell)
 stub(0x3d42a0,c.c_ubyte,[c.c_void_p],lambda *_:0)
 stub(0x45c5f0,c.c_int,[c.c_void_p],lambda *_:8000)
 select=c.WINFUNCTYPE(None,c.c_void_p,c.c_uint,c.c_uint,c.c_ubyte)(mem+0x66ab90)
 selection_cases=[]
 for h in [0,1]:
  for slot in [0,1]:
   selection.clear();select(address(actor),h,slot,0)
   assert selection==[('hand',h),('slot',slot)]
   actual=c.c_int.from_address(address(magic)+8).value;assert actual==4000+100*(h+1)+slot
   selection_cases.append(dict(hand=h,slot=slot,activeSpell=actual))
 selection.clear();select(address(actor),1,0,1);assert not selection and c.c_int.from_address(address(magic)+8).value==8000
 # Preserve actual uncharged/charged field selection; replace only external
 # effect multiplier queries and rounding with controlled fixtures.
 rates=[1.,1.,1.]
 for kind,rva in enumerate([0x507310,0x507410,0x5081e0]):stub(rva,c.c_float,[c.c_void_p],lambda _,i=kind:rates[i])
 stub(0x217e710,c.c_float,[c.c_float],lambda v:float(math.ceil(v)))
 row=c.create_string_buffer(0x100);ref=c.create_string_buffer(0x10);put(ref,8,'<Q',address(row));put(row,0x10,'<h',20);put(row,0x5c,'<h',30)
 cost=c.WINFUNCTYPE(c.c_uint64,c.c_void_p,c.c_void_p,c.c_ubyte)(mem+0x6c5ec0);cost_cases=[]
 for kind in [0,1,2]:
  put(row,0x26,'<B',kind)
  for rate in [0.,.5,1.,2.]:
   rates[:]=[rate]*3
   for charge in [0,1]:
    actual=cost(address(ref),address(actor),charge);expected=int((20 if charge==0 else 30)*rate);assert actual==expected
    cost_cases.append(dict(kind=kind,rate=rate,charged=charge,cost=actual))
 rates[:]=[0.]*3;assert cost(address(ref),None,0)==20
 report=dict(exeSha256=hashlib.sha256(raw).hexdigest(),selectionCases=selection_cases,skillBypassesCatalystSlots=True,costCases=cost_cases,normalFpOffset='0x10',chargedFpOffset='0x5c',gameplayCostCallersVerified=True,stubs=['equipment references','magic virtuals and network refresh','effect multiplier queries','ceil'],gameMemoryWritten=False,gameplayVerified=False,multiplayerVerified=False)
 out=ROOT/'.work/archmage-secondary';out.mkdir(parents=True,exist_ok=True);(out/'native-validation.json').write_text(json.dumps(report,indent=2))
 print('PASS: native hand/slot selection, skill separation, 24 FP cost cases and gameplay call sites')
finally:k.VirtualFree(mem,0,0x8000)
