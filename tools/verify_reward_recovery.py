"""Read-only ABI/control-flow evidence for moving existing private rewards."""
from pathlib import Path
import ctypes as c, hashlib, json, struct
from local_paths import GAME
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
root=Path(__file__).resolve().parents[1]
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
def read(r,n):
    o=pe.get_offset_from_rva(r);return raw[o:o+n]
def check(r,s):assert read(r,len(bytes.fromhex(s)))==bytes.fromhex(s),hex(r)
def target(r):
    assert read(r,1)==b'\xe8';return r+5+struct.unpack('<i',read(r+1,4))[0]
assert target(0x576dc4)==0x3d0e60 and target(0x576dec)==0x631740
check(0x576daf,'48 8D 44 24 38')
check(0x576d6b,'89 4C 24 38') # Source packed map, same native constructor.
check(0x576d5c,'83 F9 FF 75 0A')
check(0x576d61,'48 8B 05 90 B6 69 03 8B 48 2C') # Player handle FFFFFFFF -> current map context.
assert 0x576d68+struct.unpack('<i',read(0x576d64,4))[0]==0x3c123f8
assert target(0x3d0e6d)==0x39bd50 and target(0x39bd68)==0x466470
check(0x466470,'0F 10 41 70 48 8B C2 0F 29 02 C3')
check(0x5749e0,'40 53 48 83 EC 30 B8 00 08 00 00 C6 81 F8 00 00 00 00')
assert target(0x57223e)==0x57b540
check(0x57b58c,'48 83 79 18 00 74 07 32 C0') # Children do not render another ball.
check(0x57b59d,'48 8D 91 98 00 00 00 4C 8D 81 94 00 00 00')
assert target(0x57b5af)==0x631530 # Map/local -> actual visual position.
check(0x57b5d7,'0F B6 83 F9 00 00 00 3C 01')
check(0x57bc05,'33 FF 89 BB FC 00 00 00')
check(0x580b7a,'48 C7 83 F0 00 00 00 00 00 00 00')
check(0x580b8a,'48 8D 8B E8 00 00 00') # Native releases render handle, not reward node.

# Execute the exact native character -> physics position wrappers and leaf in
# isolated memory. Only relative calls relocate; no game APIs or memory used.
k=c.WinDLL('kernel32',use_last_error=True)
k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong,c.c_ulong];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong]
k.FlushInstructionCache.argtypes=[c.c_void_p,c.c_void_p,c.c_size_t]
m=k.VirtualAlloc(None,4096,0x3000,0x40);assert m
def put(o,b):c.memmove(m+o,b,len(b))
def relocate(o,target):put(o,b'\xe8'+struct.pack('<i',target-o-5))
try:
    put(0,read(0x3d0e60,27));put(0x100,read(0x39bd50,38));put(0x200,read(0x466470,11))
    relocate(0xd,0x100);relocate(0x118,0x200)
    actor=c.create_string_buffer(0x1c0);coord=c.create_string_buffer(0x20);modules=c.create_string_buffer(0x70);physics=c.create_string_buffer(0x90)
    struct.pack_into('<Q',actor,0x60,c.addressof(coord));struct.pack_into('<Q',coord,0x10,c.addressof(actor))
    struct.pack_into('<Q',actor,0x1b8,c.addressof(modules));struct.pack_into('<Q',modules,0x68,c.addressof(physics))
    struct.pack_into('<ffff',physics,0x70,100.,20.,300.,1.)
    output=c.create_string_buffer(48);out=(c.addressof(output)+15)&~15
    k.FlushInstructionCache(c.c_void_p(-1),m,4096)
    getter=c.WINFUNCTYPE(c.c_void_p,c.c_void_p,c.c_void_p)(m)
    assert getter(c.addressof(actor),out)==out
    assert struct.unpack('<ffff',c.string_at(out,16))==(100.,20.,300.,1.)
finally:k.VirtualFree(m,0,0x8000)

source=(root/'src/lib.rs').read_text(encoding='utf8')
move=source[source.index('unsafe fn relocate_unclaimed_hexes'):source.index('unsafe fn update_level_rewards')]
assert not any(s in move for s in ['spawn_personal','random_order','APPLY_SELF_RVA','ptr::write_unaligned((node+0x78)'])
assert move.index('convert(')<move.index('hide(node)')<move.index('set_location(')<move.index('show(node)')
assert 'for &node in &nodes' in move and 'node!=0{return false;}' in move
update=source.split('unsafe extern "system" fn on_update',1)[1].split('#[cfg(test)]',1)[0]
assert update.index('update_reward_recovery(player,base)')<update.index('update_level_rewards(player,base)')
recovery=source[source.index('unsafe fn update_reward_recovery'):source.index('unsafe fn relocate_unclaimed_hexes')]
assert '102100' in recovery and '102150' in recovery
assert 'map==u32::MAX' in recovery and 'MAP_CONTEXT_RVA' in recovery
md=Cs(CS_ARCH_X86,CS_MODE_64)
evidence={hex(r):[f'{i.address-pe.OPTIONAL_HEADER.ImageBase:08X} {i.mnemonic} {i.op_str}' for i in md.disasm(read(r,n),pe.OPTIONAL_HEADER.ImageBase+r)] for r,n in [(0x3d0e60,27),(0x39bd50,38),(0x466470,11),(0x5749e0,116),(0x572220,64),(0x57b58c,71),(0x580b20,123)]}
(root/'dist/reward-recovery-validation.json').write_text(json.dumps(dict(version='0.12.5',nativePositionAbiIsolatedExecutionPassed=True,nativeMapConversionAndVisualRefreshReadBack=True,existingChoicesAndReceiptsNotRerolled=True,localPrivateOwnerGuardPresent=True,deadAndNearDeathGuards=True,evidence=evidence,gameplayVerified=False,multiplayerVerified=False),indent=2),encoding='utf8')
print('PASS: native position ABI, map conversion, grouped visual cleanup/rebuild and no-reroll recovery scope.')
