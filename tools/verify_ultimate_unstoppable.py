"""Read installed resources and run compiled successful-ultimate/native Endure paths."""
import hashlib,json,os,subprocess
from local_paths import ROOT,GAME
work=ROOT/'work/ultimate-unstoppable';work.mkdir(parents=True,exist_ok=True);env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
def run(command,log,expected):
 r=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');output=r.stdout+r.stderr;(work/log).write_text(output)
 assert r.returncode==0 and expected in output,output
run(['dotnet','run','--project',str(ROOT/'tools/HexBuild'),'--',str(ROOT),'--ultimate-unstoppable-check'],'installed-check.log','PASS: installed Ultimate Unstoppable')
fixture=work/'installed-effects.param';assert fixture.exists();env['NRHEX_UNSTOPPABLE_FIXTURES']=str(fixture)
if os.name!='nt' and env.get('NRHEX_CARGO_TARGET','').startswith('x86_64-pc-windows'):env['NRHEX_UNSTOPPABLE_FIXTURES']='Z:'+str(fixture).replace('/',chr(92))
command=['cargo','test','--release']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','native_ultimate_unstoppable_endure_reaction_and_private_replacement','--','--ignored','--test-threads=1']
run(command,'native-test.log','1 passed; 0 failed;')
sources=['src/ultimate_unstoppable.rs','src/ultimate_unstoppable_native_tests.rs','src/lib.rs','src/hex_pool.rs','tools/HexBuild/UltimateUnstoppable.cs','tools/HexBuild/RewardCatalog.cs','tools/verify_ultimate_unstoppable.py']
report=dict(installedParameterReadbackPassed=True,nativeExecutionPassed=True,seconds=15,trigger=6999615,owner=95003900,buff=95003901,nativeSource=1650,nativeVfx=8570,nativeSfx=440111,
 nativeFunctions=['50D060 native collision selection before allocation','510B20 same-ID candidate','510BD0 category conflicts','507C40 and EA0CF0 original Endure reaction mapping','504E80 remove by effect ID','50B5B0 native doubly-linked list removal','511700 native duration initialization'],
 cases=['compiled shared successful-ultimate hook','other/rejected notifications do not activate','nested self-application does not loop','native category1001 reaction mapping matches Endure for 13 levels and two flags','category20 negative control loses stagger resistance','eight native damage multipliers restored to one','same-ID category1001 collision found before replacement','only private buff removed before reapplication','refresh lifetime to 15 rather than add time','vanilla Endure and Sword IDs and durations preserved','one private instance after repeated ultimates','expired but not yet removed node does not block the next activation','Ultimate Awakening remains active at its own ten-second duration','ownerless/reset/down/dead/reviving/remote/inactive/missing-stats calls ignored','failed application does not create a permanent reservation','all-hero/affinity/party choice eligibility'],
 stubs=['native accepted application result and row lookup','effect allocation/storage; native add routine returns at collision decision before allocator','native removal executes list splice but skips allocator destruction/category bookkeeping','elapsed game-time update and network replication not executed'],
 sources={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS: Ultimate Unstoppable compiled ultimate routing, native Endure reactions, private replacement and coexistence; no game process')
