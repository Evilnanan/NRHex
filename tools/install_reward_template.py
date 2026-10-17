"""Install compact rewards and the matching DLL; preserve all other assets/settings."""
from pathlib import Path
from datetime import datetime
import argparse,hashlib,json,re,shutil,tomllib
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('--dll',type=Path,default=ROOT/'target/release/NRHex.dll');args=p.parse_args()
work=ROOT/'work/reward-scaling';stage=work/'staged';dist=ROOT/'dist'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def assets():return {p.relative_to(dist).as_posix():sha(p) for p in (dist/'mod').rglob('*') if p.is_file() and p.suffix!='.log'}
before=assets();previous=read(dist/'reward-template-validation.json') if (dist/'reward-template-validation.json').exists() else None
parameters=read(stage/'validation.json');native=read(work/'native-validation.json')
assert parameters['activePackedHistoryLimit']>0
for path,value in parameters['sourceHashes'].items():assert sha(ROOT/path)==value.lower(),'Parameter generator changed after staging: '+path
assert parameters['templateRows']==3 and parameters['allRetainedRowsAndOtherTablesUnchanged'] and parameters['nativeAndRedEnvelopePreserved']
assert all(s['rewardRows']==3 and s['savedRowsAndBothResetsGenerated'] for s in parameters['simulations'])
assert before['mod/regulation.bin']==parameters['inputSha256'].lower() and sha(stage/'regulation.bin')==parameters['outputSha256'].lower()
assert native['nativeExecutionPassed'] and native['compiledHookExecuted']
for path,value in native['sources'].items():assert sha(ROOT/path)==value,'Source changed after native verification: '+path
assert (ROOT/'src/hex_pool.rs').read_bytes()==(stage/'hex_pool.rs').read_bytes()
sources={'mod/regulation.bin':stage/'regulation.bin','mod/natives/NRHex.dll':args.dll}
backup=work/'before'/datetime.now().strftime('%Y%m%d-%H%M%S-%f')
for relative in sources:
 target=backup/relative;target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(dist/relative,target)
try:
 for relative,source in sources.items():shutil.copyfile(source,dist/relative);assert sha(dist/relative)==sha(source)
 after=assets();changed=sorted(p for p in before.keys()|after.keys() if before.get(p)!=after.get(p));assert set(changed)<=sources.keys(),'Unrelated assets changed'
except Exception:
 for relative in sources:shutil.copy2(backup/relative,dist/relative)
 raise
signature_count=int(re.search(r"fn signatures\(\)->\[\(&'static str,u32\);(\d+)\]",(ROOT/'src/lib.rs').read_text()).group(1))
version=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version']
report=dict(version=version,changedAssets=changed,files={p:after[p] for p in sources},parameters=parameters,native=native,backup=str(backup),allOtherResourcesAndUserSettingsPreserved=True,gameplayVerified=False,multiplayerVerified=False)
if previous and previous['files'].get('mod/regulation.bin')==before['mod/regulation.bin']:
 report['initialUpgrade']=previous.get('initialUpgrade',previous['parameters'])
(dist/'reward-template-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
(dist/'build-validation.json').write_text(json.dumps(dict(version=version,buildPassed=True,scope='Runtime reward sampling, compact native template and dynamic ownership/history sets',dllSha256=after['mod/natives/NRHex.dll'],signatureCount=signature_count,validation='reward-template-validation.json',gameplayVerified=False,multiplayerVerified=False),indent=2),encoding='utf8')
(dist/'runtime-validation.json').write_text(json.dumps(dict(version=version,active=None,pid=None,dllSha256=after['mod/natives/NRHex.dll'],build='build-validation.json',gameplayVerified=False,multiplayerVerified=False,pendingChecks=['Initial three-choice ball contents, names/icons and normal claim in game','Per-slot reroll, exposure rollover and role/terrain exclusions in game','Death/revival, queued level rewards and new-expedition reset','Host/client private visibility, ownership and claim','Erosion and Death Dance gameplay interactions']),indent=2),encoding='utf8')
print('PASS: compact parameter table and matching DLL installed; events, texts, other resources and user settings preserved')
