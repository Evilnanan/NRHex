"""Run the compiled descriptor hook with original native dispatch/copy/item reads."""
import hashlib,json,os,subprocess,struct
import pefile
from local_paths import ROOT,GAME
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
command=['cargo','test','--release']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','compiled_runtime_choices_and_native_dispatch_copy_and_read_execute_together','--','--ignored','--test-threads=1']
result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
output=result.stdout+result.stderr;work=ROOT/'work/reward-scaling';work.mkdir(parents=True,exist_ok=True)
(work/'native-test.log').write_text(output,encoding='utf8')
assert result.returncode==0 and '1 passed; 0 failed;' in output,output
pe=pefile.PE(str(GAME/'nightreign.exe'),fast_load=True)
saved_capacity=struct.unpack('<ii',pe.get_data(0x2be6dc0+10*8,8));assert saved_capacity==(31,50)
sources=['src/lib.rs','src/hex_pool.rs','src/personal_reward.rs','src/reward_choices.rs','src/reward_cycle.rs','src/reward_cycle_runtime.rs','src/reward_reroll.rs','src/reward_recovery.rs','src/reward_template_native_tests.rs']
report=dict(nativeExecutionPassed=True,compiledHookExecuted=True,nativeSaveCategory10Capacity=saved_capacity[1],historyEncoding="eight stable registry bits per nonzero byte effect",nativeFunctions=['5759E0 dispatch','6A1BA0 network-bit predicate','570410 descriptor copy','570FC0 item read'],descriptorGoodOffset='8',descriptorChoiceOffset='A9',nodeGoodOffset='28',
 cases=['each of three local player slots','all other recipients suppressed','out-of-order choice descriptors','all three requested goods present before creation','native item lookup reads replaced goods','full native descriptor copy retains UID/group/position/owner/quality','temporary descriptor edits restored','repeat callback cannot duplicate a choice','wrong manager untouched','other lot untouched','native weapon untouched','absent TLS scope untouched','invalid ordinals rejected','unexpected item category rejected','partial creation counted without duplicate callback','zero creation can retry in a fresh scope'],
 stubs=['local allocation/grouping/rendering replaced with a fixture invoking real descriptor copy','UID lookup resolves fixture nodes'],sources={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f"PASS: compiled runtime-choice hook and native dispatch/copy/read paths; {len(report['cases'])} scenario groups; no live game process")
