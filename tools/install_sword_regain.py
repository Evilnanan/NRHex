"""Install the staged native-regain update without replacing unrelated assets."""
from pathlib import Path
from datetime import datetime
import hashlib,json,re,shutil,tomllib

root=Path(__file__).resolve().parents[1]
dist=root/'dist';work=root/'.work/sword-regain';texts=root/'.work/texts'

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def assets():return {path.relative_to(dist).as_posix():sha(path) for path in (dist/'mod').rglob('*') if path.is_file() and path.suffix!='.log'}

before=assets()
parameter=json.loads((work/'parameter-validation.json').read_text())
native=json.loads((work/'native-validation.json').read_text())
text_report=json.loads((texts/'validation.json').read_text(encoding='utf-8-sig'))
assert parameter['onlyIntendedFieldChanged'] and parameter['nativeState']==449
assert native['nativeCases']==28 and native['sharedPool'] and not native['customHpWrites']
assert native['nativeTierCases']==12 and native['privateWeaponDescriptor'] and native['originalHitUnchanged']
sources={'mod/natives/NRHex.dll':root/'target/release/NRHex.dll','mod/regulation.bin':work/'regulation.bin'}
assert before['mod/regulation.bin']==parameter['inputSha256'].lower()
assert sha(sources['mod/regulation.bin'])==parameter['outputSha256'].lower()
for path_key,hash_key in [('source','sourceSha256'),('englishSource','englishSourceSha256'),('statusSource','statusSourceSha256')]:
 assert sha(root/text_report[path_key])==text_report[hash_key].lower(),'Text source changed after staging'
expected={f'{language}/{filename}_dlc01.msgbnd.dcx' for language in ['zhocn','engus'] for filename in ['item','menu']}
assert {f"{entry['language']}/{entry['file']}" for entry in text_report['reports']}==expected
for entry in text_report['reports']:
 relative=f"{entry['language']}/{entry['file']}";target=f'mod/msg/{relative}'
 assert entry['unrelatedEntriesAndMembersUnchanged']
 assert before[target]==entry['inputSha256'].lower(),'Text target changed after staging'
 assert sha(texts/relative)==entry['outputSha256'].lower(),'Text stage changed'
 if entry['inputSha256']!=entry['outputSha256']:sources[target]=texts/relative
backup=work/'before'/datetime.now().strftime('%Y%m%d-%H%M%S-%f')
for relative in sources:
 target=backup/relative;target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(dist/relative,target)
copied=[]
try:
 for relative,source in sources.items():
  copied.append(relative);shutil.copyfile(source,dist/relative);assert sha(dist/relative)==sha(source)
 after=assets();changed=sorted(relative for relative in before.keys()|after.keys() if before.get(relative)!=after.get(relative))
 assert set(changed)<=sources.keys(),changed
except Exception:
 for relative in reversed(copied):
  if sha(dist/relative)!=sha(backup/relative):shutil.copyfile(backup/relative,dist/relative)
 raise
version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
report=dict(version=version,changedAssets=changed,backup=str(backup),unrelatedAssetsAndUserConfigPreserved=True,files={relative:after[relative] for relative in sources},parameters=parameter,native=native,texts=text_report,gameplayVerified=False,multiplayerVerified=False)
(dist/'sword-regain-validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
signature_count=int(re.search(r"fn signatures\(\)->\[\(&'static str,u32\);(\d+)\]",(root/'src/lib.rs').read_text()).group(1))
build=dict(version=version,buildPassed=True,scope='Incremental Sword native regain and descriptions',dllSha256=after['mod/natives/NRHex.dll'],signatureCount=signature_count,validation='sword-regain-validation.json',gameplayVerified=False,multiplayerVerified=False)
(dist/'build-validation.json').write_text(json.dumps(build,indent=2),encoding='utf-8')
print(f'PASS: {version} Sword native regain installed; {len(changed)} assets changed; unrelated assets and settings preserved')
