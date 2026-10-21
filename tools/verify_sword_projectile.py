"""Run compiled Sword flight hooks through native motion, retirement and cleanup."""
import hashlib,json,os,struct,subprocess
from local_paths import ROOT,GAME
import pefile
work=ROOT/'work/sword-projectile-range';work.mkdir(parents=True,exist_ok=True)
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
def target(rva):
 assert pe.get_data(rva,1)==b'\xe8'
 return rva+5+struct.unpack('<i',pe.get_data(rva+1,4))[0]
# BulletMan performs native cleanup immediately after a false instance update.
assert target(0x35cf58)==0x34f130
# The hook follows actual movement, and its false-return destination is native.
assert pe.get_data(0x34f58f,4)==bytes.fromhex('FF 54 C4 60')
assert pe.get_data(0x34f50a,7)==bytes.fromhex('32 C0 E9 10 04 00 00')
assert pe.get_data(0x34f593,13)==bytes.fromhex('8B 97 90 00 00 00 48 8B 8F 88 00 00 00')
assert pe.get_data(0x34e76f,3)==bytes.fromhex('8B 52 04')
assert target(0x34e78a)==0xe87800
assert target(0x3553e6)==0x353df0
assert target(0x355404)==0x350fb0
assert pe.get_data(0x34f552,4)==bytes.fromhex('48 89 4D 80')
assert target(0x34f1e4)==0x34a490
assert target(0x34a832)==0x353cb0
assert pe.get_data(0x34ba38,7)==bytes.fromhex('80 8B 80 05 00 00 10')
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
command=['cargo','test','--release','--features','cage-demand']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','native_sword_projectile_fixed_launch_retirement_and_independent_children','--','--ignored','--test-threads=1']
r=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');log=r.stdout+r.stderr;(work/'native-test.log').write_text(log)
assert r.returncode==0 and '1 passed; 0 failed;' in log,log
lib=(ROOT/'src/lib.rs').read_text();regain=(ROOT/'src/sword_regain.rs').read_text();rules=(ROOT/'src/sword_hit_rules.rs').read_text()
assert 'glancing_display' not in lib and 'flags|2' not in lib[lib.index('unsafe extern "system" fn on_rear_calculated'):lib.index('unsafe extern "system" fn on_rear_calculated')+6000]
assert 'sword_hit_distance' not in regain and 'sword_hit_distance' not in rules
sources=['src/sword_projectile.rs','src/sword_projectile_stages.rs','src/sword_projectile_native_tests.rs','src/sword_map.rs','src/sword_regain.rs','src/sword_hit_rules.rs','src/strength_sword.rs','src/echo_cast.rs','src/lib.rs','tools/build_sword_ranges.py','tools/verify_sword_projectile.py','tools/SwordStagesProbe/Program.cs']
report=dict(nativeExecutionPassed=True,rangeMetres=6,distance='3D straight-line distance from each instance fixed actual launch point',casterPositionReadForFlight=False,damageDistanceFallback=False,
 nativeFunctions=['34E750 native initialize dispatcher and stack arguments','34F130 native per-instance update with compiled 34F593 post-motion hook','3559C0 native free-flight integration and orientation math','354AA0 current Bullet row selection','355270 direct state-4 retirement and reviewed state-3 native child emission','35F5C0 native manager cleanup and 342FB0 group unlink','34A6F0 native handle generation reset, active flag clear and collision unregister calls'],
cases=['exact 6m boundary in three axes followed by native out-of-range retirement','changing source position does not change launch point','full native cleanup: generation increment, inactive flag, group handle removal, two collision unregister calls','native state-4 transition invokes component exit without impact-child generation','looping flight counts radial displacement rather than cumulative trajectory','non-continuation child emissions and delayed group initializations start fresh 6m budgets at their own origins','non-continuation children outside parent sphere are permitted; non-continuation grandchildren receive fresh origins; non-spell parents cannot infer a new root','attached and synchronized effects retain native range; zero-speed generators remain unanchored until independent motion','release after attachment motion anchors the release position','generation mismatch, group finish, missing native Magic ID, mismatched Magic and source ownership'],
 stubs=['actor/effect and parameter lookup','354AF0 lower-level instance setup; real 34E750 dispatcher executes','native component update/exit fixture vtables','child creation body; compiled child detour executes; 353DF0 collision-child eligibility returns false','355D20/355FE0/350410 motion-related scene and collider-transform services','rendering, empty payload/target construction and reference teardown, component release and collision manager storage; native reset invokes both unregister fixture callbacks','stack cookie checker'],
 sourceHashes={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
report['cases']+=['all 18 normal/charged Glintstone Stars and Star Shower trajectories preserve first-flight origin across two explicit continuation edges','continuation emissions and delayed initializations outside inherited sphere are rejected; unrelated children of the same parent remain independent','native bind reads Bullet ID from payload+4, matching original parameter lookup operand']
report['cases']+=['Glintstone Arc three body continuations and four lateral collision emissions share the original six-metre sphere','both Crystal Barrage roots anchor despite dummy-poly sync bit; native 30m/s movement retires beyond six metres','Crystal Barrage attached follow type remains exempt; zero-speed setup anchors only upon independent motion']
report['cases']+=['six Rancor continuations, 34 Beast Claw/Gurranq continuations and 26 lightning distribution continuations retain initial origin','Whirl O Flame normal and charged roots use native 5/8m/s motion despite sync bit','six Shattering Crystal carriers extend their private remaining lifetime and use native state 3 exactly at 3m, with one original child and manager cleanup','both Discus roots enter native pause/return at 6m; return branch is not range tracked','Honed Bolt positioning root enters native downward placement at 6m','reviewed lightning components survive 20m vertical descent but retire beyond 6m horizontal displacement','all Founding Rain roots excluded with explicit and native sentinel Magic IDs']
report['groundLightningMetric']='horizontal distance for 73 reviewed components of Magic 6910/6920/6921/6930; native vertical positioning preserved'
report['continuationEdges']=109
report['crystalBurstMetres']=3
report['cases']+=['all 16 reviewed dragon breath roots limit native movement to 6m, including four FollowType-4 streams; attached type 1 remains exempt']
report['nativeFunctions']+=['34B9D0 native deferred visual-release flag','34A490 and 214EB90 native shared visual-reference release','353CB0 native erase-vs-reference-release cleanup selection']
report['cases']+=['Triple Rings pause transition retains shared VFX until native next-frame reference handoff; parent cleanup issues no erase and leaves child reference alive','negative control: immediate parent cleanup after state 3 erases shared VFX','Placidusax first beam root anchors native 300m/s flight despite sync bit and retires past 6m']
report['stubs']+=['visual existence/renderer erase callbacks; shared VFX control block starts with parent+child references; actual native reference decrement executes; default stage-4 component vtable relocated to fixture']
report['sourceHashes']['src/hook.rs']=hashlib.sha256((ROOT/'src/hook.rs').read_bytes()).hexdigest()
(work/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS: Sword native flight/cleanup, 109 continuation edges, 3m crystal burst, Discus return, lightning placement, Whirl O Flame and Founding Rain exclusion; game/multiplayer untested')
