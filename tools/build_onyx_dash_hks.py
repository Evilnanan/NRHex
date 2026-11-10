"""Scale only the planted-foot startup window; keep the native travel fallback."""
import argparse, hashlib, json, struct
from pathlib import Path
from hks_bytecode import Reader, decode
from build_onyx_hks import abx, abc

ROOT = Path(__file__).resolve().parents[1]
QUERY = 990057
FLAG = 28
SCALE = 2.5

def patch(source):
    r = Reader(source); r.function()
    f = next(f for f in r.functions if f['name'] == 'ExecTravelMagnification')
    if QUERY in f['constants']:
        verify(source); return source, {}
    assert struct.unpack('>I', f['header'][4:8])[0] == 1
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
    def glob(reg, name):
        k = constant(name); return [abx(91, reg, k), abx(76, 20, k+1)]
    s = f['slots']; code = []; exits = []
    def require(a, b):
        code.append(abc(4, 0, a, b)); exits.append(len(code)); code.append(0)
    # No mutation of the incoming attack type or any original register.
    code += glob(s, 'ATTACK_RIGHT_HEAVYDASH'); require(0, s)
    code += glob(s, 'env') + [abx(25, s+1, constant(QUERY)), abc(2, s, 2, 2), abx(25, s+1, constant(1))]
    require(s, s+1)
    code += glob(s, 'env') + [abx(25, s+1, constant(301)), abx(25, s+2, constant(FLAG)), abc(2, s, 3, 2), abx(25, s+1, constant(1))]
    require(s, s+1)
    code += glob(s, 'act') + [abx(25, s+1, constant(2001)), abx(25, s+2, constant(SCALE)), abc(2, s, 3, 1)]
    code += glob(s, 'TRUE') + [abc(9, s, 2, 0)]
    for pc in exits: code[pc] = abx(28, 0, 0x10000 + len(code)-pc-2)
    while len(encoded) % 4: encoded.append(0); extra.append(None)
    prefix = b''.join(struct.pack('>I', w) for w in code); n = len(code)
    edits = [(f['slots_offset'],4,struct.pack('>I',s+3)),
             (f['code_count_offset'],4,struct.pack('>I',f['count']+n)),
             (f['code_offset'],len(f['code']),prefix+f['code']),
             (f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),
             (f['constants_end'],0,bytes(encoded)),
             (f['line_count_offset'],4,struct.pack('>I',f['count']+n)),
             (f['lines_offset'],len(f['lines']),f['lines'][:4]*n+f['lines'])]
    for _,a,start,b,end in f['locals']:
        edits += [(a,4,struct.pack('>I',start+n if start else 0)),(b,4,struct.pack('>I',end+n))]
    target = source
    for at,size,data in sorted(edits,reverse=True): target=target[:at]+data+target[at+size:]
    check=Reader(target); check.function()
    for old,new in zip(r.functions,check.functions):
        assert new['constants'][:len(old['constants'])] == old['constants']
        assert new['code'][len(prefix) if old['id']==f['id'] else 0:] == old['code']
    inverse=[]; shift=0
    for at,size,data in sorted(edits): inverse.append((at+shift,len(data),source[at:at+size])); shift+=len(data)-size
    restored=target
    for at,size,data in reversed(inverse): restored=restored[:at]+data+restored[at+size:]
    assert restored==source
    verify(target)
    return target, dict(function=f['name'],prefixInstructions=n,originalRegisters=s,exactInversePassed=True)

def verify(data):
    from mmv_hks_fragments import Fragment
    before=Reader((ROOT/'assets/original/action/script/c0000.hks').read_bytes()); before.function()
    after=Reader(data); after.function()
    f=next(f for f in after.functions if f['name']=='ExecTravelMagnification')
    native=next(f for f in before.functions if f['name']=='ExecTravelMagnification')
    assert QUERY in f['constants'] and f['code'].endswith(native['code'])
    fragment=Fragment(f['code'][:-len(native['code'])],f['constants'],inputs={0:'attack_type'}).lua()
    assert all(str(v) in fragment for v in (QUERY,FLAG,SCALE,301,2001))
    return True

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--inplace',action='store_true');p.add_argument('--check',action='store_true');a=p.parse_args()
    path=ROOT/'dist/mod/action/script/c0000.hks'; source=path.read_bytes()
    if a.check: verify(source); print('PASS: Onyx single-hand running-heavy startup movement adapter');raise SystemExit
    target,report=patch(source); out=ROOT/'work/onyx-dash/hks';out.mkdir(parents=True,exist_ok=True)
    (out/'c0000.hks').write_bytes(target)
    (out/'validation.json').write_text(json.dumps(dict(inputSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(target).hexdigest(),**report),indent=2)+'\n')
    if a.inplace:path.write_bytes(target)
    print('PASS: Onyx startup travel prefix; native fallback and all original control flow retained')
