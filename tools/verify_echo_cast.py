"""Execute compiled echo hooks with real native dispatch and reference helpers."""
import hashlib,json,os,subprocess
from pathlib import Path
from local_paths import ROOT,GAME
subprocess.run(['python',str(ROOT/'tools/build_echo_cast.py'),'--check'],check=True)
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
command=['cargo','test','--release']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','echo_compiled_hooks_replay_native_dispatch_and_release_references','--','--ignored','--test-threads=1']
r=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');log=r.stdout+r.stderr
w=ROOT/'work/echo-casting';w.mkdir(parents=True,exist_ok=True);(w/'native-test.log').write_text(log)
assert r.returncode==0 and '1 passed; 0 failed;'in log,log
sources=['src/echo_cast.rs','src/echo_cast_native_tests.rs','src/echo_cast_data.rs','src/lib.rs','src/reward_choices.rs','src/hex_pool.rs','tools/build_echo_cast.py','data/reference/combat/echo-cast.json','tools/HexBuild/EchoCast.cs']
report=dict(nativeExecutionPassed=True,delaySeconds=.5,spellFpRate=2.25,eligibleRoles=[4,6,7],eligibleSpells=50,eligibleRoots=79,
 cases=['compiled emit/factory detours with real native BulletMan root dispatch','original pose/magic version survive source-buffer reuse and caster movement','native resolved target retained across delay; vanished target cancels without reacquisition','zero dt and 0.49-second hold followed by 0.5-second replay','one capture per actual root; nested creation and replay cannot recurse','native target copy/refcount atomics and descriptor release balance flagged counters','failed creation, owner loss, and changed expedition context discard retained references','native network enqueue branch retained; peer transport is not simulated','skills and unsupported root IDs do not schedule echoes'],
 nativeFunctions=['35A460 root dispatch','3482E0 target copy','3482B0 target release','214EAF0 / 214EB90 reference atomics','33E650 descriptor release','3494C0 target validity'],
 stubs=['actor/effect lookup and target resolution','33D010 projectile calculation and 35DA70 allocation','network packet serialization/copy and transport consumers','allocator/string/rendering context teardown'],
 sourceHashes={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(w/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS: Echo Casting compiled hooks, native dispatch/network enqueue and target/reference lifecycle; actual gameplay/multiplayer untested')
