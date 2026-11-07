"""Observe roll/backstep displacement at native action boundaries."""
import argparse,hashlib,json,struct
from pathlib import Path
from hks_bytecode import Reader,decode
ROOT=Path(__file__).resolve().parents[1]
QUERY=990055
ACTIONS=['Rolling','Rolling_Selftrans','Rolling_Selftrans2','DefaultBackStep','LadyBackStep','LadyBackStep2']

def abx(op,a,bx):return op<<25|bx<<8|a

def abc(op,a,b,c):return op<<25|b<<17|c<<8|a

def patch(source):
 r=Reader(source);r.function();edits=[];changed={}
 selected={f['id']:(ACTIONS.index(f['name'].rsplit('_',1)[0])+1,['onActivate','onUpdate','onDeactivate'].index(f['name'].rsplit('_',1)[1]))for f in r.functions if f['name']and f['name'].rsplit('_',1)[0]in ACTIONS and f['name'].rsplit('_',1)[1]in ['onActivate','onUpdate','onDeactivate']}
 assert len(selected)==17
 for fid,(action,phase) in selected.items():
  f=r.functions[fid];assert QUERY not in f['constants'],'Evasion observer already installed'
  cs=f['constants'][:];extra=[];encoded=bytearray()
  def constant(value):
   if value in cs:return cs.index(value)
   if value in extra:return len(cs)+extra.index(value)
   index=len(cs)+len(extra)
   if isinstance(value,str):extra.extend([value,None,None]);b=value.encode()+b'\0';encoded.extend(b'\x04'+struct.pack('>Q',len(b))+b+b'\0\0')
   else:extra.append(float(value));encoded.extend(b'\x03'+struct.pack('>f',value))
   return index
  scratch=f['slots']
  def glob(reg,key):
   k=constant(key);return [abx(91,reg,k),abx(76,20,k+1)]
  prefix=glob(scratch,'env')+[abx(25,scratch+1,constant(QUERY)),abx(25,scratch+2,constant(phase)),abx(25,scratch+3,constant(action)),abc(2,scratch,4,1)]
  old=[x[0]for x in struct.iter_unpack('>I',f['code'])];body=old[:]
  while len(encoded)%4:encoded.extend(b'\0');extra.append(None)
  new=prefix+body
  # Entry insertion preserves every relative branch and skip pair unchanged.
  code=b''.join(struct.pack('>I',x)for x in new);n=len(prefix)
  edits.extend([(f['slots_offset'],4,struct.pack('>I',scratch+4)),(f['code_count_offset'],4,struct.pack('>I',len(new))),(f['code_offset'],len(f['code']),code),(f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),(f['line_count_offset'],4,struct.pack('>I',len(new))),(f['lines_offset'],len(f['lines']),f['lines'][:4]*n+f['lines'])])
  for _,a,start,b,end in f['locals']:edits.extend([(a,4,struct.pack('>I',start+n if start else 0)),(b,4,struct.pack('>I',end+n))])
  changed[fid]=dict(name=f['name'],phase=phase,action=action,originalInstructions=len(old),addedInstructions=n)
 patched=source
 for off,size,value in sorted(edits,reverse=True):patched=patched[:off]+value+patched[off+size:]
 actual=Reader(patched);actual.function();assert len(actual.functions)==len(r.functions)
 for f,a in zip(r.functions,actual.functions):
  if f['id']in changed:
   info=changed[f['id']];tail=bytearray(a['code'][info['addedInstructions']*4:])
   assert tail==f['code']and a['constants'][:len(f['constants'])]==f['constants']
  else:assert source[f['start']:f['end']]==patched[a['start']:a['end']]if not f['children']else a['code']==f['code']
 inverse=[];shift=0
 for off,size,value in sorted(edits):inverse.append((off+shift,len(value),source[off:off+size]));shift+=len(value)-size
 restored=patched
 for off,size,value in reversed(inverse):restored=restored[:off]+value+restored[off+size:]
 assert restored==source
 return patched,changed

def verify(data):
 r=Reader(data);r.function();fs=[f for f in r.functions if f['name']and f['name'].rsplit('_',1)[0]in ACTIONS];assert len(fs)==17 and all(QUERY in f['constants']for f in fs);return True
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--inplace',action='store_true');p.add_argument('--check',action='store_true');a=p.parse_args();path=ROOT/'dist/mod/action/script/c0000.hks';source=path.read_bytes()
 if a.check:verify(source);print('PASS: Evasion action callbacks present');raise SystemExit
 try:verify(source);print('PASS: Evasion callbacks already present');raise SystemExit
 except AssertionError:pass
 target,changed=patch(source);out=ROOT/'work/static-evasion/hks';out.mkdir(parents=True,exist_ok=True);(out/'before.hks').write_bytes(source);(out/'c0000.hks').write_bytes(target)
 (out/'validation.json').write_text(json.dumps(dict(inputSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(target).hexdigest(),functions=changed,originalControlFlowRetained=True,exactInversePassed=True),indent=2)+'\n')
 if a.inplace:path.write_bytes(target)
 print('PASS: Evasion action callbacks; original control flow retained and exact inverse checked')
