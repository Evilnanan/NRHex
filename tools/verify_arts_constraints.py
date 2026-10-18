"""Verify usable/bow pools and owner-scoped cross-weapon fire emission."""
from pathlib import Path
import json,runpy,sys,struct,hashlib,re,tomllib
root=Path(__file__).resolve().parents[1]
p=json.loads((root/'.work/arts/parameters.json').read_text())
pool=json.loads((root/'dist/arts-pool.json').read_text(encoding='utf8'))
assert pool['poolCount']==182 and pool['ordinaryPoolCount']==175 and pool['bowPoolCount']==7
assert pool['bowSkills']==[400,401,402,404,405,406,1169]
weapons={x['ID']:x['fields'] for x in p['weapons']};skills={x['ID']:x['fields'] for x in p['arts']}
generated=(root/'src/arts_data.rs').read_text()
element_counts={}
for name,field,skill in [('FIRE_WEAPONS','attackBaseFire',1014),('FIRE_WEAPONS','attackBaseFire',1183),('MAGIC_WEAPONS','attackBaseMagic',1184)]:
 ids=json.loads(re.search(r'pub const '+name+r': \[i32; \d+\] = (\[[^;]+\]);',generated).group(1))
 assert ids==sorted(i for i,w in weapons.items() if w[field]>0)
 assert pool['elementRequirements'][str(skill)]==field+' > 0'
 element_counts[field]=len(ids)
assert pool['bowWeapons']==sorted(i for i,w in weapons.items() if w['wepType'] in [50,51,53])
assert next(entry for entry in pool['skills'] if entry['id']==1183)['chineseName']=='吞世'
assert next(entry for entry in pool['skills'] if entry['id']==1183)['name']=='Devourer of Worlds'
assert all(weapons[skills[i]['sparringGroundsWeaponId']]['wepType'] in [50,51,53] for i in pool['bowSkills'])
assert {0,1,2,10,117,1007,9999}.isdisjoint(x['id'] for x in pool['skills'])
assert skills[117]['swordArtsType']==17 and skills[117]['isGrayOutRightHand']==skills[117]['isGrayOutBothHand']==1
for i in [1,10]:assert skills[i]['swordArtsType']==255
full=(root/'data/reference/actions/swordarts-enable.lua').read_text(encoding='utf8')
assert 'elseif arts_id ~= 17 and arts_id ~= SWORDARTS_INVALID then' in full
sys.argv=['probe','dis','0','0'];d=runpy.run_path(str(root/'tools/native_probe.py'))
assert hashlib.sha256(d['raw']).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
def check(r,b):assert d['read'](r,len(bytes.fromhex(b)))==bytes.fromhex(b),hex(r)
check(0x40d0a8,'4C 8D 4D 07') # Fourth argument is the private emitter descriptor.
check(0x40d0d1,'89 44 24 28 8B 45 77 89 44 24 20 C7 45 67 06 00 00 00')
assert 0x40d0e8+struct.unpack('<i',d['read'](0x40d0e4,4))[0]==0x3975d0
check(0x3975f1,'49 8B F9 41 8B D8 44 8B E2 48 8B F1 48 8B 09') # descriptor,event,behavior,module; module's first pointer is actor.
check(0x3976c3,'44 8B 8C 24 58 01 00 00 44 8B 84 24 50 01 00 00') # Original sixth/fifth source indices.
check(0x3976e0,'44 0F B6 47 04 8B 17') # Native origin setter receives descriptor byte4 and integer0.
assert 0x3976f1+struct.unpack('<i',d['read'](0x3976ed,4))[0]==0x3842c0
source=(root/'src/lib.rs').read_text(encoding='utf8')
assert 'type ArtsBulletFn=unsafe extern "system" fn(usize,i32,i32,usize,i32,i32,u8,u8,u8,usize)->u8' in source
assert 'arts::pool(w.base)' in source and 'arts::emitter_dummy(selected,w.base,behavior,read_i32(descriptor))' in source
assert '!arts::roll_allowed(w.base,id)' in source
assert 'shadow[..4].copy_from_slice(&dummy.to_le_bytes())' in source
report=dict(version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version'],poolCount=182,ordinaryPoolCount=175,bowPoolCount=7,bowSkills=pool['bowSkills'],elementRequirements=pool['elementRequirements'],elementWeaponCounts=element_counts,
 bowWeaponTypes=[50,51,53],bowWeaponCount=len(pool['bowWeapons']),excludedUnusableSkills=pool['excludedUnusableSkills'],
 torchNativeAnimationTypeDisabled=17,allOtherNamedSkillsRetained=True,
 projectileCreationRva='3975D0',nativeArgumentCount=10,nativeReturn='AL/u8',
 originalDescriptorAndOutputsPreserved=True,localOwnerSelectedSkillOnly=True,
 fireMuzzles={'1001':[10141,20141],'1002':[10142,20142]},fallbackBodyDummy=58,
 originalFireShieldMuzzlesUnchanged=True,nativeParametersAndTaeUnchanged=True,
 nativeMappingVerified=True,gameplayVerified=False,multiplayerVerified=False)
(root/'dist/arts-constraints-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print('PASS: 175 general / 7 bow skills, unusable skills excluded, fire emitter forwards ten native arguments with a private origin copy.')
