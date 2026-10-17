"""Install the reviewed Demon’s Summoning build; preserve user settings/assets."""
from pathlib import Path
from datetime import datetime
import argparse,hashlib,json,re,shutil,tomllib
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('--dll',type=Path,default=ROOT/'target/release/NRHex.dll');args=p.parse_args()
work=ROOT/'work/demons-call';stage=work/'staged';dist=ROOT/'dist'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
parameters=read(stage/'validation.json');native=read(work/'native-validation.json')
assert parameters['allOriginalRowsAndOtherTablesUnchanged'] and parameters['generatedLoadouts']==0 and parameters['gestureChainUnchanged']
assert parameters['npcRows']==60 and native['nativeExecutionPassed'] and native['compiledHooksExecuted']
assert native['isolatedNativeTestsPassed']==6 and native['signatureCount']==156
for report in [parameters,native]:
 for path,value in report.get('sourceHashes',report.get('sources',{})).items():assert sha(ROOT/path)==value.lower(),'Source changed after verification: '+path
assert sha(dist/'mod/regulation.bin')==parameters['inputSha256'].lower()
assert sha(stage/'regulation.bin')==parameters['outputSha256'].lower()
for name in ['demons_data.rs','hex_pool.rs']:assert (ROOT/'src'/name).read_bytes()==(stage/name).read_bytes()
audit=parameters['textSemanticReadback']
for key in ['source','englishSource','statusSource']:assert sha(ROOT/audit[key])==audit[key+'Sha256'].lower()
assert all(change['id']==95003300 for report in audit['reports'] for change in report['changes'])
sources={'mod/regulation.bin':stage/'regulation.bin','mod/natives/NRHex.dll':args.dll}
for text in parameters['texts']:
 relative=text['path'];assert sha(dist/relative)==text['inputSha256'].lower()
 source=stage/'texts'/Path(relative).relative_to('mod/msg');assert sha(source)==text['outputSha256'].lower();sources[relative]=source
def assets():return {p.relative_to(dist).as_posix():sha(p) for p in (dist/'mod').rglob('*') if p.is_file() and p.suffix!='.log'}
before=assets();backup=work/'before'/datetime.now().strftime('%Y%m%d-%H%M%S-%f')
for relative in sources:
 target=backup/relative;target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(dist/relative,target)
try:
 for relative,source in sources.items():shutil.copyfile(source,dist/relative);assert sha(dist/relative)==sha(source)
 after=assets();changed=sorted(p for p in before.keys()|after.keys() if before.get(p)!=after.get(p));assert set(changed)<=sources.keys(),'Unrelated asset changed'
except Exception:
 for relative in sources:shutil.copy2(backup/relative,dist/relative)
 raise
version=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version']
report=dict(version=version,changedAssets=changed,files={p:after[p]for p in sources},parameters=parameters,native=native,allOtherResourcesAndUserSettingsPreserved=True,appearance='Existing native Necromancy SFX; Libra-specific appearance not integrated',gameplayVerified=False,multiplayerVerified=False)
(dist/'demons-call-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
(dist/'build-validation.json').write_text(json.dumps(dict(version=version,buildPassed=True,scope='Demon’s Summoning with runtime loadouts and native reward art/magic lotteries',dllSha256=after['mod/natives/NRHex.dll'],signatureCount=156,validation='demons-call-validation.json',checks={k:native[k]for k in ['unitTestsPassed','isolatedNativeTestsPassed','noHotkeysBuildPassed']},gameplayVerified=False,multiplayerVerified=False),indent=2),encoding='utf8')
(dist/'runtime-validation.json').write_text(json.dumps(dict(version=version,active=None,pid=None,dllSha256=after['mod/natives/NRHex.dll'],build='build-validation.json',gameplayVerified=False,multiplayerVerified=False,pendingChecks=['Cross-Legged native madness stacking and actual madness trigger','Native human NPC appearance, hostility and random weapon AI','No death reward, 1/60 HP drain, death-started 60-second cooldown','Owner down/revive, map transitions and new expedition','Host/client creation, identical gear and cleanup']),indent=2),encoding='utf8')
print('PASS: matching DLL, regulation and texts installed; unrelated assets/settings preserved')
