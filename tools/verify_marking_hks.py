"""Execute the serialized HKS prefixes, checking real event arguments survive."""
import struct
from build_marking_hks import check,QUERY,EVENTS
from hks_bytecode import instructions

def verify(data):
 r=check(data);cases=0
 events=[*EVENTS,*(s+'_Upper'for s in EVENTS),'W_Rolling','W_Stealth_Rolling','W_SwordArtsRolling','W_AttackRightLight1','W_AttackRightLight2','W_AttackRightBackstep','W_Move','W_Idle','W_DamageLv3','UpperDefaultState0','W_MagicFireRight']
 for fid in [1,2,3,4]:
  f=r.functions[fid];words=instructions(f);callsites=[i for i,x in enumerate(words)if x['op']==2 and x['c']==1]
  # The first call is the observer; native callbacks follow it unchanged.
  stop=callsites[0]+1;a=words[0]['a'];assert a>=3
  for event in events:
   regs={i:('original',i)for i in range(a)};regs[0]=event;saved=dict(regs);calls=[]
   globals_={'env':lambda *args:calls.append(args)};pc=0
   for _ in range(160):
    if pc==stop:break
    x=words[pc];pc+=1;op,ra,b,c,bx=x['op'],x['a'],x['b'],x['c'],x['bx']
    if op==91:regs[ra]=globals_[f['constants'][bx]]
    elif op==76:pass
    elif op==25:regs[ra]=f['constants'][bx]
    elif op==4:
     if (regs[b]==regs[c])!=bool(ra):pc+=1
    elif op==28:pc+=bx-0x10000+1
    elif op==2:regs[ra](*(regs[i]for i in range(ra+1,ra+b)))
    else:raise AssertionError((fid,op))
   else:raise AssertionError('Observer loop')
   mode=next((i+(10 if event.endswith('_Upper')else 0)for i,name in enumerate(EVENTS,1)if event in[name,name+'_Upper']),0)
   assert calls==[(QUERY,float(mode))],(fid,event,calls)
   assert all(regs[i]==v for i,v in saved.items());cases+=1
 return cases
if __name__=='__main__':
 from pathlib import Path
 n=verify((Path(__file__).resolve().parents[1]/'dist/mod/action/script/c0000.hks').read_bytes());print(f'PASS: {n} serialized Marking transition cases preserve every original argument')
