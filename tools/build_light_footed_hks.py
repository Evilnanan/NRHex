"""Scoped mobile casting: retain sprint, native costs and all cancel branches."""
import argparse, hashlib, json, struct
from pathlib import Path
from hks_bytecode import Reader, decode, instructions
from build_onyx_hks import abx, abc
ROOT = Path(__file__).resolve().parents[1]
QUERY = 990058
NAMES = ['Update', 'ExecEvent', 'ExecEventNoReset', 'BanSprint', 'ExecSprintStart',
         'MagicCommonFunction', 'ItemCommonFunction']

def patch(source):
    r = Reader(source); r.function(); edits = []; report = {}
    fs = {f['name']: f for f in r.functions if f['name'] in NAMES}
    assert set(fs) == set(NAMES)
    installed = [QUERY in f['constants'] for f in fs.values()]
    if all(installed): return source, {}
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
        skip_set = name in ['MagicCommonFunction', 'ItemCommonFunction']
        if skip_set:
            sites = [i for i, w in enumerate(old[:-4]) if decode(w)['op'] == 91
                     and cs[decode(w)['bx']] == 'SetVariable'
                     and decode(old[i+2])['op'] == 25 and cs[decode(old[i+2])['bx']] == 'ToggleDash']
            assert len(sites) == 1
            at = sites[0]
            assert [decode(w)['op'] for w in old[at:at+5]] == [91,76,25,25,2]
            assert cs[decode(old[at+3])['bx']] == 0
            # Equal -> execute jump over just the native ToggleDash reset.
            code += lit(s+1, 1) + [abc(4, 1, s, s+1), abx(28, 0, 0x10000+4)]
        elif name in ['BanSprint', 'ExecSprintStart']:
            code += lit(s+1, 1) + [abc(4, 0, s, s+1), 0]; jump = len(code)-1
            if name == 'ExecSprintStart':
                # SpeedUpdate already passed native input/slow-feet guards.
                # Do not fire W_SprintStart (it replaces the casting upper body).
                # The real activation helper retains its native fee and the
                # existing Ambulance fee adapter, exactly once per activation.
                code += glob(s, 'FALSE') + [abx(27, s, constant('ISENABLE_SPRINTSTART')),
                                          abx(27, s, constant('ISENABLE_SPRINTSTOP'))]
                code += glob(s, 'act') + glob(s+1, 'LockonFixedAngleCancel') + [abc(2,s,2,1)]
                code += glob(s, 'ChangeMoveSpeedIndex') + lit(s+1, 3) + [abc(2,s,2,1)]
                code += glob(s, 'SprintStart_onActivate') + [abc(2,s,1,1)]
                code += glob(s, 'TRUE') + [abc(9,s,2,0)]
            else: code += [abc(9,0,1,0)]
            code[jump] = abx(28,0,0x10000+len(code)-jump-2)
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
                            fragment=[at,at+n+(5 if skip_set else 0)])
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

def update_plan(before,after,report):
    from mmv_hks_stage import hash_function
    from mmv_hks_fragments import functions
    from local_paths import ROOT
    import os
    mmv=Path(os.environ.get('NRHEX_MMV',ROOT.parent/'MMV'))
    source=(mmv/'mod/action/script/c0000.hks').read_bytes().decode('utf-8-sig'); named=functions(source)
    baseline=Reader((ROOT/'assets/original/action/script/c0000.hks').read_bytes());baseline.function()
    actual=Reader(after);actual.function();oldfs={f['name']:f for f in baseline.functions}
    path=ROOT/'compat/mmv/hks-interior.json';plan=json.loads(path.read_text())
    for name in ['MagicCommonFunction','ItemCommonFunction']:
        if name not in report:continue
        f=next(f for f in actual.functions if f['name']==name);old=oldfs[name];s=named[name];info=report[name]
        prior=Reader(before);prior.function();p=next(f for f in prior.functions if f['name']==name)
        assert p['code'].endswith(old['code'])
        n=p['count']-old['count'];actions=[]
        if n:actions.append(dict(kind='prefix',range=[0,n]))
        actions.append(dict(kind='replace-fragment',anchor='SetVariable("ToggleDash", 0)',range=info['fragment']))
        plan['functions'][name]=dict(forceInterior=True,originalHash=hash_function(old),nrhexHash=hash_function(f),
          mmvFunctionHash=hashlib.sha256(source[s['start']:s['end']].encode()).hexdigest(),actions=actions)
    path.write_text(json.dumps(plan,ensure_ascii=False,indent=2)+'\n')

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--inplace',action='store_true');p.add_argument('--plan',action='store_true');a=p.parse_args()
    path=ROOT/'dist/mod/action/script/c0000.hks';before=path.read_bytes();after,report=patch(before)
    out=ROOT/'work/light-footed/hks';out.mkdir(parents=True,exist_ok=True)
    (out/'before.hks').write_bytes(before);(out/'c0000.hks').write_bytes(after)
    (out/'validation.json').write_text(json.dumps(dict(functions=report,exactInversePassed=True,originalInstructionsRetained=True,gameplayVerified=False),indent=2)+'\n')
    if a.plan:update_plan(before,after,report)
    if a.inplace:path.write_bytes(after)
    print('PASS: Light-footed HKS control flow, register preservation and exact inverse')
