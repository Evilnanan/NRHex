"""Install the reviewed fission resources and DLL without touching user settings."""
from pathlib import Path
from datetime import datetime
import argparse,hashlib,json,re,shutil,tomllib
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('--dll',type=Path,default=ROOT/'target/release/NRHex.dll');args=p.parse_args()
work=ROOT/'work/potential-fission';stage=work/'staged';dist=ROOT/'dist'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
parameters=read(stage/'validation.json');native=read(work/'native-validation.json')
assert parameters['allOriginalRowsAndOtherTablesUnchanged'] and parameters['weaponGeneratorsUnchanged']
assert parameters['audit']['allRootsProbabilityMassChecked'] and parameters['audit']['goldDropsExactlyOneTier']
assert native['nativeExecutionPassed'] and native['compiledHooksExecuted']
for report in [parameters,native]:
 for path,value in report.get('sourceHashes',report.get('sources',{})).items():assert sha(ROOT/path)==value.lower(),'Source changed after verification: '+path
assert sha(dist/'mod/regulation.bin')==parameters['inputSha256'].lower()
assert sha(stage/'regulation.bin')==parameters['outputSha256'].lower()
for name in ['fission_catalog.rs','hex_pool.rs']:assert (ROOT/'src'/name).read_bytes()==(stage/name).read_bytes()
texts=parameters.get('texts')
if texts is None:
 texts=read(stage/'texts/validation.json')['reports']
 # HexTexts records use language/file and compressed hashes on Windows.
 assert isinstance(texts,list)
text_audit=parameters.get('textSemanticReadback') or read(stage/'texts/validation.json')
for key in ['source','englishSource','statusSource']:assert sha(ROOT/text_audit[key])==text_audit[key+'Sha256'].lower()
assert all(change['id']==95003200 for report in text_audit['reports'] for change in report['changes']),'Unrelated text changed'
sources={'mod/regulation.bin':stage/'regulation.bin','mod/natives/NRHex.dll':args.dll}
for text in texts:
 relative=text.get('path') or 'mod/msg/'+text['language']+'/'+text['file']
 assert sha(dist/relative)==text['inputSha256'].lower()
 source=stage/'texts'/Path(relative).relative_to('mod/msg')
 assert sha(source)==text['outputSha256'].lower();sources[relative]=source
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
signatures=int(re.search(r"fn signatures\(\)->\[\(&'static str,u32\);(\d+)\]",(ROOT/'src/lib.rs').read_text()).group(1))
report=dict(version=version,changedAssets=changed,files={p:after[p]for p in sources},parameters=parameters,native=native,allOtherResourcesAndUserSettingsPreserved=True,gameplayVerified=False,multiplayerVerified=False)
(dist/'potential-fission-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
(dist/'build-validation.json').write_text(json.dumps(dict(version=version,buildPassed=True,scope='Potential Fission, native shared downgrade pools and independent local choice groups',dllSha256=after['mod/natives/NRHex.dll'],signatureCount=signatures,validation='potential-fission-validation.json',gameplayVerified=False,multiplayerVerified=False),indent=2),encoding='utf8')
(dist/'runtime-validation.json').write_text(json.dumps(dict(version=version,active=None,pid=None,dllSha256=after['mod/natives/NRHex.dll'],build='build-validation.json',gameplayVerified=False,multiplayerVerified=False,pendingChecks=['Acquire Fission before/after an existing native ball; only three new sources split','Claim A/B independently and verify weapons, passives, arts and theme/class tendencies','Death/revival, map transitions and new-expedition counter reset','Host/client private ownership and rewards']),indent=2),encoding='utf8')
print('PASS: fission resources and matching DLL installed; all other assets and settings preserved')
