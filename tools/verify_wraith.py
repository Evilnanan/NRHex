"""Validate current Wraith assets and replay original native contact/slot code."""
import os,json,hashlib,subprocess
from pathlib import Path
from local_paths import ROOT,GAME
w=ROOT/'work/wraith';w.mkdir(exist_ok=True)
subprocess.run(['dotnet','run','--project',str(ROOT/'tools/HexBuild'),'--',str(ROOT),'--wraith-check'],check=True)
subprocess.run(['python',str(ROOT/'tools/build_wraith_hks.py'),'--check'],check=True)
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'));fixture=w/'installed-goods.param';env['NRHEX_WRAITH_FIXTURE']=str(fixture)
if os.name!='nt'and env.get('NRHEX_CARGO_TARGET','').startswith('x86_64-pc-windows'):env['NRHEX_WRAITH_FIXTURE']='Z:'+str(fixture).replace('/',chr(92))
cmd=['cargo','test','--release','--features','cage-demand']
if env.get('NRHEX_CARGO_TARGET'):cmd+=['--target',env['NRHEX_CARGO_TARGET']]
cmd+=['--lib','native_wraith_hit_credit_fp_scopes_and_deferred_slot_upgrade','--','--ignored','--test-threads=1']
r=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');log=r.stdout+r.stderr;(w/'native-test.log').write_text(log);assert r.returncode==0 and '1 passed; 0 failed;'in log,log
sources=['src/wraith.rs','src/wraith_native_tests.rs','src/lib.rs','src/echo_cast.rs','src/ambulance.rs','src/reward_cycle_runtime.rs','src/hex_pool.rs','tools/HexBuild/Wraith.cs','tools/HexBuild/Program.cs','tools/build_wraith_hks.py','tools/verify_wraith.py','docs/hexes.md','docs/hexes.en.md']
report=dict(owner=95004300,nativeExecutionPassed=True,installedReadbackPassed=True,stages=[3080,3010,3011],effectiveUsesPerStage=[20,10],freeFpOnlyForLocalOwner=True,nativeFunctions=['66F040 pending-token registration (existing-record branch)','66D3B0 token lookup','669A20 consume token','34E0B0 original XP call and compiled hook','1F7840 quickslot setter'],cases=['root registration preserves native source identity and rejects parent/foreign emissions','misses do not progress; first-contact token consumed once','Scholar retains native category XP; other heroes receive only independent progress','FP eligibility/use scopes and original parameter immutability','continuous actions defer; three empty frame samples; original slot and selection retained','late lower-tier flights cannot progress a higher tier'],stubs=['native item inventory grant/removal command execution and network transport not executed','source/effect lookup and native FP legality consumer fixtures','pending-list allocator release and inventory lookup/UID lookup fixtures','native category XP body replaced by recording fixture'],sourceHashes={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameplayVerified=False,multiplayerVerified=False)
(w/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n');print('PASS: Wraith native first-contact credit, local FP scopes and deferred native quickslot replacement; no game process')
