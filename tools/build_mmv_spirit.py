"""Generate private summons from the selected overlay and actual MMV boss HUD bindings."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct

ROOT = Path(__file__).resolve().parents[1]
REPORTS = ROOT / 'work/compat/mmv'
FIRST, LIMIT = 95400000, 95500000


def pairs(path):
    return [tuple(map(int, m)) for m in re.findall(r'\(\s*(-?\d+)\s*,\s*(-?\d+)\s*\)', path.read_text(encoding='utf8'))]


def boss_bindings(maps, exported):
    events = {name: {e['ID']: e for e in rows} for name, rows in exported.items()}
    common = {**events.get('common', {}), **events.get('common_func', {})}
    proof = []
    for name, enemies in maps.items():
        local = {**common, **events.get(name, {}), **events.get(name + '_dlc01', {})}
        wanted, seen = set(), set()

        def walk(event, arguments=b''):
            key = event['ID'], arguments
            if key in seen:
                return
            seen.add(key)
            if len(seen) > 100000:
                raise ValueError('Unbounded event expansion: ' + name)
            for index, instruction in enumerate(event['instructions']):
                data = bytearray.fromhex(instruction['hex'])
                unresolved = False
                for binding in event['Parameters']:
                    if binding['InstructionIndex'] != index:
                        continue
                    start, n, target = (binding[k] for k in ('SourceStartByte', 'ByteCount', 'TargetStartByte'))
                    if start + n > len(arguments):
                        unresolved = True
                    else:
                        data[target:target+n] = arguments[start:start+n]
                if unresolved:
                    continue
                bank, op = instruction['Bank'], instruction['ID']
                if bank == 2003 and op in (11, 102) and len(data) >= 12:
                    entity = struct.unpack_from('<I', data, 4 if op == 11 else 8)[0]
                    if entity > 20000:
                        wanted.add(entity)
                elif bank == 2000 and op in (0, 6) and len(data) >= 8:
                    target = struct.unpack_from('<I', data, 4)[0]
                    if target in local:
                        walk(local[target], bytes(data[8:]))
        if 0 in local:
            walk(local[0])
        for event in events.get(name, {}).values():
            if not event['Parameters']:
                walk(event)
        for enemy in enemies:
            if enemy['entity'] in wanted or any(g and g in wanted for g in enemy['groups']):
                proof.append(dict(map=name, npc=enemy['NpcParamId'], entity=enemy['entity'], model=enemy['ModelName'], direct=enemy['entity'] in wanted))
    return proof


def generate(data):
    rows = {r['ID']: r for r in data['rows']}
    if len(rows) != len(data['rows']):
        raise ValueError('Ambiguous NPC rows')
    native = pairs(ROOT / 'src/spirit_npcs.rs')
    if any(id != FIRST + i for i, (_, id) in enumerate(native)):
        raise ValueError('Native private IDs are no longer contiguous')
    registry = json.loads((ROOT / 'compat/mmv/spirit-npc-ids.json').read_text(encoding='utf8'))['pairs']
    established = native + [tuple(pair) for pair in registry]
    if len({npc for npc, _ in established}) != len(established) or any(id != FIRST+i for i, (_,id) in enumerate(established)):
        raise ValueError('Invalid established MMV private ID registry')
    existing = dict(established)
    models = {}
    for enemies in data['maps'].values():
        for e in enemies:
            if re.fullmatch(r'c\d{4}', e['ModelName'] or ''):
                models.setdefault(e['NpcParamId'], set()).add(int(e['ModelName'][1:]))
    ambiguous = sorted(npc for npc, found in models.items() if len(found) != 1)
    unsupported = sorted(npc for npc, row in rows.items() if npc not in existing and not FIRST <= npc < LIMIT
                         # npcType=1 means Boss, not a player/CharaInit descriptor.
                         and (models.get(npc, {npc // 10000 % 10000}) & {0, 1000}
                              or (npc >= 100000000 and npc not in models)))
    # Preserve all established private IDs. Append new rows in stable source order.
    extra = sorted(npc for npc in rows if npc >= 0 and not FIRST <= npc < LIMIT
                   and npc not in existing and npc not in ambiguous and npc not in unsupported)
    mapping = established + [(npc, FIRST + len(established) + i) for i, npc in enumerate(extra)]
    if FIRST + len(mapping) > LIMIT:
        raise ValueError('Spirit private ID range exhausted')
    # New private rows must not overwrite any unrelated MMV row.
    for npc, private in mapping[len(native):]:
        if private in rows:
            raise ValueError(f'MMV/private NPC collision: {private}')
    cage = dict(pairs(ROOT / 'src/spirit_cage_models.rs'))
    model_pairs = []
    for npc, private in mapping:
        if npc not in rows:
            raise ValueError(f'Missing established spirit source: {npc}')
        found = models.get(npc, set())
        model = cage.get(npc, next(iter(found)) if len(found) == 1 else npc // 10000 % 10000)
        model_pairs.append((private, model))
    proof = boss_bindings(data['maps'], data['events'])
    supported = {npc for npc, _ in mapping}
    bosses = sorted({p['npc'] for p in proof} & supported)
    new = mapping[len(native):]
    def array(name, entries):
        return f'pub const {name}: &[(i32,i32)] = &[\n' + ''.join(f' ({a},{b}),\n' for a, b in entries) + '];\n'
    rust = '// Generated MMV private NPCs; native private IDs are preserved.\n'
    rust += array('EXTRA_NPCS', sorted(new)) + array('MODELS', sorted(model_pairs))
    rust += 'pub const BOSSES: &[i32] = &[' + ','.join(map(str, bosses)) + '];\n'
    plan = dict(regulationSha256=data['regulationSha256'], pairs=mapping)
    report = dict(existingPrivateRows=len(native), newPrivateRows=len(new), establishedMmvPrivateRows=len(registry), appendedPrivateRows=len(extra), bossNpcVariants=len(bosses),
                  bossBindings=proof, ambiguousModelNpcs=ambiguous, unsupportedDescriptorNpcs=unsupported, nativeKillSettlementUnchanged=True,
                  runtimeThreatTagFallback=True, sourceNpcRowsUnchanged=True, gameplayVerified=False)
    return rust, plan, report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--overlay', type=Path, required=True)
    args = parser.parse_args()
    data = json.loads((REPORTS / 'spirit-parameters.json').read_text(encoding='utf8'))
    if Path(data['overlay']).resolve() != args.overlay.resolve():
        raise ValueError('Spirit export belongs to a different overlay')
    def sha(path):
        return hashlib.sha256(path.read_bytes()).hexdigest()
    if sha(args.overlay / 'mod/regulation.bin') != data['regulationSha256']:
        raise ValueError('Spirit regulation changed since export')
    for path, expected in data['inputs'].items():
        if sha(Path(path)) != expected:
            raise ValueError('Spirit event/map input changed: ' + path)
    rust, plan, report = generate(data)
    output = REPORTS / 'staged'
    output.mkdir(parents=True, exist_ok=True)
    (output / 'spirit_target_data.rs').write_text(rust, encoding='utf8')
    for name, value in [('spirit-plan', plan), ('spirit-validation', report)]:
        (output / f'{name}.json').write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding='utf8')
    print(f"Spirit: {report['existingPrivateRows']} rebased + {report['newPrivateRows']} new private rows; "
          f"{report['bossNpcVariants']} HUD-bound NPC variants; ambiguous models {report['ambiguousModelNpcs']}")


if __name__ == '__main__':
    main()
