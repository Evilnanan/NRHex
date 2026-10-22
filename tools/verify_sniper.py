"""Validate installed Sniper resources and execute native range/compiled hit logic."""
import os,json,hashlib,subprocess
import pefile
from local_paths import ROOT,GAME
w=ROOT/'work/sniper';w.mkdir(exist_ok=True)
subprocess.run(['dotnet','run','--project',str(ROOT/'tools/HexBuild'),'--',str(ROOT),'--sniper-check'],check=True)
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
for rva,signature in [(0x5065e1,'44 0f b7 81 3c 01 00 00'),(0xe875d0,'f3 0f 10 70 14'),(0x34f9cf,'f3 0f 10 8b d0 02 00 00'),(0x344862,'f3 0f 10 86 b4 04 00 00')]:
    start=pe.get_offset_from_rva(rva);expected=bytes.fromhex(signature)
    assert raw[start:start+len(expected)]==expected,hex(rva)
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
fixture=w/'installed-effects.param';env['NRHEX_SNIPER_FIXTURE']=str(fixture)
if os.name!='nt'and env.get('NRHEX_CARGO_TARGET','').startswith('x86_64-pc-windows'):env['NRHEX_SNIPER_FIXTURE']='Z:'+str(fixture).replace('/',chr(92))
cmd=['cargo','test','--release','--features','cage-demand']
if env.get('NRHEX_CARGO_TARGET'):cmd+=['--target',env['NRHEX_CARGO_TARGET']]
cmd+=['--lib','native_sniper_range_and_shared_damage_scope','--','--ignored','--test-threads=1','--nocapture']
r=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');output=r.stdout+r.stderr;(w/'native-test.log').write_text(output)
assert r.returncode==0 and '1 passed; 0 failed;'in output,output
sources=['src/sniper.rs','src/sniper_native_tests.rs','src/lib.rs','src/reward_choices.rs','src/hex_pool.rs','tools/HexBuild/Sniper.cs','tools/HexBuild/Program.cs','tools/HexBuild/HexCatalog.cs','tools/HexBuild/RewardCatalog.cs','tools/verify_sniper.py','docs/hexes.md','docs/hexes.en.md']
report=dict(owner=95004500,installedReadbackPassed=True,nativeExecutionPassed=True,attackRate=.7,bowDistRate=200,distanceCap=50,maxDistanceBonus=1.5,rainBonusRate=.6,conflicts=[95000900],historyBit=42,historyBytes=6,nativeFunctions=['5065C0 active distance modifiers summed','E875A0 native range threshold','compiled shared on_rear_calculated callback, including native cache guard'],cases=['normal arrow 12 to 36m; arrow-rain positioning 43 to 129m','native 50 distance modifier adds to owner 200','all five native AttackRate fields are 0.7 in installed row','ordinary, spell and skill hits receive distance bonus','four exact rain attack IDs receive 60% of the increment','current player to collision point; launch position and target pivot excluded','local ownership, reset, cache, foreign sources, self hits, zero damage, missing position','only final HP damage changes; status, poise, stamina and other hit bytes preserved','regular tests cover bidirectional Sword exclusion and actual-ownership exceptions'],stubs=['weapon PARAM lookup, effect ownership, world and actors are fixtures','native collision, defense calculation, network transport and real actors not executed'],sourceHashes={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameplayVerified=False,multiplayerVerified=False)
(w/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS: Sniper installed penalties, native distance correction, compiled damage scope and Sword conflicts; no game process')
