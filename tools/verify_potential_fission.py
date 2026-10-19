"""Execute the compiled fission hooks with private native dispatch/copy/grouping."""
import hashlib,json,os,subprocess
from local_paths import ROOT,GAME
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
command=['cargo','test','--release']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','compiled_fission_routes_saved_charges_and_native_independent_groups','--','--ignored','--test-threads=1']
result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
output=result.stdout+result.stderr;work=ROOT/'work/potential-fission';work.mkdir(parents=True,exist_ok=True)
(work/'native-test.log').write_text(output,encoding='utf8')
assert result.returncode==0 and '1 passed; 0 failed;' in output,output
sources=['src/lib.rs','src/potential_fission.rs','src/potential_fission_native_tests.rs','src/fission_catalog.rs','src/hex_pool.rs','src/personal_reward.rs','src/reward_cycle_runtime.rs','src/snack.rs']
report=dict(nativeExecutionPassed=True,compiledHooksExecuted=True,
 nativeFunctions=['5759E0 reward dispatch','6A1BA0 network-bit predicate','570410 descriptor copy','57A840 choice grouping'],
 cases=['all three local recipient slots and offline slot zero','teammate rewards unchanged','pre-acquisition group stays original even if the hook missed its first creation','one charge per multi-option source','three charges only','two separate native choice chains','partial creation retries missing options only','caller buffers unchanged','saved counter restored','explicit expedition reset','negative control: repeated unscoped Snack emissions merge into one native choice chain','three immediate Snack emissions with identical source and position create three independent three-choice chains for every recipient slot','native linked-choice traversal never reaches a different Snack reward','unrelated reward emitted between Snack rewards stays separate in both directions','temporary grouping scope restored after each emission'],
 stubs=['allocation/rendering replaced by fixture invoking native descriptor copy','lottery emits deterministic descriptors; actual native lottery requires game parameter managers','character/effect state and UID lookup fixtures'],
 sources={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print('PASS: compiled fission hooks and original dispatch/copy/grouping; actual game and multiplayer remain untested')
