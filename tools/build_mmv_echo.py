"""Generate a reviewed MMV echo catalogue without changing native data or assets."""
import argparse
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / 'compat/mmv/echo-spells.json'


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def tables(raw):
    result = {}
    for name in ('Magic', 'Bullet', 'AtkParam_Pc'):
        grouped = {}
        for row in raw['tables'][name]:
            variants = grouped.setdefault(row['ID'], [])
            if row['fields'] not in variants:
                variants.append(row['fields'])
        if name != 'AtkParam_Pc' and any(len(v) != 1 for v in grouped.values()):
            raise ValueError(f'Ambiguous {name} rows')
        result[name] = grouped if name == 'AtkParam_Pc' else {i: v[0] for i, v in grouped.items()}
    return result


def roots(magic):
    return sorted({magic[f'refId{i}'] for i in range(1, 11)
                   if magic[f'refCategory{i}'] == 1 and magic[f'refId{i}'] > 0})


def descendants(ids, bullets):
    seen = set()
    todo = list(ids)
    while todo:
        bid = todo.pop()
        if bid in seen or bid <= 0:
            continue
        if bid not in bullets:
            raise ValueError(f'Missing bullet {bid}')
        seen.add(bid)
        todo.extend(bullets[bid][key] for key in ('HitBulletID', 'intervalCreateBulletId'))
    return seen


def evidence(mid, data):
    magic = data['Magic'][mid]
    chain = sorted(descendants(roots(magic), data['Bullet']))
    attacks = sorted({data['Bullet'][bid]['atkId_Bullet'] for bid in chain})
    return digest(dict(magic=magic, bullets={i: data['Bullet'][i] for i in chain},
                       attacks={i: data['AtkParam_Pc'].get(i, []) for i in attacks}))


def damaging(bid, data):
    return any(any(a.get(k, 0) > 0 for k in ('atkPhys', 'atkMag', 'atkFire', 'atkThun', 'atkDark'))
               for child in descendants([bid], data['Bullet'])
               for a in data['AtkParam_Pc'].get(data['Bullet'][child]['atkId_Bullet'], []))


def body(source, name):
    return re.search(r'pub const ' + name + r':[^=]+=&\[(.*?)\];', source, re.S)[1]


def ids(source, name):
    return list(map(int, re.findall(r'\d+', body(source, name))))


def replace(source, name, value):
    return re.sub(r'(pub const ' + name + r':[^=]+=&\[).*?(\];)',
                  lambda m: m[1] + value + m[2], source, flags=re.S)


def generate(raw, profile, native):
    data = tables(raw)
    original = json.loads((ROOT / 'data/reference/combat/sword-graph.json').read_text(encoding='utf8'))
    added = set(data['Magic']) - {int(i) for i in original['Magic']}
    entries = profile['spells']
    if profile['schemaVersion'] != 1 or [p['magic'] for p in entries] != sorted(added):
        raise ValueError('MMV Magic additions changed; review every new entry')
    if ids(native, 'EXCLUDED') != [4110, 4200, 4520, 4700, 4701]:
        raise ValueError('Requested native exclusions changed')
    catalog = {int(mid): list(map(int, re.findall(r'\d+', value))) for mid, value in
               re.findall(r'\((\d+),&\[([^]]*)\]\)', body(native, 'ROOTS'))}
    original_catalog = dict(catalog)
    expanded, streams = set(ids(native, 'EXPANDED')), set(ids(native, 'STREAMS'))
    independent = set(ids(native, 'INDEPENDENT_HITS'))
    world = []
    enabled = []
    for p in entries:
        mid = p['magic']
        m = data['Magic'][mid]
        if evidence(mid, data) != p['evidenceSha256']:
            raise ValueError(f'Reviewed MMV spell/chain changed: {mid} {p["name"]}')
        selected = p['roots']
        if not selected:
            if not p.get('reason'):
                raise ValueError(f'Unexplained exclusion: {mid}')
            continue
        if m['ezStateBehaviorType'] not in (0, 1) or m['isEnchant'] or m['isShieldEnchant']:
            raise ValueError(f'Not an eligible spell: {mid}')
        if selected != sorted(set(selected)) or not set(selected) <= set(roots(m)):
            raise ValueError(f'Non-root or duplicate registration: {mid}')
        for bid in selected:
            b = data['Bullet'][bid]
            if b['life'] <= 0 or b['FollowType'] not in (0, 4):
                raise ValueError(f'Unreviewed persistent/attached root: {mid}/{bid}')
            if not damaging(bid, data) and p.get('support') != 'projectile-heal':
                raise ValueError(f'Non-damaging helper registered: {mid}/{bid}')
        for bid in p['worldRoots']:
            b = data['Bullet'][bid]
            if bid not in selected or any(b[k] != 0 for k in
                    ('FollowType', 'isSyncBulletCulcDumypolyPos', 'initVellocity', 'maxVellocity', 'gravityInRange')) or b['numShoot'] != 1:
                raise ValueError(f'World-area root is moving/attached/multi-shot: {mid}/{bid}')
            world.append((mid, bid))
        if p['stream']:
            if p['worldRoots']:
                raise ValueError('Stream rounds require captured per-shot matrices')
            streams.add(mid)
        catalog[mid] = selected
        expanded.add(mid)
        independent.add(mid)
        enabled.append(mid)
    # Magic=-1 reverse lookup must stay unique, including old support and held roots.
    all_roots = [bid for values in catalog.values() for bid in values]
    all_roots += [int(b) for _, b, _ in re.findall(r'\((\d+),(\d+),(\d+)\)', body(native, 'AREAS'))]
    all_roots += [int(b) for _, held, flight in re.findall(r'\((\d+),(\d+),(\d+)\)', body(native, 'RELEASES')) for b in (held, flight)]
    if len(all_roots) != len(set(all_roots)):
        raise ValueError('Ambiguous direct echo root')
    result = native.replace('// Generated from original regulation 10350000; reviewed finite spell roots.',
                            '// Generated MMV echo overlay; original catalogue preserved; reviewed additions only.')
    result = replace(result, 'ROOTS', '\n' + ''.join(f' ({mid},&[{",".join(map(str, values))}]),\n'
                                                   for mid, values in sorted(catalog.items())))
    for name, values in [('EXPANDED', expanded), ('STREAMS', streams), ('INDEPENDENT_HITS', independent), ('MMV_SPELLS', enabled)]:
        result = replace(result, name, ','.join(map(str, sorted(values))))
    result = replace(result, 'WORLD_ROOTS', ','.join(f'({mid},{bid})' for mid, bid in sorted(world)))
    report = dict(addedSpells=len(enabled), addedRoots=sum(len(catalog[mid]) for mid in enabled),
                  streams=[p['magic'] for p in entries if p['stream'] and p['roots']], worldRoots=world,
                  spells=entries, nativeEntriesUnchanged=all(catalog[mid] == value for mid, value in original_catalog.items()),
                  regulationSha256=raw['regulationSha256'], profileSha256=digest(profile),
                  nativeCatalogueSha256=hashlib.sha256(native.encode()).hexdigest(),
                  catalogueSha256=hashlib.sha256(result.encode()).hexdigest(), gameplayVerified=False, multiplayerVerified=False)
    return result, report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--overlay', type=Path, default=ROOT / 'work/compat/mmv/staged')
    parser.add_argument('--parameters', type=Path, default=ROOT / 'work/compat/mmv/sword-parameters.json')
    args = parser.parse_args()
    raw = json.loads(args.parameters.read_text(encoding='utf8'))
    regulation = args.overlay / 'mod/regulation.bin'
    if hashlib.sha256(regulation.read_bytes()).hexdigest().lower() != raw['regulationSha256'].lower():
        raise ValueError('Stale parameter export; export the selected overlay first')
    native = (ROOT / 'src/echo_cast_data.rs').read_text(encoding='utf8')
    result, report = generate(raw, json.loads(PROFILE.read_text(encoding='utf8')), native)
    (args.overlay / 'echo_cast_data.rs').write_text(result, encoding='utf8', newline='\n')
    (args.overlay / 'echo-validation.json').write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf8')
    print(f'PASS: MMV echo: {report["addedSpells"]} new spells, {report["addedRoots"]} roots; native entries unchanged')


if __name__ == '__main__':
    main()
