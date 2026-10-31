"""Keep the native switch-enchant chain; adapt ownership and its random result only."""
import argparse
import hashlib
import json
import struct
from pathlib import Path
from hks_bytecode import Reader, decode
from build_marking_hks import abc, abx
from mmv_hks_stage import hash_function

ROOT = Path(__file__).resolve().parents[1]
OWNER = 95004900
QUERY = 990049
MARKERS = tuple(range(6999981, 6999985))
FID = 218
NATIVE_HASH = 'fb0727ba21dd5618bad2ad32db62d74d79c9725f6cea47b80844e9a2f4f13729'


def reader(source):
    r = Reader(source); r.function(); return r


def patch(source):
    before = reader(source); f = before.functions[FID]
    assert f['name'] == 'WeaponChangeCommonFunction' and hash_function(f) == NATIVE_HASH, 'Native switch chain changed'
    cs = f['constants']; extra = []; encoded = bytearray()
    def const(v):
        if v in cs: return cs.index(v)
        if v in extra: return len(cs) + extra.index(v)
        index = len(cs) + len(extra)
        if isinstance(v, str):
            extra.extend([v, None, None]); b = v.encode() + b'\0'
            encoded.extend(b'\x04' + struct.pack('>Q', len(b)) + b + b'\0\0')
        else:
            assert struct.unpack('>f', struct.pack('>f', v))[0] == v, 'HKS number loses integer precision'
            extra.append(float(v)); encoded.extend(b'\x03' + struct.pack('>f', v))
        return index
    a = f['slots']
    def glob(reg, name): return [abx(91, reg, const(name)), abx(76, 20, const(name) + 1)]
    def query(effect):
        if effect == OWNER: return glob(a, 'env') + [abx(25, a+1, const(QUERY)), abc(2, a, 2, 2)]
        return glob(a, 'env') + [abx(25, a+1, const(1116)), abx(25, a+2, const(effect)), abc(2, a, 3, 2)]
    def act(code, effect): return glob(a, 'act') + [abx(25, a+1, const(code)), abx(25, a+2, const(effect)), abc(2, a, 3, 1)]
    def branch(code, effect, destination, fixups):
        code += query(effect) + glob(a+1, 'TRUE') + [abc(4, 0, a, a+1)]
        fixups.append((len(code), destination)); code.append(0)  # absent -> destination
    def finish(code, labels, fixups):
        for pc, label in fixups: code[pc] = abx(28, 0, 0x10000 + labels[label] - pc - 2)
        return code
    own = []; fix = []
    branch(own, OWNER, 'end', fix)
    own += glob(1, 'TRUE')
    finish(own, {'end': len(own)}, fix)
    rotate = []; fix = []; labels = {}
    branch(rotate, OWNER, 'end', fix)
    # The saved marker is the last selected element; no marker starts at Magic.
    for i, (prior, roll, next_marker) in enumerate(zip(MARKERS[:3], (50, 25, 1), MARKERS[1:])):
        branch(rotate, prior, f'next{i}', fix)
        rotate += [abx(25, 1, const(roll)), abx(25, a+3, const(next_marker))]
        fix.append((len(rotate), 'apply')); rotate.append(0)
        labels[f'next{i}'] = len(rotate)
    rotate += [abx(25, 1, const(75)), abx(25, a+3, const(MARKERS[0]))]
    labels['apply'] = len(rotate)
    for marker in MARKERS: rotate += act(9001, marker)
    # Selected marker remains in a+3, the second argument of this call.
    rotate += glob(a+1, 'act') + [abx(25, a+2, const(2002)), abc(2, a+1, 3, 1)]
    labels['end'] = len(rotate); finish(rotate, labels, fix)
    inserts = {8: own, 65: rotate}
    old = [w[0] for w in struct.iter_unpack('>I', f['code'])]
    start = lambda pc: pc + sum(len(v) for k, v in inserts.items() if k < pc)
    original = lambda pc: start(pc) + len(inserts.get(pc, []))
    code = []; lines = bytearray()
    for pc, word in enumerate(old):
        code += inserts.get(pc, [])
        lines += f['lines'][pc*4:pc*4+4] * (len(inserts.get(pc, [])) + 1)
        ins = decode(word)
        if ins['op'] == 28:
            dest = pc + 2 + ins['bx'] - 0x10000
            word = abx(28, ins['a'], 0x10000 + start(dest) - original(pc) - 2)
        code.append(word)
    while len(encoded) % 4: encoded.append(0); extra.append(None)
    u32 = lambda n: struct.pack('>I', n)
    edits = [(f['slots_offset'],4,u32(a+4)),(f['code_count_offset'],4,u32(len(code))),
             (f['code_offset'],len(f['code']),b''.join(u32(w) for w in code)),
             (f['constants_count_offset'],4,u32(len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),
             (f['line_count_offset'],4,u32(len(code))),(f['lines_offset'],len(f['lines']),bytes(lines))]
    for _, p, lo, q, hi in f['locals']: edits += [(p,4,u32(start(lo))),(q,4,u32(start(hi)))]
    target = source
    for p, n, b in sorted(edits, reverse=True): target = target[:p] + b + target[p+n:]
    after = reader(target)
    for left, right in zip(before.functions, after.functions):
        if left['id'] == FID: continue
        assert left['code'] == right['code'] and left['constants'] == right['constants']
        if not left['children']: assert source[left['start']:left['end']] == target[right['start']:right['end']]
    inverse = []; shift = 0
    for p, n, b in sorted(edits): inverse.append((p+shift,len(b),source[p:p+n])); shift += len(b)-n
    restored = target
    for p, n, b in reversed(inverse): restored = restored[:p]+b+restored[p+n:]
    assert restored == source
    ranges = {name:[start(pc),start(pc)+len(inserts[pc])] for name, pc in [('owner',8),('roll',65)]}
    return target, ranges


def check(source):
    expected, ranges = patch((ROOT/'assets/original/action/script/c0000.hks').read_bytes())
    f = reader(source).functions[FID]; e = reader(expected).functions[FID]
    assert all(f[k] == e[k] for k in ('code','constants','slots','lines')), 'Bartender patch missing or changed'
    return ranges


def write_plan(source, ranges, mmv):
    from mmv_hks_fragments import functions
    original = reader((ROOT/'assets/original/action/script/c0000.hks').read_bytes()).functions[FID]
    current = reader(source).functions[FID]
    text = (mmv/'mod/action/script/c0000.hks').read_bytes().decode('utf-8-sig')
    name = current['name']; span = functions(text)[name]
    helpers = [dict(name='NRHexMMV_BartenderOwner',range=ranges['owner'],register=1,old='env(GetSpEffectID, 7035700)'),
               dict(name='NRHexMMV_BartenderRoll',range=ranges['roll'],register=1,old='math.random(1, 100)')]
    path = ROOT/'compat/mmv/hks-interior.json'; plan = json.loads(path.read_text())
    plan['functions'][name] = dict(originalHash=hash_function(original),nrhexHash=hash_function(current),
        mmvFunctionHash=hashlib.sha256(text[span['start']:span['end']].encode()).hexdigest(),
        actions=[dict(kind='value-helpers',helpers=helpers)])
    path.write_text(json.dumps(plan,indent=2)+'\n')


if __name__ == '__main__':
    p=argparse.ArgumentParser();p.add_argument('--inplace',action='store_true');p.add_argument('--check',action='store_true');p.add_argument('--write-mmv-plan',type=Path);args=p.parse_args()
    path=ROOT/'dist/mod/action/script/c0000.hks';source=path.read_bytes()
    if QUERY in reader(source).functions[FID]['constants']: target=source;ranges=check(source)
    else:
        assert not args.check, 'Bartender patch missing'
        target,ranges=patch(source);check(target)
        out=ROOT/'work/bartender/hks';out.mkdir(parents=True,exist_ok=True)
        (out/'before.hks').write_bytes(source);(out/'c0000.hks').write_bytes(target)
        (out/'validation.json').write_text(json.dumps(dict(ranges=ranges,exactInversePassed=True,onlyOwnershipAndRollAdapted=True),indent=2)+'\n')
        if args.inplace:path.write_bytes(target)
    if args.write_mmv_plan:write_plan(target,ranges,args.write_mmv_plan)
    print('PASS: Bartender adapts ownership and element selection; native switch chain preserved')
