"""Installed Heartstealer charged-hit, native charm emission and cooldown checks."""
import hashlib,json,os,subprocess
from local_paths import ROOT,GAME
work=ROOT/'work/heartstealer-rework';work.mkdir(parents=True,exist_ok=True)
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
r=subprocess.run(['dotnet','run','--project',str(ROOT/'tools/HexBuild'),'--',str(ROOT),'--heart-mist-check'],cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
(work/'installed-check.log').write_text(r.stdout+r.stderr);assert r.returncode==0,r.stdout+r.stderr
fixture=work/'installed-effects.param';assert fixture.exists();env['NRHEX_HEARTSTEALER_FIXTURES']=str(fixture)
if os.name!='nt' and env.get('NRHEX_CARGO_TARGET','').startswith('x86_64-pc-windows'):env['NRHEX_HEARTSTEALER_FIXTURES']='Z:'+str(fixture).replace('/',chr(92))
command=['cargo','test','--release','--features','cage-demand']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','native_heartstealer_charge_selector_and_mist_emit_use_original_chain','--','--ignored','--test-threads=1']
r=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');output=r.stdout+r.stderr;(work/'native-test.log').write_text(output)
assert r.returncode==0 and '1 passed; 0 failed;' in output,output
sources=['src/heartstealer.rs','src/heartstealer_native_tests.rs','src/lib.rs','src/reward_cycle_runtime.rs','tools/HexBuild/HeartMist.cs','tools/HexBuild/HeartReady.cs','tools/HexBuild/ContractHeart.cs','tools/verify_heartstealer.py']
report=dict(installedParametersAndEventsVerified=True,nativeExecutionPassed=True,readinessCases=['ready lease starts when eligible','successful claim clears lease immediately','deferred cooldown still blocks readiness','saved cooldown survives empty local reservation','dead/down/reviving/unowned actors hide readiness','new expedition clears lease and reset marker prevents reapplication'],chargedAttackCategory=100,cooldownSeconds=30,nativeMistBehavior=863030000,nativeMistBullets=[12335000,12335001],nativeCharmEffect=503350,
 cases=['actual 505E90 charged-hit selector and native applicability/category filters','43C9DC attacker notification delivery and missing-flag negative control','compiled DLL notification dispatch','ordinary hits and charged spells excluded by native weapon rules','multitarget/reentrant notifications and deferred effect storage','fixed 30-second game-time cooldown, invalid deltas rejected','restored native cooldown respected','non-local/unowned/down/dead/reviving actors ignored','failed effect submission does not spend cooldown','actual 50AA20 and 396AE0 dispatch native behavior at dummy80 for first-frame nodes only','ordinary, expired and removal-flagged nodes do not emit','new expedition clears cooldown and emission','retired branch grant/return events removed and native item bullets restored'],
 stubs=['actor/world and effect storage/application','owner-reference and emission-suppression lookups','final 397570 projectile dispatch recorded instead of rendering or network replication','stack-cookie check and notification gate exits'],
 fixtureSha256=hashlib.sha256(fixture.read_bytes()).hexdigest(),sourceHashes={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS: Heartstealer native charge selector/delivery, charm emitter, 30-second cooldown and retired branch events; no game process')
