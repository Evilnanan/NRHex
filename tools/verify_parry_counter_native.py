"""Execute the original weapon-behavior stamina debit in an isolated image.

The stamina calculator and final stats setter are stubs; dispatch, row access,
module lookup, negation and native AddStamina remain executable game code.
This is ABI/data verification, not an in-game guard-counter test.
"""
import ctypes as c, hashlib, json, struct
from pathlib import Path
import pefile
from local_paths import ROOT, GAME

raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
p=pefile.PE(data=raw,fast_load=True)
params=json.loads((ROOT/'.work/parry-counter/parameters.json').read_text())
assert params['offsets']==[0x4e,0x4f,0x1c8,0x1c9,0x1ca]
assert p.get_data(0x434768,5)==bytes.fromhex('48 8b ce 8b d8')
assert p.get_data(0x43473a,5)==bytes.fromhex('48 8b 44 24 58')
assert p.get_data(0x434744,3)==bytes.fromhex('8b 50 14')
assert p.get_data(0x434762,6)==bytes.fromhex('ff 90 48 03 00 00')
assert p.get_data(0x434431,19)==bytes.fromhex('44 0f b6 78 09 44 0f b6 60 1c 45 84 ff 75 0c 44 8b 70 0c')
assert p.get_data(0x4261d0,11)==bytes.fromhex('03 91 5c 01 00 00 e9 25 1d 00 00')
assert p.get_data(0x42c120,5)==bytes.fromhex('48 8b 41 08 c3')
assert struct.unpack('<Q',p.get_data(0x2c307f8+0x348,8))[0]==p.OPTIONAL_HEADER.ImageBase+0x668730
# The selector sends kind+7 to ParamRepository: NPC=7, PC=8.
assert p.get_data(0xe6d69d,11)==bytes.fromhex('8d 55 07 45 33 c0 e8 d8 0f 03 00')
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
m=k.VirtualAlloc(None,p.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert m
cases=[];observed=[]
try:
 for s in p.sections:c.memmove(m+s.VirtualAddress,s.get_data(),s.SizeOfRawData)
 player=c.create_string_buffer(0x1c0);vt=c.create_string_buffer(0x350)
 stats=c.create_string_buffer(0x160);modules=(c.c_void_p*1)(c.addressof(stats))
 attack_module=(c.c_void_p*2)(0,c.addressof(player));row=c.create_string_buffer(0x20)
 c.c_void_p.from_buffer(player).value=c.addressof(vt)
 c.c_void_p.from_buffer(player,0x1b8).value=c.addressof(modules)
 @c.WINFUNCTYPE(c.c_int,c.c_void_p,c.c_int,c.c_int,c.c_ubyte,c.c_int)
 def calculate(actor,amount,source,flags,extra):
  observed.append(('calculate',actor,amount,source,flags,extra));return amount*2+3
 @c.WINFUNCTYPE(None,c.c_void_p,c.c_int,c.c_ubyte)
 def set_stamina(ptr,value,flag):
  observed.append(('set',ptr,value,flag))
 c.c_void_p.from_buffer(vt,0x348).value=c.cast(calculate,c.c_void_p).value
 setter=b'\x48\xb8'+struct.pack('<Q',c.cast(set_stamina,c.c_void_p).value)+b'\xff\xe0'
 c.memmove(m+0x427f00,setter,len(setter))
 # Native call-site stack is 16-byte aligned. Preserve nonvolatile registers
 # and provide its real +58 row / RBP arguments / RSI module / R13D flag.
 wrapper=bytes.fromhex('53 55 56 41 55 48 81 ec 28 01 00 00 48 8b f1 48 8d 6c 24 40 48 89 54 24 58 49 8b c0')
 wrapper+=bytes.fromhex('c7 85 98 00 00 00 fe ff ff ff c6 85 a8 00 00 00 81 41 bd 07 00 00 00')
 wrapper+=b'\x49\xbb'+struct.pack('<Q',m+0x434737)+b'\x41\xff\xe3'
 c.memmove(m+0x1000,wrapper,len(wrapper))
 end=bytes.fromhex('48 81 c4 28 01 00 00 41 5d 5e 5d 5b c3')
 c.memmove(m+0x434788,end,len(end))
 run=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p,c.c_void_p)(m+0x1000)
 for cost in [0,1,20,35,100]:
  for present in [False,True]:
   observed.clear();c.c_int.from_buffer(stats,0x15c).value=150
   c.c_int.from_buffer(row,0x14).value=cost
   run(c.addressof(attack_module),c.addressof(row) if present else 0,c.addressof(player))
   amount=cost if present else 0
   assert observed==[('calculate',c.addressof(player),amount,-2,0x81,7),('set',c.addressof(stats),150-(amount*2+3),0)],observed
   cases.append(dict(behaviorPresent=present,baseCost=amount,finalDebit=amount*2+3))
finally:k.VirtualFree(m,0,0x8000)
report=dict(nativeCases=len(cases),cases=cases,parameters=params,hookAfterNativeCalculator=True,staminaSetterStubbed=True,gameplayVerified=False)
(ROOT/'.work/parry-counter/native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f'PASS: {len(cases)} original debit-path cases; current behavior row, five calculator arguments, signed debit and stats recipient verified')
