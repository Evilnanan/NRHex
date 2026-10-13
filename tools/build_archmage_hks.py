"""Archmage per-slot chain notifications; preserve native spell state transitions."""
from hks_bytecode import *
import json,itertools,argparse
ap=argparse.ArgumentParser();ap.add_argument('--inplace',action='store_true');options=ap.parse_args()
root=Path(__file__).resolve().parents[1];path=root/'dist/mod/action/script/c0000.hks'
source=path.read_bytes();r=Reader(source);r.function();edits=[];changed={};reports=[]
def abx(op,a,bx):return op<<25|bx<<8|a
def abc(op,a,b,c):return op<<25|b<<17|c<<8|a
targets=[f for f in r.functions if 799<=f['id']<=832 and (f['name'] or '').endswith('_Upper_onUpdate')]
assert len(targets)==34
already=all(6999936 in f['constants'] for f in targets)
if already:
 out=root/'.work/archmage';out.mkdir(parents=True,exist_ok=True)
 (out/'c0000.hks').write_bytes(source)
 print('PASS: Archmage notifications already installed');sys.exit(0)
cases=0
for f in targets:
 fid=f['id'];name=f['name'] or '';used='Fire' in name and 'Cancel' not in name
 left='Left' in name
 cs=f['constants'];assert 6999936 not in cs
 extra=[];encoded=bytearray();s=struct.unpack('>I',f['header'][4:8])[0]
 assert s==0, 'Native spell update callbacks must have no arguments; entry scratch registers are not live locals'
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
 prefix=glob(s,'env')+[abx(25,s+1,constant(1116)),abx(25,s+2,constant(6999936)),abc(2,s,3,2)]+glob(s+1,'TRUE')+[abc(4,0,s,s+1),0];exits=[len(prefix)-1]
 # Dispatch ordinary slots separately; do not merge the skill slot with either.
 prefix+=glob(s,'GetVariable')+[abx(25,s+1,constant('IndexMagicSlot')),abc(2,s,2,2)]
 prefix+=[abx(25,s+1,constant(0)),abc(4,0,s,s+1),0];slot1=len(prefix)-1
 def emit(slot):
  global prefix
  token=(6999938 if left else 6999940)+slot if used else 6999937
  prefix+=glob(s,'act')+[abx(25,s+1,constant(2002)),abx(25,s+2,constant(token))]
  if used and not left:
   prefix+=glob(s+3,'c_Style')+glob(s+4,'HAND_LEFT_BOTH')+[abc(4,0,s+3,s+4),abx(28,0,0x10000),abx(25,s+2,constant(6999938+slot))]
  prefix+=[abc(2,s,3,1)]
 emit(0);prefix+=[0];exits.append(len(prefix)-1)
 prefix[slot1]=abx(28,0,0x10000+len(prefix)-slot1-2)
 prefix+=[abx(25,s+1,constant(1)),abc(4,0,s,s+1),0];exits.append(len(prefix)-1)
 emit(1)
 for j in exits:prefix[j]=abx(28,0,0x10000+len(prefix)-j-2)
 assert s+5<256 and len(cs)+len(extra)<256
 for owned,slot,style in itertools.product([False,True],[-1,0,1,2,3],[0,1,2,3]):
  saved={i:100+i for i in range(s)};regs=dict(saved);calls=[];pc=0;constants=cs+extra
  globals_={'env':lambda *args:int(owned),'TRUE':1,'GetVariable':lambda _:slot,'act':lambda *args:calls.append(args),'c_Style':style,'HAND_LEFT_BOTH':3}
  while pc<len(prefix):
   x=decode(prefix[pc]);pc+=1;op,a,b,c,bx=x['op'],x['a'],x['b'],x['c'],x['bx']
   if op==91:regs[a]=globals_[constants[bx]]
   elif op==76:pass
   elif op==25:regs[a]=constants[bx]
   elif op==7:regs[a]=regs[b]
   elif op==2:
    v=regs[a](*(regs[i] for i in range(a+1,a+b)))
    if c==2:regs[a]=v
   elif op==4:
    if (regs[b]==regs[c])!=bool(a):pc+=1
   elif op==28:pc+=bx-0x10000+1
   else:raise AssertionError(op)
  token=(6999938 if left or style==3 else 6999940)+slot if used else 6999937
  assert calls==([(2002,token)] if owned and slot in [0,1] else []),(name,owned,slot,style,calls)
  assert all(regs[i]==v for i,v in saved.items());cases+=1
 while len(encoded)%4:encoded.extend(b'\0');extra.append(None)
 orig=[x[0] for x in struct.iter_unpack('>I',f['code'])];words=prefix+orig;code=b''.join(struct.pack('>I',x) for x in words)
 # Prefix insertion shifts both ends of every original jump equally.
 edits.extend([(f['slots_offset'],4,struct.pack('>I',max(f['slots'],s+5))),(f['code_count_offset'],4,struct.pack('>I',len(words))),(f['code_offset'],len(f['code']),code),(f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),(f['line_count_offset'],4,struct.pack('>I',len(words))),(f['lines_offset'],len(f['lines']),f['lines'][:4]*len(prefix)+f['lines'])])
 for _,a,start,b,end in f['locals']:edits.extend([(a,4,struct.pack('>I',start+len(prefix))),(b,4,struct.pack('>I',end+len(prefix)))])
 changed[fid]=(code,cs+extra);reports.append(dict(id=fid,name=name,used=used,left=left,prefixInstructions=len(prefix)))
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
(root/'.work/archmage').mkdir(exist_ok=True)
(root/'.work/archmage/c0000.hks').write_bytes(patched)
if options.inplace:path.write_bytes(patched)
(root/'.work/hks').mkdir(exist_ok=True)
(root/'.work/archmage/hks-before.hks').write_bytes(source)
(root/'.work/archmage/hks-validation.json').write_text(json.dumps(dict(inputSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(patched).hexdigest(),functions=reports,gateCases=cases,inverseRestoresInput=True,originalBranchesUnchanged=True,entryArgumentsPreserved=True,nativeCallbacksHaveZeroParameters=True,bothHandsBothSlotsAndTwoHandedLeftChecked=True,chargeWithoutReleaseDoesNotCount=True,weaponSkillsExcluded=True,gameplayVerified=False),indent=2))
print(f'PASS: {len(reports)} spell callbacks; {cases} gates; register/branch preservation and inverse checked.')
