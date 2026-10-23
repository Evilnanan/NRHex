"""Guarded original env(224) getter executed in an isolated memory fixture."""
import ctypes as c,hashlib,json,struct,tomllib
import pefile,capstone
from local_paths import GAME, ROOT as root
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True);base=pe.OPTIONAL_HEADER.ImageBase
def read(rva,n):return raw[pe.get_offset_from_rva(rva):pe.get_offset_from_rva(rva)+n]
idx=read(0x3fb0d4+1,1)[0];assert struct.unpack('<I',read(0x3fadf0+idx*4,4))[0]==0x3f81dc
assert read(0x3f81dc,19)==bytes.fromhex('48 8b 06 48 8b 88 b8 01 00 00 48 8b 49 70 e8 81 9e 04 00')
assert struct.unpack('<f',read(0x3f81ef+8+0x2efb505,4))[0]==100.
assert read(0x466470,11)==bytes.fromhex('0f 10 41 70 48 8b c2 0f 29 02 c3')
original=json.loads((root/'data/reference/combat/jump-parameters.json').read_text(encoding='utf-8'))
assert all(row['fields']['magicSubCategoryChange1']==102 for row in original['nativeJumpEffects'])
k=c.WinDLL('kernel32',use_last_error=True)
k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong,c.c_ulong];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong]
arena=k.VirtualAlloc(None,0x4000,0x3000,0x40);assert arena
def put(offset,data):c.memmove(arena+offset,data,len(data))
put(0x400,bytes.fromhex('48 8b 41 08 c3')) # 42C120: verified ChrModule owner getter.
put(0x420,read(0x466470,11))
put(0x440,bytes.fromhex('31 c0 c3')) # Actor-specific flag absent in fixture.
put(0x460,b'\xc3') # Stack cookie check irrelevant in private fixture.
calls={0x42c120:0x400,0x466470:0x420,0x4271b0:0x440,0x26909a0:0x460}
decoder=capstone.Cs(capstone.CS_ARCH_X86,capstone.CS_MODE_64);decoder.detail=True
code=bytearray(read(0x442070,0x9c));constant_offset=0x800
for ins in decoder.disasm(bytes(code),base+0x442070):
 offset=ins.address-base-0x442070
 if ins.mnemonic=='call' and ins.operands[0].type==capstone.CS_OP_IMM:
  target=ins.operands[0].imm-base;assert target in calls,hex(target)
  struct.pack_into('<i',code,offset+1,arena+calls[target]-(arena+offset+ins.size))
 for op in ins.operands:
  if op.type==capstone.CS_OP_MEM and op.mem.base==capstone.x86.X86_REG_RIP:
   target=ins.address+ins.size+op.mem.disp-base
   put(constant_offset,read(target,16))
   struct.pack_into('<i',code,offset+ins.disp_offset,arena+constant_offset-(arena+offset+ins.size));constant_offset+=16
put(0,bytes(code))
player=c.create_string_buffer(0x200);modules=c.create_string_buffer(0x100);physics=c.create_string_buffer(0x200);fall=c.create_string_buffer(0x40)
def q(buffer,offset,value):struct.pack_into('<Q',buffer,offset,value)
q(fall,8,c.addressof(player));q(player,0x1b8,c.addressof(modules));q(modules,0,c.addressof(player));q(modules,0x68,c.addressof(physics))
getter=c.CFUNCTYPE(c.c_float,c.c_void_p)(arena)
cases=[]
try:
 for y,height in [(100.,0.),(100.,3.75),(100.,7.5),(100.,15.),(100.,30.),(-100.,60.)]:
  struct.pack_into('<ffff',physics,0x70,0.,y,0.,1.);struct.pack_into('<ffff',physics,0x160,0.,y+height,0.,1.)
  result=getter(c.addressof(fall));assert abs(result-height)<1e-5
  cases.append(dict(apex=y+height,current=y,returnedMetres=result))
finally:k.VirtualFree(arena,0,0x8000)
version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
report=dict(version=version,nativeJumpCategory=102,nativeFallHeightGetter='0x442070',scriptEnvironmentId=224,worldMetresToScriptCentimetres=100,isolatedNativeHeightCases=cases,heightBaselineMetres=1,formula='max(0, 1.00 * (1 + log2(h / 1)))',nativeHeightOnlyOutsideSpiritspring=True,spiritspringUsesActualTrackedApex=True,landingGraceMilliseconds=250,groundedSamplesRenewGrace=False,multiHitUsesFirstConnectedHitHeight=False,eachHitUsesCurrentHeight=True,currentGroundFlagOffset='0x92',previousGroundFlagOffset='0x93',gameplayVerified=False,multiplayerVerified=False)
(root/'dist/reports/cat-jump-native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print('PASS: original category 102, env224, metre conversion, and six isolated native fall-height cases.')
