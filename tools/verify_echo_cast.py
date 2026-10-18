"""Execute compiled echo hooks with real native dispatch and reference helpers."""
import hashlib,json,os,subprocess
from pathlib import Path
from local_paths import ROOT,GAME
w=ROOT/'work/echo-casting';w.mkdir(parents=True,exist_ok=True)
subprocess.run(['python',str(ROOT/'tools/build_echo_cast.py'),'--check','--report',str(w/'catalog.json')],check=True)
catalog=json.loads((w/'catalog.json').read_text())
subprocess.run(['dotnet','run','--project',str(ROOT/'tools/HexBuild'),'--',str(ROOT),'--echo-cast-areas-check'],check=True)
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
command=['cargo','test','--release']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','echo_compiled_hooks_replay_native_dispatch_and_release_references','--','--ignored','--test-threads=1']
r=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');log=r.stdout+r.stderr
(w/'native-test.log').write_text(log)
assert r.returncode==0 and '1 passed; 0 failed;'in log,log
sources=['src/echo_cast.rs','src/echo_cast_native_tests.rs','src/echo_cast_data.rs','src/lib.rs','src/reward_choices.rs','src/hex_pool.rs','tools/build_echo_cast.py','tools/verify_echo_cast.py','data/reference/combat/echo-cast.json','data/reference/combat/sword-graph.json','data/balance/echo-cast-areas.json','tools/HexBuild/EchoCast.cs']
report=dict(nativeExecutionPassed=True,delaySeconds=.5,spellFpRate=2.25,eligibleRoles=[4,6,7],eligibleSpells=catalog['eligibleSpells'],eligibleRoots=catalog['eligibleRoots'],groupEffects=catalog['groupEffects'],catalog=catalog,groupReadback=json.loads((w/'areas/installed.json').read_text()),
 cases=['compiled emit/factory detours with real native BulletMan root dispatch','original resolved pose and aim restored before allocation despite recalculation selecting a changed pose/lock','entity targets retained; vanished entities cancel without reacquiring another enemy','forward-only sentinel, fixed aim point, and point at local origin replay without an entity-validity requirement','recalculation changing the number of independent targets cancels rather than selecting the first one','zero dt and 0.49-second hold followed by 0.5-second replay','all normal/charged/no-cost side roots for Glintstone Stars, Star Shower, Crystal Barrage, Stars of Ruin, Shattering Crystal, Crystal Release, Beast Claw, and Gurranq Beast Claw replay once each','child and parented emissions cannot reschedule a full volley; replay is not recursive','native target copy/refcount atomics and descriptor release balance flagged counters, including replacing a newly selected enemy with original untargeted aim','failed creation, owner loss, and changed expedition context discard retained references','native network enqueue branch retained; peer transport is not simulated','all 11 group effects replay dedicated area IDs at the captured position with empty/multiple original targets and a dead enemy lock','group replays retain no enemy target reference; source/weapon context references balance','personal heal, control bullets, held channels, and generated area IDs do not schedule echoes','installed group parameter readback retains native ally amounts, modifiers, radii and category rules; enables self and independent hit records'],
 nativeFunctions=['35A460 root dispatch','3482E0 target copy','3482B0 target release','214EAF0 / 214EB90 reference atomics','33E650 descriptor release','3494C0 target validity'],
 stubs=['actor/effect lookup and target resolution','33D010 projectile calculation including deliberate current-lock/pose changes; 35DA70 allocation and scatter/child production','network packet serialization/copy and transport consumers','fixture target-vector allocation/cleanup; allocator/string/rendering context teardown'],
 sourceHashes={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(w/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS: Echo Casting compiled hooks, native dispatch/network enqueue and target/reference lifecycle; actual gameplay/multiplayer untested')
