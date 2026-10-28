"""Check the installed observer and compiled native damage/commit boundaries."""
import hashlib,json,os,subprocess,sys,tomllib,struct
from pathlib import Path
from local_paths import ROOT,GAME
from hks_bytecode import Reader,decode
from build_deliberate_punch_hks import check,QUERY,EVENTS
source=(ROOT/'dist/mod/action/script/c0000.hks').read_bytes();after=check(source)
before=Reader((ROOT/'assets/original/action/script/c0000.hks').read_bytes());before.function()
for fid in [157,604,605,606,607]:
 assert after.functions[fid]['code']==before.functions[fid]['code']
 assert after.functions[fid]['constants']==before.functions[fid]['constants']
old,new=before.functions[9],after.functions[9]
assert new['code'].endswith(old['code'])
code=new['code'][:-len(old['code'])]
for event,mode in [(EVENTS[0],1),(EVENTS[1],2),(EVENTS[2],3),('W_Idle',0),('W_Rolling',0),('W_AddDamageSmashAttackStance',0)]:
 calls=[];regs={0:event};constants=new['constants'];words=[w[0]for w in struct.iter_unpack('>I',code)];pc=0
 while pc<len(words):
  d=decode(words[pc]);pc+=1;op,a,b,c,bx=[d[k]for k in ['op','a','b','c','bx']]
  rk=lambda i:constants[i-256]if i>=256 else regs[i]
  if op==91:assert constants[bx]=='env';regs[a]=lambda *args:calls.append(args)
  elif op==76:pass
  elif op==25:regs[a]=constants[bx]
  elif op==4:
   if (rk(b)==rk(c))!=bool(a):pc+=1
  elif op==28:pc+=bx-0x10000+1
  elif op==2:regs[a](*(regs[i]for i in range(a+1,a+b)))
  else:raise AssertionError(op)
 assert calls==[(float(QUERY),float(mode))],calls
# The observer is deliberately absent from no-reset overlays.
assert QUERY not in after.functions[3]['constants']and QUERY not in after.functions[4]['constants']
env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
cmd=['cargo','test','--release','--features','cage-demand']
if env.get('NRHEX_CARGO_TARGET'):cmd+=['--target',env['NRHEX_CARGO_TARGET']]
cmd+=['--lib','native_punch_shadow_commit_and_additive_bonus','--','--ignored','--test-threads=1']
r=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace')
out=ROOT/'work/deliberate-punch';out.mkdir(exist_ok=True);(out/'native-test.log').write_text(r.stdout+r.stderr)
assert r.returncode==0 and '1 passed; 0 failed;'in r.stdout,r.stdout+r.stderr
report=dict(version=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version'],owner=95004800,
 nativeInputMultiplicationAndCombinerPassed=True,compiledCommitAndAdditiveBonusPassed=True,
 originalDamageAndSmashCallbacksUnchanged=True,observerBytecodeExecuted=True,
 checks=['independent pre/post totals; max after summation; two-max-HP cap; frozen multi-target bonus',
 'zero object defense and zero final damage; preserve additive attacker damage without dividing by defense',
 'accepted hit commits once at pre-HP-clamp load; 1 HP retains damage; preview/cache/nullified hits excluded',
 'standalone and Back to Basics additive output; full-body cancellation; native no-reset hurt overlay preserved',
 'ordinary and Double Surprise candidates share actual Raider exclusivity'],
 gameplayVerified=False,multiplayerVerified=False,
 sourceHashes={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in ['src/deliberate_punch.rs','src/deliberate_punch_native_tests.rs','src/glass_cannon.rs','src/basics.rs','src/lib.rs','src/hex_pool.rs','tools/build_deliberate_punch_hks.py','tools/verify_deliberate_punch.py']})
(out/'runtime-validation.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS: Deliberate Punch native shadow/commit/additive damage and original observer dispatch; no live game process')
