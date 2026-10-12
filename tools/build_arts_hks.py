"""Prefix only native skill callbacks; preserve all original state transitions."""
from hks_bytecode import *
import json,itertools
root=Path(__file__).resolve().parents[1];path=root/'dist/mod/action/script/c0000.hks'
source=path.read_bytes();r=Reader(source);r.function();edits=[];changed={};reports=[]
def abx(op,a,bx):return op<<25|bx<<8|a
def abc(op,a,b,c):return op<<25|b<<17|c<<8|a
used_ids=set(range(541,576))|{579,587,588}
busy_ids={533,535}|set(range(536,582))|set(range(584,590))
for f in r.functions:
 fid=f['id'];name=f['name'] or ''
 magic=('Magic' in name and name.endswith('_onUpdate') and 799<=fid<=832)
 if fid not in busy_ids and not magic:continue
 used=fid in used_ids or (magic and 'Fire' in name and 'Cancel' not in name)
 # These native attack stances deal damage in the loop itself, unlike Unsheathe.
 attack_stance=fid in {537,538}
 cs=f['constants'];extra=[];encoded=bytearray();s=struct.unpack('>I',f['header'][4:8])[0]
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
 prefix=glob(s,'env')+[abx(25,s+1,constant(1116)),abx(25,s+2,constant(6999800)),abc(2,s,3,2)]+glob(s+1,'TRUE')+[abc(4,0,s,s+1),0];exits=[len(prefix)-1]
 if magic:
  prefix+=glob(s,'GetVariable')+[abx(25,s+1,constant('IndexMagicSlot')),abc(2,s,2,2),abx(25,s+1,constant(2)),abc(4,0,s,s+1),0];exits.append(len(prefix)-1)
 def emit(token):return glob(s,'act')+[abx(25,s+1,constant(2002)),abx(25,s+2,constant(token)),abc(2,s,3,1)]
 if used or attack_stance:
  if attack_stance:
   prefix+=glob(s,'c_SwordArtsID');matches=[]
   for kind in [10,11,25,239]:
    prefix+=[abx(25,s+1,constant(kind)),abc(4,1,s,s+1),0];matches.append(len(prefix)-1)
   prefix+=emit(6999801)+[0];exits.append(len(prefix)-1)
   for j in matches:prefix[j]=abx(28,0,0x10000+len(prefix)-j-2)
  prefix+=glob(s,'c_SwordArtsHand')+glob(s+1,'HAND_LEFT')+[abc(4,0,s,s+1),0];right=len(prefix)-1
  prefix+=emit(6999802)+[0];exits.append(len(prefix)-1)
  prefix[right]=abx(28,0,0x10000+len(prefix)-right-2)
  prefix+=emit(6999803)
 else:prefix+=emit(6999801)
 for j in exits:prefix[j]=abx(28,0,0x10000+len(prefix)-j-2)
 for owned,skill,left,kind in itertools.product([False,True],[False,True],[False,True],[10,11,25,239,14,15,260]):
  saved={i:100+i for i in range(s)};regs=dict(saved);calls=[];pc=0;constants=cs+extra
  globals_={'env':lambda *args:int(owned),'TRUE':1,'GetVariable':lambda _:2 if skill else 0,'act':lambda *args:calls.append(args),'c_SwordArtsHand':0 if left else 1,'HAND_LEFT':0,'c_SwordArtsID':kind}
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
   elif op==28:pc+=bx-0x10000+1
   else:raise AssertionError(op)
  expected=6999802 if left else 6999803
  actual_use=used or (attack_stance and kind in [10,11,25,239])
  assert calls==([(2002,expected if actual_use else 6999801)] if owned and (skill or not magic) else [])
  assert all(regs[i]==v for i,v in saved.items())
 while len(encoded)%4:encoded.extend(b'\0');extra.append(None)
 orig=[x[0] for x in struct.iter_unpack('>I',f['code'])];words=prefix+orig;code=b''.join(struct.pack('>I',x) for x in words)
 # Prefix insertion shifts both ends of every original jump equally.
 edits.extend([(f['slots_offset'],4,struct.pack('>I',max(f['slots'],s+3))),(f['code_count_offset'],4,struct.pack('>I',len(words))),(f['code_offset'],len(f['code']),code),(f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),(f['line_count_offset'],4,struct.pack('>I',len(words))),(f['lines_offset'],len(f['lines']),f['lines'][:4]*len(prefix)+f['lines'])])
 for _,a,start,b,end in f['locals']:edits.extend([(a,4,struct.pack('>I',start+len(prefix))),(b,4,struct.pack('>I',end+len(prefix)))])
 changed[fid]=(code,cs+extra);reports.append(dict(id=fid,name=name,used=used,attackStanceOnly=attack_stance,magicSkillOnly=magic,prefixInstructions=len(prefix)))
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
path.write_bytes(patched)
(root/'.work/hks').mkdir(exist_ok=True)
(root/'.work/hks/arts-hks-base.hks').write_bytes(source)
(root/'dist/arts-hks-validation.json').write_text(json.dumps(dict(version='0.15.2',functions=reports,inverseRestoresInput=True,originalBranchesUnchanged=True,registersPreserved=True,ownerNonownerAndBothHandsChecked=True,stanceOnlyDoesNotCountAsUse=True,continuousAttackStanceTypes=[10,11,25,239],normalSpellExcluded=True,gameplayVerified=False),indent=2))
print(f'PASS: {len(reports)} skill-node callbacks; both hands, stance preparation, combo exit, spell skill gates and inverse checked.')
