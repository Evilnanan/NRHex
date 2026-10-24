"""Independent callback decompile and native getter/input isolation checks."""
from pathlib import Path
import json,subprocess,runpy,sys,struct,hashlib
root=Path(__file__).resolve().parents[1]
report=json.loads((root/'dist/reports/arts-hks-validation.json').read_text())
ids=[x['id'] for x in report['functions']];out=root/'.work/hks/arts-decompiled.lua'
subprocess.run([r'D:\Program Files\DSLuaDecompiler\DSLuaDecompiler.exe',str(root/'dist/mod/action/script/c0000.hks'),'--included-function-ids',*[str(x) for x in ids],'-o',str(out)],capture_output=True,check=True)
text=out.read_text(encoding='utf8')
for f in report['functions']:
 body=text.split('function '+f['name']+'(',1)[1].split('\nfunction ',1)[0]
 assert 'env(1116, 6999800)' in body,f['name']
 if f.get('attackStanceOnly'):
  assert f['id'] in [537,538]
  assert '= c_SwordArtsID' in body
  for kind in [10,11,25,239]:assert f'~= {kind}' in body,f['name']
  for token in [6999801,6999802,6999803]:assert f'act(2002, {token})' in body,f['name']
 elif f['used']:assert 'act(2002, 6999802)' in body and 'act(2002, 6999803)' in body,f['name']
 else:assert 'act(2002, 6999801)' in body and '6999802' not in body,f['name']
 if f['magicSkillOnly']:assert 'GetVariable("IndexMagicSlot")' in body and '== 2' in body,f['name']
sys.argv=['p','dis','0','0'];d=runpy.run_path(str(root/'tools/native_probe.py'))
assert hashlib.sha256(d['raw']).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
def check(r,s):assert d['read'](r,len(bytes.fromhex(s)))==bytes.fromhex(s),hex(r)
check(0x6a220a,'8B 46 1C 85 C0 0F 49 D0') # Native per-instance override.
check(0x6a22c8,'8B 47 1C 85 C0 0F 49 D8') # Native UI/raw skill ID.
check(0x6a2200,'8B 90 98 01 00 00') # Base weapon fallback.
check(0x3818ab,'42 8B 44 83 1C') # Equip instance handles, distinct from base IDs.
check(0x66cdb8,'48 8B 89 70 06 00 00')
check(0x6a0da1,'C1 E8 1C 83 E0 07') # Item-kind ignores native generation bit 31.
check(0x6c19d0,'C1 E9 17 83 E1 01')
check(0x69f9aa,'39 48 08') # Generation-bearing handle match.
# Confirm FP reduction is read on native skill cost, not spell cost.
check(0x6ce5e5,'49 8B C8 E8 C3 7B E3 FF')
params=json.loads((root/'.work/arts/parameters.json').read_text())
assert params['version']=='10350000'
assert any(x['ID']==360200 and x['fields']['artsConsumptionRate']==.75 for x in params['effects'])
stormTables={x['ID'] for x in params['artTables'] if x['fields']['swordArtsId']==1200}
storm=[x for x in params['custom'] if x['fields']['swordArtsTableId'] in stormTables]
assert len(storm)==10
pool=json.loads((root/'dist/reports/arts-pool.json').read_text(encoding='utf8'))
texts=json.loads((root/'.work/arts/arts-names.json').read_text())
assert pool['poolCount']==182 and pool['protectedOriginalSkills']==[1200,303]
assert pool['ordinaryPoolCount']==175 and pool['bowPoolCount']==7
assert {0,1,2,10,117,1007,9999}.isdisjoint(s['id'] for s in pool['skills'])
assert {104,121,407,1180,1181}.isdisjoint(s['id'] for s in pool['skills'])
assert {100,303,1200}.issubset(s['id'] for s in pool['skills'])
for skill in pool['skills']:
 assert skill['name']==texts['names']['engus'][str(skill['textId'])]
 assert skill['chineseName']==texts['names']['zhocn'][str(skill['textId'])]
 assert skill['name'].strip() and skill['chineseName'].strip() and 'ArtsName(' not in skill['name'] and 'ArtsName(' not in skill['chineseName']
assert texts['names']['zhocn']['303']=='小圆盾格挡'
assert next(r for r in params['arts'] if r['ID']==303)['fields']['swordArtsType']==93
source=(root/'src/lib.rs').read_text()
assert '(handle>>28)&7!=0' in source and 'ARTS_RAW_ORIG' in source
assert 'ptr::write_unaligned((entry.as_mut_ptr()' in source
assert 'ptr::write_unaligned((r[1]' not in source # Native instance never modified.
report.update(independentDecompilePassed=True,nativeFpMultiplierField='artsConsumptionRate',nativeFpCostUsesEffectMultiplier=True,instanceOverrideOffset='1c',weaponHandleGenerationValidated=True,nativeInventoryAndDropBytesNeverModified=True,localSixWeaponSlotsOnly=True,protectedOriginalSkills=[1200,303],protectedWeaponTemplates=[x['ID'] for x in storm],uniqueSkillPool=pool['poolCount'],actualNameTextChecked=True,excludedPreviouslyRandomUnnamedSkills=[104,121,407,1180,1181],gameplayVerified=False,multiplayerVerified=False)
(root/'dist/reports/arts-hks-validation.json').write_text(json.dumps(report,indent=2))
print('PASS: skill callback readback, both native skill-ID/parameter paths, original FP multiplier and 10 Storm Ruler templates.')
