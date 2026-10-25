"""Validate direct Hex grants, bound common workers and compiled delivery state."""
import os,json,hashlib,subprocess
from pathlib import Path
from local_paths import ROOT,GAME
w=ROOT/'work/double-dip';w.mkdir(exist_ok=True)
subprocess.run(['dotnet','run','--project',str(ROOT/'tools/HexBuild'),'--',str(ROOT),'--double-dip-check'],check=True)
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
cmd=['cargo','test','--release','--features','cage-demand']
if env.get('NRHEX_CARGO_TARGET'):cmd+=['--target',env['NRHEX_CARGO_TARGET']]
cmd+=['--lib','compiled_double_dip_deferred_selection_grant_and_recovery','--','--ignored','--test-threads=1','--nocapture']
r=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');log=r.stdout+r.stderr;(w/'native-test.log').write_text(log);assert r.returncode==0 and '1 passed; 0 failed;'in log,log
sources=['src/double_dip.rs','src/double_dip_native_tests.rs','src/hex_pool.rs','src/lib.rs','src/reward_choices.rs','src/reward_cycle_runtime.rs','tools/HexBuild/DoubleDip.cs','tools/HexBuild/HexCatalog.cs','tools/HexBuild/RewardCatalog.cs','tools/HexBuild/Program.cs','tools/verify_double_dip.py']
report=dict(version='0.17.253',owner=95004700,directGrant=True,rewardNodesCreated=False,inventoryUsed=False,compiledRuntimeProtocolPassed=True,installedParameterAndEventReadbackPassed=True,
 checks=['all hero IDs and unknown values only filter foreign character-exclusive powers, ownership and self','current Ironeye can draw Slide; current Revenant can draw Enhanced Necromancy','owned conflicts, caster affinity, multi-role restrictions, difficulty count, terrain and exposure history do not filter this draw','two distinct unowned results; RNG failure never submits a partial new choice','persist both selections before READY; partial saved selection fills without rerolling its original target','parameterized local common worker binds selected marker, actual PermanentBuff owner and popup for every possible target','native AddSpecialEffect followed by DisplayPermanentBuffPopup and wait for actual ownership; preowned target cannot be reapplied','deferred native requests submitted once; actor reconstruction retains frozen selection','receipt only after both actual owner effects; cleanup only after saved receipt is live; no re-grant on restoration','common worker itself rejects completed/reset states before DLL cleanup, including absent target with stale READY/selection','unknown hero, non-owner, down/dead/reset/foreign actor cannot initiate delivery','catalog append only; six history bytes; all existing parameter rows and unrelated tables/events preserved'],
 stubs=['native effect application/query/removal and event-worker scheduling are deferred fixtures','native common event VM, popup presentation, gameplay and multiplayer are not executed'],
 sourceHashes={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameplayVerified=False,multiplayerVerified=False)
(w/'runtime-validation.json').write_text(json.dumps(report,indent=2)+'\n');print('PASS: Double the Joy bound local direct-grant workers and compiled deferred/recovery protocol; no live game process')
