"""Execute the compiled hook and retained native ailment code in a private image."""
import hashlib,json,os,subprocess
from pathlib import Path
from local_paths import ROOT,GAME

subprocess.run(['dotnet','run','--project',str(ROOT/'tools/HexBuild'),'--',str(ROOT),'--erosion-items-check'],check=True)
env=os.environ.copy()
env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
command=['cargo','test','--release','--features','cage-demand']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','compiled_erosion_and_native_ailment_paths_execute_together','--','--ignored','--test-threads=1']
result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
output=result.stdout+result.stderr
work=ROOT/'work/erosion';work.mkdir(parents=True,exist_ok=True)
(work/'native-test.log').write_text(output,encoding='utf8')
assert result.returncode==0 and '1 passed; 0 failed;' in output,output
sources=['src/erosion.rs','src/erosion_native_tests.rs','src/lib.rs']
report=dict(nativeExecutionPassed=True,compiledHookExecuted=True,compiledSharedApplicabilityHookExecuted=True,compiledCollectorHookExecuted=True,
 weaponBuildup=30,spellBuildup=10,itemBuildup=10,passiveBuildup=5,nativeZeroCorrectionPreserved=True,
 spellRequiresSword=True,bodyContactsExcluded=True,nativeBulletIdCopyExecuted=True,
 nativeImmunityChain=['0x509790','0x510fd0','0xea19c0'],nativeAddedAilmentSelector='0x506200',nativeAttackApplicability='0x511010',
 nativeMotionValueFragment=['0x43c869','0x43c899'],
 cases=['all four carriers map to each of seven weakest ailments','only mapped local Sword spells collect the 10-point carrier; missing Sword/remote/unmapped/table mismatch excluded',
 'native applicability remains enforced and type10 remains excluded','other native ailment additions unchanged',
 'source goods take precedence; preexisting carrier vector entries unchanged','passive projectile IDs select 5-point carrier without leaking to weapon contacts',
 'unmodified native launch-payload copy verifies BulletIns+A4','two native motion values multiply; either zero stays zero',
 'body contact exclusion, active/immune statuses, current thresholds, ties and source ownership',
 'selected payloads are not remapped; all13 arguments andAL preserved'],
 stubs=['projectile collision continuation dispatches to native collector; no world/AI execution','security cookie check',
 'ownership and active-state lookup','native apply continuation records parameters; proc/damage/network not executed'],
 sources={name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f"PASS: compiled Erosion hook and original immunity/selection/correction instructions; {len(report['cases'])} scenario groups, no live game process")
