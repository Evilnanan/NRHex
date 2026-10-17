"""Verify the final DLL sources, isolated native paths and supported EXE guards."""
import hashlib,json,os,re,subprocess
from local_paths import ROOT,GAME

work=ROOT/'work/demons-call';work.mkdir(parents=True,exist_ok=True)
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
target=['--target',env['NRHEX_CARGO_TARGET']] if env.get('NRHEX_CARGO_TARGET') else []
def run(label,args):
 result=subprocess.run(args,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
 output=result.stdout+result.stderr;(work/(label+'.log')).write_text(output,encoding='utf8')
 assert result.returncode==0,output
 return output

run('release-build',['cargo','build','--release',*target])
unit=run('tests-release',['cargo','test','--release',*target,'--lib','--','--test-threads=1'])
native=run('native-release',['cargo','test','--release',*target,'--lib','--','--ignored','--test-threads=1'])
scan=run('signatures-release',['cargo','run','--release',*target,'--bin','scancheck','--',env['NRHEX_NATIVE_EXE']])
run('no-hotkeys-check',['cargo','check','--release',*target,'--no-default-features'])
assert '0 failed;' in unit and '0 failed;' in native
assert '7 passed;' in native and 'PASS: current EXE hash' in scan
sources=['Cargo.toml','Cargo.lock','src/lib.rs','src/actor_lifecycle.rs','src/balloon.rs','src/balloon_runtime.rs','src/balloon_team.rs','src/balloon_team/native_tests.rs','src/native_actor.rs','src/death_dance.rs','src/death_dance_native_tests.rs','src/reward_cycle_runtime.rs','src/spirit_runtime.rs','src/spirit_runtime/tests.rs','src/demons.rs','src/demons_data.rs','src/demons_params.rs','src/demons_params/native_tests.rs','src/demons_lottery.rs','src/demons_lottery/native_tests.rs','src/demons_runtime.rs','src/hex_pool.rs']
signature_count=int(re.search(r"fn signatures\(\)->\[\(&'static str,u32\);(\d+)\]",(ROOT/'src/lib.rs').read_text(encoding='utf8')).group(1))
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
report=dict(nativeExecutionPassed=True,compiledHooksExecuted=True,
 unitTestsPassed=int(re.search(r'test result: ok\. (\d+) passed;',unit).group(1)),isolatedNativeTestsPassed=7,signatureCount=signature_count,
 nativeFunctions=['E88090 CharaInit reference','EA7FC0 weapon reference and EA9A60 reinforcement','E86730 weighted art selector','E86840 selected art','E7B030 weighted non-duplicate available magic selector','E7B010 selected magic','F561D0 float RNG'],
 fixtures=['private copied executable; protected absolute jump immediates relocated only in test copy','parameter repository, table iteration and magic availability fixtures','native magic selector temporary allocator fixture','constructor integration uses lottery stubs, tested independently with real selectors','native safe-slot and summon packet tests use creation/rendering/network fixtures'],
 cases=['same-seed replay without consuming global RNG','native chance weights and zero-weight exclusion','native primary and unavailable magic exclusion','unique-art preservation','immutable aligned virtual rows and preserved reinforcement structures','all 19-bit IDs fit native item mask','human descriptor seed retained in original 48-byte packet','one live sinner, repeated-death deduplication, cooldown and bounded placement retries'],
 sources={p:sha(ROOT/p) for p in sources},noHotkeysBuildPassed=True,gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f"PASS: {report['unitTestsPassed']} unit tests, 7 private native execution tests, {signature_count} EXE signatures and no-hotkeys configuration; actual game/coop untested")
