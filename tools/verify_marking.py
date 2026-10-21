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
sources=['src/marking.rs','src/marking_native_tests.rs','src/lib.rs','src/reward_choices.rs','src/hex_pool.rs','tools/HexBuild/Marking.cs','tools/HexBuild/HexCatalog.cs','tools/HexBuild/RewardCatalog.cs','tools/verify_marking.py']
report=dict(nativeExecutionPassed=True,installedReadbackPassed=True,owner=95004200,attack=600000110,referenceAttack=300000290,impact=9,originalImpact=1,ironeyeOnly=True,historyBit=39,originalAttackRowsPreserved=True,nativeInstructions=['E709D4 AtkParam+72 read','E709D8 native branch','E709E8 outgoing hit+24 write'],compiledSharedConstructorWrapperExecuted=True,nativeRecipientReactionSignaturesChecked=True,sevenNativeArgumentsAndReturnPreserved=True,cases=['owned/unowned, local/remote, both attack table kinds','Marking, poison relic follow-up, Upward Cut','Mighty coexistence, native attack immutability, cached copies retain input','all hero/affinity/party combinations in regular reward-filter tests'],stubs=['full constructor uses fixture prologue around original native impact copy instructions','actor/effect lookup, no real animation/collision/network transport'],sources={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in sources},gameplayVerified=False,multiplayerVerified=False,gameProcessAccessed=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n');print('PASS: Marking compiled constructor and native Upward Cut impact copy, owner/source/table filters and original rows; no game process')
