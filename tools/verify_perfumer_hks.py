"""Execute composed free-locomotion bytecode with player input fixtures."""
from pathlib import Path
import json,struct
from hks_bytecode import Reader,instructions
from local_paths import ROOT
from build_perfumer_hks import TARGETS,BEGIN,ACTIVE,INACTIVE,MANUAL
raw=(ROOT/'dist/mod/action/script/c0000.hks').read_bytes();r=Reader(raw);r.function();cases=0
for fid in [187,212]:
 f=r.functions[fid];assert f['name']==TARGETS[fid]
 for speed in [0,1,2,3]:
  for kind in [0,1,2,3]:
   for manual in [None,'ExecAttack','ExecMagic','ExecEvasion','ExecGesture','ExecWeaponChange','IsDemonSwordMode','blocked_grapple','blocked_115','blocked_113']:
    calls=[];regs={i:0 for i in range(f['slots'])};regs[0]=10
    def env(code,*args):
     if code==BEGIN:calls.append(('begin',));return kind
     if (code==115 and manual=='blocked_115')or(code==113 and manual=='blocked_113'):return 1
     return 0
    def helper(name):
     def run(*args):
      if name=='GetVariable':return speed if args[0]=='MoveSpeedLevel'else int(manual=='blocked_grapple'and args[0]=='AddFookShotBlend')
      if name=='ExecEventHalfBlend':calls.append(('event',*args));return 0
      if name==manual:return 1
      return 0
     return run
    globals_={name:helper(name)for name in f['constants']if isinstance(name,str)}
    globals_.update(env=env,TRUE=1,FALSE=0,ALLBODY=10,UPPER=11,LOWER=12,c_Style=0,c_IsStealth=0,HAND_RIGHT_BOTH=20,HAND_LEFT_BOTH=21)
    # Global event and enum values are data, not callable helpers.
    for name in globals_.copy():
     if name.startswith(('Event_','QUICKTYPE_','ACTION_','FALL_TYPE_','ESTEP_','TO_'))or name.startswith('HAND_')and name not in ['HAND_RIGHT_BOTH','HAND_LEFT_BOTH']:globals_[name]=name
    words=instructions(f);pc=0
    def rk(index):return f['constants'][index-256]if index>=256 else regs[index]
    for _ in range(3000):
     x=words[pc];pc+=1;op,a,b,c,bx=x['op'],x['a'],x['b'],x['c'],x['bx']
     if op==91:regs[a]=globals_[f['constants'][bx]]
     elif op==76:pass
     elif op==25:regs[a]=f['constants'][bx]
     elif op==7:regs[a]=regs[b]
     elif op in [2,69]:
      val=regs[a](*(regs[i]for i in range(a+1,a+b)))
      if c==2:regs[a]=val
     elif op==4:
      if (regs[b]==rk(c))!=bool(a):pc+=1
     elif op in [57,59]:
      val=f['constants'][b]<rk(c)if op==57 else f['constants'][b]<=rk(c)
      if val!=bool(a):pc+=1
     elif op==28:pc+=bx-0x10000+1
     elif op==9:
      result=regs[a]if b==2 else None;break
     else:raise AssertionError((fid,pc-1,op))
    else:raise AssertionError('bytecode loop')
    if manual:
     expected=0 if manual in ['IsDemonSwordMode','blocked_grapple','blocked_115','blocked_113']else 1
     assert calls==[]and result==expected,(fid,speed,kind,manual,calls,result)
    else:
     assert calls==[('begin',)]+([('event','Event_ItemAromaUp'if kind==2 else'Event_ItemAromaDrink',10)]if kind else[]),(fid,speed,kind,calls)
     assert result==int(kind!=0)
    cases+=1
# Independently execute the Update entry prefix, leaving all original registers
# below its added scratch range untouched. Both actual native graph names exist.
f=r.functions[1255];words=instructions(f);end=next(i+2 for i,x in enumerate(words)if x['op']==25 and f['constants'][x['bx']]==INACTIVE);entry={'scratch':words[0]['a']}
for up in [0,1]:
 for drink in [0,1]:
  regs={i:1000+i for i in range(f['slots'])};saved=dict(regs);calls=[];globals_={'TRUE':1,'IsNodeActive':lambda name:up if name=='ItemAromaUp_Upper'else drink,'env':lambda code:calls.append(code)or 0};pc=0
  for _ in range(100):
   if pc==end:break
   x=words[pc];pc+=1;op,a,b,c,bx=x['op'],x['a'],x['b'],x['c'],x['bx']
   if op==91:regs[a]=globals_[f['constants'][bx]]
   elif op==76:pass
   elif op==25:regs[a]=f['constants'][bx]
   elif op==2:
    val=regs[a](*(regs[i]for i in range(a+1,a+b)))
    if c==2:regs[a]=val
   elif op==4:
    if (regs[b]==regs[c])!=bool(a):pc+=1
   elif op==28:pc+=bx-0x10000+1
   else:raise AssertionError(op)
  else:raise AssertionError('prefix loop')
  assert calls==[ACTIVE if up or drink else INACTIVE]
  assert all(regs[i]==saved[i]for i in range(entry['scratch']));cases+=1
# Manual cancellation is inside the accepted original use path, before act2024.
f=r.functions[127];words=instructions(f);manual=[i for i,x in enumerate(words)if x['op']==25 and f['constants'][x['bx']]==MANUAL];assert len(manual)==1
act2024=next(i for i,x in enumerate(words)if x['op']==25 and f['constants'][x['bx']]==2024);assert manual[0]<act2024
print(f'PASS: Perfumer {cases} HKS cases; movement allowed, manual actions prioritized, native perfume graph events and clip cleanup')
