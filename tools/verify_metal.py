"""Read back Metallization assets and execute the native item deduction path."""
import os,json,hashlib,subprocess
from pathlib import Path
from local_paths import ROOT,GAME
w=ROOT/'work/metal';w.mkdir(exist_ok=True)
subprocess.run(['dotnet','run','--project',str(ROOT/'tools/HexBuild'),'--',str(ROOT),'--metal-check'],check=True)
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'));fixture=w/'installed-goods.param';env['NRHEX_METAL_FIXTURE']=str(fixture)
if os.name!='nt'and env.get('NRHEX_CARGO_TARGET','').startswith('x86_64-pc-windows'):env['NRHEX_METAL_FIXTURE']='Z:'+str(fixture).replace('/',chr(92))
cmd=['cargo','test','--release','--features','cage-demand']
if env.get('NRHEX_CARGO_TARGET'):cmd+=['--target',env['NRHEX_CARGO_TARGET']]
cmd+=['--lib','native_metal_consumption_scope_and_gift_transactions','--','--ignored','--test-threads=1','--nocapture']
r=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');log=r.stdout+r.stderr;(w/'native-test.log').write_text(log);assert r.returncode==0 and '1 passed; 0 failed;'in log,log
sources=['src/metal.rs','src/metal_native_tests.rs','src/wraith.rs','src/lib.rs','src/hex_pool.rs','src/reward_cycle_runtime.rs','src/potential_fission_native_tests.rs','tools/HexBuild/Metal.cs','tools/HexBuild/HexCatalog.cs','tools/HexBuild/RewardCatalog.cs','tools/HexBuild/Program.cs','tools/verify_metal.py']
report=dict(version='0.17.251',owner=95004600,nativeGood=3520,nativeExecutionPassed=True,installedParameterAndEventReadbackPassed=True,originalIronjarAndEffectsUnchanged=True,sharedRowNeverWritten=True,
 nativeFunctions=['TAE65 dispatch table and original successful-use vtable slot point to669AE0','compiled669AE0 hook scopes sharedE8D840 query','original1F12B0 isConsume branch and deduction call','original1F5CA0 quantity reads from native inventory entries'],
 cases=['only local owner Ironjar consumption is exempt; other two perfumes deduct normally','owner absent, inactive, reset and remote actors retain consumption','ordinary out-of-scope row query returns original pointer; all original bytes preserved','nested TLS scope restores its parent','gift waits for three non-item samples; existing Wraith HKS observer shared','new serial or native stack growth acknowledges delivery; existing copies do not','no blind retry of consumed grant requests; pending delivery survives actor reconstruction','full item slots or native stack drop one ordinary Ironjar; failed spawn can retry','unclaimed gift or dropped gift never automatically repeats; saved receipt survives reconstruction'],
 stubs=['669AE0 non-inventory side effects are a recording callback; actual1F12B0 quantity predicate/branch executes','inventory lookup, count read and decrement target, native event AwardItemLot execution and network transport are fixtures','full ground descriptor/copy/linking covered by potential-fission native suite'],
 sources={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameplayVerified=False,multiplayerVerified=False)
(w/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n');print('PASS: Metal native quantity branch, compiled local scope and gift transactions; no game process')
