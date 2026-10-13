"""Notify only the native accepted sprint-toggle cancellation branch."""
from hks_bytecode import *
import json,argparse
ap=argparse.ArgumentParser();ap.add_argument('--inplace',action='store_true');options=ap.parse_args()
root=Path(__file__).resolve().parents[1];path=root/'dist/mod/action/script/c0000.hks'
source=path.read_bytes();r=Reader(source);r.function();edits=[];changed={};reports=[]
def abx(op,a,bx):return op<<25|bx<<8|a
def abc(op,a,b,c):return op<<25|b<<17|c<<8|a
tokens={1245:6999964};expected={1245:'SpeedUpdate'}
targets=[f for f in r.functions if f['id'] in tokens]
assert len(targets)==1 and targets[0]['name']=='SpeedUpdate'
out=root/'.work/slingshot-stacks';out.mkdir(parents=True,exist_ok=True)
if 6999964 in targets[0]['constants']:
 (out/'c0000.hks').write_bytes(source)
 print('PASS: Slingshot accepted-cancellation notification already installed');sys.exit(0)
cases=0
for f in targets:
 fid=f['id'];name=f['name'] or '';cs=f['constants'];assert 6999960 not in cs
 extra=[];encoded=bytearray();s=6
 assert struct.unpack('>I',f['header'][4:8])[0]==0
 # Native branch uses r6-r8 as disposable call scratch; r0-r5 are live locals.
 original_ins=instructions(f);cs=f['constants']
 assert f['count']==720
 assert original_ins[421]['op']==27 and cs[original_ins[421]['bx']]=='ISENABLE_SPRINTSTART'
 assert original_ins[422]['op']==91 and cs[original_ins[422]['bx']]=='ChangeMoveSpeedIndex'
 assert original_ins[424]['op']==25 and cs[original_ins[424]['bx']]==2
 assert original_ins[425]=={'op':2,'a':6,'b':2,'c':1,'bx':1025}
 assert original_ins[426]['op']==28
 cut=426
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
 prefix=glob(s,'env')+[abx(25,s+1,constant(1116)),abx(25,s+2,constant(6999960)),abc(2,s,3,2)]+glob(s+1,'TRUE')+[abc(4,0,s,s+1),0];skip=len(prefix)-1
 prefix+=glob(s,'act')+[abx(25,s+1,constant(2002)),abx(25,s+2,constant(tokens[fid])),abc(2,s,3,1)]
 prefix[skip]=abx(28,0,0x10000+len(prefix)-skip-2)
 assert s+3<256 and len(cs)+len(extra)<256
 for owned in [False,True]:
  saved={i:100+i for i in range(s)};regs=dict(saved);calls=[];pc=0;constants=cs+extra
  globals_={'env':lambda *args:int(owned),'TRUE':1,'act':lambda *args:calls.append(args)}
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
  assert calls==([(2002,tokens[fid])] if owned else []),(name,owned,calls)
  assert all(regs[i]==v for i,v in saved.items());cases+=1
 while len(encoded)%4:encoded.extend(b'\0');extra.append(None)
 orig=[x[0] for x in struct.iter_unpack('>I',f['code'])]
 def moved(pc):return pc+(len(prefix) if pc>=cut else 0)
 relocated=[];branch_count=0
 for pc,word in enumerate(orig):
  d=decode(word)
  if d['op']==28:
   dest=pc+2+d['bx']-0x10000
   assert dest!=cut, 'Unexpected branch to insertion boundary'
   word=abx(28,d['a'],0x10000+moved(dest)-moved(pc)-2);branch_count+=1
  relocated.append(word)
 words=relocated[:cut]+prefix+relocated[cut:]
 # Removing notification and undoing every relocation recovers original words.
 for pc,original in enumerate(orig):
  word=words[moved(pc)];d=decode(word)
  if d['op']==28:
   before=decode(original);dest=pc+2+before['bx']-0x10000
   assert moved(pc)+2+d['bx']-0x10000==moved(dest)
  else:assert word==original
 code=b''.join(struct.pack('>I',x) for x in words)
 # Code and debug ranges follow the verified relocation across the insertion.
 edits.extend([(f['slots_offset'],4,struct.pack('>I',max(f['slots'],s+3))),(f['code_count_offset'],4,struct.pack('>I',len(words))),(f['code_offset'],len(f['code']),code),(f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),(f['line_count_offset'],4,struct.pack('>I',len(words))),(f['lines_offset'],len(f['lines']),f['lines'][:cut*4]+f['lines'][cut*4:cut*4+4]*len(prefix)+f['lines'][cut*4:])])
 for local_index,(_,a,start,b,end) in enumerate(f['locals']):edits.extend([(a,4,struct.pack('>I',moved(start))),(b,4,struct.pack('>I',moved(end)))])
 changed[fid]=(code,cs+extra);reports.append(dict(id=fid,name=name,token=tokens[fid],insertedInstructions=len(prefix),insertionPc=cut,relocatedBranches=branch_count))
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
(root/'.work/slingshot-stacks').mkdir(exist_ok=True)
(root/'.work/slingshot-stacks/c0000.hks').write_bytes(patched)
if options.inplace:path.write_bytes(patched)
(root/'.work/hks').mkdir(exist_ok=True)
(root/'.work/slingshot-stacks/hks-before.hks').write_bytes(source)
(root/'.work/slingshot-stacks/hks-validation.json').write_text(json.dumps(dict(inputSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(patched).hexdigest(),functions=reports,gateCases=cases,inverseRestoresInput=True,originalBranchesUnchanged=True,entryArgumentsPreserved=True,nativeArgumentsPreserved=True,nativeSprintToggleUnchanged=True,onlyAcceptedToggleCancellationClearsStacks=True,originalStaminaAndMovementCallsUnchanged=True,gameplayVerified=False),indent=2))
print(f'PASS: {len(reports)} accepted sprint-cancel branch; {cases} gates; register/branch preservation and inverse checked.')
