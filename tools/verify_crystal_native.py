"""Read-only native shield reference plus isolated expiry criteria execution."""
import ctypes as c, hashlib, json, struct, tomllib
import pefile
from local_paths import GAME, ROOT as root
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
def read(rva,n):
 off=pe.get_offset_from_rva(rva);return raw[off:off+n]
def check(rva,h):
 b=bytes.fromhex(h);assert read(rva,len(b))==b,hex(rva)
check(0x504d71,'0F B6 88 27 03 00 00')
assert struct.unpack('<I',read(0x504df8,4))[0]==0x504de3
check(0x504de3,'C7 42 4C 00 00 00 00')
check(0x43cb82,'66 41 0F 6E B6 80 02 00 00')
assert 0x43cbb6+5+struct.unpack('<i',read(0x43cbb7,4))[0]==0x504d30
# The native function is a self-contained loop and RVA jump table; copied RIP
# base and table RVAs rebase together. Only synthetic manager/nodes are used.
k=c.WinDLL('kernel32',use_last_error=True)
k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong,c.c_ulong];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong];k.VirtualFree.restype=c.c_int
code=read(0x504d30,0x110);memory=k.VirtualAlloc(None,len(code),0x3000,0x40);assert memory
results=[]
try:
 c.memmove(memory,code,len(code));native=c.WINFUNCTYPE(None,c.c_void_p,c.c_float,c.c_ubyte,c.c_ubyte,c.c_ubyte)(memory)
 for damage in [0.0,1.0,100.0]:
  for criteria in [0,1]:
   manager=c.create_string_buffer(0x80);params=c.create_string_buffer(0x400);params[0x327]=criteria
   nodes=[c.create_string_buffer(0x80) for _ in range(3)]
   struct.pack_into('<Q',manager,8,c.addressof(nodes[0]))
   for i,node in enumerate(nodes):
    struct.pack_into('<Q',node,0,c.addressof(params));struct.pack_into('<i',node,8,95000701)
    struct.pack_into('<Q',node,0x38,c.addressof(nodes[i+1]) if i<2 else 0)
    struct.pack_into('<f',node,0x4c,123.0);struct.pack_into('<I',node,0x68,4)
   native(c.addressof(manager),damage,3,0,1)
   times=[struct.unpack_from('<f',node,0x4c)[0] for node in nodes]
   expected=0.0 if damage>0 and criteria==1 else 123.0
   assert times==[expected]*3,(damage,criteria,times)
   results.append(dict(damage=damage,criteria=criteria,remaining=times))
finally:assert k.VirtualFree(memory,0,0x8000)
chain=json.loads((root/'data/reference/combat/shell-chain.json').read_text(encoding='utf8'))
reference=next(r['fields'] for r in chain['effects'] if r['ID']==8610702)
assert reference['deleteCriteriaDamage']==1 and reference['vfxId']==5867
source=(root/'src/crystal.rs').read_text(encoding='utf8')
assert 'VISUAL_REFRESH' not in source and 's.beast' in source
version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
(root/'dist/reports/crystal-native-validation.json').write_text(json.dumps(dict(version=version,referenceEffect=8610702,referenceVisual=5867,customVisual=5866,deleteCriteriaDamage=1,isolatedNativeTests=results,allMatchingCopiesExpireOnNativePositiveDamage=True,viewerReplicaCleanupRemoved='clean_replica_crystal' not in (root/'src/lib.rs').read_text(),renewalValidation='effect-sync-validation.json',gameplayVerified=False,multiplayerVerified=False),indent=2),encoding='utf8')
print('PASS: native 8610702 damage-expiry criterion; 6 isolated scenarios including 3 duplicate nodes; renewal checked separately.')
