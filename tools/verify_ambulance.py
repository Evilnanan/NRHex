"""Execute compiled Ambulance hooks and independently interpret its HKS guard.

No live game process is accessed. Actor/VM/rendering/session services in the
native fixture are stubs; native stamina, rescue and packet code really execute.
"""
import hashlib,json,os,struct,subprocess
from pathlib import Path
from hks_bytecode import Reader,instructions
from build_ambulance_hks import patch,QUERY
from local_paths import ROOT,GAME

def execute(words,constants,qualified,angle=17.,speed=2.,prefix_only=False,initial=None):
 regs=dict(initial or {});calls=[];pc=0
 def query(code,a,s):
  assert (code,a,s)==(QUERY,angle,speed)
  return int(qualified)
 globals_={'env':query,'TRUE':1,'GetVariable':lambda name:{'MoveAngle':angle,'MoveSpeedLevel':speed}[name],
           'act':lambda *args:calls.append(('act',args)), 'AddStamina':lambda value:calls.append(('stamina',value)), 'STAMINA_REDUCE_SPRINTSTART':-10.}
 for _ in range(500):
  if pc==len(words):assert prefix_only;return regs,calls
  x=words[pc];pc+=1;op,a,b,c,bx=x['op'],x['a'],x['b'],x['c'],x['bx']
  if op==91:regs[a]=globals_[constants[bx]]
  elif op==76:pass
  elif op==25:regs[a]=constants[bx]
  elif op==2:
   value=regs[a](*(regs[i]for i in range(a+1,a+b)))
   if c==2:regs[a]=value
  elif op==4:
   if (regs[b]==regs[c])!=bool(a):pc+=1
  elif op==28:pc+=bx-0x10000+1
  elif op==7:regs[a]=regs[b]
  elif op==9:return regs,calls
  else:raise AssertionError(op)
 raise AssertionError('Loop')

source=(ROOT/'assets/original/action/script/c0000.hks').read_bytes();patched,reports=patch(source)
a=Reader(source);a.function();b=Reader(patched);b.function();cases=0
for qualified in [False,True]:
 for angle in [-150.,0.,17.,30.,150.]:
  for speed in [0.,1.,2.,3.]:
   sprint=b.functions[259];_,calls=execute(instructions(sprint),sprint['constants'],qualified,angle,speed)
   assert calls==[('act',(101.,1)),('stamina',0. if qualified else -10.)];cases+=1
update=next(r for r in reports if r['id']==1255)
f=b.functions[1255];old=a.functions[1255];n=f['count']-old['count'];regs,calls=execute(instructions(f)[:n],f['constants'],False,prefix_only=True,initial={i:123456+i for i in range(8)})
assert calls==[]
argument_count=struct.unpack('>I',old['header'][4:8])[0]
assert all(regs[i]==123456+i for i in range(argument_count))
# Independent original branch target check for the prefixed Update.
original=instructions(old);new=instructions(f)[n:]
for pc,(before,after)in enumerate(zip(original,new)):
 if before['op']==28:
  assert pc+before['bx']==pc+after['bx']
 else:assert before==after
# Every patch performs a fresh query: no persistent discount written into HKS.
installed=Reader((ROOT/'dist/mod/action/script/c0000.hks').read_bytes());installed.function()
if QUERY in installed.functions[259]['constants']:
 assert QUERY in installed.functions[1255]['constants']
 assert sum(QUERY in f['constants']for f in installed.functions)==2
 for qualified in [False,True]:
  f=installed.functions[259]
  # Slingshot's existing entry notification is outside Ambulance's cost suffix.
  words=instructions(f);start=next(i for i,x in enumerate(words)if x['op']==91 and f['constants'][x['bx']]=='AddStamina')
  _,calls=execute(words[start:],f['constants'],qualified)
  assert calls==[('stamina',0. if qualified else -10.)]

env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
command=['cargo','test','--release']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','ambulance_compiled_detours_execute_outgoing_rescue_before_authority','--','--ignored','--test-threads=1']
result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');output=result.stdout+result.stderr
work=ROOT/'work/ambulance';work.mkdir(parents=True,exist_ok=True);(work/'native-test.log').write_text(output)
assert result.returncode==0 and '1 passed; 0 failed;'in output,output
sources=['src/ambulance.rs','src/ambulance_native_tests.rs','src/reward_choices.rs','src/hex_pool.rs','src/lib.rs','tools/HexBuild/Ambulance.cs','tools/build_ambulance_hks.py']
report=dict(nativeExecutionPassed=True,hksCases=cases,coneHalfAngleDegrees=30,periodicDebitEffects=[100300,102022],preservedPeriodicEffect=100320,staminaPerSegment=40,segmentRescueValues=[40,45,80],sharedProgress=True,idleGraceSeconds=5,decayStandardPointsPerSecond=30,standardPointsPerSegment=40,
 cases=['compiled hooks installed over native functions','native periodic cost summation matches non-owner and filters only movement effects','zero input, side/away direction, revived allies, expired movement samples','actual native hit-dispatch hook refunds before rejected recipient authority; gauge unchanged','last-hit overflow and stale zero gauge retain full outgoing credit','250 native rescue arithmetic comparisons across mode/tier/correction/minimum-one','other rescuers, no owner, revived target and self give no recovery','native rescue application and packet decoder in both formats never refund again','cross-teammate cross-tier half segments combine into one 40-stamina reward','five-second game-time grace, boundary split, 30-standard-point decay, zero-output no reset, positive-output reset','real native stamina cap, multi-segment overkill and no deferred full-stamina rewards','recovery visual requested only when positive stamina is granted; no visual for incomplete segments'],
 stubs=['actor/effect/roster lookup','Havok movement variables and rotation getter','network identity lookup and rendering/state transitions','native common-parameter getter and optional mode query supplied by fixture','recipient authority predicate forced false; native dispatch branch retained','native rescue statistics and network sending suppressed','recovery visual application recorded without a renderer'],
 sourceHashes={name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest()for name in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2))
print(f'PASS: Ambulance compiled native hooks, outgoing rescue before authority, native arithmetic/packet/stamina code; {cases} HKS guard cases; no live game process')
