"""Prefix only the three all-body Ram actions; preserve every native instruction."""
import argparse, hashlib, json, struct
from pathlib import Path
from hks_bytecode import Reader
from build_onyx_hks import abx, abc
from mmv_hks_fragments import Fragment
ROOT=Path(__file__).resolve().parents[1]
QUERY=990060
NAMES={'SwordArtsOneShot_onUpdate','SwordArtsOneShotComboEnd_onUpdate','SwordArtsOneShotComboEnd_2_onUpdate'}
def patch(source):
 r=Reader(source);r.function();edits=[];report={}
 for f in r.functions:
  if f['name'] not in NAMES:continue
  if QUERY in f['constants']:continue
  cs=f['constants'];extra=[];encoded=bytearray();code=[];labels={};jumps=[];s=f['slots'];q=s+8
  def k(v):
   if v in cs:return cs.index(v)
   if v in extra:return len(cs)+extra.index(v)
   i=len(cs)+len(extra)
   if isinstance(v,str):
    extra.extend([v,None,None]);b=v.encode()+b'\0';encoded.extend(b'\x04'+struct.pack('>Q',len(b))+b+b'\0\0')
   else:extra.append(v);encoded.extend(b'\x03'+struct.pack('>f',v))
   return i
  def glob(reg,key):
   i=k(key);code.extend([abx(91,reg,i),abx(76,20,i+1)])
  def const(reg,v):code.append(abx(25,reg,k(v)))
  def call(reg,n,ret=True):code.append(abc(2,reg,n+1,2 if ret else 1))
  def jump(label):jumps.append((len(code),label));code.append(0)
  def eq(reg,value,label):const(q+1,value);code.append(abc(4,1,reg,q+1));jump(label)
  def end():code.append(abc(9,0,1,0))
  # Evaluate inputs in separate scratch registers, retaining original locals.
  glob(s,'env');const(s+1,QUERY);glob(s+2,'c_SwordArtsID');glob(s+3,'c_SwordArtsHand')
  glob(s+4,'env');const(s+5,1108);glob(s+6,'ACTION_ARM_L2');call(s+4,2)
  glob(s+5,'env');const(s+6,1116);const(s+7,100052);call(s+5,2)
  glob(s+6,'GetVariable');const(s+7,'MoveAngle');call(s+6,1);call(s,6)
  eq(s,0,'native')
  # Keep forced transitions, excluding the native fall/cancel paths that
  # would erase high-speed inertia at a ledge or on button release.
  for name,args in [('env',[1112]),('ExecNearDeath',[]),('ExecDeath',[]),('ExecTalk',[]),('ExecMovableEventAnim',[]),('env',[346]),('ExecDamage',[0,0]),('ExecBirdAct',[]),('ExecFookShot',[])]:
   glob(q,name)
   for i,v in enumerate(args):const(q+i+1,v)
   call(q,len(args));eq(q,1,'abort')
  eq(s,3,'recoil')
  glob(q,'act');const(q+1,2004);const(q+2,0);call(q,2,False)
  eq(s,2,'loop');end()
  labels['loop']=len(code);glob(q,'ExecEventAllBody');const(q+1,'W_SwordArtsOneShotComboEnd');call(q,1,False);end()
  labels['recoil']=len(code);glob(q,'ExecEventAllBody');const(q+1,'W_Repelled_Wall');call(q,1,False);end()
  labels['abort']=len(code);glob(q,'env');const(q+1,990061);call(q,1,False);end()
  labels['native']=len(code)
  for pc,label in jumps:code[pc]=abx(28,0,0x10000+labels[label]-pc-2)
  while len(encoded)%4:encoded.append(0);extra.append(None)
  prefix=b''.join(struct.pack('>I',w)for w in code);n=len(code)
  Fragment(prefix,cs+extra).lua() # Validate branch targets and register definition paths.
  edits += [(f['slots_offset'],4,struct.pack('>I',q+3)),(f['code_count_offset'],4,struct.pack('>I',f['count']+n)),(f['code_offset'],len(f['code']),prefix+f['code']),(f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),(f['line_count_offset'],4,struct.pack('>I',f['count']+n)),(f['lines_offset'],len(f['lines']),f['lines'][:4]*n+f['lines'])]
  for _,a,start,b,endpc in f['locals']:edits += [(a,4,struct.pack('>I',start+n if start else 0)),(b,4,struct.pack('>I',endpc+n))]
  report[f['name']]=n
 target=source
 for at,size,data in sorted(edits,reverse=True):target=target[:at]+data+target[at+size:]
 check=Reader(target);check.function()
 for old,new in zip(r.functions,check.functions):
  n=report.get(old['name'],0);assert new['code'][n*4:]==old['code'];assert new['constants'][:len(old['constants'])]==old['constants']
 inverse=[];shift=0
 for at,size,data in sorted(edits):inverse.append((at+shift,len(data),source[at:at+size]));shift+=len(data)-size
 restored=target
 for at,size,data in reversed(inverse):restored=restored[:at]+data+restored[at+size:]
 assert restored==source
 return target,report
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--inplace',action='store_true');a=p.parse_args()
 path=ROOT/'dist/mod/action/script/c0000.hks';target,report=patch(path.read_bytes())
 out=ROOT/'work/lightning-ram/hks';out.mkdir(parents=True,exist_ok=True);(out/'c0000.hks').write_bytes(target)
 (out/'validation.json').write_text(json.dumps(dict(prefixes=report,sha256=hashlib.sha256(target).hexdigest(),exactInverse=True,nativeBodiesPreserved=True),indent=2)+'\n')
 if a.inplace:path.write_bytes(target)
 print('PASS: Lightning Ram action prefixes, original control flow and exact inverse')
