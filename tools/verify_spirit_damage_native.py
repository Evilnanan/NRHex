"""Execute native 6C2210 in isolation; virtual accessors use controlled objects."""
from pathlib import Path
import ctypes as c,hashlib,json,struct
import pefile
from capstone import Cs,CS_ARCH_X86,CS_MODE_64
root=Path(__file__).resolve().parents[1]
exe=Path(root.anchor)/'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game/nightreign.exe'
raw=exe.read_bytes();digest=hashlib.sha256(raw).hexdigest()
assert digest=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True);code=pe.get_data(0x6c2210,0x75)
instructions=list(Cs(CS_ARCH_X86,CS_MODE_64).disasm(code,0x6c2210))
assert instructions[-1].mnemonic=='ret'
assert not any('rip' in i.op_str for i in instructions)
assert [i.op_str for i in instructions if i.mnemonic=='call']==['qword ptr [rax + 0x118]','qword ptr [r8 + 0x1a0]','qword ptr [rax + 0x1a0]']
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint32,c.c_uint32];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint32]
memory=k.VirtualAlloc(None,4096,0x3000,0x40);assert memory
vtable=(c.c_size_t*64)();actor=(c.c_size_t*2)(c.addressof(vtable),0)
row=(c.c_ubyte*0x400)();reference=c.c_size_t(c.addressof(row));state=dict(player=0,reference=c.addressof(reference))
@c.WINFUNCTYPE(c.c_ubyte,c.c_void_p)
def player(_):return state['player']
@c.WINFUNCTYPE(c.c_void_p,c.c_void_p)
def npc(_):return state['reference']
vtable[0x118//8]=c.cast(player,c.c_void_p).value;vtable[0x1a0//8]=c.cast(npc,c.c_void_p).value
count=0
try:
 c.memmove(memory,code,len(code));classify=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p,c.c_void_p)(memory)
 for virtual in [0,1]:
  state['player']=virtual
  for flags in range(256):
   row[0x15d]=flags
   assert classify(None,c.addressof(actor))==((flags>>3)&1)
   count+=1
  reference.value=0
  assert classify(None,c.addressof(actor))==0;count+=1
  state['reference']=None
  assert classify(None,c.addressof(actor))==virtual;count+=1
  state['reference']=c.addressof(reference);reference.value=c.addressof(row)
 report=dict(version='0.17.46',exeSha256=digest,function='6C2210',cases=count,field='NpcParam +15D bit 3 / isCalculatePvPDamage',nativeCodeExecuted=True,virtualGettersMocked=True,gameplayVerified=False)
 out=root/'.work/chant-spirit-01746';out.mkdir(exist_ok=True)
 (out/'native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
 print(f'PASS: {count} original native damage classification cases; no live process accessed')
finally:k.VirtualFree(memory,0,0x8000)
