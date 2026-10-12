"""Exercise native force-death dispatch and unconditional HP setter in isolation.

The full death cleanup is a controlled callback, not a simulated game scene.
No live process is accessed. Rendering, network death and actual buffs still
require gameplay verification.
"""
from pathlib import Path
import ctypes as c,hashlib,json,struct,os
import pefile,capstone
root=Path(__file__).resolve().parents[1]
game=Path(os.environ.get('NRHEX_GAME',str(Path(root.anchor)/'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game')))
raw=(game/'nightreign.exe').read_bytes();digest=hashlib.sha256(raw).hexdigest()
assert digest=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True);base=pe.OPTIONAL_HEADER.ImageBase
md=capstone.Cs(capstone.CS_ARCH_X86,capstone.CS_MODE_64);md.detail=True
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong,c.c_ulong];k.VirtualAlloc.restype=c.c_void_p;k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong]
arena=k.VirtualAlloc(None,0x8000,0x3000,0x40);assert arena
functions={0x3d5010:(0,0x1ad),0x426b00:(0x400,0x238)}
data={};stubs={};next_offset=0x1000
def alloc():
 global next_offset
 result=next_offset;next_offset+=0x20;assert next_offset<0x8000;return result
try:
 for rva,(offset,size) in functions.items():
  code=bytearray(pe.get_data(rva,size))
  for ins in md.disasm(bytes(code),base+rva):
   at=ins.address-base-rva
   for operand in ins.operands:
    if operand.type==capstone.CS_OP_MEM and operand.mem.base==capstone.x86.X86_REG_RIP:
     target=ins.address+ins.size+operand.mem.disp-base
     if target not in data:data[target]=alloc();c.memmove(arena+data[target],pe.get_data(target,16),16)
     struct.pack_into('<i',code,at+ins.disp_offset,data[target]-(offset+at+ins.size))
   if (ins.mnemonic.startswith('j') or ins.mnemonic=='call') and ins.operands[0].type==capstone.CS_OP_IMM:
    target=ins.operands[0].imm-base
    if rva<=target<rva+size:continue
    assert ins.imm_size==4
    if target not in stubs:
     stubs[target]=alloc();c.memmove(arena+stubs[target],b'\x31\xc0\x0f\x57\xc0\xc3',6)
    struct.pack_into('<i',code,at+ins.imm_offset,stubs[target]-(offset+at+ins.size))
  c.memmove(arena+offset,bytes(code),len(code))
 # End the player's secondary red-HP presentation branch after the native
 # setter has performed its unconditional HP store. NPC deaths do not need it.
 c.memmove(arena+stubs[0x6ada30],b'\xb0\x01\xc3',3)
 hp=c.CFUNCTYPE(None,c.c_void_p,c.c_int,c.c_ubyte,c.c_float,c.c_float,c.c_ubyte)(arena+0x400)
 actor=c.create_string_buffer(0x600);stats=c.create_string_buffer(0x180)
 vtable=c.create_string_buffer(0x500);reference=c.create_string_buffer(8);npc=c.create_string_buffer(0x240);modules=c.create_string_buffer(0x70)
 struct.pack_into('<Q',actor,0,c.addressof(vtable));struct.pack_into('<Q',actor,0x1b8,c.addressof(modules));struct.pack_into('<Q',modules,0x68,1);struct.pack_into('<Q',reference,0,c.addressof(npc))
 calls=[]
 @c.CFUNCTYPE(c.c_void_p,c.c_void_p)
 def npc_ref(_):return c.addressof(reference)
 @c.CFUNCTYPE(None,c.c_void_p)
 def cleanup(value):
  calls.append(value);hp(stats,0,0,0.,1.,1)
 struct.pack_into('<Q',vtable,0x1a0,c.cast(npc_ref,c.c_void_p).value)
 jump=b'\x48\xb8'+struct.pack('<Q',c.cast(cleanup,c.c_void_p).value)+b'\xff\xe0'
 c.memmove(arena+stubs[0x3dd3d0],jump,len(jump))
 kill=c.CFUNCTYPE(None,c.c_void_p,c.c_ubyte,c.c_ubyte)(arena)
 cases=0
 for special in [0,2]:
  npc[0x22d]=special
  for flags in range(256):
   actor[0x1f4]=0;actor[0x1f7]=flags;actor[0x1f9]=flags
   struct.pack_into('<ii',stats,0x140,10000,10000)
   before=len(calls);kill(actor,0,0)
   assert len(calls)==before+1 and calls[-1]==c.addressof(actor)
   assert struct.unpack_from('<i',stats,0x140)[0]==0
   assert actor.raw[0x1f4]&6==6
   cases+=1
 # Native family cleanup itself uses actor, false, false at this call site.
 assert pe.get_data(0x386681,1)==b'\xe8'
 assert 0x386686+struct.unpack('<i',pe.get_data(0x386682,4))[0]==0x3d5010
 report=dict(version='0.17.39',exeSha256=digest,nativeKillRva='3D5010',nativeHpSetterRva='426B00',nativeFamilyCallerRva='386681',cases=cases,zeroHpUnconditional=True,rewardFlagsDisabled=True,controlledDependencies=['death cleanup callback','red-HP presentation gate','fall-height getter returns zero','security-cookie check'],gameplayVerified=False,multiplayerVerified=False)
 out=root/'.work/spirit-ultimate-01739';out.mkdir(parents=True,exist_ok=True)
 (out/'native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
 print(f'PASS: {cases} native forced-death dispatch cases and original zero-HP setter; gameplay not inferred')
finally:k.VirtualFree(arena,0,0x8000)
