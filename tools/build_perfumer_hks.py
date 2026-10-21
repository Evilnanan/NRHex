"""Request native aromatics only after idle/movement input processing.

Preserve existing instructions and relocate branches/debug ranges together.
"""
import argparse,hashlib,json,struct
from pathlib import Path
from hks_bytecode import Reader,decode,instructions
ROOT=Path(__file__).resolve().parents[1]
BEGIN,ACTIVE,INACTIVE,MANUAL=990041,990043,990044,990045
TARGETS={187:'IdleCommonFunction',212:'MoveCommonFunction',1255:'Update',127:'ExecItem'}
def abx(op,a,bx):return op<<25|bx<<8|a
def abc(op,a,b,c):return op<<25|b<<17|c<<8|a
def patch(source):
 r=Reader(source);r.function();edits=[];changed={};reports=[]
 for fid,name in TARGETS.items():
  f=r.functions[fid];assert f['name']==name;assert BEGIN not in f['constants'] and MANUAL not in f['constants']
  cs=f['constants'][:];extra=[];encoded=bytearray();code=[];labels={};jumps=[]
  def constant(v):
   if v in cs:return cs.index(v)
   if v in extra:return len(cs)+extra.index(v)
   idx=len(cs)+len(extra)
   if isinstance(v,str):
    extra.extend([v,None,None]);b=v.encode()+b'\0';encoded.extend(b'\x04'+struct.pack('>Q',len(b))+b+b'\0\0')
   else:extra.append(float(v));encoded.extend(b'\x03'+struct.pack('>f',v))
   return idx
  def glob(a,v):code.extend([abx(91,a,constant(v)),abx(76,20,constant(v)+1)])
  def num(a,v):code.append(abx(25,a,constant(v)))
  def call(a,n=0,returns=0):code.append(abc(2,a,n+1,returns+1))
  def jump(label):jumps.append((len(code),label));code.append(0)
  def label(name):labels[name]=len(code)
  def equal(a,b,yes,no):code.append(abc(4,0,a,b));jump(no);jump(yes)
  def env(a,which):glob(a,'env');num(a+1,which);call(a,1)
  a=f['slots'];old=[w[0]for w in struct.iter_unpack('>I',f['code'])]
  if fid in [187,212]:
   at=f['count']-4;assert [x['op']for x in instructions(f)[at:]]==[91,76,9,9] and f['constants'][decode(old[at])['bx']]=='FALSE'
   # Preserve native non-request item gates for overlays and special modes.
   for index,guard in enumerate(['grapple','demon',115,113]):
    if guard=='grapple':glob(a,'GetVariable');num(a+1,'AddFookShotBlend');call(a,1,1)
    elif guard=='demon':glob(a,'IsDemonSwordMode');call(a,0,1)
    else:glob(a,'env');num(a+1,guard);call(a,1,1)
    num(a+1,1);code.append(abc(4,0,a,a+1));jump('guard'+str(index));glob(a+2,'FALSE');code.append(abc(9,a+2,2,0));label('guard'+str(index))
   glob(a,'env');num(a+1,BEGIN);call(a,1,1);num(a+1,0)
   code.append(abc(4,0,a,a+1));jump('use');glob(a+2,'FALSE');code.append(abc(9,a+2,2,0))
   label('use');num(a+1,2);code.append(abc(4,0,a,a+1));jump('drink')
   def use_event(event):
    glob(a+2,'ExecEventHalfBlend');glob(a+3,event)
    if fid==187:glob(a+4,'ALLBODY')
    else:code.append(abc(7,a+4,0,0))
    call(a+2,2)
   use_event('Event_ItemAromaUp');jump('used');label('drink');use_event('Event_ItemAromaDrink');label('used')
   glob(a+2,'act');num(a+3,150);call(a+2,1);glob(a+2,'SetAIActionState');call(a+2)
   glob(a+2,'TRUE');code.append(abc(9,a+2,2,0))
  elif fid==1255:
   at=0
   for index,node in enumerate(['ItemAromaUp_Upper','ItemAromaDrink_Upper']):
    glob(a,'IsNodeActive');num(a+1,node);call(a,1,1);glob(a+1,'TRUE');code.append(abc(4,0,a,a+1));jump('second'if index==0 else'inactive');env(a,ACTIVE);jump('done')
    if index==0:label('second')
   label('inactive');env(a,INACTIVE);label('done')
  else:
   sites=[i for i,x in enumerate(instructions(f)[:-2])if x['op']==91 and f['constants'][x['bx']]=='act' and decode(old[i+2])['op']==25 and f['constants'][decode(old[i+2])['bx']]==2024];assert len(sites)==1;at=sites[0]
   env(a,MANUAL)
  for pc,to in jumps:code[pc]=abx(28,0,labels[to]-pc-2+0x10000)
  n=len(code);new=[]
  for pc,word in enumerate(old):
   if pc==at:new.extend(code)
   ins=decode(word)
   if ins['op']==28:
    target=pc+ins['bx']-0x10000+2;newpc=pc+(n if pc>=at else 0);newtarget=target+(n if target>at or target==at and fid==1255 else 0)
    word=abx(28,ins['a'],newtarget-newpc-2+0x10000)
   new.append(word)
  while len(encoded)%4:encoded.extend(b'\0');extra.append(None)
  out=b''.join(struct.pack('>I',w)for w in new)
  edits.extend([(f['slots_offset'],4,struct.pack('>I',a+6)),(f['code_count_offset'],4,struct.pack('>I',len(new))),(f['code_offset'],len(f['code']),out),(f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),(f['line_count_offset'],4,struct.pack('>I',len(new))),(f['lines_offset'],len(f['lines']),f['lines'][:at*4]+f['lines'][at*4:at*4+4]*n+f['lines'][at*4:])])
  for _,so,start,eo,end in f['locals']:edits.extend([(so,4,struct.pack('>I',start+(n if start>=at and start!=0 else 0))),(eo,4,struct.pack('>I',end+(n if end>=at else 0)))])
  changed[fid]=(out,cs+extra);reports.append(dict(id=fid,name=name,insertion=at,prefix=code,scratch=a,constants=cs+extra))
 patched=source
 for off,length,value in sorted(edits,reverse=True):patched=patched[:off]+value+patched[off+length:]
 inverse=[];shift=0
 for off,length,value in sorted(edits):inverse.append((off+shift,len(value),source[off:off+length]));shift+=len(value)-length
 restored=patched
 for off,length,value in sorted(inverse,reverse=True):restored=restored[:off]+value+restored[off+length:]
 assert restored==source
 after=Reader(patched);after.function();assert source[r.p:]==patched[after.p:]
 for old,new in zip(r.functions,after.functions):
  if old['id']in changed:assert(new['code'],new['constants'])==changed[old['id']]
  elif old['id']!=0:assert source[old['start']:old['end']]==patched[new['start']:new['end']]
  else:assert old['code']==new['code']and old['constants']==new['constants']
 return patched,reports
if __name__=='__main__':
 ap=argparse.ArgumentParser();ap.add_argument('--inplace',action='store_true');args=ap.parse_args();path=ROOT/'dist/mod/action/script/c0000.hks';source=path.read_bytes()
 patched,reports=patch(source);w=ROOT/'work/perfumer/hks';w.mkdir(parents=True,exist_ok=True);(w/'before.hks').write_bytes(source);(w/'c0000.hks').write_bytes(patched)
 (w/'validation.json').write_text(json.dumps(dict(inputSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(patched).hexdigest(),functions=reports,inverseRestoresInput=True,otherFunctionsUnchanged=True,gameplayVerified=False),indent=2)+'\n')
 if args.inplace:path.write_bytes(patched)
 print('PASS: Perfumer native-use requests added after idle/movement inputs; manual item use and clip lifetime retain priority')
