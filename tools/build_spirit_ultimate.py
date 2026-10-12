"""Restore only native family recall; Ultimate Art triggering is observed natively."""
from pathlib import Path
import hashlib,json
from hks_bytecode import Reader,decode,instructions
root=Path(__file__).resolve().parents[1]
source=(root/'dist/mod/action/script/c0000.hks').read_bytes()
native=(root/'assets/original/action/script/c0000.hks').read_bytes()
def parse(data):
 r=Reader(data);r.function();return r
old,original=parse(source),parse(native)
f=next(f for f in old.functions if f['name']=='ExecSummon')
n=next(f for f in original.functions if f['name']=='ExecSummon')
assert not f['children'] and not n['children']
assert 6999911 in f['constants'] and 6999910 in f['constants']
assert n['count']==265 and n['constants'][20]=='IsEnableSummonAct'
# Prove the existing function contains only the known 21-instruction recall
# notification over native, before replacing its complete serialized body.
assert f['count']==n['count']+21 and f['constants'][:len(n['constants'])]==n['constants']
for i,a in enumerate(instructions(n)):
 j=i if i<51 else i+21;b=instructions(f)[j]
 if a['op']==28:
  target=i+a['bx']-0x10000+2
  assert j+b['bx']-0x10000+2==target+(21 if target>51 else 0)
 else:assert a==b
replacement=native[n['start']:n['end']]
assert (len(replacement)-(f['end']-f['start']))%4==0
patched=source[:f['start']]+replacement+source[f['end']:]
check=parse(patched)
assert len(check.functions)==len(old.functions)
for a,b in zip(old.functions,check.functions):
 if a['name']=='ExecSummon':
  assert patched[b['start']:b['end']]==replacement
  assert 6999911 not in b['constants']
 elif a['id']:
  assert source[a['start']:a['end']]==patched[b['start']:b['end']]
 else:assert a['code']==b['code'] and a['constants']==b['constants']
assert patched[check.p:]==source[old.p:]
restored=patched[:f['start']]+source[f['start']:f['end']]+patched[f['start']+len(replacement):]
assert restored==source
out=root/'.work/spirit-ultimate-01739';out.mkdir(exist_ok=True,parents=True)
(out/'c0000.hks').write_bytes(patched)
(out/'hks-validation.json').write_text(json.dumps(dict(version='0.17.39',inputSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(patched).hexdigest(),restoredFunction='ExecSummon',nativeRecallByteExact=True,unrelatedFunctionsUnchanged=True,inverseRestoresInput=True,ultimateTrigger='Native successful SpEffect 6999615; no custom recall pulse',gameplayVerified=False),indent=2),encoding='utf8')
print('PASS: native family recall restored byte-for-byte; all other functions retained')
