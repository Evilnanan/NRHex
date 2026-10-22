"""Observe native item action frames without changing the original action code."""
import argparse,hashlib,json,struct
from pathlib import Path
from hks_bytecode import Reader,decode
ROOT=Path(__file__).resolve().parents[1]
FRAME=990043;BUSY=990044

def abx(op,a,bx):return op<<25|bx<<8|a

def abc(op,a,b,c):return op<<25|b<<17|c<<8|a

def patch(source):
 r=Reader(source);r.function();edits=[];changed={}
 selected={1255:FRAME,208:BUSY,209:BUSY,210:BUSY,723:BUSY}
 # Riding uses a dedicated onUpdate path instead of ItemCommonFunction.
 selected.update({f['id']:BUSY for f in r.functions if f['name']and f['name'].startswith('RideItem')and f['name'].endswith('_onUpdate')})
 for fid,query in selected.items():
  f=r.functions[fid];assert query not in f['constants'],'Wraith observer already installed'
  cs=f['constants'][:];extra=[];encoded=bytearray()
  def constant(value):
   if value in cs:return cs.index(value)
   if value in extra:return len(cs)+extra.index(value)
   index=len(cs)+len(extra)
   if isinstance(value,str):extra.extend([value,None,None]);b=value.encode()+b'\0';encoded.extend(b'\x04'+struct.pack('>Q',len(b))+b+b'\0\0')
   else:extra.append(float(value));encoded.extend(b'\x03'+struct.pack('>f',value))
   return index
  scratch=f['slots'];prefix=[abx(91,scratch,constant('env')),abx(76,20,constant('env')+1),abx(25,scratch+1,constant(query)),abc(2,scratch,2,1)]
  while len(encoded)%4:encoded.extend(b'\0');extra.append(None)
  old=[x[0]for x in struct.iter_unpack('>I',f['code'])];new=prefix+old
  # Entry insertion preserves every relative branch and skip pair unchanged.
  code=b''.join(struct.pack('>I',x)for x in new);n=len(prefix)
  edits.extend([(f['slots_offset'],4,struct.pack('>I',scratch+2)),(f['code_count_offset'],4,struct.pack('>I',len(new))),(f['code_offset'],len(f['code']),code),(f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),(f['line_count_offset'],4,struct.pack('>I',len(new))),(f['lines_offset'],len(f['lines']),f['lines'][:4]*n+f['lines'])])
  for _,a,start,b,end in f['locals']:edits.extend([(a,4,struct.pack('>I',start+n if start else 0)),(b,4,struct.pack('>I',end+n))])
  changed[fid]=dict(name=f['name'],query=query,originalInstructions=len(old),addedInstructions=n)
 patched=source
 for off,size,value in sorted(edits,reverse=True):patched=patched[:off]+value+patched[off+size:]
 actual=Reader(patched);actual.function();assert len(actual.functions)==len(r.functions)
 for f,a in zip(r.functions,actual.functions):
  if f['id']in changed:assert a['code'][16:]==f['code']and a['constants'][:len(f['constants'])]==f['constants']
  else:assert source[f['start']:f['end']]==patched[a['start']:a['end']]if not f['children']else a['code']==f['code']
 inverse=[];shift=0
 for off,size,value in sorted(edits):inverse.append((off+shift,len(value),source[off:off+size]));shift+=len(value)-size
 restored=patched
 for off,size,value in reversed(inverse):restored=restored[:off]+value+restored[off+size:]
 assert restored==source
 return patched,changed

def verify(data):
 r=Reader(data);r.function();assert FRAME in r.functions[1255]['constants']
 for fid in [208,209,210,723]:assert BUSY in r.functions[fid]['constants']
 return True
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--inplace',action='store_true');p.add_argument('--check',action='store_true');a=p.parse_args();path=ROOT/'dist/mod/action/script/c0000.hks';source=path.read_bytes()
 if a.check:verify(source);print('PASS: Wraith item-chain HKS observers present');raise SystemExit
 try:verify(source);print('PASS: Wraith observers already present');raise SystemExit
 except AssertionError:pass
 target,changed=patch(source);out=ROOT/'work/wraith/hks';out.mkdir(parents=True,exist_ok=True);(out/'before.hks').write_bytes(source);(out/'c0000.hks').write_bytes(target)
 (out/'validation.json').write_text(json.dumps(dict(inputSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(target).hexdigest(),functions=changed,allOriginalInstructionsRetained=True,exactInversePassed=True),indent=2)+'\n')
 if a.inplace:path.write_bytes(target)
 print('PASS: Wraith item/common/riding observers; original bytecode preserved and exact inverse checked')
