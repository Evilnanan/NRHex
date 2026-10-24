"""Observe native Marking follow-up transitions; preserve native event dispatch."""
import argparse,hashlib,json,struct
from pathlib import Path
from hks_bytecode import Reader,decode
ROOT=Path(__file__).resolve().parents[1];QUERY=990042
EVENTS=['W_AttackRightBackstep','W_AttackBothBackstep','W_AttackDualBackStep','W_AttackArrowBothFireBackStep']
ORIGIN_QUERY=990047
ORIGIN_EVENTS=['W_HurtDaggerAttack','W_AirHurtDaggerAttack']
def abx(op,a,bx):return op<<25|bx<<8|a
def abc(op,a,b,c):return op<<25|b<<17|c<<8|a
def patch_observer(source,query,events):
 r=Reader(source);r.function();edits=[];audit={}
 for fid in [1,2,3,4]:
  f=r.functions[fid];cs=f['constants'];assert query not in cs;extra=[];encoded=bytearray()
  def const(v):
   if v in cs:return cs.index(v)
   if v in extra:return len(cs)+extra.index(v)
   i=len(cs)+len(extra)
   if isinstance(v,str):
    extra.extend([v,None,None]);b=v.encode()+b'\0';encoded.extend(b'\x04'+struct.pack('>Q',len(b))+b+b'\0\0')
   else:extra.append(float(v));encoded.extend(b'\x03'+struct.pack('>f',v))
   return i
  def glob(a,v):return [abx(91,a,const(v)),abx(76,20,const(v)+1)]
  a=f['slots'];prefix=glob(a,'env')+[abx(25,a+1,const(query)),abx(25,a+2,const(0))]
  for i,name in enumerate(events,1):
   for suffix,mode in [('',i),('_Upper',i+10)]:
    prefix+=[abx(25,a+3,const(name+suffix)),abc(4,0,0,a+3),abx(28,0,0x10000),abx(25,a+2,const(mode))]
  prefix+=[abc(2,a,3,1)]
  while len(encoded)%4:encoded.append(0);extra.append(None)
  old=[v[0]for v in struct.iter_unpack('>I',f['code'])];new=prefix+old;n=len(prefix)
  edits.extend([(f['slots_offset'],4,struct.pack('>I',a+4)),(f['code_count_offset'],4,struct.pack('>I',len(new))),(f['code_offset'],len(f['code']),b''.join(struct.pack('>I',v)for v in new)),(f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),(f['line_count_offset'],4,struct.pack('>I',len(new))),(f['lines_offset'],len(f['lines']),f['lines'][:4]*n+f['lines'])])
  for _,p,start,q,end in f['locals']:edits.extend([(p,4,struct.pack('>I',start+n if start else 0)),(q,4,struct.pack('>I',end+n))])
  audit[fid]=dict(name=f['name'],addedInstructions=n,scratch=a)
 target=source
 for p,n,b in sorted(edits,reverse=True):target=target[:p]+b+target[p+n:]
 after=Reader(target);after.function()
 for f,g in zip(r.functions,after.functions):
  if f['id']in audit:
   n=audit[f['id']]['addedInstructions'];assert g['code'][n*4:]==f['code']and g['constants'][:len(f['constants'])]==f['constants']and g['lines'][n*4:]==f['lines']
  elif not f['children']:assert target[g['start']:g['end']]==source[f['start']:f['end']]
  else:assert g['code']==f['code']and g['constants']==f['constants']
 inverse=[];shift=0
 for p,n,b in sorted(edits):inverse.append((p+shift,len(b),source[p:p+n]));shift+=len(b)-n
 restored=target
 for p,n,b in reversed(inverse):restored=restored[:p]+b+restored[p+n:]
 assert restored==source
 return target,audit
def patch(source):
 target=source;audit={}
 for name,query,events in [('followup',QUERY,EVENTS),('origin',ORIGIN_QUERY,ORIGIN_EVENTS)]:
  r=Reader(target);r.function();present=[query in r.functions[i]['constants']for i in [1,2,3,4]]
  assert all(present)or not any(present),'Partially installed Marking observer'
  if not all(present):target,audit[name]=patch_observer(target,query,events)
 return target,audit
def check(source):
 r=Reader(source);r.function()
 for i in [1,2,3,4]:assert all(q in r.functions[i]['constants']for q in [QUERY,ORIGIN_QUERY])and all(n in r.functions[i]['constants']for n in EVENTS+ORIGIN_EVENTS)
 return r
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');p.add_argument('--inplace',action='store_true');a=p.parse_args();path=ROOT/'dist/mod/action/script/c0000.hks';source=path.read_bytes()
 if a.check:check(source);print('PASS: Marking follow-up observers preserve native event dispatch');raise SystemExit
 try:check(source);print('PASS: Marking HKS observers already installed');raise SystemExit
 except AssertionError:pass
 target,audit=patch(source);out=ROOT/'work/marking-poison/hks';out.mkdir(parents=True,exist_ok=True);(out/'before.hks').write_bytes(source);(out/'c0000.hks').write_bytes(target)
 (out/'validation.json').write_text(json.dumps(dict(inputSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(target).hexdigest(),functions=audit,exactInversePassed=True,originalInstructionsAndOtherFunctionsPreserved=True),indent=2)+'\n')
 if a.inplace:path.write_bytes(target)
 print('PASS: Marking origin/follow-up observers staged; exact inverse and untouched native instructions verified')
