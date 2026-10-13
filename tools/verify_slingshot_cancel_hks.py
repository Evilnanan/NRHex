"""Read back the accepted cancellation branch and execute its native guards."""
from pathlib import Path
import json,subprocess,os,re,itertools
from hks_bytecode import Reader,instructions
root=Path(__file__).resolve().parents[1];work=root/'.work/slingshot-stacks'
report=json.loads((work/'hks-validation.json').read_text())
decompiler=Path(os.environ.get('NRHEX_DSLUA',str(root/'.work/tools/DSLuaDecompiler/DSLuaDecompiler/bin/Release/net9.0/DSLuaDecompiler.exe')))
parsed=[]
for source,name in [('hks-before.hks','before.lua'),('c0000.hks','after.lua')]:
 subprocess.run([str(decompiler),str(work/source),'--included-function-ids','1245','-o',str(work/name)],check=True)
 r=Reader((work/source).read_bytes());r.function();parsed.append(r.functions[1245])
before=(work/'before.lua').read_text(encoding='utf-8-sig');after=(work/'after.lua').read_text(encoding='utf-8-sig')
clean,count=re.subn(r'            if env\(1116, 6999960\) == TRUE then\n                act\(2002, 6999964\)\n            end\n','',after)
assert count==1 and clean==before,'Unexpected decompiled semantic changes'
def execute(f,owned,allowed,pressed):
 cs=f['constants'];code=instructions(f);regs={i:100+i for i in range(6)};calls=[];pc=405
 delta=f['count']-720
 def env(*args):
  if args==(1108,42):return pressed
  assert args==(1116,6999960);return int(owned)
 globals_={'ISENABLE_SPRINTSTOP':int(allowed),'TRUE':1,'FALSE':0,'env':env,'ACTION_ARM_L3':42,'ChangeMoveSpeedIndex':lambda *a:calls.append(('speed',*a)),'act':lambda *a:calls.append(('act',*a))}
 while 405<=pc<427+delta:
  x=code[pc];pc+=1;op,a,b,c,bx=[x[n] for n in ['op','a','b','c','bx']]
  rk=lambda n:cs[n-256] if n>=256 else regs[n]
  if op==91:regs[a]=globals_[cs[bx]]
  elif op==76:pass
  elif op==25:regs[a]=cs[bx]
  elif op==27:globals_[cs[bx]]=regs[a]
  elif op==2:
   v=regs[a](*(regs[j] for j in range(a+1,a+b)))
   if c==2:regs[a]=v
  elif op==4:
   if (rk(b)==rk(c))!=bool(a):pc+=1
  elif op==57:
   if (cs[b]<rk(c))!=bool(a):pc+=1
  elif op==28:pc+=bx-0x10000+1
  else:raise AssertionError(x)
 assert all(regs[i]==100+i for i in range(6))
 return calls,globals_.get('ISENABLE_SPRINTSTART')
cases=0
for owned,allowed,pressed in itertools.product([False,True],[False,True],[0,1,2]):
 a=execute(parsed[0],owned,allowed,pressed);b=execute(parsed[1],owned,allowed,pressed)
 assert [v for v in b[0] if v[0]!='act']==a[0] and b[1]==a[1]
 assert [v for v in b[0] if v[0]=='act']==([('act',2002,6999964)] if owned and allowed and pressed>0 else [])
 cases+=1
report.update(independentDecompilerReadbackPassed=True,nativeCancellationGuardCases=cases,liveRegistersPreserved=True)
(work/'hks-validation.json').write_text(json.dumps(report,indent=2))
print(f'PASS: only the accepted native cancel branch changed; {cases} ownership/input/latch cases; live locals preserved')
