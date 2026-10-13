"""Validate/execute the current native spell getter without a game process."""
from pathlib import Path
import ctypes as c, hashlib, json, re, struct, sys
import pefile
from local_paths import ROOT, GAME

raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
def read(rva,n):
 off=pe.get_offset_from_rva(rva);return raw[off:off+n]
def check(rva,hex_):
 b=bytes.fromhex(hex_);assert read(rva,len(b))==b,hex(rva)
def call(rva,target):
 assert read(rva,1)==b'\xe8' and rva+5+struct.unpack('<i',read(rva+1,4))[0]==target

check(0x6a1e10,'48 89 6C 24 10 48 89 74 24 18 57 48 83 EC 50 48 8B 71 08 48 8B E9 8B FA 48 85 F6 0F 84 C5 00 00 00')
call(0x6a1e45,0x6a1f10)
check(0x6a1ee4,'83 FF 02 73 06 8B 44 BE 2C')
check(0x6a1f10,'83 FA 02 73 07 8B C2 8B 44 81 2C C3')
# Current-spell assignment, spell bullet lookup and original serialization.
call(0x66ac1f,0x6a1e10);call(0x82fa80,0x6a1e10);call(0x6c1806,0x6a1e10)
check(0x6c17fa,'49 8D 7E 14');check(0x6c180b,'89 07 FF C3 48 8D 7F 04 83 FB 02')
source=(ROOT/'src/archmage_runtime.rs').read_text(encoding='utf-8')
assert '==0x6c180b' in source and 'if !transfer&&!equipped' in source
catalog=json.loads((ROOT/'.work/archmage/native-parameters.json').read_text())
magic={r['ID']:r['fields'] for r in catalog['Magic']}
eligible={r['fields']['magicId'] for r in catalog['MagicTableParam'] if r['fields']['chanceWeight']>0}
spells=sorted((i,f['ezStateBehaviorType']) for i,f in magic.items() if f['ezStateBehaviorType'] in (0,1))
compiled=[(int(i),int(k)) for i,k in re.findall(r'\((\d+),(\d+)\)',(ROOT/'src/archmage_data.rs').read_text())]
assert compiled==spells and len(spells)==178
random=sorted(i for i,_ in spells if i in eligible and not magic[i]['isEnchant'] and not magic[i]['isShieldEnchant'])
random_text=(ROOT/'src/archmage_data.rs').read_text().split('pub const RANDOM_SPELLS:&[i32]=&[',1)[1].split('];',1)[0]
assert [int(i) for i in random_text.split(',') if i]==random and len(random)==151
excluded=sorted((set(i for i,_ in spells)&eligible)-set(random))
assert excluded==[4460,4470,4490,6250,6320,6740,6770,6960,7230]


k=c.WinDLL('kernel32',use_last_error=True)
k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint];k.VirtualFree.restype=c.c_int
# The only call taken by populated slots is the leaf at +100. The native
# getter/leaf execute unchanged at the same relative spacing. Negative-slot
# fallback requires parameter lookup and is intentionally not executed here.
memory=k.VirtualAlloc(None,0x1000,0x3000,0x40);assert memory
cases=[]
try:
 c.memmove(memory,read(0x6a1e10,0x112),0x112)
 native=c.WINFUNCTYPE(c.c_uint64,c.c_void_p,c.c_uint)(memory)
 for a,b in [(4000,4001),(6000,6000),(4000,6000)]:
  entry=c.create_string_buffer(0x80);reference=c.create_string_buffer(24)
  struct.pack_into('<Q',reference,8,c.addressof(entry));struct.pack_into('<ii',entry,0x2c,a,b)
  old=entry.raw
  for slot,expected in [(0,a),(1,b)]:
   actual=native(c.addressof(reference),slot);assert actual==expected
   cases.append(dict(slots=[a,b],slot=slot,result=actual))
  assert entry.raw==old
 empty=c.create_string_buffer(24)
 assert native(c.addressof(empty),0)==0xffffffff
finally:k.VirtualFree(memory,0,0x8000)
report=dict(exeSha256=hashlib.sha256(raw).hexdigest(),getterRva='6A1E10',instanceMagicOffsets=['2C','30'],serializedMagicOffsets=['14','18'],nativeCases=cases,emptyReferenceReturnsMinusOne=True,knownSpellTypes=178,sorceries=64,incantations=87,excludedEnchantmentIds=excluded,gameplayVerified=False,multiplayerVerified=False)
(ROOT/'.work/archmage/native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print('PASS: native spell getter execution, both slot offsets, spell assignment/bullet/serialization callers and 151-spell pools without weapon/shield enchantments')
