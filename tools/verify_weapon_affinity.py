"""Execute native default/override discovery selection without a game process."""
import hashlib,json,os,subprocess
from local_paths import ROOT,GAME

directory=ROOT/'work/weapon-affinity';directory.mkdir(parents=True,exist_ok=True)
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
reference=json.loads((ROOT/'data/reference/rewards/weapon-affinity.json').read_text())
assert reference['version']=='10350000' and reference['regulationSha256']=='876a3ca279a4561d0c69f81fe5e510c75c59ab9a8a201694f8cf4a42c91e0268'
assert reference['heroDefaults']=={str(h):1000+h*100 for h in range(1,11)}
assert len(reference['relicOverrides'])==33 and reference['relicOverrides']['7633000']==57 and reference['relicOverrides']['7633100']==61
command=['cargo','test','--release']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','native_weapon_affinity_follows_relic_overrides_and_hero_defaults','--','--ignored','--test-threads=1']
result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
output=result.stdout+result.stderr;(directory/'native-test.log').write_text(output)
assert result.returncode==0 and '1 passed; 0 failed;'in output,output
sources=['src/weapon_affinity.rs','src/weapon_affinity_native_tests.rs','src/reward_choices.rs','src/lib.rs','tools/verify_weapon_affinity.py','data/reference/rewards/weapon-affinity.json']
report=dict(nativeExecutionPassed=True,nativeFunctions=['56ACF0 reward discovery context','5072D0 SpEffect state 2122 discovery override'],
 casterCategories=[57,61,1600,1700],normalCasterOwners=[95000900,95002000,95003500],archmageException=95002100,
 physicalOwners=[95001100,95002600,95002400,95003100],swordExceptionOwners=[95001100,95002600,95003100],
 cases=['all ten HeroParam defaults','Duchess dagger default is not caster','all 33 relic discovery categories override caster and physical defaults','stat-growth state 2123 is ignored','multiple matching effects follow native first-match order','null parameter rows skipped','first override -1 preserves hero default','missing HeroParam permits a valid override; no usable result defers','live changes are queried without caching','invalid hero, absent actor/effect module, and absent base do not call native','native writes only reward context+10; actor and effect module unchanged'],
 regularTests='seven ability filters across heroes, affinities, archmage/sword ownership, display cycles, focused/random pools and refresh; owned/conflicting abilities stay excluded',
 stubs=['E785B0 HeroParam lookup uses fixture rows','E782E0 scoped parameter-reference release uses fixture no-op'],
 sourceHashes={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(directory/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS: native weapon discovery defaults/overrides, first-match order and query ABI; no game process')
