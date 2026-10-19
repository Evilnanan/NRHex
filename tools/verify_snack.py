"""Verify actual native settled-price calls and the installed reward resources.

No game process is accessed. Effects, item creation and the rest of the shop UI
are fixtures; native price multiplication/statistics and compiled hooks execute.
"""
import hashlib,json,os,subprocess
from local_paths import ROOT,GAME

work=ROOT/'work/snack';work.mkdir(parents=True,exist_ok=True)
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
command=['cargo','test','--release']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','native_snack_uses_settled_shop_payments_and_private_queued_rewards','--','--ignored','--test-threads=1']
result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
output=result.stdout+result.stderr;(work/'native-test.log').write_text(output,encoding='utf8')
assert result.returncode==0 and '1 passed; 0 failed;'in output,output
result=subprocess.run(['dotnet','run','--project',str(ROOT/'tools/HexBuild'),'--',str(ROOT),'--snack-check'],cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
output=result.stdout+result.stderr;(work/'installed-resource-check.log').write_text(output,encoding='utf8')
assert result.returncode==0 and 'PASS: installed Extra Helping'in output,output
sources=['src/snack.rs','src/snack_native_tests.rs','src/lib.rs','src/hex_pool.rs','src/reward_cycle_runtime.rs','src/reward_recovery.rs','tools/HexBuild/Snack.cs','tools/HexBuild/RewardCatalog.cs','tools/verify_snack.py']
report=dict(nativeExecutionPassed=True,installedParameterReadbackPassed=True,
 nativeFunctions=['1FC0C0 settled shop statistic increment','A7CBC6 price * quantity and A7CBCF call','A7CFC6 price * quantity and A7CFCF call'],
 successfulPurchaseReturnSites=['A7CBD4','A7CFD4'],excludedOtherSpendReturnSite='A403EF',threshold=20000,
 choices=[{'basic':100},{'basic':100},{'basic':80,'mechanism':10,'high':10}],
 cases=['discounted settled price and purchase quantity','both native successful purchase callbacks','native stat cap preserved without limiting reward credit','other spending forwarded without credit','missing ownership and inactive hook','nonpositive payments','multiple purchases before deferred save acknowledgement','four packed saved bytes','private pending reward blocks delivery','shop-update ABI and closing-state heartbeat','incapacitation defers delivery','all three local player slots','zero creations refunded','partial creation consumes one reward','multiple thresholds queue serially','new-run reset ignores stale records until removal'],
 stubs=['purchase callback setup/return around original amount multiplication and CALL','native effect query/application/removal and save acknowledgements','native item-spawn service and empty item list','shop continuation after original first instruction; no real UI'],
 sources={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
print('PASS: Extra Helping native settled purchases, queued private lotteries, ABI and installed parameters; no game process')
