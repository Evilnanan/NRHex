"""Generate immutable runtime Sword data from target parameters, never hand CSVs."""
import argparse
import csv
import hashlib
import json
import math
import re
from pathlib import Path
from sword_range_estimate import compare, smooth_arc_gravity

ROOT = Path(__file__).resolve().parents[1]
# Reviewed lobbed carriers with a real impact explosion or lava chain. Fireball
# (6010) is intentionally excluded even though it has a small impact child.
DROP_ROOTS = {4080: [10408000,10408050], 4800: [10480000,10480050],
              4820: [10482000,10482050], 6100: [10610000,10610050], 6310: [10631000,10631005],
              2006210: [210621000,210621050]}


def rules():
    code = (ROOT/'src/sword_projectile.rs').read_text(encoding='utf8')
    def ids(fn):
        body = re.search(r'fn '+fn+r'\(bullet:i32\)->bool\{(.*?)\}',code,re.S)[1]
        result=set()
        for item in re.search(r'matches!\(bullet,([^)]*)\)',body)[1].split('|'):
            pair=item.strip().split('..=')
            result.update([int(pair[0])] if len(pair)==1 else range(int(pair[0]),int(pair[1])+1))
        return result
    stages=(ROOT/'src/sword_projectile_stages.rs').read_text(encoding='utf8')
    continuation={tuple(map(int,p)) for p in re.findall(r'\((\d+),(\d+)\)',stages)}
    return ids('ground_projectile'),set(),ids('synchronized_flight'),ids('breath_stream'),continuation


def released_edges():
    code=(ROOT/'src/sword_projectile.rs').read_text(encoding='utf8')
    body=re.search(r'fn native_return\(parent:i32,child:i32\)->bool\{(.*?)\}',code,re.S)[1]
    return {tuple(map(int,p)) for p in re.findall(r'\((\d+),(\d+)\)',body)}


def generate(raw, mmv=False):
    tables={};ambiguous_attacks=set()
    for name,rows in raw['tables'].items():
        unique={}
        for r in rows:
            if r['ID'] in unique and unique[r['ID']]!=r['fields']:
                if name=='AtkParam_Pc':
                    ambiguous_attacks.add(r['ID']);continue
                raise ValueError(f'Ambiguous parameter rows: {name}/{r["ID"]}')
            unique[r['ID']]=r['fields']
        tables[name]=unique
    bullets,magic,attacks=(tables[n] for n in ['Bullet','Magic','AtkParam_Pc'])
    names={int(k):v for k,v in raw['names'].items()}
    eligible={i for i,r in magic.items() if 4000<=i<8000 or
              (mmv and i>8000 and i not in (8100,8101,999999999) and i in names and r['ezStateBehaviorType'] in (0,1))}
    ground,crystal,sync,breath,continuations=rules()
    released=released_edges()
    # MMV Black Flame Serpent reuses an existing ground-fire art chain.
    if 6908 in magic and magic[6908]['refId1']==12407000:
        chain=list(range(12407000,12408400,100))
        for a,b in zip(chain,chain[1:]):
            assert bullets[a]['HitBulletID']==b and bullets[a]['FollowType']==0 and bullets[a]['gravityInRange']>=100
            continuations.add((a,b))
    # Glintstone Nail/Nails projectiles alternate motion and a stationary
    # delay. Both edges carry the same flight origin, not a new child budget.
    for mid,starts in [(2004500,(210450000,210450050)),(2004510,(210451000,210451050))]:
        if mid not in eligible:continue
        mr=magic[mid]
        actual={mr[f'refId{i}'] for i in range(1,11) if mr[f'refCategory{i}']==1 and mr[f'refId{i}']>0}
        assert actual==set(starts),f'Finger spell roots changed: {mid}'
        for start in starts:
            for offset in range(11):
                bid=start+offset;v=bullets[bid]
                assert v['HitBulletID']==(bid+1 if offset<10 else -1)
                assert v['intervalCreateBulletId']==-1 and v['FollowType']==0 and not v['isSyncBulletCulcDumypolyPos']
                assert v['numShoot']==(6 if mid==2004510 and offset==0 else 1)
                assert v['launchConditionType']==4 and abs(v['life']-.2)<.0001
                assert (v['initVellocity']==0)==bool(offset%2)
                if offset<10:continuations.add((bid,bid+1))
    for parent,child in released:
        if parent not in bullets:continue
        v=bullets[parent]
        if child not in (v["HitBulletID"],v["intervalCreateBulletId"]):
            raise ValueError(f"Reviewed native branch changed: {parent}/{child}")
    def free(b):
        r=bullets[b]
        return (r['FollowType']==0 or r['FollowType']==4 and b in breath) and (not r['isSyncBulletCulcDumypolyPos'] or b in sync)
    def motion(b):
        return any(abs(bullets[b][k])>.0001 for k in ['initVellocity','accelInRange','accelOutRange','gravityInRange','gravityOutRange'])
    def useful(a):
        if a in ambiguous_attacks:raise ValueError(f'Spell references ambiguous attack {a}')
        r=attacks.get(a,{})
        return any(r.get(k,0)>0 for k in ['atkPhys','atkMag','atkFire','atkThun','atkDark','dmgLevel'])
    roots={}; attack_map={}; bonuses={}; report=[]; drops=set(); ends={}
    for mid in sorted(eligible):
        r=magic[mid]
        start={r[f'refId{i}'] for i in range(1,11) if r[f'refCategory{i}']==1 and r[f'refId{i}'] in bullets}
        for b in start:roots.setdefault(b,set()).add(mid)
        aids={r[f'refId{i}'] for i in range(1,11) if r[f'refCategory{i}']==0 and useful(r[f'refId{i}'])}
        todo=list(start); seen=set()
        while todo:
            b=todo.pop()
            if b in seen:continue
            seen.add(b);v=bullets[b]
            if useful(v['atkId_Bullet']):aids.add(v['atkId_Bullet'])
            for k in ['HitBulletID','intervalCreateBulletId']:
                child=v[k]
                if child>0 and child in bullets and (b,child) not in released:todo.append(child)
        # Returning discs stop range tracking, not damage/recovery eligibility.
        damage_seen=set();damage_todo=list(start)
        while damage_todo:
            b=damage_todo.pop()
            if b<=0 or b not in bullets or b in damage_seen:continue
            damage_seen.add(b);v=bullets[b]
            if useful(v['atkId_Bullet']):aids.add(v['atkId_Bullet'])
            damage_todo.extend([v['HitBulletID'],v['intervalCreateBulletId']])
        moving={b for b in seen if free(b) and motion(b)}
        # Preserve native expiry effects, never synthesize collision-only ones.
        # Ground probes must reach actual terrain; continuations keep their budget.
        spell_ends={}
        for b in sorted(moving):
            v=bullets[b];child=v['HitBulletID'];c=bullets.get(child)
            if mid in (4210,6010) or b in crystal or b in DROP_ROOTS.get(mid,[]) or not c or child<=0 or (b,child) in continuations:
                continue
            if max(abs(v['gravityInRange']),abs(v['gravityOutRange']))>=100 or v['shootAngleXZ']<=-60:
                continue
            if not motion(child) and c['FollowType']==0 and c['launchConditionType'] in (0,3,5):
                spell_ends[b]=child
        if mid in (4360,4361):
            # Both moons end in a stationary blast. Its residual acceleration
            # is 0.1 despite both speed bounds being zero; motion() alone
            # therefore mistakes it for another flying stage.
            parent=10436000 if mid==4360 else 10436100
            child=parent+1;v=bullets[parent];c=bullets[child]
            assert start=={parent} and v['HitBulletID']==child
            assert c['FollowType']==0 and c['launchConditionType']==0
            assert all(c[k]==0 for k in ('initVellocity','minVellocity','maxVellocity','gravityInRange','gravityOutRange'))
            assert abs(c['accelInRange']-.1)<.0001 and c['accelOutRange']==0
            assert c['HitBulletID']==-1 and c['intervalCreateBulletId']==-1 and useful(c['atkId_Bullet'])
            spell_ends[parent]=child
        if mid==6110:
            assert bullets[10611003]['HitBulletID']==10611004 and bullets[10611004]['HitBulletID']==10611005
            assert bullets[10611004]['launchConditionType']==0 and not motion(10611005)
            spell_ends[10611003]=10611004  # native ignition cue before explosion
        if mid==6720:
            chain=[10672000,10672004,10672005,10672006,10672002,10672003]
            assert all(bullets[a]['HitBulletID']==b and bullets[b]['launchConditionType']==0 for a,b in zip(chain,chain[1:]))
            assert not motion(chain[-1])
            spell_ends.update({b:chain[-1] for b in chain[:-1]})
        ends.update(spell_ends)
        # Preserve the existing explicit Founding Rain range exception. With no
        # comparable tracked flight its automatic percentage is zero.
        value=0.;method='无可比较飞行距离，自动0%'
        if moving and mid!=4210:
            value,_,method,_=compare(mid,seen,moving,bullets,continuations,ground,crystal,6.,3.,start)
        if not math.isfinite(value) or value<0:raise ValueError(f'Invalid automatic distance {mid}: {value}')
        if mid==2007810:
            assert start=={210781000,210781050}
            assert all(bullets[b]['EmittePosType']==6 and bullets[b]['initVellocity']==0 for b in start)
            value=max(bullets[b]['dist'] for b in start)
            method='目标召唤距离：原生EmittePosType=6读取dist；不是落火飞行距离；超距回退施术者'
        value=round(value,1)
        bonuses[mid]=value
        for a in aids:attack_map.setdefault(a,set()).add(mid)
        for b in DROP_ROOTS.get(mid,[]):
            v=bullets.get(b);c=bullets.get(v['HitBulletID']) if v else None
            if b not in start or not v or not c or not free(b) or v['initVellocity']<=0 or not 0<max(v['gravityInRange'],v['gravityOutRange'])<100:
                raise ValueError(f'Reviewed falling carrier changed: {mid}/{b}')
            impact_seen=set();impact_todo=[v['HitBulletID']]; impact_damage=False
            while impact_todo:
                child=impact_todo.pop()
                if child not in bullets or child in impact_seen:continue
                impact_seen.add(child);cr=bullets[child]
                impact_damage |= cr['initVellocity']==0 and useful(cr['atkId_Bullet'])
                impact_todo.extend([cr['HitBulletID'],cr['intervalCreateBulletId']])
            if c['initVellocity']!=0 or not impact_damage:
                raise ValueError(f'Reviewed impact damage changed: {mid}/{b}')
            drops.add(b)
        report.append(dict(MagicID=mid,name=names.get(mid,str(mid)),percentage=value,method=method,
                           quickDrop=DROP_ROOTS.get(mid,[]),nativeRangeEnds=spell_ends))
    lines=['// Generated by tools/build_sword_auto.py; do not edit percentages.',
           '// 1 metre of estimated original travel = 1 percentage point. No hand CSV input.',
           f'// Regulation SHA256: {raw["regulationSha256"]}',
           'pub const DEFAULT_BONUSES: &[(i32,f32)] = &[']
    lines += [f' ({i},{v:.1f}f32),' for i,v in sorted(bonuses.items())]
    lines += ['];']
    for name,values in [('ATTACKS',attack_map),('ROOTS',roots)]:
        lines += [f'pub const {name}: &[(i32,&[i32])] = &[']
        lines += [f' ({i},&[{",".join(map(str,sorted(v)))}]),' for i,v in sorted(values.items())]
        lines += ['];']
    lines += ['pub const QUICK_DROP: &[i32] = &['+','.join(map(str,sorted(drops)))+'];']
    lines += ['pub const DROP_ARCS: &[(i32,f32)] = &[']
    lines += [f' ({bid},{smooth_arc_gravity(bullets[bid]):.6f}f32),' for bid in sorted(drops)]
    lines += ['];']
    lines += ['pub const FLIGHT_CONTINUATIONS: &[(i32,i32)] = &[']
    lines += [f' ({parent},{child}),' for parent,child in sorted(continuations)]
    lines += ['];']
    lines += ['pub const RANGE_ENDS: &[(i32,i32)] = &[']
    lines += [f' ({bid},{child}),' for bid,child in sorted(ends.items())]
    lines += ['];']
    return '\n'.join(lines)+'\n',dict(regulationSha256=raw['regulationSha256'],spells=report,
                                     quickDrop=sorted(drops),manualBalanceInputs=False,
                                     unreferencedAmbiguousAttacks=sorted(ambiguous_attacks))


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--mmv',action='store_true');p.add_argument('--overlay')
    a=p.parse_args()
    source=ROOT/('work/compat/mmv/sword-parameters.json' if a.mmv else 'work/sword-range-export/parameters.json')
    raw=json.loads(source.read_text(encoding='utf8'))
    regulation=(Path(a.overlay) if a.overlay else ROOT/('dist/mmv' if a.mmv else 'dist'))/'mod/regulation.bin'
    if hashlib.sha256(regulation.read_bytes()).hexdigest()!=raw['regulationSha256'].lower():raise ValueError('Stale target regulation export')
    text,report=generate(raw,a.mmv)
    output=ROOT/('work/compat/mmv/staged/sword_map.rs' if a.mmv else 'src/sword_map.rs')
    output.parent.mkdir(parents=True,exist_ok=True);output.write_text(text,encoding='utf8')
    dest=ROOT/('work/compat/mmv/sword-auto.json' if a.mmv else 'work/sword-range-export/automatic-bonuses.json')
    dest.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
    review=ROOT/('docs/research/sword-auto-mmv.csv' if a.mmv else 'docs/research/sword-auto-native.csv')
    with review.open('w',encoding='utf-8-sig',newline='') as f:
        w=csv.writer(f);w.writerow(['MagicID','名称','自动基础加成%','平滑下坠弹体','估算口径'])
        for r in report['spells']:w.writerow([r['MagicID'],r['name'],r['percentage'],'|'.join(map(str,r['quickDrop'])),r['method']])
    print(f'Automatic Sword: {len(report["spells"])} spells, {len(report["quickDrop"])} reviewed falling carriers; {output}')


if __name__=='__main__':main()
