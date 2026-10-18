"""Add a read-only movement sample and guard only native sprint start cost.

The original HKS instructions are retained, with branches/debug locations
relocated together; no Lua decompilation is ever compiled back into the game.
"""
import argparse,json,struct,hashlib
from pathlib import Path
from hks_bytecode import Reader,decode
ROOT=Path(__file__).resolve().parents[1]
QUERY=990034

def abx(op,a,bx):return op<<25|bx<<8|a

def abc(op,a,b,c):return op<<25|b<<17|c<<8|a

def patch(source):
 reader=Reader(source);reader.function();edits=[];changed={};reports=[]
 for fid,name in [(1255,'Update'),(259,'SprintStart_onActivate')]:
  f=reader.functions[fid];assert f['name']==name
  assert QUERY not in f['constants'],'Ambulance patch already installed'
  cs=f['constants'][:];extra=[];encoded=bytearray()
  def constant(value):
   if value in cs:return cs.index(value)
   if value in extra:return len(cs)+extra.index(value)
   index=len(cs)+len(extra)
   if isinstance(value,str):
    extra.extend([value,None,None]);b=value.encode()+b'\0';encoded.extend(b'\x04'+struct.pack('>Q',len(b))+b+b'\0\0')
   else:extra.append(float(value));encoded.extend(b'\x03'+struct.pack('>f',value))
   return index
  def glob(a,value):return [abx(91,a,constant(value)),abx(76,20,constant(value)+1)]
  # At Update entry, preserve its original argument. At sprint's cost site,
  # all original activation scratch values are dead; leave act(101,TRUE) alone.
  at=0
  if fid==259:
   orig=[x[0]for x in struct.iter_unpack('>I',f['code'])]
   assert f['constants'][decode(orig[-6])['bx']]=='AddStamina'
   at=len(orig)-2
   scratch=2
  else:scratch=struct.unpack('>I',f['header'][4:8])[0]
  a=scratch
  prefix=glob(a,'env')+[abx(25,a+1,constant(QUERY))]
  for target,var in [(a+2,'MoveAngle'),(a+3,'MoveSpeedLevel')]:
   prefix+=glob(target,'GetVariable')+[abx(25,target+1,constant(var)),abc(2,target,2,2)]
  prefix+=[abc(2,a,4,2)]
  if fid==259:
   prefix+=glob(a+1,'TRUE')+[abc(4,0,a,a+1)]
   # TRUE skips the jump and substitutes zero for the original cost in R1.
   # The original AddStamina helper (including its regen block) still runs.
   prefix+=[abx(28,0,0x10000),abx(25,1,constant(0))]
  while len(encoded)%4:encoded.extend(b'\0');extra.append(None)
  old=[x[0]for x in struct.iter_unpack('>I',f['code'])];new=[];n=len(prefix)
  for pc,word in enumerate(old):
   if pc==at:new+=prefix
   ins=decode(word)
   if ins['op']==28:
    target=pc+ins['bx']-0x10000+2
    newpc=pc+(n if pc>=at else 0);newtarget=target+(n if target>=at else 0)
    word=abx(28,ins['a'],newtarget-newpc-2+0x10000)
   new.append(word)
  code=b''.join(struct.pack('>I',word)for word in new)
  edits.extend([(f['slots_offset'],4,struct.pack('>I',max(f['slots'],a+5))), (f['code_count_offset'],4,struct.pack('>I',len(new))), (f['code_offset'],len(f['code']),code), (f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))), (f['constants_end'],0,bytes(encoded)), (f['line_count_offset'],4,struct.pack('>I',len(new))), (f['lines_offset'],len(f['lines']),f['lines'][:at*4]+f['lines'][at*4:at*4+4]*n+f['lines'][at*4:])])
  for _,startoff,start,endoff,end in f['locals']:
   edits.extend([(startoff,4,struct.pack('>I',start+(n if start>=at and start!=0 else 0))), (endoff,4,struct.pack('>I',end+(n if end>=at else 0)))])
  changed[fid]=(code,cs+extra);reports.append(dict(id=fid,name=name,insertion=at,prefix=prefix,scratch=a,constants=cs+extra))
 patched=source
 for offset,length,value in sorted(edits,reverse=True):patched=patched[:offset]+value+patched[offset+length:]
 # Independent structural readback and exact inverse.
 inverse=[];shift=0
 for offset,length,value in sorted(edits):inverse.append((offset+shift,len(value),source[offset:offset+length]));shift+=len(value)-length
 restored=patched
 for offset,length,value in sorted(inverse,reverse=True):restored=restored[:offset]+value+restored[offset+length:]
 assert restored==source
 after=Reader(patched);after.function();assert patched[after.p:]==source[reader.p:]
 for old,new in zip(reader.functions,after.functions):
  if old['id']in changed:assert (new['code'],new['constants'])==changed[old['id']]
  elif old['id']!=0:assert source[old['start']:old['end']]==patched[new['start']:new['end']]
  else:assert old['code']==new['code']and old['constants']==new['constants']
 return patched,reports

if __name__=='__main__':
 ap=argparse.ArgumentParser();ap.add_argument('--inplace',action='store_true');args=ap.parse_args()
 path=ROOT/'dist/mod/action/script/c0000.hks';source=path.read_bytes();r=Reader(source);r.function()
 if all(QUERY in r.functions[i]['constants'] for i in [1255,259]):print('PASS: Ambulance HKS already installed');raise SystemExit
 patched,reports=patch(source);out=ROOT/'work/ambulance/hks';out.mkdir(parents=True,exist_ok=True)
 (out/'before.hks').write_bytes(source);(out/'c0000.hks').write_bytes(patched)
 (out/'validation.json').write_text(json.dumps(dict(inputSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(patched).hexdigest(),functions=reports,inverseRestoresInput=True,otherFunctionsUnchanged=True,gameplayVerified=False),indent=2))
 if args.inplace:path.write_bytes(patched)
 print('PASS: Update movement sample and SprintStart cost guard; all original HKS instructions retained')
