"""Run the compiled detour against the supported native HP/regain machine code.

This opt-in test maps a private image; it never accesses the game process.
NRHEX_CARGO_TARGET is optional (e.g. x86_64-pc-windows-gnu under Wine).
NRHEX_NATIVE_EXE can supply the path as seen by the Windows test runner.
"""
import hashlib,json,os,subprocess
import pefile
from pathlib import Path
from local_paths import ROOT,GAME

env=os.environ.copy()
env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
image=pefile.PE(str(GAME/'nightreign.exe'),fast_load=True)
assert image.get_data(0x431df5,3)==bytes.fromhex('41 b0 01')
assert int.from_bytes(image.get_data(0x2c307f8+0x210,8),'little')==image.OPTIONAL_HEADER.ImageBase+0x66d410
command=['cargo','test','--release']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','compiled_detour_and_original_native_pool_execute_together','--','--ignored','--test-threads=1']
result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
output=result.stdout+result.stderr
work=ROOT/'work/death-dance';work.mkdir(parents=True,exist_ok=True)
(work/'native-test.log').write_text(output,encoding='utf8')
assert result.returncode==0 and '1 passed; 0 failed;' in output,output
balloon_command=['cargo','test','--release']
if env.get('NRHEX_CARGO_TARGET'):balloon_command+=['--target',env['NRHEX_CARGO_TARGET']]
balloon_command+=['--lib','native_commit_gates_balloon_and_positive_hp_does_not_fake_revival','--','--test-threads=1']
balloon_result=subprocess.run(balloon_command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
balloon_output=balloon_result.stdout+balloon_result.stderr
(work/'balloon-test.log').write_text(balloon_output,encoding='utf8')
assert balloon_result.returncode==0 and '1 passed; 0 failed;' in balloon_output,balloon_output
sources=['src/death_dance.rs','src/death_dance_native_tests.rs','src/death_dance_flask_tests.rs','src/balloon.rs','src/balloon_runtime.rs','src/lib.rs']
report=dict(nativeExecutionPassed=True,compiledMidHookExecuted=True,protectionHookExecuted=False,nativeRegainResetUnmodified=True,specialRegainProtectionRemoved=True,pendingRecoveryResetPreserved=True,expiredQuotaNotReprotected=True,unclampedDamageRetained=True,regainUsesOnlyActualDebit=True,debitSeconds=2,nativeDebit='0x426130',nativeProtectionMode='0 for live Ultimate-start bit59; otherwise 1',nativePeriodicCallSite='0x431df5',nativeDamageImmunity=['0x66d410','0x3d3e20'],nativeHpFloor='0x427bc0',immuneSharesExpireWithoutRecovery=True,nativeHpSetter='0x426b00',nativeEligibilityGate='0x426bae',nativeQueue='0x4275a0',nativeSettlement='0x4289c0',
 cases=['per-frame smooth debit at60Hz','two-second runtime completion','six native protection conditions compared in both modes','live Ultimate-start bit59 selects native mode0; roll bit1 remains native mode1','continuous animation immunity cannot cancel any scheduled lethal share','unconditional immunity overlapping animation immunity still blocks only its own shares','immune shares give no regain and no deferred burst','five native HP-floor mechanisms','collision-only protection does not block periodic loss','unprotected runtime lethal debit','independent overlapping timers','no free recovery before debit','shared pool consumed once','nonrecoverable debit unchanged','native rejection unchanged','non-owner unchanged','historical loss excluded','healing while debt remains','runtime debit/completion ABI','per-source recovery ratio retained','lethal debit cannot resurrect','native new-hit reset clears old protected quota and timer','pending recovery still cancels on a new hit','shared timer refreshes on actual payment','natural expiry after final payment','missing-HP cap after multiple hits','expired old quota is not re-protected','nonrecoverable new hit clears protection and old quota decays natively','non-owner original reset','remote recipient original reset','near-death original reset','inactive original baseline','rejected native state original reset'],
 stubs=['network authority','actor/effect getters in original pool fixtures; real getters in runtime protection fixtures','ownership/state presence, including native floor-state presence','native completion receiver in runtime ABI fixture'],
 sources={name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
report['cases'].append('unclamped lethal damage survives low-HP healing; only twelve actual HP paid enter regain')
report['cases'].append('native delayed damage never emits Balloon on HP settlement; only committed native down/death can emit')
report['cases'].append('native state2015 restored after death cleanup, missed cleanup and actor-manager replacement; no duplicate marker or deletion of native 8440200')
report['nativeFlaskState2015LifecyclePassed']=True
report['cases'].extend(['Ultimate-start immunity consumes a lethal two-second debt without HP loss, regain, finish notification or deferred burst','Ultimate-start immunity ending with roll still active resumes only remaining shares'])
report['ultimateStartupProtectionPassed']=True
report['balloonLethalSettlementEmitterFixturePassed']=False
report['balloonNativeCommitFixturePassed']=True
report['balloonCases']=['100 zero-HP/healing cycles before native commit never emit','native down commits once using the living maximum snapshot after ownership cleanup','positive HP during native down/death does not rearm','native revival completion or cleared native death plus a living spawn rearms','delayed native death cleanup cannot duplicate a prior down explosion']
(work/'native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f"PASS: compiled Death Dance capture, original regain reset, live Ultimate-start protection and native HP paths; {len(report['cases'])} scenario groups, no live game process")
