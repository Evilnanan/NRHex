"""Verify native merchant weapon purchase calls and the installed reward resources.

No game process is accessed. Effects, item creation and the rest of the shop UI
are fixtures; native price multiplication/statistics and compiled hooks execute.
"""
import hashlib,json,os,subprocess
from local_paths import ROOT,GAME

work=ROOT/'work/snack';work.mkdir(parents=True,exist_ok=True)
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
command=['cargo','test','--release']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','native_extra_practice_rewards_merchant_weapon_purchases','--','--ignored','--test-threads=1']
result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
output=result.stdout+result.stderr;(work/'native-test.log').write_text(output,encoding='utf8')
assert result.returncode==0 and '1 passed; 0 failed;'in output,output
result=subprocess.run(['dotnet','run','--project',str(ROOT/'tools/HexBuild'),'--',str(ROOT),'--snack-check'],cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
output=result.stdout+result.stderr;(work/'installed-resource-check.log').write_text(output,encoding='utf8')
assert result.returncode==0 and 'PASS: installed Extra Practice'in output,output
sources=['src/snack.rs','src/snack_native_tests.rs','src/lib.rs','src/hex_pool.rs','src/reward_cycle_runtime.rs','src/reward_recovery.rs','tools/HexBuild/Snack.cs','tools/HexBuild/RewardCatalog.cs','tools/verify_snack.py']
report=dict(nativeExecutionPassed=True,installedParameterReadbackPassed=True,
 nativeFunctions=['1FC0C0 settled shop statistic increment','A7CBC6 price * quantity and A7CBCF call','A7CFC6 price * quantity and A7CFCF call'],
 successfulPurchaseReturnSites=['A7CBD4','A7CFD4'],excludedOtherSpendReturnSite='A403EF',rewardsPerWeapon=1,priceThreshold=None,countsPurchasedQuantity=True,
 purchaseContext='RBX at the two signature-guarded native successful-purchase CALL sites',itemFields={'A7CBD4':'64','A7CFD4':'5C'},quantityFields={'A7CBD4':'118','A7CFD4':'110'},weaponEncodedKind=0,
 choices=[{'basic':100},{'basic':100},{'basic':80,'mechanism':10,'high':10}],lotRarityOverride=-1,usesResolvedItemRarity=True,
 cases=['actual native item loads and both successful purchase callbacks','one reward per purchased weapon independent of rune price','zero/1/3499/3500/20000 unit prices','shields and catalysts use native weapon category','consumables/accessories/armor/invalid item codes excluded','native price multiplication and capped statistic preserved','other spending forwarded without item-record access','missing ownership/inactive/outside expedition ignored','zero and negative quantities ignored','negative payment ignored','multiple weapon purchases accumulate pending rewards','no currency credit or persistent spending markers','existing private reward does not block delivery','next-frame delivery without closing shop','death/near-death defer delivery','all three local player slots','zero creations refunded','partial creation consumes one reward','multiple purchased weapons delivered in one update without duplicates','new-run reset drops pending rewards'],
 stubs=['purchase callback prologues and return around original item loads, amount multiplication and CALL','native ownership queries','native item-spawn service; no real shop UI or inventory grant'],
 sources={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
print('PASS: Extra Practice native weapon purchases, quantity, private lotteries and installed parameters; no game process')
