"""Report the casting hand through the existing owner/skill-gated Sword pulse."""
from hks_bytecode import *
import argparse, itertools, json

ap=argparse.ArgumentParser();ap.add_argument('--inplace',action='store_true');options=ap.parse_args()
root=Path(__file__).resolve().parents[1];path=root/'dist/mod/action/script/c0000.hks'
source=path.read_bytes();r=Reader(source);r.function();edits=[];changed={};reports=[];cases=0
def abx(op,a,bx):return op<<25|bx<<8|a
def abc(op,a,b,c):return op<<25|b<<17|c<<8|a
targets=[f for f in r.functions if 95000904 in f['constants']]
assert len(targets)==23
if all(95000944 in f['constants'] or 95000952 in f['constants'] for f in targets):
 out=root/'.work/sword-vfx';out.mkdir(parents=True,exist_ok=True)
 (out/'c0000.hks').write_bytes(source)
 (out/'hks-validation.json').write_text(json.dumps(dict(alreadyPatched=True,inputSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(source).hexdigest(),gameplayVerified=False),indent=2))
 print('PASS: casting-hand notifications already installed');sys.exit(0)
for f in targets:
 cs=f['constants'];assert 95000944 not in cs and 95000952 not in cs
 extra=[];encoded=bytearray();s=f['slots'];old=[w[0] for w in struct.iter_unpack('>I',f['code'])];ins=instructions(f)
 def constant(v):
  if v in cs:return cs.index(v)
  if v in extra:return len(cs)+extra.index(v)
  ix=len(cs)+len(extra)
  if isinstance(v,str):
   extra.extend([v,None,None]);b=v.encode()+b'\0';encoded.extend(b'\x04'+struct.pack('>Q',len(b))+b+b'\0\0')
  else:
   assert struct.unpack('>f',struct.pack('>f',v))[0]==v
   extra.append(float(v));encoded.extend(b'\x03'+struct.pack('>f',v))
  return ix
 def glob(a,v):
  k=constant(v);return [abx(91,a,k),abx(76,20,k+1)]
 # Only change the argument of act(2002, CAST_PULSE); retain its existing
 # owner, non-weapon-skill and pulse guards and all other injected features.
 candidates=[i for i,x in enumerate(ins[:-1]) if x['op']==25 and cs[x['bx']]==95000904 and ins[i+1]['op']==2 and ins[i+1]['c']==1]
 assert len(candidates)==1
 at=candidates[0];arg=ins[at]['a'];assert ins[at-1]['op']==25 and cs[ins[at-1]['bx']]==2002
 # Left callbacks always use the left catalyst. Native right callbacks also
 # serve a two-handed left catalyst. IndexMagicSlot identifies spell slots,
 # not hands, and must never be used to choose the weapon.
 left='MagicFireLeft' in f['name'] or 'MagicLoopLeft' in f['name'] or 'MagicLaunchLeft' in f['name']
 p=[abx(25,arg,constant(95000952 if left else 95000944))]
 if not left:
  p+=glob(s,'c_Style')+glob(s+1,'HAND_LEFT_BOTH')+[abc(4,0,s,s+1),abx(28,0,0x10000),abx(25,arg,constant(95000952))]
 assert s+2<256 and len(cs)+len(extra)<256
 for style in [0,1,2,3]:
  regs={i:100+i for i in range(s)};saved=dict(regs);pc=0;constants=cs+extra;globals_={'c_Style':style,'HAND_LEFT_BOTH':3}
  while pc<len(p):
   x=decode(p[pc]);pc+=1;op,a,b,c,bx=x['op'],x['a'],x['b'],x['c'],x['bx']
   if op==25:regs[a]=constants[bx]
   elif op==91:regs[a]=globals_[constants[bx]]
   elif op==76:pass
   elif op==4:
    if (regs[b]==regs[c])!=bool(a):pc+=1
   elif op==28:pc+=bx-0x10000+1
   else:raise AssertionError(op)
  assert regs[arg]==(95000952 if left or style==3 else 95000944)
  assert all(regs[i]==v for i,v in saved.items() if i!=arg);cases+=1
 while len(encoded)%4:encoded.extend(b'\0');extra.append(None)
 delta=len(p)-1
 def moved(i):return i+delta if i>at else i
 relocated=old.copy()
 for i,x in enumerate(ins):
  if x['op']==28:
   target=i+x['bx']-0x10000+2
   relocated[i]=abx(28,x['a'],0x10000+moved(target)-moved(i)-2)
 words=relocated[:at]+p+relocated[at+1:];code=b''.join(struct.pack('>I',w) for w in words)
 for i,x in enumerate(ins):
  if i==at:continue
  y=decode(words[moved(i)])
  if x['op']==28:assert moved(i)+y['bx']-0x10000+2==moved(i+x['bx']-0x10000+2)
  else:assert x==y
 lines=f['lines'][:at*4]+f['lines'][at*4:at*4+4]*len(p)+f['lines'][(at+1)*4:]
 edits.extend([(f['slots_offset'],4,struct.pack('>I',max(s,s+2 if not left else s))),(f['code_count_offset'],4,struct.pack('>I',len(words))),(f['code_offset'],len(f['code']),code),(f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),(f['line_count_offset'],4,struct.pack('>I',len(words))),(f['lines_offset'],len(f['lines']),lines)])
 for _,a,start,b,end in f['locals']:edits.extend([(a,4,struct.pack('>I',moved(start))),(b,4,struct.pack('>I',moved(end)))])
 changed[f['id']]=(code,cs+extra);reports.append(dict(id=f['id'],name=f['name'],left=left))
patched=source
for off,n,v in sorted(edits,reverse=True):patched=patched[:off]+v+patched[off+n:]
inverse=[];shift=0
for off,n,v in sorted(edits):inverse.append((off+shift,len(v),source[off:off+n]));shift+=len(v)-n
restored=patched
for off,n,v in sorted(inverse,reverse=True):restored=restored[:off]+v+restored[off+n:]
assert restored==source
new=Reader(patched);new.function();assert patched[new.p:]==source[r.p:]
for a,b in zip(r.functions,new.functions):
 if a['id'] in changed:assert (b['code'],b['constants'])==changed[a['id']]
 elif a['id']!=0:assert source[a['start']:a['end']]==patched[b['start']:b['end']]
 else:assert a['code']==b['code'] and a['constants']==b['constants']
out=root/'.work/sword-vfx';out.mkdir(parents=True,exist_ok=True)
(out/'c0000.hks').write_bytes(patched)
(out/'hks-validation.json').write_text(json.dumps(dict(functions=reports,handCases=cases,originalGuardsAndBranchesPreserved=True,inverseRestoresInput=True,inputSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(patched).hexdigest(),gameplayVerified=False),indent=2))
if options.inplace:path.write_bytes(patched)
print(f'PASS: {len(reports)} gated casting callbacks; {cases} hand cases; original guards, branches and other callbacks preserved')
