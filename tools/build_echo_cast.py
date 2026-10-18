"""Build checked native spell-root eligibility; no IDs are invented at runtime."""
import json,argparse
from pathlib import Path
r=Path(__file__).resolve().parents[1];p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');args=p.parse_args()
v=json.loads((r/'data/reference/combat/echo-cast.json').read_text());assert v['version']==10350000 and v['regulationSha256']=='876a3ca279a4561d0c69f81fe5e510c75c59ab9a8a201694f8cf4a42c91e0268'
b=v['Bullet'];catalog=[]
for mid,m in sorted((int(id),row)for id,row in v['Magic'].items()):
 if m['consumeLoopMP_forMenu']>=0 or any(m[f'consumeType{i}']==2 for i in range(1,11)):continue
 roots=sorted({m[f'refId{i}']for i in range(1,11)if m[f'refCategory{i}']==1 and str(m[f'refId{i}'])in b})
 roots=[id for id in roots if b[str(id)]['life']>0 and b[str(id)]['FollowType']==0 and b[str(id)]['isSyncBulletCulcDumypolyPos']==0]
 if roots:catalog.append((mid,roots))
assert dict(catalog)[4021]==[10402100,10402150]
s='// Generated from original regulation 10350000; finite independent spell roots.\n'
s+='pub const ROOTS:&[(i32,&[i32])]=&[\n'+''.join(f' ({mid},&[{",".join(map(str,ids))}]),\n'for mid,ids in catalog)+'];\n'
path=r/'src/echo_cast_data.rs'
if args.check:assert path.read_text()==s,'Echo root catalog changed; regenerate with tools/build_echo_cast.py'
else:path.write_text(s)
print(f'PASS: {len(catalog)} eligible spells / {sum(len(ids)for _,ids in catalog)} independent roots; sustained/attached types excluded')
