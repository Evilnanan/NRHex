"""Observe full-body Retaliate transitions, preserving native bytecode exactly."""
import argparse,hashlib,json
from pathlib import Path
from build_marking_hks import patch_observer
from hks_bytecode import Reader
ROOT=Path(__file__).resolve().parents[1]
QUERY=990048
EVENTS=['W_SmashAttackStanceStart','W_SmashAttack1','W_SmashAttack2']
def check(source):
 r=Reader(source);r.function();f=r.functions[9]
 assert f['name']=='ExecEventAllBody' and QUERY in f['constants']
 assert all(e in f['constants']for e in EVENTS)
 return r
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');p.add_argument('--inplace',action='store_true');a=p.parse_args()
 path=ROOT/'dist/mod/action/script/c0000.hks';source=path.read_bytes()
 if a.check:check(source);print('PASS: Deliberate Punch full-body observer installed');raise SystemExit
 r=Reader(source);r.function();assert r.functions[9]['name']=='ExecEventAllBody'
 if QUERY in r.functions[9]['constants']:check(source);print('PASS: Deliberate Punch observer already installed');raise SystemExit
 target,audit=patch_observer(source,QUERY,EVENTS,fids=(9,));check(target)
 out=ROOT/'work/deliberate-punch/hks';out.mkdir(parents=True,exist_ok=True)
 (out/'before.hks').write_bytes(source);(out/'c0000.hks').write_bytes(target)
 (out/'validation.json').write_text(json.dumps(dict(inputSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(target).hexdigest(),functions=audit,exactInversePassed=True,nativeInstructionsPreserved=True),indent=2)+'\n')
 if a.inplace:path.write_bytes(target)
 print('PASS: full-body observer added; original bytecode and exact inverse verified')
