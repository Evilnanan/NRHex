"""Surgical ownership guards at actual wall-request functions, no recompilation."""
from hks_bytecode import *
import json
root=Path(__file__).resolve().parents[1]
source=(root/'assets/original/action/script/c0000.hks').read_bytes()
assert hashlib.sha256(source).hexdigest()=='2ec9f8ad913a8a6f1cf4a6f4794692a59e1fecda156bc9a87b3068f226c0dc1d'
r=Reader(source);r.function();edits=[];changed={};reports=[]
def abx(op,a,bx):return (op<<25)|(bx<<8)|a
def abc(op,a,b,c):return (op<<25)|(b<<17)|(c<<8)|a
for name,fid,count in [('JumpCommonFunction',1508,1072),('ExecWallJump',1514,119),
 ('WallJumpCancelEarly_onUpdate',1579,27),('WallJumpCancelLate_onUpdate',1581,27)]:
 f=next(f for f in r.functions if f['name']==name)
 assert f['id']==fid and f['count']==count
 cs=f['constants'];extra=[];encoded=b''
 def number(v):
  global encoded
  if float(v) in cs:return cs.index(float(v))
  idx=len(cs)+len(extra);extra.append(float(v));encoded+=b'\x03'+struct.pack('>f',v)
  assert struct.unpack('>f',struct.pack('>f',v))[0]==v
  return idx
 def cached_global(v):
  global encoded
  if v in cs:return cs.index(v)
  idx=len(cs)+len(extra);extra.extend([v,None,None]);value=v.encode()+b'\0'
  encoded+=b'\x04'+struct.pack('>Q',len(value))+value+b'\0\0'
  return idx
 env=cs.index('env');true=cs.index('TRUE')
 assert cs[env+1:env+3]==[None,None] and cs[true+1:true+3]==[None,None]
 scratch=struct.unpack('>I',f['header'][4:8])[0]
 assert scratch+3<=f['slots'] # Preserve the function's parameter registers.
 check=number(990008);owner=number(95000800)
 words=[abx(91,scratch,env),abx(76,20,env+1),abx(25,scratch+1,check),abx(25,scratch+2,owner),abc(2,scratch,3,2),
  abx(91,scratch+1,true),abx(76,20,true+1),abc(4,0,scratch,scratch+1),0]
 jumps=[8]
 cancel='WallJumpCancel' in name
 if not cancel:
  enabled=cs.index('ISENABLE_WALLJUMP')
  words.extend([abx(91,scratch,true),abx(76,20,true+1),abx(27,scratch,enabled)])
 if cancel:
  grounded=number(201)
  words.extend([abx(91,scratch,env),abx(76,20,env+1),abx(25,scratch+1,grounded),abc(2,scratch,2,2),
   abx(91,scratch+1,true),abx(76,20,true+1),abc(4,1,scratch,scratch+1),0])
  jumps.append(len(words)-1)
  helper=cached_global('ExecWallJump')
  words.extend([abx(91,scratch,helper),abx(76,20,helper+1),abc(69,scratch,1,2),
   abx(91,scratch+1,true),abx(76,20,true+1),abc(4,0,scratch,scratch+1),0,abc(9,0,1,0)])
  jumps.append(len(words)-2)
 for jump in jumps:words[jump]=abx(28,0,0x10000+len(words)-jump-2)
 while len(encoded)%4:encoded+=b'\0';extra.append(None)
 prefix=b''.join(struct.pack('>I',w) for w in words);n=len(words)
 middle=[];insert=55
 if fid==1508:
  # R0 is jump_type, R1 is fall height, R2 is equip_arm_no. They are live here.
  # Act_Jump's damage/death handling and early return remain before this block.
  s=3;helper=cs.index('ExecWallJump');grounded=number(201)
  middle=[abx(91,s,env),abx(76,20,env+1),abx(25,s+1,check),abx(25,s+2,owner),abc(2,s,3,2),
   abx(91,s+1,true),abx(76,20,true+1),abc(4,0,s,s+1),0,
   abx(91,s,env),abx(76,20,env+1),abx(25,s+1,grounded),abc(2,s,2,2),
   abx(91,s+1,true),abx(76,20,true+1),abc(4,1,s,s+1),0,
   abx(91,s,helper),abx(76,20,helper+1),abc(69,s,1,2),
   abx(91,s+1,true),abx(76,20,true+1),abc(4,0,s,s+1),0,
   abx(91,s,true),abx(76,20,true+1),abc(9,s,2,0)]
  for jump in [8,16,23]:middle[jump]=abx(28,0,0x10000+len(middle)-jump-2)
  for owned,grounded_now,accepted in __import__('itertools').product([False,True],repeat=3):
   calls=[];regs={0:17,1:12.5,2:1};pc=0;returned=False
   globals_={'TRUE':1,'env':lambda *args:int(owned) if args==(990008,95000800) else int(grounded_now),
    'ExecWallJump':lambda:calls.append('helper') or int(accepted),'act':lambda *args:calls.append(args)}
   constants=cs+extra
   while pc<len(middle):
    i=decode(middle[pc]);pc+=1;op,a,b,c,bx=i['op'],i['a'],i['b'],i['c'],i['bx']
    if op==91:regs[a]=globals_[constants[bx]]
    elif op==76:pass
    elif op==25:regs[a]=constants[bx]
    elif op in [2,69]:
     result=regs[a](*(regs[j] for j in range(a+1,a+b)))
     if c==2:regs[a]=result
    elif op==4:
     if (regs[b]==regs[c])!=bool(a):pc+=1
    elif op==28:pc+=1+(bx&0xffff)
    elif op==9:assert regs[a]==1;returned=True;break
    else:raise AssertionError(op)
   assert [regs[i] for i in range(3)]==[17,12.5,1]
   assert calls==(['helper'] if owned and not grounded_now else [])
   assert returned==bool(owned and not grounded_now and accepted)
 # Adding constants above must retain 4-byte alignment of subsequent prototypes.
 while len(encoded)%4:encoded+=b'\0';extra.append(None)
 origwords=[v[0] for v in struct.iter_unpack('>I',f['code'])];relocated=list(origwords)
 if middle:
  for pc,w in enumerate(origwords):
   i=decode(w)
   if i['op']!=28:continue
   delta=i['bx']-0x10000+1;target=pc+1+delta
   newpc=pc+(len(middle) if pc>=insert else 0)
   # Branches to the insertion boundary execute the new guard first.
   newtarget=target+(len(middle) if target>insert else 0)
   relocated[pc]=abx(28,i['a'],0x10000+newtarget-newpc-2)
 body=relocated[:insert]+middle+relocated[insert:] if middle else relocated
 code=prefix+b''.join(struct.pack('>I',w) for w in body);total=n+len(middle)
 edits.extend([(f['code_count_offset'],4,struct.pack('>I',count+total)),(f['code_offset'],count*4,code),
  (f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,encoded),
  (f['line_count_offset'],4,struct.pack('>I',count+total)),(f['lines_offset'],len(f['lines']),f['lines'][:4]*n+f['lines'][:insert*4]+f['lines'][insert*4:insert*4+4]*len(middle)+f['lines'][insert*4:])])
 for lname,a,start,b,end in f['locals']:
  # Entry parameters remain live during the prefix, including their names.
  debug_start=start if scratch and start==0 else start+n+(len(middle) if start>=insert else 0)
  edits.extend([(a,4,struct.pack('>I',debug_start)),(b,4,struct.pack('>I',end+n+(len(middle) if end>=insert else 0)))])
 changed[fid]=(code,extra)
 for owned,enabled_before,grounded_now,requested,accepted in __import__('itertools').product([False,True],[0,1],[False,True],[False,True],[False,True]):
   calls=[];reg={i:123+i for i in range(scratch)};saved=dict(reg)
   def mock_env(*args):
    return {(990008,95000800):int(owned),(201,):int(grounded_now),(1106,6):int(requested)}.get(args,-1)
   def mock_helper():calls.append(('ExecWallJump',));return int(accepted)
   glob={'env':mock_env,'act':lambda *args:calls.append(args),'ExecWallJump':mock_helper,'TRUE':1,'ISENABLE_WALLJUMP':enabled_before}
   constants=cs+extra;pc=0
   returned=False
   for step in range(60):
    if pc>=n:break
    i=decode(words[pc]);pc+=1;op,a,b,c,bx=i['op'],i['a'],i['b'],i['c'],i['bx']
    if op==91:reg[a]=glob[constants[bx]]
    elif op==76:pass
    elif op==25:reg[a]=constants[bx]
    elif op in [2,69]:
     result=reg[a](*(reg[j] for j in range(a+1,a+b)))
     if c==2:reg[a]=result
    elif op==4:
     if (reg[b]==reg[c])!=bool(a):pc+=1
    elif op==28:pc+=1+(bx&0xffff)
    elif op==27:glob[constants[bx]]=reg[a]
    elif op==9:assert a==0 and b==1;returned=True;break
    else:raise AssertionError(op)
   else:raise AssertionError('Prefix did not finish')
   assert glob['ISENABLE_WALLJUMP']==(1 if owned and not cancel else enabled_before)
   assert all(reg[i]==v for i,v in saved.items())
   expected=[]
   if owned and cancel and not grounded_now:expected=[('ExecWallJump',)]
   assert calls==expected
   assert returned==bool(owned and cancel and not grounded_now and accepted)
 reports.append({'name':name,'functionId':fid,'originalInstructions':count,'patchedInstructions':count+total,'argumentRegistersPreserved':scratch,'earlyRetryAfterOriginalInstruction':insert if middle else None,'earlyRetryLiveRegistersPreserved':[0,1,2] if middle else [],'relocatedBranches':sum(a!=b for a,b in zip(origwords,relocated))})
# Owner-only casting protection. Add a tiny prefix to actual spell animation
# callbacks; all original instructions and branch deltas remain intact. Skip
# cancel/recovery callbacks and weapon-skill casts (IndexMagicSlot == 2).
cast_names=[]
for f in r.functions:
 name=f['name'] or ''
 if not (name.endswith('_Upper_onUpdate') and ('MagicLaunch' in name or 'MagicLoop' in name or 'MagicFire' in name) and 'Cancel' not in name and not name.startswith('SAMagic')):continue
 cast_names.append(name)
 cs=f['constants'];extra=[];encoded=bytearray()
 def add_cast(v):
  if v in cs:return cs.index(v)
  if v in extra:return len(cs)+extra.index(v)
  ix=len(cs)+len(extra)
  if isinstance(v,str):
   extra.extend([v,None,None]);b=v.encode()+b'\0';encoded.extend(b'\x04'+struct.pack('>Q',len(b))+b+b'\0\0')
  else:
   assert struct.unpack('>f',struct.pack('>f',v))[0]==v
   extra.append(float(v));encoded.extend(b'\x03'+struct.pack('>f',v))
  return ix
 env=add_cast('env');act=add_cast('act');true=add_cast('TRUE');var=add_cast('GetVariable');slot=add_cast('IndexMagicSlot')
 check=add_cast(1116);owner=add_cast(95000912);guard=add_cast(95000904);command=add_cast(2002);two=add_cast(2)
 scratch=struct.unpack('>I',f['header'][4:8])[0]
 assert scratch+3<=f['slots']
 words=[abx(91,scratch,env),abx(76,20,env+1),abx(25,scratch+1,check),abx(25,scratch+2,owner),abc(2,scratch,3,2),
  abx(91,scratch+1,true),abx(76,20,true+1),abc(4,0,scratch,scratch+1),0,
  abx(91,scratch,var),abx(76,20,var+1),abx(25,scratch+1,slot),abc(2,scratch,2,2),abx(25,scratch+1,two),abc(4,1,scratch,scratch+1),0,
  abx(91,scratch,env),abx(76,20,env+1),abx(25,scratch+1,check),abx(25,scratch+2,guard),abc(2,scratch,3,2),
  abx(91,scratch+1,true),abx(76,20,true+1),abc(4,1,scratch,scratch+1),0,
  abx(91,scratch,act),abx(76,20,act+1),abx(25,scratch+1,command),abx(25,scratch+2,guard),abc(2,scratch,3,1)]
 for j in [8,15,24]:words[j]=abx(28,0,0x10000+len(words)-j-2)
 # Verify every ownership, slot and live-guard branch independently.
 for owned,weapon_skill,present in __import__('itertools').product([False,True],repeat=3):
  pc=0;reg={};calls=[];constants=cs+extra
  glob={'TRUE':1,'env':lambda _,effect:int(owned if effect==95000912 else present),
   'GetVariable':lambda _:2 if weapon_skill else 0,'act':lambda *args:calls.append(args)}
  while pc<len(words):
   i=decode(words[pc]);pc+=1;op,a,b,c,bx=i['op'],i['a'],i['b'],i['c'],i['bx']
   if op==91:reg[a]=glob[constants[bx]]
   elif op==76:pass
   elif op==25:reg[a]=constants[bx]
   elif op==2:
    result=reg[a](*(reg[j] for j in range(a+1,a+b)))
    if c==2:reg[a]=result
   elif op==4:
    if (reg[b]==reg[c])!=bool(a):pc+=1
   elif op==28:pc+=1+(bx&0xffff)
   else:raise AssertionError(op)
  assert calls==([(2002,95000904)] if owned and not weapon_skill and not present else [])
 while len(encoded)%4:encoded.extend(b'\0');extra.append(None)
 n=len(words);code=b''.join(struct.pack('>I',w) for w in words)+f['code'];count=f['count']
 edits.extend([(f['code_count_offset'],4,struct.pack('>I',count+n)),(f['code_offset'],count*4,code),
  (f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),
  (f['line_count_offset'],4,struct.pack('>I',count+n)),(f['lines_offset'],len(f['lines']),f['lines'][:4]*n+f['lines'])])
 for _,a,start,b,end in f['locals']:edits.extend([(a,4,struct.pack('>I',start+n)),(b,4,struct.pack('>I',end+n))])
 changed[f['id']]=(code,extra)
 reports.append({'name':name,'functionId':f['id'],'originalInstructions':count,'patchedInstructions':count+n,'castOwnerGuardPathsVerified':True})
assert len(cast_names)>=20
patched=source
for off,n,new in sorted(edits,reverse=True):patched=patched[:off]+new+patched[off+n:]
shift=0;inverse=[]
for off,n,new in sorted(edits):
 inverse.append((off+shift,len(new),source[off:off+n]));shift+=len(new)-n
restored=patched
for off,n,old in sorted(inverse,reverse=True):restored=restored[:off]+old+restored[off+n:]
assert restored==source
new=Reader(patched);new.function();assert patched[new.p:]==source[r.p:]
for a,b in zip(r.functions,new.functions):
 assert a['id']==b['id'] and a['name']==b['name']
 if a['id'] in changed:
  code,extra=changed[a['id']]
  assert b['code']==code and b['constants']==a['constants']+extra
 elif a['id']!=0:assert patched[b['start']:b['end']]==source[a['start']:a['end']]
 else:assert a['code']==b['code'] and a['constants']==b['constants']
out=root/'dist/mod/action/script/c0000.hks'
if '--check' in sys.argv:assert out.read_bytes()==patched,'Exact validated original-bytecode patch required'
else:
 out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(patched)
 report={'version':'0.9.0','method':'Owner-only airborne retry after Act_Jump and before animation-phase early return; preserve R0/R1/R2 and original safety path',
  'originalSha256':hashlib.sha256(source).hexdigest(),'patchedSha256':hashlib.sha256(patched).hexdigest(),
  'addedBytes':shift,'functions':reports,'otherPrototypesUnchanged':len(r.functions)-len(changed),
  'inversePatchReconstructsOriginal':True,'ownerAndNonownerPathsVerified':True,'serializedDebugLineCountMatchesInstructions':True,
  'argumentRegistersPreserved':True,'removedWallDiagnosticPulses':[95000808,95000816,95000824,95000832],
  'ownerGroundRequestAcceptedPathsVerified':True,'earlyRetryOwnerGroundAcceptedPathsVerified':True,'gameplayVerified':False}
 (root/'dist/hks-validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
print(f'PASS: original HKS +{shift} bytes; early and cancel retries, live registers, control flow and inverse edits verified.')


