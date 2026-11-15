"""Execute native TAE source and bow->ammo conversion in private memory only."""
import ctypes as c
import hashlib
import json
import struct
from pathlib import Path
import pefile
from local_paths import ROOT,GAME

raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
assert pe.get_data(0x66b2d0,5)==bytes.fromhex('48 89 5c 24 08')
assert pe.get_data(0x66b346,7)==bytes.fromhex('0f b6 80 e6 00 00 00')
assert pe.get_data(0x416d20,6)==bytes.fromhex('ff 90 60 02 00 00')
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
base=k.VirtualAlloc(None,pe.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert base
callbacks=[];cases=[]
def put(buf,off,fmt,value):struct.pack_into(fmt,buf,off,value)
def stub(rva,result,args,fn):
 callback=c.WINFUNCTYPE(result,*args)(fn);callbacks.append(callback)
 code=b'\x48\xb8'+struct.pack('<Q',c.cast(callback,c.c_void_p).value)+b'\xff\xe0'
 c.memmove(base+rva,code,len(code))
try:
 for section in pe.sections:c.memmove(base+section.VirtualAddress,section.get_data(),section.SizeOfRawData)
 actor=c.create_string_buffer(0x710);equip=c.create_string_buffer(0x120);vt=c.create_string_buffer(0x330)
 module=c.create_string_buffer(0x30);row=c.create_string_buffer(0x300)
 put(actor,0,'<Q',c.addressof(vt));put(actor,0x670,'<Q',c.addressof(equip));put(module,0x18,'<Q',c.addressof(actor))
 put(vt,0x260,'<Q',base+0x66b8c0)
 @c.WINFUNCTYPE(c.c_int,c.c_void_p,c.c_int)
 def weapon(a,source):return 40010000
 callbacks.append(weapon);put(vt,0x250,'<Q',c.cast(weapon,c.c_void_p).value)
 stub(0xea9a60,None,[c.c_void_p,c.c_int],lambda out,kind:None)
 def lookup(out,weapon_id):
  c.c_void_p.from_address(out+8).value=c.addressof(row)
  return out
 stub(0xea7fc0,c.c_void_p,[c.c_void_p,c.c_int],lookup)
 tae=c.WINFUNCTYPE(c.c_int,c.c_void_p,c.c_ubyte,c.c_void_p)(base+0x416d10)
 ammo=c.WINFUNCTYPE(c.c_int,c.c_void_p,c.c_int)(base+0x66b2d0)
 for style in [1,2,3]:
  put(equip,0,'<i',style)
  for attack_hand in [0,1]:
   put(actor,0x704,'<i',attack_hand)
   for override in [0,1,2]:
    source=tae(c.addressof(module),override,None)
    expected=(-2+attack_hand) if override==0 else (-1 if override==1 else -2)
    assert source==expected
    for category in [10,11,13,0]:
     put(row,0xe6,'<B',category)
     resolved=ammo(c.addressof(actor),source)
     want=source-4 if category==10 else source-2 if category==11 else source
     assert resolved==want,(style,attack_hand,override,category,source,resolved,want)
     cases.append(dict(style=style,attackHand=attack_hand,taeOverride=override,category=category,weaponSource=source,ammoSource=resolved))
 output=ROOT/'work/arts-bow-live/native-validation.json';output.parent.mkdir(parents=True,exist_ok=True)
 output.write_text(json.dumps(dict(nativeCodeExecuted=True,cases=len(cases),taeSourceRva='416D10',bowConversionRva='66B2D0',leftBowSource=-6,rightBowSource=-5,independentOfEquipmentStyle=True,gameplayVerified=False,results=cases),indent=2),encoding='utf8')
 print(f'PASS: {len(cases)} native TAE/ammunition cases; arrows preserve firing hand independently of equipment style.')
finally:k.VirtualFree(base,0,0x8000)
