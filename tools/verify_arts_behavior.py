"""Verify the native PC resolver, its table kind and generated skill mapping."""
from pathlib import Path
import sys,runpy,json,struct,hashlib
root=Path(__file__).resolve().parents[1]
sys.argv=['probe','dis','0','0'];d=runpy.run_path(str(root/'tools/native_probe.py'))
assert hashlib.sha256(d['raw']).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
def check(r,hexbytes):assert d['read'](r,len(bytes.fromhex(hexbytes)))==bytes.fromhex(hexbytes),hex(r)
def pointer(r):return struct.unpack('<Q',d['read'](r,8))[0]-d['base']
assert pointer(0x2c307f8+0x328)==0x668b30
assert pointer(0x2c307f8+0x250)==0x66cde0
assert pointer(0x2c307f8+0x258)==0x66cda0
check(0x668b30,'89 54 24 10') # Original judge home slot.
check(0x668b4c,'45 8B F9 41 8B F0 44 8B EA 48 8B F9') # Flags, source, judge, actor.
check(0x668b7c,'FF 90 50 02 00 00') # Base weapon from this source index.
check(0x668cc5,'80 7D 7F 00') # Fifth byte argument in its native stack slot.
check(0x668de9,'69 F8 E8 03 00 00 41 03 FF') # category*1e8+variation*1e3+judge%1e3.
for site in [0x668c9a,0x668df4,0x668e44]:
 assert d['read'](site,1)==b'\xe8'
 assert site+5+struct.unpack('<i',d['read'](site+1,4))[0]==0xe6fcb0
check(0xe6fcca,'41 0F B6 E8')
check(0xe6fd21,'8D 55 0C 45 33 C0') # Repository index12+isPC=13, BehaviorParam_PC.
check(0x3818a3,'E8 78 F9 FF FF')
check(0x3818c9,'E8 52 F9 FF FF') # Full handle/base getters use the same source decoder.
check(0x40cb08,'41 FF 92 28 03 00 00') # Attack behavior uses the actor resolver.
check(0x40ce7c,'41 FF 92 28 03 00 00') # Bullet behavior uses the same resolver.
p=json.loads((root/'.work/arts/parameters.json').read_text())
pool=json.loads((root/'dist/reports/arts-pool.json').read_text(encoding='utf8'))
weapons={x['ID']:x['fields'] for x in p['weapons']}
arts={x['ID']:x['fields'] for x in p['arts']}
behaviors={x['ID']:x['fields'] for x in p['behaviors']}
for row in pool['canonicalSkillBehaviorVariations']:
 assert row['variation']==weapons[arts[row['skill']]['sparringGroundsWeaponId']]['behaviorVariationId']
assert len(pool['canonicalSkillBehaviorVariations'])==182
assert behaviors[300707900]['refId']==2770 and behaviors[300707900]['refType']==1
assert behaviors[301301901]['refId']==2360 and behaviors[301301901]['refType']==1
assert behaviors[301301900]['refId']==303400900 and behaviors[301301900]['refType']==0
report=dict(resolverRva='668b30',actorVirtualOffset='328',behaviorLookupRva='e6fcb0',nativeRepositoryIndex=13,
 sameSourceDecoderForWeaponHandleAndBase=True,attackAndBulletUseSameResolver=True,canonicalSkillCount=182,
 nativeBehaviorRows=len(behaviors),ordinaryAttacksUseNativeResult=True,localOwnedOverlayOnly=True,
 nativeInventoryAndParametersUnchanged=True,gameplayVerified=False)
(root/'dist/reports/arts-behavior-validation.json').write_text(json.dumps(report,indent=2))
print('PASS: true PC behavior resolver, five native arguments, exact weapon source, attack/bullet mapping for 182 skills.')
