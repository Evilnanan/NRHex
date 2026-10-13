"""Observe accepted sprint animation requests and native input edges without changing movement."""
from hks_bytecode import *
import json,argparse
ap=argparse.ArgumentParser();ap.add_argument('--inplace',action='store_true');options=ap.parse_args()
root=Path(__file__).resolve().parents[1];path=root/'dist/mod/action/script/c0000.hks'
source=path.read_bytes();r=Reader(source);r.function();edits=[];changed={};reports=[]
def abx(op,a,bx):return op<<25|bx<<8|a
def abc(op,a,b,c):return op<<25|b<<17|c<<8|a
tokens={9:6999967,1255:6999965}
expected={9:'ExecEventAllBody',1255:'Update'}
targets=[f for f in r.functions if f['id'] in tokens]
assert len(targets)==2 and all(f['name']==expected[f['id']] for f in targets)
out=root/'.work/slingshot-requests';out.mkdir(parents=True,exist_ok=True)
if all(6999965 in f['constants'] for f in targets):
 (out/'c0000.hks').write_bytes(source)
 print('PASS: Slingshot request/input notifications already installed');sys.exit(0)
cases=0
for f in targets:
 fid=f['id'];name=f['name'] or '';cs=f['constants'];assert 6999960 not in cs
 extra=[];encoded=bytearray();s=struct.unpack('>I',f['header'][4:8])[0]
 assert s in [0,1], 'Entry scratch must follow all native arguments'
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
 prefix=[]
 def emit(words):prefix.extend(words)
 def jump():
  pos=len(prefix);prefix.append(0);return pos
 def target(pos):prefix[pos]=abx(28,0,0x10000+len(prefix)-pos-2)
 def pulse(token):emit(glob(s,'act')+[abx(25,s+1,constant(2002)),abx(25,s+2,constant(token)),abc(2,s,3,1)])
 # r0 is the native event argument in ExecEventAllBody; never overwrite it.
 if fid==9:
  assert s==1 and f['count']==10 and cs==['SetVariable',None,None,'MoveSpeedLevelReal',0.,'ExecEvent',None,None]
  emit([abx(25,s,constant('W_SprintStart')),abc(4,0,0,s)])
  skip_event=jump()
 else:assert s==0 and f['count']==617
 emit(glob(s,'env')+[abx(25,s+1,constant(1116)),abx(25,s+2,constant(6999960)),abc(2,s,3,2)]+glob(s+1,'TRUE')+[abc(4,0,s,s+1)])
 skip_owner=jump()
 emit(glob(s,'env')+[abx(25,s+1,constant(1108))]+glob(s+2,'ACTION_ARM_L3')+[abc(2,s,3,2),abc(57,0,constant(0),s)])
 released=jump()
 pulse(6999965)
 sampled=jump()
 target(released);pulse(6999966);target(sampled)
 if fid==9:pulse(6999967)
 target(skip_owner)
 if fid==9:target(skip_event)
 assert s+3<256 and len(cs)+len(extra)<256
 # Execute every prefix branch independently from the runtime state tests.
 for owned in [False,True]:
  for held in [-1,0,.01,1]:
   for event in ['W_SprintStart','W_Jump','W_SprintStop']:
    saved={0:event} if s else {};regs=dict(saved);calls=[];pc=0;constants=cs+extra
    def env(kind,value):
     assert (kind,value) in [(1116,6999960),(1108,123)]
     return int(owned) if kind==1116 else held
    globals_={'env':env,'TRUE':1,'ACTION_ARM_L3':123,'act':lambda *args:calls.append(args)}
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
     elif op==57:
      if (constants[b]<regs[c])!=bool(a):pc+=1
     elif op==28:pc+=bx-0x10000+1
     else:raise AssertionError(op)
    want=[]
    if owned and (fid!=9 or event=='W_SprintStart'):
     want=[(2002,6999965 if held>0 else 6999966)]
     if fid==9:want.append((2002,6999967))
    assert calls==want,(name,owned,held,event,calls,want)
    assert all(regs[i]==v for i,v in saved.items());cases+=1
 while len(encoded)%4:encoded.extend(b'\0');extra.append(None)
 orig=[x[0] for x in struct.iter_unpack('>I',f['code'])];words=prefix+orig;code=b''.join(struct.pack('>I',x) for x in words)
 # Prefix insertion shifts both ends of every original jump equally.
 edits.extend([(f['slots_offset'],4,struct.pack('>I',max(f['slots'],s+3))),(f['code_count_offset'],4,struct.pack('>I',len(words))),(f['code_offset'],len(f['code']),code),(f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),(f['line_count_offset'],4,struct.pack('>I',len(words))),(f['lines_offset'],len(f['lines']),f['lines'][:4]*len(prefix)+f['lines'])])
 for local_index,(_,a,start,b,end) in enumerate(f['locals']):edits.extend([(a,4,struct.pack('>I',0 if local_index<s else start+len(prefix))),(b,4,struct.pack('>I',end+len(prefix)))])
 changed[fid]=(code,cs+extra);reports.append(dict(id=fid,name=name,token=tokens[fid],prefixInstructions=len(prefix)))
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
(root/'.work/slingshot-requests').mkdir(exist_ok=True)
(root/'.work/slingshot-requests/c0000.hks').write_bytes(patched)
if options.inplace:path.write_bytes(patched)
(root/'.work/hks').mkdir(exist_ok=True)
(root/'.work/slingshot-requests/hks-before.hks').write_bytes(source)
(root/'.work/slingshot-requests/hks-validation.json').write_text(json.dumps(dict(inputSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(patched).hexdigest(),functions=reports,gateCases=cases,inverseRestoresInput=True,originalBranchesUnchanged=True,entryArgumentsPreserved=True,nativeArgumentsPreserved=True,nativeSprintToggleUnchanged=True,acceptedSprintRequestOnly=True,inputEdgesSampledBeforeRequest=True,originalStaminaAndMovementCallsUnchanged=True,gameplayVerified=False),indent=2))
print(f'PASS: {len(reports)} sprint request/input callbacks; {cases} gates; register/branch preservation and inverse checked.')
