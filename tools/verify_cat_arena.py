"""Verify Cat arena HKS guards and execute compiled query/input/impulse routes."""
import hashlib,json,os,subprocess
from pathlib import Path
from hks_bytecode import Reader,instructions
from build_cat_arena_hks import check,QUERY,OWNER,TARGETS
from local_paths import ROOT,GAME
source=(ROOT/'dist/mod/action/script/c0000.hks').read_bytes();assert check(source)==5
reader=Reader(source);reader.function();cases=0
# Independently execute the existing eligibility guard's control flow. Only
# its query result changes; same parameters/locals/branches drive wall retries.
for fid,(name,count)in TARGETS.items():
 f=reader.functions[fid];words=instructions(f);sites=[]
 for i,x in enumerate(words[:-2]):
  if x['op']==25 and f['constants'][x['bx']]==QUERY and words[i+1]['op']==25 and f['constants'][words[i+1]['bx']]==OWNER:sites.append(i)
 for site in sites:
  start=site-2;jump=site+6;end=jump+words[jump]['bx']-0x10000+2;scratch=words[site]['a']-1
  for owned in [False,True]:
   for arena in [False,True]:
    for grounded in [False,True]:
     for accepted in [False,True]:
      calls=[];regs={i:1000+i for i in range(16)};saved=dict(regs)
      def env(*args):
       if args==(QUERY,OWNER):return int(owned and not arena)
       assert args==(201.,);return int(grounded)
      globals_={'env':env,'TRUE':1,'ISENABLE_WALLJUMP':0,'ExecWallJump':lambda:calls.append('wall') or int(accepted)}
      pc=start;returned=False
      for _ in range(80):
       if pc==end:break
       x=words[pc];pc+=1;op,a,b,c,bx=x['op'],x['a'],x['b'],x['c'],x['bx']
       if op==91:regs[a]=globals_[f['constants'][bx]]
       elif op==76:pass
       elif op==25:regs[a]=f['constants'][bx]
       elif op in [2,69]:
        v=regs[a](*(regs[i] for i in range(a+1,a+b)))
        if c==2:regs[a]=v
       elif op==4:
        if (regs[b]==regs[c])!=bool(a):pc+=1
       elif op==28:pc+=bx-0x10000+1
       elif op==27:globals_[f['constants'][bx]]=regs[a]
       elif op==9:returned=True;break
       else:raise AssertionError((name,pc-1,op))
      else:raise AssertionError('Guard loop')
      retry=('Cancel' in name)or(fid==1508 and site==sites[-1]);enabled=owned and not arena
      assert calls==(['wall']if retry and enabled and not grounded else [])
      assert returned==(retry and enabled and not grounded and accepted)
      assert globals_['ISENABLE_WALLJUMP']==int(enabled and not retry)
      assert all(regs[i]==saved[i]for i in range(scratch));cases+=1
# Verify all known Nightlord arena families, including DLC alternate phase.
alias_path=Path(os.environ.get('NRHEX_SMITHBOX',''))/'Assets/Aliases/NR/MapNames.json'
if alias_path.exists():
 aliases=json.loads(alias_path.read_text());arenas=[r['ID']for r in aliases if r['ID'].startswith(('m16_','m18_','m19_'))]
 assert {'m16_00_00_00','m16_10_00_00','m16_11_00_00','m18_00_00_00','m19_00_00_00'}<=set(arenas)
else:arenas=['m16_*','m18_*','m19_*']
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'));command=['cargo','test','--release']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','cat_arena_compiled_query_input_and_impulse_follow_native_current_map','--','--ignored','--test-threads=1']
r=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');output=r.stdout+r.stderr;work=ROOT/'work/cat-arena';work.mkdir(parents=True,exist_ok=True);(work/'native-test.log').write_text(output)
assert r.returncode==0 and '1 passed; 0 failed;'in output,output
sources=['src/cat_wall.rs','src/cat_arena_native_tests.rs','src/ambulance.rs','src/slingshot_runtime.rs','src/lib.rs','tools/build_walljump.py','tools/build_cat_arena_hks.py','tools/verify_cat_arena.py']
report=dict(hksQuerySites=5,hksGuardCases=cases,nativeExecutionPassed=True,nightlordMapFamilies=[16,18,19],knownArenaMaps=arenas,
 cases=['native source-map fragment 576D44 with actor-map and active-map fallback','live map changes and invalid/loading contexts','compiled shared HKS env hook and unchanged ordinary queries','raw-input retry disabled but accepted native input retained','compiled launch scope with real E737B0 speed getter','owner effect remains present in arenas','down/dead/reviving/non-local actors rejected','five serialized HKS guard paths retain live arguments and native branches'],
 stubs=['actor/effect storage','ordinary env query and native input return','launch wrapper calls compiled speed override over native getter'],
 hksSha256=hashlib.sha256(source).hexdigest(),sourceHashes={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n');print(f'PASS: Cat arena gate, {cases} HKS cases and compiled native map/query/input/impulse routes; no game process')
