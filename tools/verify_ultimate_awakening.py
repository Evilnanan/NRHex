"""Read installed resources and run compiled successful-ultimate/native FP paths."""
import hashlib,json,os,subprocess
from local_paths import ROOT,GAME
work=ROOT/'work/ultimate-awakening';work.mkdir(parents=True,exist_ok=True);env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
def run(command,log,expected):
 r=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');output=r.stdout+r.stderr;(work/log).write_text(output)
 assert r.returncode==0 and expected in output,output
run(['dotnet','run','--project',str(ROOT/'tools/HexBuild'),'--',str(ROOT),'--ultimate-awakening-check'],'installed-check.log','PASS: installed Ultimate Awakening')
fixture=work/'installed-effects.param';assert fixture.exists();env['NRHEX_AWAKENING_FIXTURES']=str(fixture)
if os.name!='nt' and env.get('NRHEX_CARGO_TARGET','').startswith('x86_64-pc-windows'):env['NRHEX_AWAKENING_FIXTURES']='Z:'+str(fixture).replace('/',chr(92))
command=['cargo','test','--release','--features','cage-demand']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','native_ultimate_awakening_success_refresh_and_fp_aggregation','--','--ignored','--test-threads=1']
run(command,'native-test.log','1 passed; 0 failed;')
sources=['src/ultimate_awakening.rs','src/ultimate_awakening_native_tests.rs','src/lib.rs','src/hex_pool.rs','tools/HexBuild/UltimateAwakening.cs','tools/HexBuild/RewardCatalog.cs','tools/verify_ultimate_awakening.py']
report=dict(installedParameterReadbackPassed=True,nativeExecutionPassed=True,seconds=10,trigger=95000404,owner=95003800,buff=95003801,nativeSource=511060,nativeVfx=5860,nativeSfx=330111,
 nativeFunctions=['510B20 same-ID refresh eligibility','511450 category20 ID isolation','511700 original duration refresh','5061B0 arts FP multiplier','507310 magic FP multiplier','5081E0 shaman FP multiplier','507410 miracle FP multiplier'],
 cases=['accepted ultimate notification via compiled shared effect hook','rejected/other notifications do not activate','nested self-application does not loop','refresh remaining duration to 10, never add time','installed zero multipliers override Echo Casting x2','inactive native node restores normal FP rates','native tear coexists and its remaining duration stays unchanged','ownerless, reset, down, dead, reviving, remote and inactive calls rejected','missing stats and failed effect submission','all-hero/affinity/party random/focused/reroll eligibility'],
 stubs=['native effect submission and accepted application result','effect-row lookup called by native lifetime reset','actor/effect storage and expiry flags; full update loop not simulated'],
 sources={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS: Ultimate Awakening compiled successful-ultimate routing, original refresh and FP aggregation; no game process')
