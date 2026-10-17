"""Verify existing native action notifications and the Balloon-only team hooks."""
import hashlib,json,os,subprocess
from hks_bytecode import Reader,decode,struct
from local_paths import ROOT,GAME

work=ROOT/'work/balloon-trigger';work.mkdir(parents=True,exist_ok=True)
script=ROOT/'dist/mod/action/script/c0000.hks'
source=script.read_bytes();reader=Reader(source);reader.function()
callbacks=[]
for function in reader.functions:
 if function['name'] not in ['Death_Activate','NearDeath_Activate']:continue
 instructions=[decode(value[0]) for value in struct.iter_unpack('>I',function['code'])][:14]
 assert [value['op'] for value in instructions]==[91,76,25,25,2,91,76,4,28,91,76,25,25,2]
 constants=function['constants']
 assert [constants[value['bx']] for value in instructions if value['op']==25]==[1116.,6999900.,2002.,6999901.]
 assert [constants[value['bx']] for value in instructions if value['op']==91]==['env','TRUE','act']
 callbacks.append(function['name'])
assert len(callbacks)==2
environment=os.environ.copy();environment.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
command=['cargo','test','--release']
if environment.get('NRHEX_CARGO_TARGET'):command+=['--target',environment['NRHEX_CARGO_TARGET']]
command+=['--lib','compiled_balloon_hooks_execute_native_relation_masks_without_changing_the_actor','--','--ignored','--test-threads=1']
result=subprocess.run(command,cwd=ROOT,env=environment,capture_output=True,text=True,errors='replace')
output=result.stdout+result.stderr;(work/'native-team-test.log').write_text(output,encoding='utf8')
assert result.returncode==0 and '1 passed; 0 failed;' in output,output
sources=['Cargo.toml','Cargo.lock','src/lib.rs','src/balloon.rs','src/balloon_runtime.rs','src/balloon_team.rs','src/balloon_team/native_tests.rs','tools/verify_balloon_trigger.py']
report=dict(nativeExecutionPassed=True,nativeActionCallbacks=callbacks,hksSha256=hashlib.sha256(source).hexdigest(),hksUnmodified=True,hpZeroIsNotATrigger=True,healingBeforeNativeAcknowledgmentCannotRearm=True,nearDeathTeam=77,onlyPrivateBullet=95001750,onlyPrivateAttack=95001760,privatePcAttackTable=1,nativeHitDispatch='0x52cef0',nativeAttackRelation='0x52cb90',nativeBulletTeamGetter='0x34d1a0',nativeActorTeamGetter='0x3d1b30',nativeRelationshipMethods=['0x52c900','0x52c910','0x52c920'],cases=['native source remains team77 outside the private collision scope','Balloon bullet query recovers the unchanged base team2','native relation rejects a team77 source and accepts Balloon against teams6/7','native friendly and self masks still reject living allies and self','ordinary Golden Shockwave and non-PC attack remain suppressed while down','ordinary bullet retains team77','scoped source is restored after native dispatch; actor fields unchanged'],stubs=['actor handle resolution','effect-driven team override module lookup','outer native bullet collision delegates to the real native attack relation predicate; geometry, damage and networking not simulated'],sources={relative:hashlib.sha256((ROOT/relative).read_bytes()).hexdigest() for relative in sources},gameplayVerified=False,multiplayerVerified=False,deathSoundTimingVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print('PASS: native action notifications and compiled Balloon-only hooks execute the original native relationship masks; no gameplay claim')
