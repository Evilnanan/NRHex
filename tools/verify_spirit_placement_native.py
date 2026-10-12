"""Run the original DB0B80 wrapper with a controlled physics worker.
Does not query game geometry or access the live process.
"""
from pathlib import Path
import ctypes as c, hashlib, json, struct
import pefile
from capstone import Cs,CS_ARCH_X86,CS_MODE_64
root=Path(__file__).resolve().parents[1]
exe=Path(root.anchor)/'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game/nightreign.exe'
raw=exe.read_bytes();digest=hashlib.sha256(raw).hexdigest()
assert digest=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True);code=bytearray(pe.get_data(0xdb0b80,0x111))
md=Cs(CS_ARCH_X86,CS_MODE_64)
calls=[i for i in md.disasm(code,0xdb0b80) if i.mnemonic=='call']
assert len(calls)==1 and calls[0].address==0xdb0c2f and calls[0].op_str=='0xdb04a0'
assert not any('rip' in i.op_str for i in md.disasm(code,0xdb0b80))
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint32,c.c_uint32];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint32]
arena=k.VirtualAlloc(None,4096,0x3000,0x40);assert arena
F=c.POINTER(c.c_float);U=c.POINTER(c.c_uint32)
Proto=c.WINFUNCTYPE(c.c_void_p,c.c_void_p,U,c.c_uint32,F,F,c.c_float,c.c_void_p,U,F,F,F,F)
errors=[];expected={};count=0
@Proto
def worker(manager,hit,mask,start,motion,radius,ignore,flags,contact,normal,fraction,extra):
 global count
 try:
  assert manager==123 and mask==0x79 and ignore is None
  assert abs(radius-expected['radius'])<1e-5
  assert list(start[:4])==expected['start'] and list(motion[:4])==[0.,-.25,0.,0.]
  flags[0]=0x12345678;fraction[0]=.25
  for p,values in [(contact,[1.,2.,3.,4.]),(normal,[5.,6.,7.,8.]),(extra,[9.,10.,11.,12.])]:
   for i,v in enumerate(values):p[i]=v
  hit[0]=expected['hit'];count+=1
 except BaseException as error:errors.append(repr(error))
 return c.addressof(hit.contents)
try:
 # Redirect only the wrapper's one direct call to a local callback trampoline.
 off=0xdb0c2f-0xdb0b80;struct.pack_into('<i',code,off+1,0x300-(0x100+off+5))
 c.memmove(arena+0x100,bytes(code),len(code))
 stub=b'\x48\xb8'+struct.pack('<Q',c.cast(worker,c.c_void_p).value)+b'\xff\xe0'
 c.memmove(arena+0x300,stub,len(stub));sweep=Proto(arena+0x100)
 for radius in [.4,.8,1.5,2.,3.,5.,8.,12.]:
  for hitid in [0xffffff,17]:
   expected=dict(radius=radius,start=[1.,float(c.c_float(2.+radius+.5).value),8.,1.],hit=hitid)
   start=(c.c_float*4)(*expected['start']);motion=(c.c_float*4)(0.,-.25,0.,0.)
   hit=c.c_uint32(0);flags=(c.c_uint32*3)(0xabcdef01,0,0xabcdef02);fraction=c.c_float(0)
   contact=(c.c_float*4)();normal=(c.c_float*4)();extra=(c.c_float*4)()
   ret=sweep(123,c.byref(hit),0x79,start,motion,radius,None,c.cast(c.byref(flags,4),U),contact,normal,c.byref(fraction),extra)
   assert not errors,errors
   assert ret==c.addressof(hit) and hit.value==hitid
   assert list(flags)==[0xabcdef01,0x12345678,0xabcdef02] and fraction.value==.25
   assert list(contact)==([1.,2.,3.,4.] if hitid==17 else [0.]*4)
   assert list(normal)==([5.,6.,7.,8.] if hitid==17 else [0.]*4)
   assert list(extra)==([9.,10.,11.,12.] if hitid==17 else [0.]*4)
 report=dict(version='0.17.44',exeSha256=digest,wrapper='DB0B80',worker='DB04A0 mocked',cases=count,gameplayVerified=False,scope='Actual wrapper/12-argument ABI/output widths, no real physics or gameplay')
 out=root/'.work/spirit-placement-01744';out.mkdir(exist_ok=True)
 (out/'native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
 print(f'PASS: {count} original sphere-sweep wrapper ABI cases; physics mocked')
finally:k.VirtualFree(arena,0,0x8000)
