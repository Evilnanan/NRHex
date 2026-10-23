"""Execute original env366 getter paths in a guarded private memory fixture."""
from pathlib import Path
import ctypes as c,hashlib,json,struct
import pefile,capstone
root=Path(__file__).resolve().parents[1]
raw=Path(r'D:\Program Files (x86)\Steam\steamapps\common\ELDEN RING NIGHTREIGN\Game\nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True);base=pe.OPTIONAL_HEADER.ImageBase
def read(rva,n):return raw[pe.get_offset_from_rva(rva):pe.get_offset_from_rva(rva)+n]
assert read(0xba7000,0x25)==bytes.fromhex('48 8b 41 18 48 85 c0 74 08 48 8b c8 e9 3f f4 ff ff 48 8b 89 60 02 00 00 48 85 c9 0f 85 ff c9 fe ff 0f 57 c0 c3')
assert 0x3fa4c0+7+struct.unpack('<i',read(0x3fa4c0+3,4))[0]==0x3c17d48
assert read(0xe434b9,10)==bytes.fromhex('83 78 08 2e 75 05 48 8b 40 58')
assert read(0xe42dc9,10)==bytes.fromhex('83 78 08 3c 75 05 48 8b 40 58')
k=c.WinDLL('kernel32',use_last_error=True);k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong,c.c_ulong];k.VirtualAlloc.restype=c.c_void_p;k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong]
arena=k.VirtualAlloc(None,0x1000,0x3000,0x40);assert arena
functions={0xba7000:(0,0x25),0xba6450:(0x100,0x2e),0xb93a20:(0x200,0x2e),0xe434b0:(0x300,0x17),0xe42dc0:(0x400,0x17)}
decoder=capstone.Cs(capstone.CS_ARCH_X86,capstone.CS_MODE_64);decoder.detail=True
cases=[]
try:
 for rva,(offset,size) in functions.items():
  code=bytearray(read(rva,size))
  for ins in decoder.disasm(bytes(code),base+rva):
   if ins.mnemonic in ['call','jmp','jne','je'] and ins.operands[0].type==capstone.CS_OP_IMM:
    target=ins.operands[0].imm-base
    if rva<=target<rva+size:continue
    assert target in functions,hex(target)
    displacement=arena+functions[target][0]-(arena+offset+ins.address-base-rva+ins.size)
    assert ins.imm_size==4;struct.pack_into('<i',code,ins.address-base-rva+ins.imm_offset,displacement)
   assert not any(o.type==capstone.CS_OP_MEM and o.mem.base==capstone.x86.X86_REG_RIP for o in ins.operands)
  c.memmove(arena+offset,bytes(code),size)
 getter=c.CFUNCTYPE(c.c_float,c.c_void_p)(arena)
 for primary_type,fallback_type,height,expected in [(None,None,48.,0.),(46,None,48.,48.),(None,60,47.,47.),(52,None,48.,0.),(None,46,47.,0.),(52,60,47.,0.),(46,None,0.,0.),(46,None,-5.,-5.)]:
  context=c.create_string_buffer(0x300);primary=c.create_string_buffer(0x40);fallback=c.create_string_buffer(0x40);meta1=c.create_string_buffer(0x60);meta2=c.create_string_buffer(0x60);data=c.create_string_buffer(4);struct.pack_into('<f',data,0,height)
  for slot,wrapper,meta,region_type in [(0x18,primary,meta1,primary_type),(0x260,fallback,meta2,fallback_type)]:
   if region_type is None:continue
   struct.pack_into('<Q',context,slot,c.addressof(wrapper));struct.pack_into('<Q',wrapper,0x18,c.addressof(meta));struct.pack_into('<i',meta,8,region_type);struct.pack_into('<Q',meta,0x58,c.addressof(data))
  result=getter(c.addressof(context));assert abs(result-expected)<1e-5
  cases.append(dict(primaryType=primary_type,fallbackType=fallback_type,configuredHeight=height,nativeResult=result,springAvailable=result>0))
finally:k.VirtualFree(arena,0,0x8000)
report=dict(version='0.10.10',exeSha256=hashlib.sha256(raw).hexdigest(),scriptEnvironmentId=366,regionContextRva='0x3C17D48',nativeGetterRva='0xBA7000',normalRegionType=46,sealableRegionType=60,isolatedOriginalMachineCodeCases=cases,configuredHeightUsedOnlyForLaunchDetection=True,liveSpiritspringFlightVerified=False)
(root/'dist/reports/spiritspring-native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print('PASS: original spring getter, context pointer binding, ordinary/sealable/absent/wrong-type paths; eight isolated native cases.')
