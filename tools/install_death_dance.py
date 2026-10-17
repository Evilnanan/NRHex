"""Install the validated incremental Death's Dance outputs, preserving settings."""
from pathlib import Path
from datetime import datetime
import argparse,hashlib,json,re,shutil,tomllib
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('--dll',type=Path,default=ROOT/'target/release/NRHex.dll');p.add_argument('--dll-only',action='store_true');args=p.parse_args()
work=ROOT/'work/death-dance';stage=work/'staged';dist=ROOT/'dist'
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def read(path):return json.loads(path.read_text(encoding='utf-8-sig'))
def assets():return {p.relative_to(dist).as_posix():sha(p) for p in (dist/'mod').rglob('*') if p.is_file() and p.suffix!='.log'}
def record_metadata(report,after):
 signature_count=int(re.search(r"fn signatures\(\)->\[\(&'static str,u32\);(\d+)\]",(ROOT/'src/lib.rs').read_text()).group(1))
 (dist/'build-validation.json').write_text(json.dumps(dict(version=report['version'],buildPassed=True,scope=report.get('scope',"Death Dance incremental resources and Windows DLL"),dllSha256=after['mod/natives/NRHex.dll'],signatureCount=signature_count,validation='death-dance-validation.json',gameplayVerified=False,multiplayerVerified=False),indent=2),encoding='utf8')
 (dist/'runtime-validation.json').write_text(json.dumps(dict(version=report['version'],active=None,pid=None,dllSha256=after['mod/natives/NRHex.dll'],build='build-validation.json',gameplayVerified=False,multiplayerVerified=False,pendingChecks=['Independent two-second damage schedules and shared protection across new hits in game','Native gradual flasks and interactions with existing flask effects','Death, near-death, revival and multiplayer behavior']),indent=2),encoding='utf8')

if args.dll_only:
 before=assets();native=read(work/'native-validation.json');report=read(dist/'death-dance-validation.json')
 assert native['nativeExecutionPassed'] and native['compiledMidHookExecuted'] and native['protectionHookExecuted']
 assert native['protectedQuotaRetained'] and native['pendingRecoveryResetPreserved'] and native['expiredQuotaNotReprotected']
 for name,value in native['sources'].items():assert sha(ROOT/name)==value,'Source changed after native verification'
 for name,value in report['files'].items():assert before[name]==value,'Installed resources differ from validated Death Dance package: '+name
 target=dist/'mod/natives/NRHex.dll';backup=work/'before'/datetime.now().strftime('%Y%m%d-%H%M%S-%f')
 backup.mkdir(parents=True);shutil.copy2(target,backup/'NRHex.dll')
 try:
  shutil.copyfile(args.dll,target);assert sha(target)==sha(args.dll)
  after=assets();changed=sorted(r for r in before.keys()|after.keys() if before.get(r)!=after.get(r))
  assert changed==['mod/natives/NRHex.dll'],'Unrelated assets changed'
 except Exception:
  shutil.copy2(backup/'NRHex.dll',target);raise
 report.update(version=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version'],scope='DLL-only Death Dance shared regain protection',changedAssets=changed,native=native,backup=str(backup),unrelatedAssetsAndUserConfigPreserved=True,gameplayVerified=False,multiplayerVerified=False)
 report['files']['mod/natives/NRHex.dll']=after['mod/natives/NRHex.dll']
 (dist/'death-dance-validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf8')
 record_metadata(report,after)
 print('PASS: shared-protection DLL installed; all gameplay resources and user settings preserved')
 raise SystemExit(0)

before=assets();parameter=read(stage/'validation.json');native=read(work/'native-validation.json');text=read(stage/'texts/validation.json')
assert parameter['existingRowsAndTriplesPreserved'] and parameter['nativeReferencesUnchanged'] and parameter['hexCount']==27
assert before['mod/regulation.bin']==parameter['inputSha256'].lower() and sha(stage/'regulation.bin')==parameter['outputSha256'].lower()
assert native['nativeExecutionPassed'] and native['compiledMidHookExecuted']
for name,value in native['sources'].items():assert sha(ROOT/name)==value,'Native source changed after verification'
for path_key,hash_key in [('source','sourceSha256'),('englishSource','englishSourceSha256'),('statusSource','statusSourceSha256')]:
 assert sha(ROOT/text[path_key])==text[hash_key].lower(),'Text source changed after staging'
for entry in text['reports']:
 assert entry['unrelatedEntriesAndMembersUnchanged']
 assert all(change['id']==95003000 for change in entry['changes']),'Unrelated text entry changed'
containers={e['path']:e for e in read(stage/'container-validation.json')['containers']} if (stage/'container-validation.json').exists() else {}
sources={'mod/natives/NRHex.dll':args.dll,'mod/regulation.bin':stage/'regulation.bin','mod/event/common.emevd.dcx':stage/'common.emevd.dcx'}
for entry in text['reports']:
 if entry['changes']:
  path=f"mod/msg/{entry['language']}/{entry['file']}";sources[path]=stage/'texts'/entry['language']/entry['file']
for relative,source in sources.items():
 assert source.is_file()
 if relative=='mod/event/common.emevd.dcx':expected=(containers.get(relative) or parameter)['inputSha256' if relative in containers else 'eventInputSha256']
 elif relative.startswith('mod/msg/'):
  e=next(e for e in text['reports'] if relative==f"mod/msg/{e['language']}/{e['file']}");expected=(containers.get(relative) or e)['inputSha256']
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
report=dict(version=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version'],changedAssets=changed,files={p:after[p] for p in changed},unrelatedAssetsAndUserConfigPreserved=True,parameters=parameter,native=native,texts=text,containers=containers,backup=str(backup),gameplayVerified=False,multiplayerVerified=False)
(dist/'death-dance-validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf8')
record_metadata(report,after)
print(f"PASS: {len(changed)} validated assets installed; other resources and user settings preserved")
