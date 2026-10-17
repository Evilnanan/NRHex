"""Execute the compiled hook and retained native ailment code in a private image."""
import hashlib,json,os,subprocess
from pathlib import Path
from local_paths import ROOT,GAME

env=os.environ.copy()
env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
command=['cargo','test','--release']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','compiled_erosion_and_native_ailment_paths_execute_together','--','--ignored','--test-threads=1']
result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
output=result.stdout+result.stderr
work=ROOT/'work/erosion';work.mkdir(parents=True,exist_ok=True)
(work/'native-test.log').write_text(output,encoding='utf8')
assert result.returncode==0 and '1 passed; 0 failed;' in output,output
sources=['src/erosion.rs','src/erosion_native_tests.rs','src/lib.rs']
report=dict(nativeExecutionPassed=True,compiledHookExecuted=True,
 nativeImmunityChain=['0x509790','0x510fd0','0xea19c0'],nativeAddedAilmentSelector='0x506200',nativeAttackApplicability='0x511010',nativeMotionValueFragment=['0x43c869','0x43c899'],
 cases=['all seven native immunity flags','invalid immunity effects ignored','stateInfo152 accepted and2103 excluded','native spell applicability flags','two motion values multiplied','zero status correction remains zero','generic buildup multiplier retained','full threshold rather than remaining gauge','escalating threshold changes selection','progress tie and stable exact tie','immune ailment excluded','death participates normally','all immune does nothing','active ailment excluded','zero and NaN buildup rejected','dead recipient rejected','non-owner rejected','selected payload IDs not remapped again','inactive hook forwards unchanged','all13 arguments andAL result retained'],
 stubs=['security cookie check','ownership and active-state lookup','native apply continuation records parameters; proc, damage and networking are not executed'],
 sources={name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f"PASS: compiled Erosion hook and original immunity/selection/correction instructions; {len(report['cases'])} scenario groups, no live game process")
