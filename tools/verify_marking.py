"""Read installed Marking resources and execute the scoped constructor path."""
import hashlib,json,os,subprocess
import pefile
from local_paths import ROOT,GAME
work=ROOT/'work/marking';work.mkdir(exist_ok=True);env=os.environ.copy();env.setdefault('NRHEX_NATIVE_EXE',str(GAME/'nightreign.exe'))
result=subprocess.run(['dotnet','run','--project',str(ROOT/'tools/HexBuild'),'--',str(ROOT),'--marking-check'],cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');output=result.stdout+result.stderr;(work/'installed-check.log').write_text(output);assert result.returncode==0 and 'PASS: installed Marking'in output,output
# Native recipient mapping and later reaction consumers still read the outgoing
# input. This ability changes no target mapping or shared attack definition.
exe=(GAME/'nightreign.exe').read_bytes();pe=pefile.PE(data=exe,fast_load=True)
for rva,expected in [(0x43a170,'41 0f b6 56 24'),(0x43a178,'ff 50 30'),(0x43a17b,'41 88 86 b6 02 00 00'),(0x438eb2,'0f b6 53 24')]:
 offset=pe.get_offset_from_rva(rva);signature=bytes.fromhex(expected);assert exe[offset:offset+len(signature)]==signature,hex(rva)
fixture=work/'native-attacks.param';env['NRHEX_MARKING_FIXTURES']=str(fixture)
if os.name!='nt'and env.get('NRHEX_CARGO_TARGET','').startswith('x86_64-pc-windows'):env['NRHEX_MARKING_FIXTURES']='Z:'+str(fixture).replace('/',chr(92))
command=['cargo','test','--release','--features','cage-demand']
if env.get('NRHEX_CARGO_TARGET'):command+=['--target',env['NRHEX_CARGO_TARGET']]
command+=['--lib','native_marking_constructor_uses_upward_cut_and_retains_copies','--','--ignored','--test-threads=1']
result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');output=result.stdout+result.stderr;(work/'native-test.log').write_text(output);assert result.returncode==0 and '1 passed; 0 failed;'in output,output
from verify_marking_hks import verify as verify_hks
hksCases=verify_hks((ROOT/'dist/mod/action/script/c0000.hks').read_bytes())
fixture=ROOT/'work/marking-poison/installed-effects.param';assert fixture.exists();env['NRHEX_MARKING_POISON_FIXTURES']=str(fixture)
if os.name!='nt'and env.get('NRHEX_CARGO_TARGET','').startswith('x86_64-pc-windows'):env['NRHEX_MARKING_POISON_FIXTURES']='Z:'+str(fixture).replace('/',chr(92))
command[command.index('--lib')+1]='native_marking_followup_targets_and_original_collector_are_scoped'
result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,errors='replace');output=result.stdout+result.stderr;(work/'poison-native-test.log').write_text(output);assert result.returncode==0 and '1 passed; 0 failed;'in output,output
sources=['src/marking.rs','src/marking_native_tests.rs','src/lib.rs','src/reward_choices.rs','src/hex_pool.rs','tools/HexBuild/Marking.cs','tools/HexBuild/HexCatalog.cs','tools/HexBuild/RewardCatalog.cs','tools/verify_marking.py','src/marking_poison_native_tests.rs','src/erosion.rs','src/ambulance.rs','src/reward_cycle_runtime.rs','tools/HexBuild/MarkingPoison.cs','tools/build_marking_hks.py','tools/verify_marking_hks.py']
report=dict(hksTransitionCases=hksCases,poisonBurstSource=7500202,poisonBurst=95004203,requiresPoison=False,onlyImmediateBackstepFollowup=True,onlyActualSlideTargets=True,oneBurstPerFollowup=True,nativeExecutionPassed=True,installedReadbackPassed=True,owner=95004200,attack=600000110,referenceAttack=300000290,impact=9,airImpact=3,originRecordedAtNativeStart=True,landingDoesNotChangeOrigin=True,originalImpact=1,ironeyeOnly=True,historyBit=39,originalAttackRowsPreserved=True,nativeInstructions=['E709D4 AtkParam+72 read','E709D8 native branch','E709E8 outgoing hit+24 write'],compiledSharedConstructorWrapperExecuted=True,nativeRecipientReactionSignaturesChecked=True,sevenNativeArgumentsAndReturnPreserved=True,cases=['ground9/air3 through the shared constructor, including Mighty coexistence','native ground/air HKS origin queries, landing, repeated starts and identity resets','owned/unowned, local/remote, both attack table kinds','Marking, original poison relic follow-up, Upward Cut','actual native state152 collector and applicability for Marking/backstep/ordinary/body hits','compiled 13-argument recipient hook and HKS env query','same-target and wrong-target first hits; repeated/multi-hit contacts, air swings and unrelated transitions','lower/upper pair is one action; new slide miss and reused target handles','owner loss, down, death, revival and reset reject pending burst','Mighty coexistence, native attack immutability, cached copies retain input','all hero/affinity/party combinations in regular reward-filter tests'],stubs=['full constructor uses fixture prologue around original native impact copy instructions','actor/effect lookup, final burst submission; no complete animation/collision/network transport'],sources={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameplayVerified=False,multiplayerVerified=False,gameProcessAccessed=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n');print('PASS: Marking native impact and collector, scoped one-shot backstep follow-up poison burst, HKS transitions and original rows; no game process')
