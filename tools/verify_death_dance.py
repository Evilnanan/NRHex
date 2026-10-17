"""Run the compiled detour against the supported native HP/regain machine code.

This opt-in test maps a private image; it never accesses the game process.
NRHEX_CARGO_TARGET is optional (e.g. x86_64-pc-windows-gnu under Wine).
NRHEX_NATIVE_EXE can supply the path as seen by the Windows test runner.
"""
import hashlib,json,os,subprocess
from pathlib import Path
from local_paths import ROOT,GAME

env=os.environ.copy()
env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
command=['cargo','test','--release']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','compiled_detour_and_original_native_pool_execute_together','--','--ignored','--test-threads=1']
result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
output=result.stdout+result.stderr
work=ROOT/'work/death-dance';work.mkdir(parents=True,exist_ok=True)
(work/'native-test.log').write_text(output,encoding='utf8')
assert result.returncode==0 and '1 passed; 0 failed;' in output,output
sources=['src/death_dance.rs','src/death_dance_native_tests.rs','src/lib.rs']
report=dict(nativeExecutionPassed=True,compiledMidHookExecuted=True,protectionHookExecuted=True,nativeProtectionGate='0x426b79',protectedQuotaRetained=True,pendingRecoveryResetPreserved=True,expiredQuotaNotReprotected=True,nativeHpSetter='0x426b00',nativeEligibilityGate='0x426bae',nativeQueue='0x4275a0',nativeSettlement='0x4289c0',
 cases=['independent overlapping timers','no free recovery before debit','shared pool consumed once','nonrecoverable debit unchanged','native rejection unchanged','non-owner unchanged','historical loss excluded','healing while debt remains','runtime debit/completion ABI','per-source recovery ratio retained','lethal debit cannot resurrect','new-hit reset retains old protection','pending recovery still cancels on a new hit','shared timer refreshes on actual payment','natural expiry after final payment','missing-HP cap after multiple hits','expired old quota is not re-protected','nonrecoverable hit preserves old quota without renewal or new quota','non-owner original reset','remote recipient original reset','near-death original reset','inactive original baseline','rejected native state original reset'],
 stubs=['network authority','actor and effect lookup','ownership/state presence','native completion receiver in runtime ABI fixture'],
 sources={name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f"PASS: both compiled Death Dance hooks and original native HP/regain paths; {len(report['cases'])} scenario groups, no live game process")
