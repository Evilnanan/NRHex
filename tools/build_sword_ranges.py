"""Estimate spell reach from CURRENT regulation; preserve user overrides."""
from pathlib import Path
import csv, json, math, xml.etree.ElementTree as ET
root=Path(__file__).resolve().parents[1]
g=json.loads((root/'data/reference/combat/sword-graph.json').read_text(encoding='utf8'))
names={int(e.attrib['id']):e.text for e in ET.parse(root/'assets/original/msg/zhocn/item_dlc01-msgbnd-dcx/MagicName.fmg.xml').findall('./entries/text')}
bullets={int(k):v for k,v in g['Bullet'].items()}
attacks={int(k):v for k,v in g['AtkParam_Pc'].items()}
def damaging(a):
    r=attacks.get(a,{})
    return any(r.get(k,0)>0 for k in ['atkPhys','atkMag','atkFire','atkThun','atkDark'])
def useful(a):
    return a in attacks and (damaging(a) or attacks[a].get('dmgLevel',0)>0)
def reach(b):
    # dist is attenuation threshold. Integrate velocity to its speed cap.
    t=b.get('life',0);v=b.get('initVellocity',0);acc=b.get('accelInRange',0)
    radius=max(0,b.get('hitRadius',0),b.get('hitRadiusMax',0))
    if t<=0:return None,'寿命不定；需手动',radius
    if v<=0 and acc<=0:return radius,'静止碰撞半径；未计施放偏移',radius
    cap=b.get('maxVellocity',0) if acc>=0 else b.get('minVellocity',0)
    if abs(acc)<1e-8:travel=v*t
    else:
        tc=max(0,(cap-v)/acc);tc=min(t,tc)
        travel=v*tc+.5*acc*tc*tc+cap*(t-tc)
    return max(0,travel)+radius,'速度×寿命（加速度/速度上限）；未计追踪、地形',radius
spells=[]
for mid,r in sorted((int(k),v) for k,v in g['Magic'].items()):
    roots=[r[f'refId{n}'] for n in range(1,11) if r.get(f'refCategory{n}')==1 and r.get(f'refId{n}',-1) in bullets]
    direct={r[f'refId{n}'] for n in range(1,11) if r.get(f'refCategory{n}')==0 and useful(r.get(f'refId{n}',-1))}
    seen=set();paths={};todo=[(b,0.,0) for b in roots]; aids=set(direct);methods=set()
    while todo:
        bid,offset,depth=todo.pop(0)
        if bid in seen:continue
        seen.add(bid);b=bullets[bid];length,method,radius=reach(b);a=b.get('atkId_Bullet',-1)
        if useful(a):
            aids.add(a);methods.add(method)
            if length is not None:paths[bid]=offset+length
        for key in ['HitBulletID','intervalCreateBulletId']:
            child=b.get(key,-1)
            if child in bullets and depth<12:
                # Hit children begin at collision point; interval children can
                # be emitted earlier. Sum travel as an explicit upper estimate.
                todo.append((child,offset+(max(0,(length or 0)-radius)),depth+1))
    estimate=max(paths.values(),default=0.)
    if direct:methods.add('直接攻击；动画/武器碰撞未计')
    if not aids:methods.add('未找到直接攻击；辅助/附魔/间接机制需另查')
    eligible=4000<=mid<8000
    spells.append(dict(mid=mid,name=names.get(mid,str(mid)),eligible=eligible,category=('魔法' if r['ezStateBehaviorType']==0 else '祷告') if eligible else '特殊技能（排除）',range=round(estimate,2),attacks=sorted(aids),bullets=sorted(seen),notes='；'.join(sorted(methods)),damaging=any(damaging(a) for a in aids)))
comet=next(s for s in spells if s['mid']==4021)['range']
assert comet>0
path=root/'.work/combat/sword-estimates.csv'
old={}
if path.exists():
    with path.open(encoding='utf-8-sig',newline='') as f:
        old={int(r['MagicID']):r for r in csv.DictReader(f)}
fields=['MagicID','名称','类别','估算射程米','手动射程米','估算增伤百分比','攻击ID','子弹ID','估算说明']
with path.open('w',encoding='utf-8-sig',newline='') as f:
    w=csv.DictWriter(f,fields);w.writeheader()
    for s in spells:
        name=s['name']
        if name in [None,'%null%']:name=f"未命名 {s['mid']}"
        w.writerow(dict(zip(fields,[s['mid'],name,s['category'],s['range'],old.get(s['mid'],{}).get('手动射程米',''),round(10+40*min(1,s['range']/comet),2) if s['damaging'] and s['eligible'] else '', '|'.join(map(str,s['attacks'])),'|'.join(map(str,s['bullets'])),s['notes']])))
# The user's direct bonus table is independent of generated reach estimates.
# Never rewrite it during builds; Cargo embeds current CSV values via build.rs.
bonus_path=root/'data/balance/sword.csv'
with bonus_path.open(encoding='utf-8-sig',newline='') as f:
    bonus_reader=csv.DictReader(f)
    assert 'MagicID' in (bonus_reader.fieldnames or []),'Missing MagicID column'
    percentage_cols=[c for c in ['加成','吸血百分比','效果百分比','增伤百分比'] if c in bonus_reader.fieldnames]
    assert len(percentage_cols)==1,'Expected one shared percentage column'
    percentage_col=percentage_cols[0]
    bonus_rows=list(bonus_reader)
eligible_ids={s['mid'] for s in spells if s['eligible']}
bonus_values={}
for r in bonus_rows:
    assert None not in r and all(v is not None for v in r.values()),'Malformed bonus CSV row'
    mid=int(r['MagicID']);assert mid in eligible_ids and mid not in bonus_values,f'Unknown/duplicate bonus MagicID {mid}'
    raw=r[percentage_col].strip();value=float(raw) if raw else 0.
    assert math.isfinite(value) and 0<=value<=100,f'Invalid bonus for {mid}: {value}'
    bonus_values[mid]=value
assert set(bonus_values)==eligible_ids,'Incomplete spell bonus table'
mapping={}
for s in spells:
    if not s['eligible']:continue
    for a in s['attacks']:mapping.setdefault(a,[]).append(s['mid'])
lines=['// Generated from current regulation and the preserved user bonus CSV.','pub const DEFAULT_BONUSES: &[(i32,f32)] = &[']
lines += [f" ({mid},{value:.9g}f32)," for mid,value in sorted(bonus_values.items())]
lines += ['];','pub const ATTACKS: &[(i32,&[i32])] = &[']
lines += [f" ({a},&[{','.join(map(str,ids))}])," for a,ids in sorted(mapping.items())]
lines += ['];','// Exact Magic root references, including roots with native MagicID=-1.','pub const ROOTS: &[(i32,&[i32])] = &[']
roots={}
for mid,r in ((int(k),v) for k,v in g['Magic'].items()):
    if mid not in eligible_ids:continue
    for n in range(1,11):
        bid=r.get(f'refId{n}',-1)
        if r.get(f'refCategory{n}')==1 and bid in bullets:roots.setdefault(bid,set()).add(mid)
lines += [f" ({bid},&[{','.join(map(str,sorted(ids)))}])," for bid,ids in sorted(roots.items())]
lines += ['];']
(root/'src/sword_map.rs').write_text('\n'.join(lines)+'\n',encoding='utf-8')
report={'spellCount':len(spells),'mappedAttacks':len(mapping),'sharedSpellAttacks':{str(a):ids for a,ids in mapping.items() if len(ids)>1},'cometRange':comet,'warning':'Upper reach estimates, not measured in-game range. Spell/weapon shared Bullet IDs can remain ambiguous. dist is attenuation, not travel range.','spells':spells}
spell_bullets={b for s in spells if s['eligible'] for b in s['bullets']}
report['nonspellBulletAttackCollisions']={str(bid):b['atkId_Bullet'] for bid,b in bullets.items() if bid not in spell_bullets and b.get('atkId_Bullet') in mapping}
report['nonspellMagicAttackCollisions']={str(s['mid']):[a for a in s['attacks'] if a in mapping] for s in spells if not s['eligible'] and any(a in mapping for a in s['attacks'])}
(root/'.work/combat/sword-ranges.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf8')
print(f'PASS: {len(spells)} spells, {len(mapping)} attack IDs; Comet reference {comet:.2f}m. Existing manual overrides preserved.')
print(f'PASS: direct bonus table preserved; {len(bonus_values)} identities, {sum(v>0 for v in bonus_values.values())} positive shared percentages, maximum {max(bonus_values.values()):g}%.')
for mid in [4000,4021,4100,4430,6400]:
    s=next(s for s in spells if s['mid']==mid)
    print(s['name'],s['range'],s['attacks'])
