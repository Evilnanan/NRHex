"""Keep Sword alive in native spell recovery callbacks, preserving native bytecode."""
from hks_bytecode import *
import argparse, itertools, json

ap=argparse.ArgumentParser();ap.add_argument('--inplace',action='store_true');opt=ap.parse_args()
root=Path(__file__).resolve().parents[1];work=root/'.work/sword-vfx'
path=root/'dist/mod/action/script/c0000.hks' if opt.inplace else work/'c0000.hks'
source=path.read_bytes();r=Reader(source);r.function();edits=[];changed={};reports=[];cases=0
targets=[f for f in r.functions if (f['name'] or '').startswith('MagicFire') and 'Cancel_Upper_onUpdate' in f['name']]
# Numbered and jumping recovery variants have different names.
targets=[f for f in r.functions if (f['name'] or '').startswith('MagicFire') and 'Cancel' in f['name'] and f['name'].endswith('_Upper_onUpdate')]
assert len(targets)==8
def abx(op,a,bx):return op<<25|bx<<8|a
def abc(op,a,b,c):return op<<25|b<<17|c<<8|a
already=all(95000960 in f['constants'] or 95000968 in f['constants'] for f in targets)
for f in ([] if already else targets):
 cs=f['constants'];assert 95000960 not in cs and 95000968 not in cs
 assert 'MagicCommonFunction' in cs and 'HalfBlendUpperCommonFunction' in cs
 s=f['slots'];assert s+4<256
 extra=[];encoded=bytearray()
 def k(v):
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
  ix=k(v);return [abx(91,a,ix),abx(76,20,ix+1)]
 p=glob(s,'env')+[abx(25,s+1,k(1116)),abx(25,s+2,k(95000912)),abc(2,s,3,2)]+glob(s+1,'TRUE')+[abc(4,0,s,s+1),0]
 jumps=[len(p)-1]
 p+=glob(s,'GetVariable')+[abx(25,s+1,k('IndexMagicSlot')),abc(2,s,2,2),abx(25,s+1,k(2)),abc(4,1,s,s+1),0];jumps.append(len(p)-1)
 left='MagicFireLeft' in f['name'];p+=glob(s,'act')+[abx(25,s+1,k(2002)),abx(25,s+2,k(95000968 if left else 95000960))]
 if not left:p+=glob(s+3,'c_Style')+glob(s+4,'HAND_LEFT_BOTH')+[abc(4,0,s+3,s+4),abx(28,0,0x10000),abx(25,s+2,k(95000968))]
 p+=[abc(2,s,3,1)]
 for j in jumps:p[j]=abx(28,0,0x10000+len(p)-j-2)
 for owned,skill,style in itertools.product([False,True],[False,True],range(4)):
  regs={i:100+i for i in range(s)};saved=dict(regs);calls=[];pc=0;constants=cs+extra
  gs={'env':lambda *a:int(owned),'TRUE':1,'GetVariable':lambda _:2 if skill else 0,'act':lambda *a:calls.append(a),'c_Style':style,'HAND_LEFT_BOTH':3}
  while pc<len(p):
   x=decode(p[pc]);pc+=1;op,a,b,c,bx=x['op'],x['a'],x['b'],x['c'],x['bx']
   if op==91:regs[a]=gs[constants[bx]]
   elif op==76:pass
   elif op==25:regs[a]=constants[bx]
   elif op==2:
    result=regs[a](*(regs[i] for i in range(a+1,a+b)))
    if c==2:regs[a]=result
   elif op==4:
    if (regs[b]==regs[c])!=bool(a):pc+=1
   elif op==28:pc+=bx-0x10000+1
   else:raise AssertionError(op)
  assert calls==([(2002,95000968 if left or style==3 else 95000960)] if owned and not skill else [])
  assert all(regs[i]==v for i,v in saved.items());cases+=1
 while len(encoded)%4:encoded.extend(b'\0');extra.append(None)
 n=len(p);code=b''.join(struct.pack('>I',w) for w in p)+f['code']
 edits.extend([(f['slots_offset'],4,struct.pack('>I',s+5)),(f['code_count_offset'],4,struct.pack('>I',f['count']+n)),(f['code_offset'],len(f['code']),code),(f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),(f['line_count_offset'],4,struct.pack('>I',f['count']+n)),(f['lines_offset'],len(f['lines']),f['lines'][:4]*n+f['lines'])])
 for _,a,start,b,end in f['locals']:edits.extend([(a,4,struct.pack('>I',start+n)),(b,4,struct.pack('>I',end+n))])
 changed[f['id']]=(code,cs+extra);reports.append(dict(id=f['id'],name=f['name'],prefixInstructions=n))
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
(work/'recovery-before.hks').write_bytes(source);path.write_bytes(patched)
if opt.inplace:(work/'c0000.hks').write_bytes(patched)
report=json.loads((work/'hks-validation.json').read_text())
assert report['outputSha256']==hashlib.sha256(source).hexdigest()
report['recovery']=dict(alreadyPatched=already,functions=reports,cases=cases,nativeInstructionsPreserved=True,inverseRestoresInput=True)
report['outputSha256']=hashlib.sha256(patched).hexdigest()
(work/'hks-validation.json').write_text(json.dumps(report,indent=2))
print(f'PASS: {len(reports)} spell recovery callbacks, {cases} ownership/skill/hand cases; native instructions and branches preserved')
