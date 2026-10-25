"""Enumerate native/MMV spell branches and document Sword's current treatment.

Static parameter/code audit, not proof of gameplay, rendering or collision.
"""
import csv
import hashlib
import json
import re
from collections import Counter
from pathlib import Path

from build_sword_auto import generate, released_edges, rules

ROOT = Path(__file__).resolve().parents[1]
MOTION = ('initVellocity', 'accelInRange', 'accelOutRange', 'gravityInRange', 'gravityOutRange')


def audit(mmv=False):
    source = ROOT / ('work/compat/mmv/sword-parameters.json' if mmv else 'work/sword-range-export/parameters.json')
    raw = json.loads(source.read_text(encoding='utf8'))
    regulation = ROOT / ('dist/mmv' if mmv else 'dist') / 'mod/regulation.bin'
    assert hashlib.sha256(regulation.read_bytes()).hexdigest() == raw['regulationSha256'].lower()
    code, report = generate(raw, mmv)
    bullets = {r['ID']: r['fields'] for r in raw['tables']['Bullet']}
    magic = {r['ID']: r['fields'] for r in raw['tables']['Magic']}
    ground, crystal, sync, breath, continuation = rules()
    released = released_edges()
    continuation = {tuple(map(int,p)) for p in re.findall(r'\((\d+),(\d+)\)',code.split('pub const FLIGHT_CONTINUATIONS:',1)[1].split('pub const RANGE_ENDS:',1)[0])}
    output = []
    for spell in report['spells']:
        mid = spell['MagicID']
        m = magic[mid]
        todo = [m[f'refId{i}'] for i in range(1, 11) if m[f'refCategory{i}'] == 1]
        seen = set()
        branches = []
        while todo:
            bid = todo.pop()
            if bid <= 0 or bid not in bullets or bid in seen:
                continue
            seen.add(bid)
            p = bullets[bid]
            for field in ('HitBulletID', 'intervalCreateBulletId'):
                child = p[field]
                if child <= 0 or child not in bullets:
                    continue
                todo.append(child)
                q = bullets[child]
                moving = any(abs(p[k]) > .0001 for k in MOTION)
                free = (p['FollowType'] == 0 or p['FollowType'] == 4 and bid in breath) and (
                    not p['isSyncBulletCulcDumypolyPos'] or bid in sync)
                if mid == 4210:
                    treatment = '明确排除；保留原生'
                elif (bid, child) in released:
                    treatment = '原生停留/回收/探地分支解除计距'
                elif bid in spell['quickDrop']:
                    treatment = '平滑下坠；实际碰撞后原生派生'
                elif field == 'HitBulletID' and bid in spell['nativeRangeEnds']:
                    treatment = '6米原生终末流程；后续效果完整保留'
                elif bid in crystal and field == 'HitBulletID':
                    treatment = '3米原生爆散；碎片独立计距'
                elif (bid, child) in continuation:
                    treatment = '同一弹体接续；继承原点'
                elif not moving or not free:
                    treatment = '静止/附着阶段；不因前行距离销毁'
                elif field == 'intervalCreateBulletId':
                    treatment = '沿途周期派生；父体仍按自身限制'
                elif p['shootAngleXZ'] <= -60 or max(abs(p['gravityInRange']), abs(p['gravityOutRange'])) >= 100:
                    treatment = '实际探地/下落；不伪造碰撞（另看已释放上游）'
                elif q['launchConditionType'] not in (0, 3, 5):
                    treatment = '条件派生；不将射程结束伪装为命中'
                else:
                    treatment = '运动接续/独立子弹；保留现有计距，未强制跳到末段'
                branches.append(dict(版本='MMV' if mmv else '独立版', MagicID=mid, 名称=spell['name'],
                    父弹体=bid, 派生字段=field, 子弹体=child, 父寿命=p['life'], 父初速=p['initVellocity'],
                    子生成条件=q['launchConditionType'], 子特效=q['sfxId_Bullet'], 子伤害=q['atkId_Bullet'],
                    处理=treatment, 强制终末子弹=spell['nativeRangeEnds'].get(bid, ''),
                    验证边界='当前目标参数与代码静态核对；未逐个实机验证'))
        if not branches:
            branches.append(dict(版本='MMV' if mmv else '独立版', MagicID=mid, 名称=spell['name'],
                处理='已登记链无正ID派生；不证明动画/脚本无其他入口', 验证边界='静态核对'))
        output.extend(branches)
    return output


def main():
    rows = audit() + audit(True)
    fields = ['版本', 'MagicID', '名称', '父弹体', '派生字段', '子弹体', '父寿命', '父初速',
              '子生成条件', '子特效', '子伤害', '处理', '强制终末子弹', '验证边界']
    path = ROOT / 'docs/research/sword-end-effects.csv'
    with path.open('w', encoding='utf-8-sig', newline='') as f:
        writer = csv.DictWriter(f, fields)
        writer.writeheader()
        writer.writerows(rows)
    print(f'Audited {len({(r["版本"], r["MagicID"]) for r in rows})} target spells, {len(rows)} branch/summary rows: {path}')
    print(json.dumps(Counter(r['处理'] for r in rows), ensure_ascii=False))


if __name__ == '__main__':
    main()
