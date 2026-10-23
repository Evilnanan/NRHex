"""Execute current EXE NPC model and family-presence getters in isolated memory."""
from pathlib import Path
import ctypes as c,hashlib,json,struct,os
import pefile,capstone
root=Path(__file__).resolve().parents[1]
game=Path(os.environ.get('NRHEX_GAME',str(Path(root.anchor)/'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game')))
raw=(game/'nightreign.exe').read_bytes();digest=hashlib.sha256(raw).hexdigest()
assert digest=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True);base=pe.OPTIONAL_HEADER.ImageBase
# Confirm that env380 delegates to the precise family-state getter, not HP.
idx=pe.get_data(0x3fb0d4+380-223,1)[0]
assert struct.unpack('<I',pe.get_data(0x3fadf0+idx*4,4))[0]==0x3f9796
assert pe.get_data(0x3f97c2,1)==b'\xe8'
assert 0x3f97c7+struct.unpack('<i',pe.get_data(0x3f97c3,4))[0]==0x460ac0
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong,c.c_ulong];k.VirtualAlloc.restype=c.c_void_p;k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong]
arena=k.VirtualAlloc(None,0x1000,0x3000,0x40);assert arena
functions={0xe94990:(0,0x3e),0x460ac0:(0x100,0x37),0x3864c0:(0x200,0x11)}
md=capstone.Cs(capstone.CS_ARCH_X86,capstone.CS_MODE_64);md.detail=True
try:
 for rva,(offset,size) in functions.items():
  code=bytearray(pe.get_data(rva,size))
  for ins in md.disasm(bytes(code),base+rva):
   assert not any(o.type==capstone.CS_OP_MEM and o.mem.base==capstone.x86.X86_REG_RIP for o in ins.operands)
   if ins.mnemonic.startswith('j') or ins.mnemonic=='call':
    assert ins.operands[0].type==capstone.CS_OP_IMM
    target=ins.operands[0].imm-base
    if rva<=target<rva+size:continue
    assert target in functions and ins.imm_size==4
    struct.pack_into('<i',code,ins.address-base-rva+ins.imm_offset,functions[target][0]-(offset+ins.address-base-rva+ins.size))
  c.memmove(arena+offset,bytes(code),size)
 model=c.CFUNCTYPE(c.c_void_p,c.c_void_p,c.c_int)(arena)
 result=c.c_int(-1);model_cases=[]
 for npc in [-1,0,32500090,46400010,137030000,95400000,95403099]:
  assert model(c.byref(result),npc)==c.addressof(result)
  expected=-1 if npc<0 else npc//10000%10000
  assert result.value==expected;model_cases.append({'npc':npc,'nativeModel':result.value})
 family=c.CFUNCTYPE(c.c_ubyte,c.c_void_p,c.c_int)(arena+0x100)
 wrapper=c.create_string_buffer(0x18);impl=c.create_string_buffer(0x140)
 assert family(wrapper,0)==0
 struct.pack_into('<Q',wrapper,0x10,c.addressof(impl))
 for index in range(3):
  for state in range(256):
   impl[0x18+index*0x60]=state
   assert family(wrapper,index)==int(state in [10,11,20,21]),(index,state)
 report={'version':'0.17.37','exeSha256':digest,'nativeModelCases':model_cases,'env380Getter':'460AC0','familyStateCases':768,'familyPresentStates':[10,11,20,21],'nullFamilyModuleReturnsFalse':True,'gameplayVerified':False,'multiplayerVerified':False}
 (root/'dist/reports/spirit-native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
 print('PASS: original NPC model decoder and all 768 native family-state cases; env380 target verified')
finally:k.VirtualFree(arena,0,0x8000)
