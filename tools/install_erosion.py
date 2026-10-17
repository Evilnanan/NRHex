"""Install validated incremental Erosion resources, preserving unrelated assets."""
from pathlib import Path
from datetime import datetime
import argparse,hashlib,json,re,shutil,tomllib
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('--dll',type=Path,default=ROOT/'target/release/NRHex.dll');args=p.parse_args()
work=ROOT/'work/erosion';stage=work/'staged';dist=ROOT/'dist'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def assets():return {p.relative_to(dist).as_posix():sha(p) for p in (dist/'mod').rglob('*') if p.is_file() and p.suffix!='.log'}
before=assets();parameter=read(stage/'validation.json');native=read(work/'native-validation.json');text=read(stage/'texts/validation.json')
assert parameter['existingActiveRowsAndUnrelatedRowsPreserved'] and parameter['nativeReferencesUnchanged'] and parameter['hexCount']==28
assert before['mod/regulation.bin']==parameter['inputSha256'].lower() and sha(stage/'regulation.bin')==parameter['outputSha256'].lower()
assert native['nativeExecutionPassed'] and native['compiledHookExecuted']
for name,value in native['sources'].items():assert sha(ROOT/name)==value,'Source changed after verification: '+name
assert sha(ROOT/'src/hex_pool.rs')==sha(stage/'hex_pool.rs')
for path_key,hash_key in [('source','sourceSha256'),('englishSource','englishSourceSha256'),('statusSource','statusSourceSha256')]:
 assert sha(ROOT/text[path_key])==text[hash_key].lower(),'Text source changed after staging'
for entry in text['reports']:
 assert entry['unrelatedEntriesAndMembersUnchanged']
 assert all(change['id']==95003100 for change in entry['changes']),'Unrelated text entry changed'
containers={e['path']:e for e in read(stage/'container-validation.json')['containers']} if (stage/'container-validation.json').exists() else {}
sources={'mod/natives/NRHex.dll':args.dll,'mod/regulation.bin':stage/'regulation.bin','mod/event/common.emevd.dcx':stage/'common.emevd.dcx'}
for entry in text['reports']:
 if entry['changes']:
  relative=f"mod/msg/{entry['language']}/{entry['file']}";sources[relative]=stage/'texts'/entry['language']/entry['file']
for relative,source in sources.items():
 assert source.is_file()
 if relative=='mod/event/common.emevd.dcx':expected=(containers.get(relative) or parameter)['inputSha256' if relative in containers else 'eventInputSha256']
 elif relative.startswith('mod/msg/'):
  entry=next(e for e in text['reports'] if relative==f"mod/msg/{e['language']}/{e['file']}");expected=(containers.get(relative) or entry)['inputSha256']
 else:continue
 assert before[relative]==expected.lower(),'Target changed after staging: '+relative
 if relative in containers:assert sha(source)==containers[relative]['outputSha256']
backup=work/'before'/datetime.now().strftime('%Y%m%d-%H%M%S-%f')
for relative in sources:
 target=backup/relative;target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(dist/relative,target)
try:
 for relative,source in sources.items():shutil.copyfile(source,dist/relative);assert sha(dist/relative)==sha(source)
 after=assets();changed=sorted(r for r in before.keys()|after.keys() if before.get(r)!=after.get(r));assert set(changed)==sources.keys()
except Exception:
 for relative in sources:shutil.copy2(backup/relative,dist/relative)
 raise
version=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version']
report=dict(version=version,changedAssets=changed,files={p:after[p] for p in changed},unrelatedAssetsAndUserConfigPreserved=True,parameters=parameter,native=native,texts=text,containers=containers,backup=str(backup),gameplayVerified=False,multiplayerVerified=False)
(dist/'erosion-validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf8')
signature_count=int(re.search(r"fn signatures\(\)->\[\(&'static str,u32\);(\d+)\]",(ROOT/'src/lib.rs').read_text()).group(1))
(dist/'build-validation.json').write_text(json.dumps(dict(version=version,buildPassed=True,scope='Erosion incremental resources and Windows DLL',dllSha256=after['mod/natives/NRHex.dll'],signatureCount=signature_count,validation='erosion-validation.json',gameplayVerified=False,multiplayerVerified=False),indent=2),encoding='utf8')
(dist/'runtime-validation.json').write_text(json.dumps(dict(version=version,active=None,pid=None,dllSha256=after['mod/natives/NRHex.dll'],build='build-validation.json',gameplayVerified=False,multiplayerVerified=False,pendingChecks=['Erosion native attack applicability, seven ailments and immunity in game','Eochaid multi-hit and Ghostflame ground hit buildup in game','Native proc effects, escalating resistance, host/client authority and sync','Death Dance overlapping damage and shared recovery protection in game']),indent=2),encoding='utf8')
print(f"PASS: {len(changed)} validated assets installed; other resources and user settings preserved")
