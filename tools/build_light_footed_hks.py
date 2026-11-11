"""Observe mobile layers for default movement; leave native sprint routing intact."""
import argparse, hashlib, json, struct
from pathlib import Path
from hks_bytecode import Reader, decode, instructions
from build_onyx_hks import abx, abc
ROOT = Path(__file__).resolve().parents[1]
QUERY = 990058
NAMES = ['Update', 'ExecEvent', 'ExecEventNoReset']

def patch(source):
    r = Reader(source); r.function(); edits = []; report = {}
    retired = {'BanSprint', 'ExecSprintStart', 'MagicCommonFunction', 'ItemCommonFunction'}
    assert not any(QUERY in f['constants'] for f in r.functions if f['name'] in retired), \
        'Retired Light-footed sprint bypass: regenerate from the pre-feature input'
    fs = {f['name']: f for f in r.functions if f['name'] in NAMES}
    assert set(fs) == set(NAMES)
    installed = [QUERY in f['constants'] for f in fs.values()]
    if all(installed):
        return source, {}
    assert not any(installed), 'Partial Light-footed HKS patch'
    for name, f in fs.items():
        cs = f['constants']; extra = []; encoded = bytearray()
        def constant(v):
            if v in cs: return cs.index(v)
            if v in extra: return len(cs) + extra.index(v)
            k = len(cs) + len(extra)
            if isinstance(v, str):
                extra.extend([v, None, None]); b = v.encode() + b'\0'
                encoded.extend(b'\x04' + struct.pack('>Q', len(b)) + b + b'\0\0')
            else:
                extra.append(v); encoded.extend(b'\x03' + struct.pack('>f', v))
            return k
        def glob(reg, key):
            k = constant(key); return [abx(91, reg, k), abx(76, 20, k+1)]
        def lit(reg, v): return [abx(25, reg, constant(v))]
        s = f['slots']; code = glob(s, 'env') + lit(s+1, QUERY)
        if name in ['ExecEvent', 'ExecEventNoReset']:
            code += lit(s+2, -1) + lit(s+3, -1)
        else:
            for reg, key in [(s+2, 'UpperDefaultState00'), (s+3, 'LowerDefaultState00')]:
                code += glob(reg, 'GetVariable') + lit(reg+1, key) + [abc(2, reg, 2, 2)]
        code += [abc(2, s, 4, 2)]
        at = 0; old = [v[0] for v in struct.iter_unpack('>I', f['code'])]
        n = len(code); words = []
        for pc,w in enumerate(old):
            if pc == at: words.extend(code)
            d = decode(w)
            if d['op'] == 28:
                target = pc + d['bx'] - 0x10000 + 2
                # A branch TO the reset must pass the new guard, not bypass it.
                newtarget = target + (n if target > at or at == 0 else 0)
                newpc = pc + (n if pc >= at else 0)
                w = abx(28,d['a'],0x10000+newtarget-newpc-2)
            words.append(w)
        while len(encoded)%4: encoded.append(0); extra.append(None)
        newcode = b''.join(struct.pack('>I',w) for w in words)
        edits.extend([(f['slots_offset'],4,struct.pack('>I',s+5)),
                      (f['code_count_offset'],4,struct.pack('>I',len(words))),
                      (f['code_offset'],len(f['code']),newcode),
                      (f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),
                      (f['constants_end'],0,bytes(encoded)),
                      (f['line_count_offset'],4,struct.pack('>I',len(words))),
                      (f['lines_offset'],len(f['lines']),f['lines'][:at*4]+f['lines'][at*4:at*4+4]*n+f['lines'][at*4:])])
        for _,a,start,b,end in f['locals']:
            edits += [(a,4,struct.pack('>I',start+(n if start>=at and start else 0))),
                      (b,4,struct.pack('>I',end+(n if end>=at else 0)))]
        report[name] = dict(insertion=at,addedInstructions=n,originalRegisters=s,
                            fragment=[at,at+n])
    target = source
    for at,size,data in sorted(edits,reverse=True): target=target[:at]+data+target[at+size:]
    check=Reader(target);check.function()
    for old,new in zip(r.functions,check.functions):
        assert new['name']==old['name']
        info=report.get(old['name'])
        if info:
            at,n=info['insertion'],info['addedInstructions']
            words=instructions(new); original=instructions(old)
            body=words[:at]+words[at+n:]
            assert len(body)==len(original)
            for pc,(a,b) in enumerate(zip(original,body)):
                assert a['op']==b['op']
                if a['op']==28:
                    dest=pc+a['bx']-0x10000+2
                    now=pc+(n if pc>=at else 0)+b['bx']-0x10000+2
                    assert now==dest+(n if dest>at or at==0 else 0)
                else: assert a==b
            assert new['constants'][:len(old['constants'])]==old['constants']
        else: assert new['code']==old['code'] and new['constants']==old['constants']
    inverse=[];shift=0
    for at,size,data in sorted(edits):inverse.append((at+shift,len(data),source[at:at+size]));shift+=len(data)-size
    restored=target
    for at,size,data in reversed(inverse):restored=restored[:at]+data+restored[at+size:]
    assert restored==source
    return target,report

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--inplace',action='store_true');a=p.parse_args()
    path=ROOT/'dist/mod/action/script/c0000.hks';before=path.read_bytes();after,report=patch(before)
    out=ROOT/'work/light-footed/hks';out.mkdir(parents=True,exist_ok=True)
    (out/'before.hks').write_bytes(before);(out/'c0000.hks').write_bytes(after)
    (out/'validation.json').write_text(json.dumps(dict(functions=report,exactInversePassed=True,originalInstructionsRetained=True,gameplayVerified=False),indent=2)+'\n')
    if a.inplace:path.write_bytes(after)
    print('PASS: Light-footed HKS control flow, register preservation and exact inverse')
