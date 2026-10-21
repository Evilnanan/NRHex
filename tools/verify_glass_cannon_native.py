"""Run compiled Glass Cannon hooks over the original native damage calculator."""
import hashlib,json,os,subprocess
from local_paths import ROOT,GAME
work=ROOT/'work/glass-negative-resistance';work.mkdir(parents=True,exist_ok=True)
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
command=['cargo','test','--release','--features','cage-demand']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','native_glass_cannon_retains_net_negative_resistance','--','--ignored','--test-threads=1']
r=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');log=r.stdout+r.stderr;(work/'native-test.log').write_text(log)
assert r.returncode==0 and '1 passed; 0 failed;' in log,log
sources=['src/glass_cannon.rs','src/glass_cannon_native_tests.rs','tools/verify_glass_cannon_native.py','build.ps1']
report=dict(nativeExecutionPassed=True,nativeFunctions=['6C5FC0 native damage combiner with compiled inline hook','506700 native object-defense query with compiled recipient hook'],
 rule='combine model/equipment resistance and three native DamageCutRate groups per element, then retain only net coefficients above one',
 cases=['36 five-element cases: 30 owner/non-owner defensive-layer cases, two mixed repeated calculations, positive/neutral/negative intrinsic resistance and numerical immunity', 'physical, magic, fire, lightning and holy evaluated independently', 'model resistance cancels effect vulnerability before clamping', 'native attack rates, repeated-hit and positional multipliers preserved; guard/deflection/rebound/target-state bypass unchanged', '16 native object-defense flag/value cases with nested scopes and exact recipient matching', '24 shared Devil\u2019s Gaze scale/guard cases with native defenses intact', 'input arrays remain unchanged and repeated evaluations do not compound'],
 nativeCalculatorCases=36,objectDefenseCases=16,sharedGazeCases=24,dependencyStubs=[],scopeIdentityIsFixture=True,
 sourceHashes={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS: Glass Cannon compiled hooks and native net negative resistance, object defense, five elements and shared Gaze regression; no game process')
