"""Read installed Back to Basics assets and execute native cooldown/Restage code."""
import os,json,hashlib,subprocess
from pathlib import Path
from local_paths import ROOT,GAME
w=ROOT/'work/basics';w.mkdir(exist_ok=True)
subprocess.run(['dotnet','run','--project',str(ROOT/'tools/HexBuild'),'--',str(ROOT),'--basics-check'],check=True)
subprocess.run(['python',str(ROOT/'tools/build_basics_hks.py'),'--check'],check=True)
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'));fixture=w/'installed-effects.param';env['NRHEX_BASICS_FIXTURE']=str(fixture)
if os.name!='nt'and env.get('NRHEX_CARGO_TARGET','').startswith('x86_64-pc-windows'):env['NRHEX_BASICS_FIXTURE']='Z:'+str(fixture).replace('/',chr(92))
cmd=['cargo','test','--release','--features','cage-demand']
if env.get('NRHEX_CARGO_TARGET'):cmd+=['--target',env['NRHEX_CARGO_TARGET']]
cmd+=['--lib','native_basics_cooldown_product_seal_and_damage','--','--ignored','--test-threads=1']
r=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');log=r.stdout+r.stderr;(w/'native-test.log').write_text(log);assert r.returncode==0 and '1 passed; 0 failed;'in log,log
sources=['src/basics.rs','src/basics_native_tests.rs','src/lib.rs','src/ambulance.rs','src/reward_choices.rs','src/hex_pool.rs','tools/HexBuild/Basics.cs','tools/HexBuild/Program.cs','tools/build_basics_hks.py','tools/verify_basics.py','docs/hexes.md','docs/hexes.en.md']
report=dict(owner=95004400,nativeExecutionPassed=True,installedReadbackPassed=True,nativeFunctions=['508480 native cooldown product','449AC7 native Restage ratio, sum and truncation with compiled wrapper','519140 native family reference resolution through 4AC710/4AC770'],cases=['0.7/0.8/0.5 cooldown multipliers stack with native relic 0.9','local-only ground/air and two-button Ultimate gates; gauge unchanged','native Restage history multiplier retains 0.5/0.6 base and truncation','all ten hero damage scopes; ordinary attacks, healing, support and foreign actors unchanged','three resolved family actors with different reference/internal identity formats; +100% means x2','empty slots, invalid references and foreign actors with copied identity bytes do not match','only final HP damage changed; status, stamina and poise untouched','private native idle effects halve all three intervals; original and active rows preserved','acquisition upgrades existing idle effects once; 100 updates never restart healing ticks','native active transition retained; owner loss and expedition reset restore idle effects'],stubs=['native availability dispatcher result fixture','category-201 effect allocation/replacement fixture','ownership/effect lookups','actor collections and family slots are memory fixtures, not live actors','full collision, inventory awards, game transport and animation state machine not executed'],sourceHashes={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameplayVerified=False,multiplayerVerified=False)
(w/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n');print('PASS: Back to Basics native cooldown/Restage and compiled seals/role damage; no game process')
