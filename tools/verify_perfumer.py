"""Read installed aromatics and execute compiled switching and native lifetimes."""
import hashlib,json,os,subprocess
from local_paths import ROOT,GAME
work=ROOT/'work/perfumer';work.mkdir(parents=True,exist_ok=True);env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
def run(command,log,expected):
 r=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');output=r.stdout+r.stderr;(work/log).write_text(output)
 assert r.returncode==0 and expected in output,output
run(['dotnet','run','--project',str(ROOT/'tools/HexBuild'),'--',str(ROOT),'--perfumer-check'],'installed-check.log','PASS: installed Perfumer')
run(['python',str(ROOT/'tools/verify_perfumer_hks.py')],'hks-test.log','PASS: Perfumer 324 HKS cases')
command=['cargo','test','--release','--features','cage-demand']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','native_perfumer_native_use_and_target_summary','--','--ignored','--test-threads=1']
run(command,'native-test.log','1 passed; 0 failed;')
sources=['src/perfumer.rs','src/perfumer_native_tests.rs','src/lib.rs','src/ambulance.rs','src/hex_pool.rs','src/reward_cycle_runtime.rs','tools/HexBuild/Perfumer.cs','tools/HexBuild/RewardCatalog.cs','tools/build_perfumer_hks.py','tools/verify_perfumer_hks.py','tools/verify_perfumer.py']
report=dict(installedParameterReadbackPassed=True,compiledRuntimeAndNativeQueriesPassed=True,intervalSeconds=30,owner=95004100,weights=[1,1,1],onlyIronjarCannotRepeat=True,nativeBuffsAndDurationsPreserved=True,nativeUpliftingAreaPreserved=True,hksCases=324,
 cases=['equal conditional draws, excluding only Ironjar after itself','all-hero/affinity/party reward eligibility','30-second opportunity and half-second target-free grace','native602900/332AF0 query over all32 threat classes','unrecognized/alert/combat target state blocks use','missing target manager blocks use','native action frame successful use commits opportunity exactly once','rejected, interrupted and failed uses keep the opportunity','clip exit restores pending-good context','accepted manual item use cancels automatic context first','read-only virtual inventory presence only at6D06B9 under local scoped validation','real1F12B0 negative-index guard skips inventory mutation','native original good references and only isConsume differs','no custom short-duration buffs or area bullet replacements'],
 stubs=['native item permission and successful-use service callbacks; full native goods execution and network transport not simulated','actor/effect storage, input/behavior graph variables and clip transitions'],
 sources={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS: Perfumer compiled native-use hooks, original targeting query and inventory guard; gameplay and multiplayer untested')
