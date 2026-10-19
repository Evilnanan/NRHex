"""Run native on-hit selection and AttackRate multiplication against installed rows."""
import hashlib,json,os,subprocess
from local_paths import ROOT,GAME
work=ROOT/'work/momentum';work.mkdir(parents=True,exist_ok=True)
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
r=subprocess.run(['dotnet','run','--project',str(ROOT/'tools/HexBuild'),'--',str(ROOT),'--momentum-check'],cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
(work/'installed-check.log').write_text(r.stdout+r.stderr);assert r.returncode==0,r.stdout+r.stderr
fixture=work/'installed-effects.param';assert fixture.exists()
env['NRHEX_MOMENTUM_FIXTURES']=str(fixture)
if os.name!='nt' and env.get('NRHEX_CARGO_TARGET','').startswith('x86_64-pc-windows'):env['NRHEX_MOMENTUM_FIXTURES']='Z:'+str(fixture).replace('/',chr(92))
command=['cargo','test','--release']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','native_momentum_first_hit_and_unrefreshable_cycle_use_original_attack_rates','--','--ignored','--test-threads=1']
r=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');output=r.stdout+r.stderr;(work/'native-test.log').write_text(output)
assert r.returncode==0 and '1 passed; 0 failed;'in output,output
sources=['src/momentum.rs','src/momentum_native_tests.rs','src/lib.rs','src/hex_pool.rs','src/reward_cycle_runtime.rs','tools/HexBuild/Momentum.cs','tools/verify_momentum.py']
report=dict(nativeExecutionPassed=True,installedParametersReadbackPassed=True,bonus=1.3,weakAttack=0.5,weakIncoming=1.5,fullHpAttackReference=320500,attackVfxReference=320500,weakVfxReference=8800271,
 nativeFunctions=['505E90 successful hit selector','50F230 hit descriptor construction','511010 actual applicability switch through compiled shared hook','EA0790 element eligibility including pure magic/fire/lightning/holy','510750 physical AttackRate getter','503137..50314C native AttackRate multiplication'],
 cases=['first hit has bonus before hit notification','native full-HP bonus uses same multiplier field','roll/body contacts excluded','weapons and all three spell kinds','multi-target notifications cannot refresh timer','five positive then five weak seconds','native net attack multiplier .5 in weak phase','all four physical and four elemental cut fields 1.5','cycle does not repeat until a new hit','phase visuals switch and clear','non-local/missing owner/incapacitated notifications ignored','explicit reset clears phase state'],
 stubs=['native effect storage, application/removal and replication','native extra weapon/category restrictions made unconditional','stack cookie check no-op','damage applied and rendered in game not exercised'],
 sourceHashes={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},fixtureSha256=hashlib.sha256(fixture.read_bytes()).hexdigest(),gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS: Momentum native first-hit AttackRate, 5+5 cycle, all-element on-hit selection and installed VFX; no game process')
