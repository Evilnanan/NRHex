"""Read original weapon defaults and the full standard/unique skill pool."""
from pathlib import Path
import json,re,os
root=Path(__file__).resolve().parents[1]
p=json.loads((root/'.work/arts/parameters.json').read_text())
texts=json.loads((root/'.work/arts/arts-names.json').read_text())
def named(row):
 return all((name:=texts['names'][lang].get(str(row['fields']['textId']),'')) and not re.search(r'ArtsName\s*\(|^[?？\s]+$',name) for lang in ('zhocn','engus'))
candidates=[x for x in p['arts'] if x['fields']['swordArtsType']!=255 and x['fields']['textId']>0 and x['ID']!=9999]
unusable=[x for x in candidates if named(x) and x['fields']['isGrayOutRightHand'] and x['fields']['isGrayOutBothHand']]
assert [x['ID'] for x in unusable]==[117] # Native IsEnableSwordArts also rejects animation type17.
arts=[x for x in candidates if named(x) and x not in unusable and x['ID']!=1007] # Coil Shield's animated snake hit points cannot transfer to other weapons.
excluded=[x['ID'] for x in candidates if not named(x)]
assert set(excluded)=={0,104,121,407,1180,1181}
ids=[x['ID'] for x in arts];assert len(ids)==len(set(ids)) and {100,303,1200}.issubset(ids)
assert texts['names']['zhocn']['303']=='小圆盾格挡'
defaults=sorted((x['ID'],x['fields']['swordArtsParamId']) for x in p['weapons'])
weapons={x['ID']:x['fields'] for x in p['weapons']}
element_weapons={name:sorted(i for i,w in weapons.items() if w[field]>0) for name,field in [('FIRE_WEAPONS','attackBaseFire'),('MAGIC_WEAPONS','attackBaseMagic')]}
for skill,name in [(1014,"Taker's Flames"),(1183,'Devourer of Worlds'),(1184,'Familial Rancor')]:
 row=next(x for x in arts if x['ID']==skill)
 assert texts['names']['engus'][str(row['fields']['textId'])]==name
bow_weapons=sorted(i for i,w in weapons.items() if w['wepType'] in [50,51,53])
bow_skills=[x['ID'] for x in arts if weapons[x['fields']['sparringGroundsWeaponId']]['wepType'] in [50,51,53]]
assert bow_skills==[400,401,402,404,405,406,1169]
ordinary_skills=[i for i in ids if i not in bow_skills]
assert len(ids)==182 and len(ordinary_skills)==175
canonical=sorted((x['ID'],weapons[x['fields']['sparringGroundsWeaponId']]['behaviorVariationId']) for x in arts)
behaviors=sorted({x['ID'] for x in p['behaviors']})
assert dict(canonical)[1047]==1301 and dict(canonical)[1173]==707
assert {300707900,301301901}.issubset(behaviors)
text='// Generated from hash-guarded original regulation. Bows have a separate pool.\n'
text+=f'pub const POOL: [i32; {len(ordinary_skills)}] = '+str(ordinary_skills).replace(' ', '')+';\n'
text+=f'pub const BOW_POOL: [i32; {len(bow_skills)}] = '+str(bow_skills).replace(' ', '')+';\n'
text+=f'pub const BOW_WEAPONS: [i32; {len(bow_weapons)}] = '+str(bow_weapons).replace(' ', '')+';\n'
for name,rows in element_weapons.items():text+=f'pub const {name}: [i32; {len(rows)}] = '+str(rows).replace(' ', '')+';\n'
text+=f'pub const DEFAULTS: [(i32,i32); {len(defaults)}] = [\n'+''.join(f' ({a},{b}),\n' for a,b in defaults)+'];\n'
for name,rows in [('SKILL_VARIATIONS',canonical)]:
 text+=f'pub const {name}: [(i32,i32); {len(rows)}] = [\n'+''.join(f' ({a},{b}),\n' for a,b in rows)+'];\n'
text+=f'pub const BEHAVIORS: [i32; {len(behaviors)}] = [\n'+''.join(f' {i},\n' for i in behaviors)+'];\n'
(root/'src/arts_data.rs').write_text(text)
smith=Path(os.environ.get('NRHEX_SMITHBOX',str(Path(root.anchor)/'Program Files/Smithbox')))
names=json.loads((smith/'Assets/PARAM/NR/Param Row Names/English/SwordArtsParam.json').read_text())
names={x['ID']:' / '.join(x['Entries']) for x in names['Entries']}
report=dict(poolCount=len(ids),ordinaryPoolCount=len(ordinary_skills),bowPoolCount=len(bow_skills),bowSkills=bow_skills,bowWeapons=bow_weapons,bowWeaponTypes=[50,51,53],excludedUnusableSkills=[0,1,2,10,117,1007,9999],skills=[dict(id=x['ID'],textId=x['fields']['textId'],name=texts['names']['engus'][str(x['fields']['textId'])],chineseName=texts['names']['zhocn'][str(x['fields']['textId'])],alias=names.get(x['ID'],''),animationType=x['fields']['swordArtsType']) for x in arts],protectedOriginalSkills=[1200,303],excludedUnnamedSkills=excluded,actualNameTextChecked=True,textSources=texts['sources'],includesUniqueSkills=True,compatibilityFiltering='bows_and_exclude_model_dependent_viper_bite')
report.update(canonicalSkillBehaviorVariations=[dict(skill=i,variation=v) for i,v in canonical],nativeBehaviorRows=len(behaviors),genericBehaviorMapping=True)
report.update(compatibilityFiltering='bows_model_dependencies_and_weapon_base_elemental_attack',elementRequirements={'1014':'attackBaseFire > 0','1183':'attackBaseFire > 0','1184':'attackBaseMagic > 0'},elementWeaponCounts={n:len(v) for n,v in element_weapons.items()})
(root/'dist/reports/arts-pool.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf8')
print(f'PASS: {len(ids)} named standard/unique skills; actual Chinese/English texts checked; original Storm Ruler/Buckler Parry weapons protected.')
