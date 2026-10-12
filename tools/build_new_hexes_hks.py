"""Surgical action notifications over the installed script; no recompilation."""
from hks_bytecode import Reader,decode,Path,struct
import argparse,hashlib,itertools,json
root=Path(__file__).resolve().parents[1]
args=argparse.ArgumentParser();args.add_argument('--inplace',action='store_true');args.add_argument('--source',type=Path);args.add_argument('--output',type=Path);options=args.parse_args()
path=root/'dist/mod/action/script/c0000.hks';source=(options.source or path).read_bytes()
r=Reader(source);r.function();edits=[];changed={};reports=[];cases=0
def abx(op,a,bx):return op<<25|bx<<8|a
def abc(op,a,b,c):return op<<25|b<<17|c<<8|a
for f in r.functions:
 if f['name'] not in ['Death_Activate','NearDeath_Activate','NearDeathRevival_onActivate','Item_NearDeathRevival_onActivate']:continue
 at=0
 cs=f['constants'];assert 6999901 not in cs and 6999911 not in cs,'New hex HKS already patched'
 old=[x[0] for x in struct.iter_unpack('>I',f['code'])]
 s=struct.unpack('>I',f['header'][4:8])[0]
 extra=[];encoded=bytearray()
 def constant(value):
  if value in cs:return cs.index(value)
  if value in extra:return len(cs)+extra.index(value)
  index=len(cs)+len(extra)
  if isinstance(value,str):
   extra.extend([value,None,None]);b=value.encode()+b'\0';encoded.extend(b'\x04'+struct.pack('>Q',len(b))+b+b'\0\0')
  else:
   assert struct.unpack('>f',struct.pack('>f',value))[0]==value
   extra.append(float(value));encoded.extend(b'\x03'+struct.pack('>f',value))
  return index
 def glob(reg,value):
  k=constant(value);return [abx(91,reg,k),abx(76,20,k+1)]
 p=glob(s,'env')+[abx(25,s+1,constant(1116)),abx(25,s+2,constant(6999900)),abc(2,s,3,2)]
 p+=glob(s+1,'TRUE')+[abc(4,0,s,s+1),0];exits=[len(p)-1]
 token=6999902 if 'Revival' in f['name'] else 6999901
 p+=glob(s,'act')+[abx(25,s+1,constant(2002)),abx(25,s+2,constant(token)),abc(2,s,3,1)]
 for j in exits:p[j]=abx(28,0,0x10000+len(p)-j-2)
 for owned,family in itertools.product([False,True],[False,True]):
  saved={i:1000+i for i in range(s)};regs=dict(saved);calls=[];pc=0
  globals_={'env':lambda *a:int(owned),'TRUE':1,'GetVariable':lambda *a:int(not family),'act':lambda *a:calls.append(a)}
  while pc<len(p):
   x=decode(p[pc]);pc+=1;op,a,b,c,bx=x['op'],x['a'],x['b'],x['c'],x['bx'];constants=cs+extra
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
  assert calls==([(2002,token)] if owned else [])
  assert all(regs[i]==v for i,v in saved.items());cases+=1
 while len(encoded)%4:encoded.extend(b'\0');extra.append(None)
 def moved(i):return i+len(p) if i>=at else i
 # Branches to the insertion point must execute its notification.
 def destination(i):return i+len(p) if i>at else i
 adjusted=[]
 for i,w in enumerate(old):
  x=decode(w)
  if x['op']==28:
   target=i+x['bx']-0x10000+2
   w=abx(28,x['a'],0x10000+destination(target)-moved(i)-2)
  adjusted.append(w)
 words=adjusted[:at]+p+adjusted[at:];code=b''.join(struct.pack('>I',w) for w in words)
 # Recover every old jump and instruction, independently of inserted code.
 for i,w in enumerate(adjusted):
  x=decode(w);oldx=decode(old[i])
  if x['op']==28:assert moved(i)+x['bx']-0x10000+2==destination(i+oldx['bx']-0x10000+2)
  else:assert w==old[i]
 lines=f['lines'][:at*4]+f['lines'][at*4:at*4+4]*len(p)+f['lines'][at*4:]
 edits.extend([(f['slots_offset'],4,struct.pack('>I',max(f['slots'],s+3))),(f['code_count_offset'],4,struct.pack('>I',len(words))),(f['code_offset'],len(f['code']),code),(f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),(f['line_count_offset'],4,struct.pack('>I',len(words))),(f['lines_offset'],len(f['lines']),lines)])
 for _,a,start,b,end in f['locals']:edits.extend([(a,4,struct.pack('>I',moved(start))),(b,4,struct.pack('>I',moved(end)))])
 changed[f['id']]=(code,cs+extra);reports.append({'id':f['id'],'name':f['name'],'at':at,'prefixInstructions':len(p)})
assert len(reports)==4
patched=source
for off,n,value in sorted(edits,reverse=True):patched=patched[:off]+value+patched[off+n:]
inverse=[];shift=0
for off,n,value in sorted(edits):inverse.append((off+shift,len(value),source[off:off+n]));shift+=len(value)-n
restored=patched
for off,n,value in sorted(inverse,reverse=True):restored=restored[:off]+value+restored[off+n:]
assert restored==source
new=Reader(patched);new.function();assert patched[new.p:]==source[r.p:]
for a,b in zip(r.functions,new.functions):
 if a['id'] in changed:assert (b['code'],b['constants'])==changed[a['id']]
 elif a['id']!=0:assert source[a['start']:a['end']]==patched[b['start']:b['end']]
 else:assert a['code']==b['code'] and a['constants']==b['constants']
out=options.output or root/'.work/new-hexes-01733/staged';out.mkdir(parents=True,exist_ok=True)
(out/'c0000.hks').write_bytes(patched)
if options.inplace:path.write_bytes(patched)
(out/'hks-validation.json').write_text(json.dumps({'version':'0.17.33','inputSha256':hashlib.sha256(source).hexdigest(),'outputSha256':hashlib.sha256(patched).hexdigest(),'functions':reports,'predicateCases':cases,'inverseRestoresInput':True,'originalBranchesAndRegistersPreserved':True,'nativeFamilyRecallUnchanged':True,'gameplayVerified':False},indent=2))
print('PASS: 4 Balloon action notifications; ordinary family recall untouched')
