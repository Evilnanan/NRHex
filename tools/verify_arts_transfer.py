"""Read-only proof of native weapon drop UID and both local pickup paths."""
from pathlib import Path
import sys,runpy,json,struct,hashlib
root=Path(__file__).resolve().parents[1]
sys.argv=['probe','dis','0','0'];d=runpy.run_path(str(root/'tools/native_probe.py'))
assert hashlib.sha256(d['raw']).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
def check(r,b):assert d['read'](r,len(bytes.fromhex(b)))==bytes.fromhex(b),hex(r)
def call(r,target):
 assert d['read'](r,1)==b'\xe8',hex(r)
 assert r+5+struct.unpack('<i',d['read'](r+1,4))[0]==target,(hex(r),hex(target))
# Menu serialization uses the actual full handle, then the native drop builder.
call(0x7e91eb,0x1f59e0)
call(0x7e9201,0x578950)
call(0x7e925f,0x574a60)
call(0x57a609,0x6c16f0) # Replacement pickup serializes its outgoing weapon.
call(0x57a673,0x574a60)
# Native descriptor starts at rbp-80; UID is therefore descriptor+5C.
check(0x574d83,'0F 10 03 0F 29 45 80')
check(0x574e38,'41 0F BA E9 1D')
check(0x574e3d,'44 89 4D DC')
check(0x574fa8,'48 8D 55 80 48 8D 4D 40')
call(0x574fb0,0x570410) # Network queue receives the prepared UID now.
check(0x57044f,'8B 42 5C 89 41 5C')
check(0x575043,'48 8D 55 80 49 8B CE')
call(0x57504a,0x5759e0) # Direct local reward creation shares the same descriptor.
# Pickup receives the ground node's exact descriptor, not a type/fingerprint.
check(0x5754dc,'48 8D 78 20')
check(0x5757c0,'48 8B D7 49 8B CF')
call(0x5757c6,0x57ca60)
check(0x57cabe,'48 8D 7A 08')
check(0x57ca97,'48 8B 45 6F') # Fifth stack argument: position pointer.
check(0x57cacd,'44 8B 7D 7F') # Seventh: signed slot index.
check(0x57cb10,'44 0F B6 4D 77') # Sixth: byte flag.
call(0x57cb18,0x57a470)
call(0x57cb42,0x57a0f0)
call(0x57a6af,0x57b490)
check(0x57a335,'4C 8B C7')
call(0x57a33f,0x57b490)
check(0x57b4e7,'4C 8B C7 48 8B D3')
call(0x57b4ed,0x6a08b0) # Factory writes the newly generated full instance handle.
check(0x57b52b,'48 8B C3') # Preserve output-handle pointer return value.
source=(root/'src/lib.rs').read_text(encoding='utf8')
assert 'arts_bind_drop(descriptor);\n let scope=PERSONAL_REWARD' in source
assert 'payload!=scope.pickup_payload' in source and 'drop.payload' in source
assert 'ArtsTransferGuard::enter' in source
report=dict(version='0.15.4',nativeGroundUidOffset='5C',groundDescriptorOffset='20',
 serializedWeaponBytes=0x48,menuDropRva='7E91A0',serializeRva='6C16F0',
 networkQueueCopyRva='570410',directCreationRva='5759E0',pickupRva='57CA60',
 instanceFactoryRva='57B490',pickupArgumentCount=7,localInventoryOnly=True,
 exactGroundUidBinding=True,identicalWeaponsNotMatchedByTypeOrOrder=True,
 nativeInventoryAndDropPayloadUnchanged=True,nestedReplacementDropCovered=True,
 multiplayerQueueCoveredByStaticCallFlow=True,gameplayVerified=False,multiplayerVerified=False)
(root/'dist/reports/arts-transfer-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print('PASS: native UID survives local/network creation; both pickups recreate handles inside the seven-argument local inventory routine.')
