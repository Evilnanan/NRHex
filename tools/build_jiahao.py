"""Compose owner-gated action notifications onto the validated original HKS patch."""
from hks_bytecode import *
import json,itertools
root=Path(__file__).resolve().parents[1]
path=root/'dist/mod/action/script/c0000.hks';source=path.read_bytes()
base=root/'.work/hks/jiahao-base.hks'
if '--check' in sys.argv:source=base.read_bytes()
else:base.write_bytes(source)
r=Reader(source);r.function();edits=[];changed={};reports=[]
def abx(op,a,bx):return op<<25|bx<<8|a
def abc(op,a,b,c):return op<<25|b<<17|c<<8|a
markers={269:6999701,272:6999701,275:6999701}
for f in r.functions:
 fid=f['id']
 if fid not in markers and fid!=141:continue
 cs=f['constants'];extra=[];encoded=bytearray()
 def constant(v):
  if v in cs:return cs.index(v)
  if v in extra:return len(cs)+extra.index(v)
  ix=len(cs)+len(extra)
  if isinstance(v,str):
   extra.extend([v,None,None]);value=v.encode()+b'\0';encoded.extend(b'\x04'+struct.pack('>Q',len(value))+value+b'\0\0')
  else:
   assert struct.unpack('>f',struct.pack('>f',v))[0]==v
   extra.append(float(v));encoded.extend(b'\x03'+struct.pack('>f',v))
  return ix
 def global_(s,v):
  k=constant(v);return [abx(91,s,k),abx(76,20,k+1)]
 def pulse(s,token,exclude_backstep=False):
  w=global_(s,'env')+[abx(25,s+1,constant(1116)),abx(25,s+2,constant(6999700)),abc(2,s,3,2)]+global_(s+1,'TRUE')+[abc(4,0,s,s+1),0]
  jumps=[len(w)-1]
  if exclude_backstep:
   # R3 remains the successful GetEvasionRequest result, R0-R4 stay live.
   w+=global_(s,'ATTACK_REQUEST_BACKSTEP')+[abc(4,1,3,s),0];jumps.append(len(w)-1)
  w+=global_(s,'act')+[abx(25,s+1,constant(2002)),abx(25,s+2,constant(token)),abc(2,s,3,1)]
  for j in jumps:w[j]=abx(28,0,0x10000+len(w)-j-2)
  # Exercise ownership and successful request classification independently.
  for owned,back in itertools.product([False,True],repeat=2):
   regs={3:9 if back else 8};saved={i:123+i for i in range(s) if i!=3};regs.update(saved);calls=[];pc=0;constants=cs+extra
   glob={'TRUE':1,'ATTACK_REQUEST_BACKSTEP':9,'env':lambda *args:int(owned),'act':lambda *args:calls.append(args)}
   while pc<len(w):
    i=decode(w[pc]);pc+=1;op,a,b,c,bx=i['op'],i['a'],i['b'],i['c'],i['bx']
    if op==91:regs[a]=glob[constants[bx]]
    elif op==76:pass
    elif op==25:regs[a]=constants[bx]
    elif op==2:
     v=regs[a](*(regs[j] for j in range(a+1,a+b)))
     if c==2:regs[a]=v
    elif op==4:
     if (regs[b]==regs[c])!=bool(a):pc+=1
    elif op==28:pc+=bx-0x10000+1
    else:raise AssertionError(op)
   assert calls==([(2002,token)] if owned and not(exclude_backstep and back) else [])
   assert all(regs[i]==v for i,v in saved.items())
   if exclude_backstep:assert regs[3]==(9 if back else 8)
  return w
 def enable_guard_backstep_tae():
  # Native guard branch disabled body TAE at 1372. R5-R7 are scratch before
  # ExecEvent; retain R0-R4 and the upper guard layer. Only the owner restores it.
  words=global_(5,'env')+[abx(25,6,constant(1116)),abx(25,7,constant(6999700)),abc(2,5,3,2)]+global_(6,'TRUE')+[abc(4,0,5,6),0]
  jump=len(words)-1
  words+=global_(5,'SetVariable')+[abx(25,6,constant('EnableTAE_BackStep')),abc(13,7,1,0),abc(2,5,3,1)]
  words[jump]=abx(28,0,0x10000+len(words)-jump-2)
  for owned in [False,True]:
   saved={i:123+i for i in range(5)};regs=dict(saved);calls=[];pc=0;constants=cs+extra
   def env(*args):assert args==(1116,6999700);return int(owned)
   glob={'env':env,'TRUE':1,'SetVariable':lambda *args:calls.append(args)}
   while pc<len(words):
    i=decode(words[pc]);pc+=1;op,a,b,c,bx=i['op'],i['a'],i['b'],i['c'],i['bx']
    if op==91:regs[a]=glob[constants[bx]]
    elif op==76:pass
    elif op==25:regs[a]=constants[bx]
    elif op==13:regs[a]=bool(b)
    elif op==2:
     value=regs[a](*(regs[j] for j in range(a+1,a+b)))
     if c==2:regs[a]=value
    elif op==4:
     if (regs[b]==regs[c])!=bool(a):pc+=1
    elif op==28:pc+=bx-0x10000+1
    else:raise AssertionError(op)
   assert calls==([('EnableTAE_BackStep',True)] if owned else [])
   assert all(regs[i]==value for i,value in saved.items())
  return words
 inserts={320:pulse(6,6999705),1373:enable_guard_backstep_tae(),1457:pulse(5,6999705,True)} if fid==141 else {0:pulse(0,markers[fid])}
 if fid==141:
  original=[w[0] for w in struct.iter_unpack('>I',f['code'])]
  assert decode(original[1370])['op']==25 and cs[decode(original[1370])['bx']]=='EnableTAE_BackStep'
  assert decode(original[1371])==dict(op=13,a=7,b=0,c=0,bx=0)
  assert decode(original[1372])==dict(op=2,a=5,b=3,c=1,bx=1537)
  assert decode(original[1373])['op']==91 and cs[decode(original[1373])['bx']]=='ExecEvent'
 while len(encoded)%4:encoded.extend(b'\0');extra.append(None)
 orig=[w[0] for w in struct.iter_unpack('>I',f['code'])];body=[];lines=[]
 def position(pc,boundary=False):return pc+sum(len(w) for at,w in inserts.items() if at<pc or (at==pc and not boundary))
 for pc,w in enumerate(orig):
  if pc in inserts:body+=inserts[pc];lines+=[f['lines'][pc*4:pc*4+4]]*len(inserts[pc])
  i=decode(w)
  if i['op']==28:
   target=pc+1+i['bx']-0x10000+1
   w=abx(28,i['a'],0x10000+position(target,True)-position(pc)-2)
  body.append(w);lines.append(f['lines'][pc*4:pc*4+4])
 code=b''.join(struct.pack('>I',w) for w in body)
 edits.extend([(f['slots_offset'],4,struct.pack('>I',max(3,f['slots']))),(f['code_count_offset'],4,struct.pack('>I',len(body))),(f['code_offset'],f['count']*4,code),(f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),(f['line_count_offset'],4,struct.pack('>I',len(body))),(f['lines_offset'],len(f['lines']),b''.join(lines))])
 for _,a,start,b,end in f['locals']:edits.extend([(a,4,struct.pack('>I',position(start,True))),(b,4,struct.pack('>I',position(end)))])
 changed[fid]=(code,cs+extra);reports.append({'id':fid,'name':f['name'],'insertions':{str(k):len(v) for k,v in inserts.items()},'originalInstructions':f['count'],'patchedInstructions':len(body)})
patched=source
for off,n,v in sorted(edits,reverse=True):patched=patched[:off]+v+patched[off+n:]
inverse=[];shift=0
for off,n,v in sorted(edits):inverse.append((off+shift,len(v),source[off:off+n]));shift+=len(v)-n
restored=patched
for off,n,v in sorted(inverse,reverse=True):restored=restored[:off]+v+restored[off+n:]
assert restored==source
new=Reader(patched);new.function();assert patched[new.p:]==source[r.p:]
for old,current in zip(r.functions,new.functions):
 if old['id'] in changed:assert (current['code'],current['constants'])==changed[old['id']]
 elif old['id']!=0:assert source[old['start']:old['end']]==patched[current['start']:current['end']]
 else:assert old['code']==current['code'] and old['constants']==current['constants']
if '--check' in sys.argv:assert path.read_bytes()==patched
else:
 path.write_bytes(patched)
 (root/'dist/jiahao-hks-validation.json').write_text(json.dumps(dict(version='0.14.8',baseSha256=hashlib.sha256(source).hexdigest(),sha256=hashlib.sha256(patched).hexdigest(),functions=reports,inverseRestoresValidatedBase=True,ownerAndNonownerPathsVerified=True,successfulDodgesOnly=True,backstepExcludedFromGaugeLoss=True,guardBackstepOwnerTaeRestored=True,nonownerGuardBackstepTaeDisablePreserved=True,upperGuardLayerPreserved=True,unrelatedFunctionsUnchanged=True,removedUnusedActionPulses=[6999702,6999703,6999704],gameplayVerified=False),indent=2),encoding='utf8')
print('PASS: Jiahao action markers; owner gates, dodge classification, relocations and exact inverse verified.')
