"""Gate native spell charge callbacks without altering native transitions."""
from hks_bytecode import *
import argparse,json,itertools,tomllib
from verify_chant_hks import verify
parser=argparse.ArgumentParser();parser.add_argument('--inplace',action='store_true');options=parser.parse_args()
root=Path(__file__).resolve().parents[1];path=root/'dist/mod/action/script/c0000.hks'
source=path.read_bytes();r=Reader(source);r.function();edits=[];changed={};reports=[];cases=0
stage=root/'.work/angel-chant-01740/staged';stage.mkdir(parents=True,exist_ok=True)
def abx(op,a,bx):return op<<25|bx<<8|a
def abc(op,a,b,c):return op<<25|b<<17|c<<8|a
for f in r.functions:
 fid=f['id'];name=f['name'] or ''
 if fid not in [801,806,807,808,812,814]:continue
 assert 'MagicFire' in name and 100610 in f['constants'] and 6999921 not in f['constants']
 cs=f['constants'];extra=[];encoded=bytearray();s=f['slots']
 original=instructions(f)
 anchor='CheckIfHoldMagic' if fid in [801,808] else 'LOWER_MOVE'
 insert=next(i for i,x in enumerate(original) if x['op']==91 and cs[x['bx']]==anchor)
 assert any(x['op']==91 and cs[x['bx']]=='MagicCommonFunction' for x in original[:insert])
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
 prefix=[];exits=[]
 def require_true(words,invert=False):
  prefix.extend(words+glob(s+1,'TRUE')+[abc(4,1 if invert else 0,s,s+1),0]);exits.append(len(prefix)-1)
 for effect in [6999920,100610]:require_true(glob(s,'env')+[abx(25,s+1,constant(1116)),abx(25,s+2,constant(effect)),abc(2,s,3,2)])
 require_true(glob(s,'IsChargeMagic')+[abc(2,s,1,2)])
 require_true(glob(s,'CheckIfHoldMagic')+[abc(2,s,1,2)],True)
 # Insert after the native input/FP/cancellation checks, which return on
 # ordinary release, interruption or unavailable casting. Do not rerun them.
 prefix+=glob(s,'GetVariable')+[abx(25,s+1,constant('IndexMagicSlot')),abc(2,s,2,2),abx(25,s+1,constant(2)),abc(4,1,s,s+1),0];exits.append(len(prefix)-1)
 prefix+=glob(s,'act')+[abx(25,s+1,constant(2002)),abx(25,s+2,constant(6999921)),abc(2,s,3,1)]
 for j in exits:prefix[j]=abx(28,0,0x10000+len(prefix)-j-2)
 assert len(cs)+len(extra)<256 and s+4<256
 for owned,charge,charge_spell,held_spell,slot in itertools.product([0,1],[0,1],[0,1],[0,1],[0,1,2]):
  saved={i:100+i for i in range(s)};regs=dict(saved);calls=[];pc=0;constants=cs+extra
  globals_={'env':lambda code,effect:owned if effect==6999920 else charge,'TRUE':1,'IsChargeMagic':lambda:charge_spell,'CheckIfHoldMagic':lambda:held_spell,'GetVariable':lambda _:slot,'act':lambda *args:calls.append(args)}
  while pc<len(prefix):
   x=decode(prefix[pc]);pc+=1;op,a,b,c,bx=x['op'],x['a'],x['b'],x['c'],x['bx']
   if op==91:regs[a]=globals_[constants[bx]]
   elif op==76:pass
   elif op==25:regs[a]=constants[bx]
   elif op==2:
    v=regs[a](*(regs[i] for i in range(a+1,a+b)))
    if c==2:regs[a]=v
   elif op==4:
    if (regs[b]==regs[c])!=bool(a):pc+=1
   elif op==28:pc+=bx-0x10000+1
   else:raise AssertionError(op)
  expected=owned and charge and charge_spell and not held_spell and slot!=2
  assert calls==([(2002,6999921)] if expected else [])
  assert all(regs[i]==v for i,v in saved.items());cases+=1
 while len(encoded)%4:encoded.extend(b'\0');extra.append(None)
 orig=[x[0] for x in struct.iter_unpack('>I',f['code'])]
 # A branch targeting the anchor must execute the new checks. Branches after
 # it move past the insertion. Preserve every old jump's destination.
 for i,x in enumerate(original):
  if x['op']==28:
   target=i+x['bx']-0x10000+2
   new_i=i+(len(prefix) if i>=insert else 0)
   new_target=target+(len(prefix) if target>insert else 0)
   orig[i]=abx(28,x['a'],0x10000+new_target-new_i-2)
 words=orig[:insert]+prefix+orig[insert:];code=b''.join(struct.pack('>I',x) for x in words)
 for i,x in enumerate(original):
  j=i+(len(prefix) if i>=insert else 0);y=decode(words[j])
  if x['op']==28:
   target=i+x['bx']-0x10000+2
   assert j+y['bx']-0x10000+2==target+(len(prefix) if target>insert else 0)
  else:assert x==y
 # Keep original debug spans and shift only boundaries after the insertion.
 edits.extend([(f['slots_offset'],4,struct.pack('>I',max(f['slots'],s+4))),(f['code_count_offset'],4,struct.pack('>I',len(words))),(f['code_offset'],len(f['code']),code),(f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),(f['line_count_offset'],4,struct.pack('>I',len(words))),(f['lines_offset'],len(f['lines']),f['lines'][:insert*4]+f['lines'][insert*4:insert*4+4]*len(prefix)+f['lines'][insert*4:])])
 for _,a,start,b,end in f['locals']:edits.extend([(a,4,struct.pack('>I',start+(len(prefix) if start>=insert else 0))),(b,4,struct.pack('>I',end+(len(prefix) if end>=insert else 0)))])
 changed[fid]=(code,cs+extra);reports.append(dict(id=fid,name=name,prefixInstructions=len(prefix),insertion=insert,afterNativeReleaseChecks=True))
assert set(changed)=={801,806,807,808,812,814}, 'Missing native spell charge callbacks'
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
(stage/'c0000.hks').write_bytes(patched)
(stage/'before.hks').write_bytes(source)
verify(stage/'c0000.hks',stage/'before.hks')
version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
(stage/'hks-validation.json').write_text(json.dumps(dict(version=version,functions=reports,cases=cases,inputSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(patched).hexdigest(),inverseRestoresInput=True,originalBranchesUnchanged=True,registersPreserved=True,heldChargeOnly=True,ordinarySpellAndSkillAndContinuousHoldExcluded=True,independentDecompilerMatchesAllOriginalBodies=True,gameplayVerified=False),indent=2))
if options.inplace:
 assert path.read_bytes()==source,'Input changed during verification'
 path.write_bytes(patched)
print(f'PASS: {len(reports)} spell charge callbacks; {cases} gate cases; inverse and unrelated functions preserved.')
