"""Verify native merchant weapon purchase calls and the installed reward resources.

No game process is accessed. Effects, item creation and the rest of the shop UI
are fixtures; native price multiplication/statistics and compiled hooks execute.
"""
import argparse,hashlib,json,os,subprocess
from local_paths import ROOT,GAME

work=ROOT/'work/snack';work.mkdir(parents=True,exist_ok=True)
parser=argparse.ArgumentParser();parser.add_argument('--staged-odds',action='store_true');args=parser.parse_args()
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
command=['cargo','test','--release','--features','cage-demand']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','native_extra_practice_rewards_merchant_weapon_purchases','--','--ignored','--test-threads=1']
result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
output=result.stdout+result.stderr;(work/'native-test.log').write_text(output,encoding='utf8')
assert result.returncode==0 and '1 passed; 0 failed;'in output,output
result=subprocess.run(['dotnet','run','--project',str(ROOT/'tools/HexBuild'),'--',str(ROOT),'--snack-odds-check' if args.staged_odds else '--snack-check'],cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
output=result.stdout+result.stderr;(work/'installed-resource-check.log').write_text(output,encoding='utf8')
assert result.returncode==0 and 'PASS: installed Extra Practice'in output,output
sources=['src/snack.rs','src/snack_native_tests.rs','src/lib.rs','src/hex_pool.rs','src/reward_cycle_runtime.rs','src/reward_recovery.rs','tools/HexBuild/Snack.cs','tools/HexBuild/RewardCatalog.cs','tools/verify_snack.py']
report=dict(nativeExecutionPassed=True,installedParameterReadbackPassed=not args.staged_odds,stagedParameterReadbackPassed=args.staged_odds,
 nativeFunctions=['1FC0C0 settled shop statistic increment','A7CBC6 price * quantity and A7CBCF call','A7CFC6 price * quantity and A7CFCF call'],
 successfulPurchaseReturnSites=['A7CBD4','A7CFD4'],excludedOtherSpendReturnSite='A403EF',rewardsPerWeapon=1,priceThreshold=None,countsPurchasedQuantity=True,
 purchaseContext='RBX at the two signature-guarded native successful-purchase CALL sites',itemFields={'A7CBD4':'64','A7CFD4':'5C'},quantityFields={'A7CBD4':'118','A7CFD4':'110'},weaponEncodedKind=0,
 choices=[{'basic':100},{'basic':100},{'basic':90,'mechanism':5,'high':5}],premiumChoices=[{'basic':100},{'basic':100},{'basic':0,'mechanism':50,'high':50}],premiumRarities=[2,3,9],purchaseRarityGetter='8BB780 native shop UI; record=shop+10 / shop+8',qualityCapturedAtPurchase=True,lotRarityOverride=-1,usesResolvedItemRarity=True,
 qualityCases=['both purchase callers execute original shop quality getter and native weapon rarity arithmetic','white/blue/purple/gold, red with white/purple base, unsupported grades','two rewards per purchase preserve mixed-quality order','quality changes after purchase do not change pending reward tiers'],
 cases=['actual native item loads and both successful purchase callbacks','one reward per purchased weapon independent of rune price','zero/1/3499/3500/20000 unit prices','shields and catalysts use native weapon category','consumables/accessories/armor/invalid item codes excluded','native price multiplication and capped statistic preserved','other spending forwarded without item-record access','missing ownership/inactive/outside expedition ignored','zero and negative quantities ignored','negative payment ignored','multiple weapon purchases accumulate pending rewards','no currency credit or persistent spending markers','existing private reward does not block delivery','next-frame delivery without closing shop','death/near-death defer delivery','all three local player slots','zero creations refunded','partial creation consumes one reward','multiple purchased weapons delivered in one update without duplicates','new-run reset drops pending rewards'],
 stubs=['purchase callback prologues and return around original item loads, amount multiplication and CALL','native ownership queries','weapon parameter lookup, shop cached-item validity and stack-cookie check; original shop rarity and base rarity query execute','native item-spawn service; no real shop UI or inventory grant'],
 sources={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
print('PASS: Extra Practice native weapon purchases, quantity, private lotteries and installed parameters; no game process')
