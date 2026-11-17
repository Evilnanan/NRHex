"""Guard the complete native dispatch used by the hidden impact combination."""
import hashlib,json,struct
from local_paths import ROOT,GAME
import pefile
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
def read(r,n):return pe.get_data(r,n)
def check(r,h):assert read(r,len(bytes.fromhex(h)))==bytes.fromhex(h),hex(r)
def target(r):
 assert read(r,1)==b'\xe8'
 return r+5+struct.unpack('<i',read(r+1,4))[0]
check(0x43d260,'48 8B C4 48 89 58 20 55 56 57 41 54 41 55 41 56 41 57 48 8D 68 A9 48 81 EC 00 01 00 00 0F 29 70 B8 0F 29 78 A8')
check(0x43d31e,'44 0F B6 6D 7F') # entry stack fifth argument
check(0x4366c5,'88 5C 24 20 45 0F B6 CF 4C 8B C7 49 8B D6 48 8B CE')
assert target(0x4366d6)==0x43d260
check(0x43d2bf,'83 F8 01 4C 8B E3 74 03 4D 8B E5')
consumers={0x43d4be:0x439e80,0x43da15:0x48d980,0x43e12f:0x4391e0,0x43e14c:0x438dd0}
for site,callee in consumers.items():assert target(site)==callee
check(0x43a170,'41 0F B6 56 24')
check(0x43a178,'FF 50 30') # recipient's own native reaction mapping
check(0x43a17b,'41 88 86 B6 02 00 00')
check(0x438eb2,'0F B6 53 24') # later reaction still consumes original input field
check(0x439efc,'41 F6 80 C1 02 00 00 08')
# 5363A0: original RDX source is saved to hit+228. Stack hit starts at
# RBP-D0 (RSP+30), and the source lives at RBP+158.
check(0x5363e5,'4C 8B FA')
check(0x5364b6,'4C 89 BD 58 01 00 00')
check(0x6c2deb,'FF 50 10 83 E8 01')
check(0x6c2df7,'83 F8 02 74 16') # source kind 3 enters BulletIns branch
check(0x6c2e15,'48 8B 88 B0 00 00 00') # bullet's native owner handle
source=(ROOT/'src/impact.rs').read_text(encoding='utf8')
assert 'ability_damage::source_actor(context,resolve)!=local' in source
assert 'weapon_contact::from_source(local,hit,direct,base).is_some()' in source
assert 'context+0x10e' in source and 'boost(hit)' in source
assert 'hit+0x24' in source and 'hit+0x1e' in source
check(0x4358d2,'41 0F B6 40 1E 88 87 8C 00 00 00')
check(0x4350e6,'0F B6 83 8C 00 00 00 88 45 08 C6 45 4E 01')
lib=(ROOT/'src/lib.rs').read_text(encoding='utf8')
assert 'impact::constructed(local,hit,context,base)' in lib
assert 'strength_sword::detour' not in lib
report=dict(version='0.17.358',impactStage='E70350 fresh construction',
 nativeConsumers={hex(k):hex(v)for k,v in consumers.items()},
 hitInput='0x24',wireOverride='0x1e',packetField='0x8c',
 nativeBulletKind=3,ordinaryProjectilesExcluded=True,receivedDescriptorNotReboosted=True,
 nativeExecutionCheck='impact::native_tests::compiled_modifiers_survive_native_wire_rebuild_without_reapplying',
 gameplayVerified=False,multiplayerVerified=False)
(ROOT/'dist/reports/mighty-sword-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print('PASS: fresh construction source, paired impact fields and native reconstruction path verified')
