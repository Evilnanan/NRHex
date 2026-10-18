"""Build checked shot roots and reviewed support areas; IDs are fixed at build time."""
import json,argparse,hashlib
from pathlib import Path
r=Path(__file__).resolve().parents[1];p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');p.add_argument('--report',type=Path);args=p.parse_args()
v=json.loads((r/'data/reference/combat/echo-cast.json').read_text());assert v['version']==10350000 and v['regulationSha256']=='876a3ca279a4561d0c69f81fe5e510c75c59ab9a8a201694f8cf4a42c91e0268'
b=v['Bullet'];catalog=[]
raw=(r/'data/reference/combat/sword-graph.json').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='05204b7deb7d02212abbc923462925fd33325114031000415eb0a2a4354ce67b'
graph=json.loads(raw)
def descendants(root):
 seen=set();todo=[root]
 while todo:
  id=todo.pop()
  if id in seen or str(id)not in graph['Bullet']:continue
  seen.add(id);row=graph['Bullet'][str(id)]
  todo.extend((row['HitBulletID'],row['intervalCreateBulletId']))
 return seen
def attacks(root):
 for id in descendants(root):
  attack=graph['AtkParam_Pc'].get(str(graph['Bullet'][str(id)]['atkId_Bullet']),{})
  if any(attack.get(f,0)>0 for f in ('atkPhys','atkMag','atkFire','atkThun','atkDark','atkSuperArmor','dmgLevel')):return True
 return False
# Held dragon sequences need separate adaptation. Deadly Sin is a personal
# buff whose periodic damage must not become repeated spell-cast echoes.
unsupported={7010,7011,7050,7900}
areas=json.loads((r/'data/balance/echo-cast-areas.json').read_text())
assert areas['version']==v['version']
areas=areas['areas']
assert [a['magic']for a in areas]==sorted({a['magic']for a in areas})
assert len({a['replay']for a in areas})==len(areas)
for a in areas:
 m=v['Magic'][str(a['magic'])];root=b[str(a['root'])]
 assert m['consumeLoopMP_forMenu']<0 and any(m[f'refCategory{i}']==1 and m[f'refId{i}']==a['root'] and m[f'consumeType{i}']==0 for i in range(1,11))
 assert root['life']>0 and root['FollowType']==0 and root['isSyncBulletCulcDumypolyPos']==0
 assert 95003510<=a['replay']<=95003599
for mid,m in sorted((int(id),row)for id,row in v['Magic'].items()):
 if m['consumeLoopMP_forMenu']>=0 or mid in unsupported or mid in {a['magic']for a in areas}:continue
 roots=sorted({m[f'refId{i}']for i in range(1,11)if m[f'refCategory{i}']==1 and str(m[f'refId{i}'])in b})
 roots=[id for id in roots if b[str(id)]['life']>0 and b[str(id)]['FollowType']==0 and b[str(id)]['isSyncBulletCulcDumypolyPos']==0]
 # Type 2 means no consumption, not channelled casting. Include every actual
 # attack root, including no-cost side volleys, but not empty control bullets
 # or direct/personal effects. Retain the previous independent-shot profiles.
 if any(m[f'consumeType{i}']==2 for i in range(1,11)):roots=[id for id in roots if attacks(id)]
 if roots:catalog.append((mid,roots))
assert dict(catalog)[4021]==[10402100,10402150]
for mid,ids in {4040:[10404000,10404050],4050:[10405000,10405050],4090:[10409000,10409001,10409002,10409050,10409051,10409052],4220:[10422000,10422050],4500:[10450010,10450011,10450012,10450060,10450061,10450062],4510:[10451000,10451010,10451020],6820:[10682010,10682011,10682012,10682070,10682071,10682072],6830:[10683010,10683011,10683012,10683013,10683014,10683015]}.items():assert dict(catalog)[mid]==ids
assert not {4060,4200,4520,6420,6760,6971,7000,*unsupported}.intersection(mid for mid,_ in catalog)
s='// Generated from original regulation 10350000; finite independent spell roots.\n'
s+='pub const ROOTS:&[(i32,&[i32])]=&[\n'+''.join(f' ({mid},&[{",".join(map(str,ids))}]),\n'for mid,ids in catalog)+'];\n'
s+='// Reviewed group effects: Magic, native cast root, dedicated area replay.\n'
s+='pub const AREAS:&[(i32,i32,i32)]=&[\n'+''.join(f" ({a['magic']},{a['root']},{a['replay']}),\n"for a in areas)+'];\n'
path=r/'src/echo_cast_data.rs'
if args.check:assert path.read_text()==s,'Echo root catalog changed; regenerate with tools/build_echo_cast.py'
else:path.write_text(s)
assert not set(a['magic']for a in areas).intersection(mid for mid,_ in catalog)
print(f'PASS: {len(catalog)+len(areas)} eligible spells / {sum(len(ids)for _,ids in catalog)+len(areas)} roots, including {len(areas)} reviewed group effects; personal support and sustained/attached types excluded')
if args.report:
 args.report.parent.mkdir(parents=True,exist_ok=True)
 args.report.write_text(json.dumps(dict(eligibleSpells=len(catalog)+len(areas),eligibleRoots=sum(len(ids)for _,ids in catalog)+len(areas),groupEffects=len(areas),noCostConsumptionType=2,roots=dict(catalog),unsupportedMagic=sorted(unsupported),nativeChainSha256=hashlib.sha256(raw).hexdigest()),indent=2)+'\n')
