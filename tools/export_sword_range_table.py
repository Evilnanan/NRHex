"""Export current Sword limits, not simulated or measured maximum hit distances.

Requires HexBuild --sword-range-catalog. Does not regenerate runtime maps/balance.
"""
import csv
import argparse
import hashlib
import json
import re
from pathlib import Path

from sword_range_estimate import compare, SMOOTH_ARC_REACH
from build_sword_auto import generate

ROOT = Path(__file__).resolve().parents[1]


def export_target(mmv=False):
    source = 'work/compat/mmv/sword-parameters.json' if mmv else 'work/sword-range-export/parameters.json'
    snapshot = 'work/compat/mmv/staged/sword_map.rs' if mmv else 'src/sword_map.rs'
    regulation = 'dist/mmv/mod/regulation.bin' if mmv else 'dist/mod/regulation.bin'
    inputs = ['src/sword_projectile.rs', 'src/sword_projectile_stages.rs', snapshot,
              regulation, 'tools/sword_range_estimate.py', 'tools/export_sword_range_table.py',
              'tools/build_sword_auto.py', source]
    hashes = {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in inputs}
    code = (ROOT / inputs[0]).read_text(encoding='utf8')
    stages = (ROOT / inputs[1]).read_text(encoding='utf8')
    compiled = (ROOT / snapshot).read_text(encoding='utf8')
    mapping = compiled.split('pub const ROOTS:', 1)[1].split('pub const QUICK_DROP:', 1)[0]
    raw = json.loads((ROOT / source).read_text(encoding='utf8'))
    assert raw['regulationSha256'].lower() == hashes[regulation], 'Stale parameter export'
    expected, automatic = generate(raw, mmv)
    assert expected == compiled, 'Automatic runtime snapshot is stale'
    quick_drop = set(automatic['quickDrop'])
    tables = {}
    for table in ('Magic', 'Bullet'):
        rows = raw['tables'][table]
        tables[table] = {r['ID']: r['fields'] for r in rows}
        assert all(tables[table][r['ID']] == r['fields'] for r in rows), f'Ambiguous {table} IDs'
    bullets = tables['Bullet']
    roots = {int(b): set(map(int, re.findall(r'\d+', ids)))
             for b, ids in re.findall(r'\((\d+),&\[([^]]*)\]\)', mapping)}
    continuations = {tuple(map(int, pair)) for pair in re.findall(r'\((\d+),(\d+)\)', compiled.split('pub const FLIGHT_CONTINUATIONS:',1)[1].split('pub const RANGE_ENDS:',1)[0])}
    limit = float(re.search(r'pub const RANGE:f32=([\d.]+);', code)[1])
    crystal_limit = limit

    def match_ids(function):
        body = re.search(r'fn ' + function + r'\(bullet:i32\)->bool\{(.*?)\}', code, re.S)[1]
        pattern = re.search(r'matches!\(bullet,([^)]*)\)', body)[1]
        result = set()
        for item in pattern.split('|'):
            ends = item.strip().split('..=')
            if len(ends) == 1:
                result.add(int(ends[0]))
            else:
                result.update(range(int(ends[0]), int(ends[1]) + 1))
        return result

    crystal = set()
    ground = match_ids('ground_projectile')
    sync = match_ids('synchronized_flight')
    breath = match_ids('breath_stream')
    returns = {tuple(map(int, p)) for p in re.findall(r'\((\d+),(\d+)\)',
        re.search(r'fn native_return\(.*?\{(.*?)\}', code, re.S)[1])}
    # Fail rather than silently publishing a stale exception after code changes.
    assert 'magic!=4210&&magics.contains(&magic)' in code
    assert 'follow==0||(follow==4&&breath_stream(bullet))' in code
    assert '10670000|10670100|10693000' in code

    def free(bid):
        r = bullets[bid]
        return (r['FollowType'] == 0 or (r['FollowType'] == 4 and bid in breath)) and (
            not r['isSyncBulletCulcDumypolyPos'] or bid in sync)

    def motion(bid):
        r = bullets[bid]
        return any(abs(r[k]) > 0.0001 for k in ('initVellocity', 'accelInRange', 'accelOutRange', 'gravityInRange', 'gravityOutRange'))

    fields = ['MagicID', '名称', '类别', '版本范围', '当前处理', '计距上限米', '计距方式', '到达上限后', '说明']
    comparison_fields = ['原飞行距离估算米', '限制后飞行距离估算米', '缩短估算米', '缩短估算比例%', '估算口径']
    output = []
    detail = []
    for spell in automatic['spells']:
        mid = int(spell['MagicID'])
        magic = tables['Magic'][mid]
        start = {magic[f'refId{i}'] for i in range(1, 11) if magic[f'refCategory{i}'] == 1}
        tracked = {b for b in start if b in roots and mid in roots[b] and mid != 4210 and b in bullets}
        todo = list(tracked)
        seen = set()
        inherited = set()
        released = set()
        while todo:
            b = todo.pop()
            if b in seen:
                continue
            seen.add(b)
            for key in ['HitBulletID', 'intervalCreateBulletId']:
                child = bullets[b][key]
                if child<=0 or child not in bullets:
                    continue
                if (b, child) in returns:
                    released.add(child)
                    continue
                if (b, child) in continuations:
                    inherited.add((b, child))
                todo.append(child)
        candidates = {b for b in seen if free(b)}
        moving = {b for b in candidates if motion(b)}
        row = dict(zip(fields, [mid, spell['name'], '魔法' if magic['ezStateBehaviorType'] == 0 else '祷告', 'MMV当前参数' if mmv else '独立版当前参数', '', '', '', '', '']))
        row['自动基础加成%'] = f"{spell['percentage']:.1f}"
        row['特殊下坠处理'] = '无'
        notes = []
        if mid == 2007810:
            before=max(bullets[b]['dist'] for b in tracked);after=min(before,limit)
            row.update(当前处理='原生目标召唤距离限制',计距上限米=f'{limit:g}',计距方式='原生施术者/目标三维距离',
                       到达上限后='超距沿用原生回退，在施术者位置生成；完整保留云和落火',说明='只限制召唤位置；不剪短云寿命和落火高度')
            row.update(dict(zip(comparison_fields,[f'{before:.1f}',f'{after:.1f}',f'{before-after:.1f}',f'{100*(before-after)/before:.1f}',spell['method']])))
        elif mid == 4210:
            row.update(当前处理='明确排除', 说明='创星雨不进行亮剑射程跟踪')
        elif not tracked:
            row.update(当前处理='无已登记弹体入口', 说明='不按此弹体规则缩短；直接攻击、辅助或附魔范围不能由本表推算')
        elif not moving:
            row.update(当前处理='静态数据未发现可计距运动', 说明='静止、附着或同步组件保留原生范围；若运行时脱手并满足自由飞行条件才开始计距')
        else:
            row.update(当前处理='弹体计距限制', 计距上限米=f'{limit:g}',
                       计距方式='各独立弹体发射原点的三维直线位移', 到达上限后='超出上限退场')
            if set(spell['nativeRangeEnds']) & seen:
                row['到达上限后']='进入已核对的原生终末效果；原生派生条件仍生效'
                notes.append('超距时保留爆炸/停留/引爆前摇；后续效果不再硬裁剪；不是伪造墙体碰撞')
            if seen & ground:
                row['计距方式'] = '已登记落雷定位组件按水平位移，其余三维位移'
            if seen & quick_drop:
                row.update(当前处理='平滑抛射下坠', 计距上限米=f'约{SMOOTH_ARC_REACH:g}（平地默认瞄准目标，非上限）',
                           计距方式='特殊载体不设6米截断；按出手速度和法术抛射角校准下落曲线',
                           到达上限后='不截断；保留原生碰撞和落地AOE')
                row['特殊下坠处理'] = f'出手起平滑下落，保留向前速度；平地默认瞄准约{SMOOTH_ARC_REACH:g}米；高抛/地形允许超出'
                notes.append('按发射高度1.5米校准；保留实际出手方向与水平追踪；不硬折向、不补寿命；保留原生落地爆炸/熔岩链')
            if seen & {10670000, 10670100}:
                row['到达上限后'] = f'{limit:g}米进入原生停顿和回收'
                notes.append('停顿和回收分支解除射程跟踪')
            elif seen & {210671000,210671050}:
                row['到达上限后'] = f'{limit:g}米进入原生停留阶段，持续产生伤害'
                notes.append('停留阶段解除射程跟踪；保留普通2秒/蓄力2.5秒及原生特效交接')
            elif seen & {10491000,210491000,210491001,210491002}:
                row['到达上限后'] = f'水平{limit:g}米进入终点探地和荆棘伤害；沿途探地正常保留'
                notes.append('只缩短前行定位段；终点和沿途的地面探针不裁剪垂直高度')
            elif seen & {210470000,210470050}:
                row['到达上限后'] = f'水平{limit:g}米开始原生岩刃生成链'
                notes.append('保留两侧探地、升起及蓄力接续；生成后的范围不计作前方定位距离')
            elif seen & {210772000,210670000,210741000,210741050}:
                row['到达上限后'] = f'水平{limit:g}米进入原生向下探地，落地后完整生成效果'
                notes.append('后续探地、前摇和向上伤害链解除射程跟踪，不在空中伪造落地')
            elif 10693000 in seen:
                row['到达上限后'] = f'水平{limit:g}米进入原生向下定位和落雷'
            if inherited:
                notes.append('已登记接续组件沿用原点；真正分裂的子弹独立计距')
            if seen - moving:
                notes.append('仅限制满足条件的运动部分；静止范围、贴身攻击及碰撞半径未裁剪')
            notes.append('限制不延长原生短射程；不是施法者到目标的最大命中距离')
            row['说明'] = '；'.join(notes)
        output.append(row)
        estimates = {}
        row.update(dict.fromkeys(comparison_fields, ''))
        if mid == 2007810:
            before=max(bullets[b]['dist'] for b in tracked);after=min(before,limit)
            row.update(当前处理='原生目标召唤距离限制',计距上限米=f'{limit:g}',计距方式='原生施术者/目标三维距离',
                       到达上限后='超距沿用原生回退，在施术者位置生成；完整保留云和落火',说明='只限制召唤位置；不剪短云寿命和落火高度')
            row.update(dict(zip(comparison_fields,[f'{before:.1f}',f'{after:.1f}',f'{before-after:.1f}',f'{100*(before-after)/before:.1f}',spell['method']])))
        elif mid == 4210:
            row.update(缩短估算米='0', **{'缩短估算比例%': '0'}, 估算口径='明确排除，此规则不缩短')
        elif not moving:
            row['估算口径'] = '无可比较的自由飞行距离；留空不代表0米'
        else:
            before, after, method, estimates = compare(mid, seen, moving, bullets,
                continuations, ground, crystal, limit, crystal_limit, tracked, quick_drop)
            reduction = before - after
            row.update(dict(zip(comparison_fields, [f'{before:.1f}', f'{after:.1f}',
                f'{reduction:.1f}', f'{100 * reduction / before:.1f}' if before else '0', method])))
        detail.append(dict(MagicID=mid, roots=sorted(tracked), traversedBullets=sorted(seen),
                           movingCandidates=sorted(moving), continuationEdges=sorted(inherited),
                           nativeReturnBranches=sorted(released), nominalFlightEstimates=estimates))
    assert all(hashlib.sha256((ROOT / p).read_bytes()).hexdigest() == h for p, h in hashes.items())
    return output, fields[:6] + ['特殊下坠处理', '自动基础加成%'] + comparison_fields + fields[6:], dict(inputs=hashes, details=detail)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', default='docs/research/sword-range-current.csv')
    parser.add_argument('--mmv-output', default='docs/research/sword-range-mmv.csv')
    args = parser.parse_args()
    native, fields, native_report = export_target()
    mmv, mmv_fields, mmv_report = export_target(True)
    assert fields == mmv_fields
    native_ids = {r['MagicID'] for r in native}
    combined = sorted(native + [r for r in mmv if r['MagicID'] not in native_ids], key=lambda r:r['MagicID'])
    for destination, rows in [(args.output, combined), (args.mmv_output, mmv)]:
        path = ROOT / destination
        assert len(rows) == len({r['MagicID'] for r in rows})
        with path.open('w', encoding='utf-8-sig', newline='') as f:
            writer = csv.DictWriter(f, fields)
            writer.writeheader()
            writer.writerows(rows)
        print(f'Exported {len(rows)} rows: {path}')
    report = dict(interpretation='Parameter estimates, not measured hit distances', native=native_report, mmv=mmv_report)
    (ROOT / 'work/sword-range-export/report.json').write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf8')


if __name__ == '__main__':
    main()
