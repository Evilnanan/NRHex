"""Execute only the native positive stamina add/clamp path, never the game."""
from pathlib import Path
import ctypes as c,hashlib,json,struct,os
import pefile,tomllib
root=Path(__file__).resolve().parents[1]
game=Path(os.environ.get('NRHEX_GAME',str(Path(root.anchor)/'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game')))
raw=(game/'nightreign.exe').read_bytes();digest=hashlib.sha256(raw).hexdigest()
assert digest=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
# act(1001) extracts a signed integer, calls Add(stats, amount, false).
assert pe.get_data(0x3f6302,13)==bytes.fromhex('8b d0 45 33 c0 48 8b cb e8 c1 fe 02 00')
add=bytearray(pe.get_data(0x4261d0,11));assert add[:7]==bytes.fromhex('03 91 5c 01 00 00 e9')
assert 0x4261db+struct.unpack_from('<i',add,7)[0]==0x427f00
struct.pack_into('<i',add,7,0x100-11)
setter=pe.get_data(0x427f00,0xa9)
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong,c.c_ulong];k.VirtualAlloc.restype=c.c_void_p;k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong]
arena=k.VirtualAlloc(None,0x1000,0x3000,0x40);assert arena
try:
 c.memmove(arena,bytes(add),len(add));c.memmove(arena+0x100,setter,len(setter))
 call=c.CFUNCTYPE(None,c.c_void_p,c.c_int,c.c_ubyte)(arena)
 stats=c.create_string_buffer(0x200);cases=0
 for current in [-50,-1,0,1,99,100]:
  for amount in [1,2,30,46,69,1000]:
   for flags in range(256):
    c.memset(stats,0,0x200);stats[0x189]=flags;struct.pack_into('<ii',stats,0x15c,current,100)
    before=stats.raw;call(stats,amount,0)
    assert struct.unpack_from('<i',stats,0x15c)[0]==min(100,current+amount)
    assert stats.raw[:0x15c]==before[:0x15c] and stats.raw[0x160:]==before[0x160:]
    cases+=1
 stage=root/'work/chant-rework/staged';stage.mkdir(parents=True,exist_ok=True)
 version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
 (stage/'native-validation.json').write_text(json.dumps(dict(version=version,exeSha256=digest,cases=cases,integerStaminaAddRva='4261D0',clampRva='427F00',positiveAdditionAndCapVerified=True,otherStatsUnchanged=True,gameplayVerified=False),indent=2))
 print(f'PASS: {cases} native stamina addition and cap cases; no live game accessed')
finally:k.VirtualFree(arena,0,0x8000)
