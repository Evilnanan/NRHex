"""Close a consumed preparation stance before applying the pending reforge.

Incremental patch over the shipped HKS: preserve every original instruction.
"""
from hks_bytecode import *
import argparse, hashlib, itertools, json

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--inplace', action='store_true')
args = parser.parse_args()
path = root / 'dist/mod/action/script/c0000.hks'
target = path if args.inplace else root / '.work/fixes-01727/staged/mod/action/script/c0000.hks'
source = path.read_bytes()
r = Reader(source); r.function()
edits, changed, reports = [], {}, []

def abx(op, a, bx): return op << 25 | bx << 8 | a
def abc(op, a, b, c): return op << 25 | b << 17 | c << 8 | a

for f in r.functions:
    if f['id'] not in {536, 537, 538, 539, 581, 584}: continue
    assert 6999805 not in f['constants'], 'stance fix already installed'
    cs, extra, encoded = f['constants'], [], bytearray()
    # The six native update callbacks take no arguments. Use the same entry
    # scratch registers as the existing ownership prefix; the original body
    # initializes its locals afterwards. High initial registers confuse the
    # independent decompiler's implicit-parameter inference.
    s = struct.unpack('>I', f['header'][4:8])[0]
    assert s == 0
    def constant(v):
        if v in cs: return cs.index(v)
        if v in extra: return len(cs) + extra.index(v)
        ix = len(cs) + len(extra)
        if isinstance(v, str):
            extra.extend([v, None, None]); b = v.encode() + b'\0'
            encoded.extend(b'\x04' + struct.pack('>Q', len(b)) + b + b'\0\0')
        else:
            assert struct.unpack('>f', struct.pack('>f', v))[0] == v
            extra.append(float(v)); encoded.extend(b'\x03' + struct.pack('>f', v))
        return ix
    def glob(a, v):
        k = constant(v); return [abx(91, a, k), abx(76, 20, k + 1)]
    prefix, exits = [], []
    for marker in (6999800, 6999804):
        prefix += glob(s, 'env') + [abx(25, s+1, constant(1116)), abx(25, s+2, constant(marker)), abc(2, s, 3, 2)]
        prefix += glob(s+1, 'TRUE') + [abc(4, 0, s, s+1), 0]
        exits.append(len(prefix)-1)
    # These native stances attack during their loop, so reaching this callback
    # does not mean a new preparation. Preserve their ordinary combo handling.
    if f['id'] in {536, 537, 538, 539}:
        prefix += glob(s, 'c_SwordArtsID')
        for kind in [10, 11, 25, 239]:
            prefix += [abx(25, s+1, constant(kind)), abc(4, 1, s, s+1), 0]
            exits.append(len(prefix)-1)
    prefix += glob(s, 'act') + [abx(25, s+1, constant(2002)), abx(25, s+2, constant(6999805)), abc(2, s, 3, 1)]
    prefix += glob(s+2, 'GetHalfBlendInfo') + [abc(2, s+2, 1, 2)]
    prefix += glob(s, 'ExecEventHalfBlend')
    ending = 'Event_SwordArtsArrowStanceEnd' if f['id'] in {581, 584} else 'Event_DrawStanceRightEnd'
    prefix += glob(s+1, ending) + [abc(2, s, 3, 1), abc(9, 0, 1, 0)]
    for j in exits: prefix[j] = abx(28, 0, 0x10000 + len(prefix) - j - 2)
    constants = cs + extra
    checked = 0
    for owned, pending, kind in itertools.product([False, True], [False, True], [10, 11, 25, 239, 14, 15, 260]):
        saved = {i: 100+i for i in range(s)}; regs = dict(saved); calls = []; pc = 0; returned = False
        globals_ = {'env': lambda _, m: int(owned if m == 6999800 else pending), 'TRUE': 1,
                    'act': lambda *a: calls.append(a), 'c_SwordArtsID': kind,
                    'GetHalfBlendInfo': lambda: 777, 'ExecEventHalfBlend': lambda *a: calls.append(a), ending: ending}
        while pc < len(prefix):
            x = decode(prefix[pc]); pc += 1
            op,a,b,c,bx = x['op'],x['a'],x['b'],x['c'],x['bx']
            if op == 91: regs[a] = globals_[constants[bx]]
            elif op == 76: pass
            elif op == 25: regs[a] = constants[bx]
            elif op == 2:
                v = regs[a](*(regs[i] for i in range(a+1, a+b)))
                if c == 2: regs[a] = v
            elif op == 4:
                if (regs[b] == regs[c]) != bool(a): pc += 1
            elif op == 28: pc += bx - 0x10000 + 1
            elif op == 9: returned = True; break
            else: raise AssertionError(op)
        should_close = owned and pending and not (f['id'] in {536,537,538,539} and kind in [10,11,25,239])
        assert calls == ([(2002,6999805), (ending,777)] if should_close else [])
        assert returned == should_close
        assert all(regs[i] == v for i,v in saved.items())
        checked += 1
    while len(encoded) % 4: encoded.extend(b'\0'); extra.append(None)
    orig = [x[0] for x in struct.iter_unpack('>I', f['code'])]
    words = prefix + orig; code = b''.join(struct.pack('>I', x) for x in words)
    edits.extend([(f['slots_offset'],4,struct.pack('>I',max(f['slots'],s+3))),
                  (f['code_count_offset'],4,struct.pack('>I',len(words))),
                  (f['code_offset'],len(f['code']),code),
                  (f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),
                  (f['constants_end'],0,bytes(encoded)),
                  (f['line_count_offset'],4,struct.pack('>I',len(words))),
                  (f['lines_offset'],len(f['lines']),f['lines'][:4]*len(prefix)+f['lines'])])
    for _,a,start,b,end in f['locals']:
        edits.extend([(a,4,struct.pack('>I',start+len(prefix))), (b,4,struct.pack('>I',end+len(prefix)))])
    changed[f['id']] = (code,cs+extra)
    reports.append(dict(id=f['id'],name=f['name'],cases=checked,prefixInstructions=len(prefix),endEvent=ending))
assert len(reports) == 6
patched = source
for off,n,v in sorted(edits,reverse=True): patched = patched[:off]+v+patched[off+n:]
inverse,shift = [],0
for off,n,v in sorted(edits):
    inverse.append((off+shift,len(v),source[off:off+n])); shift += len(v)-n
restored = patched
for off,n,v in sorted(inverse,reverse=True): restored = restored[:off]+v+restored[off+n:]
assert restored == source
new = Reader(patched); new.function(); assert patched[new.p:] == source[r.p:]
for a,b in zip(r.functions,new.functions):
    if a['id'] in changed: assert (b['code'],b['constants']) == changed[a['id']]
    elif a['id'] != 0: assert source[a['start']:a['end']] == patched[b['start']:b['end']]
    else: assert a['code'] == b['code'] and a['constants'] == b['constants']
target.parent.mkdir(parents=True,exist_ok=True); target.write_bytes(patched)
report = dict(version='0.17.27',functions=reports,inverseRestoresInput=True,originalBranchesUnchanged=True,
              originalArgumentsPreserved=True,preparationWithoutUseUnchanged=True,continuousAttackStancesUnchanged=True,
              sourceSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(patched).hexdigest(),gameplayVerified=False)
(root/'.work/fixes-01727/arts-stance-validation.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print('PASS: six stance preparation callbacks; 168 owner/use/stance cases; inverse and original callbacks preserved.')
