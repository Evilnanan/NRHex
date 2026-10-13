"""Validate then install the staged Archmage addition; preserve unrelated assets."""
from pathlib import Path
import argparse,hashlib,json,shutil,subprocess,tomllib

ap=argparse.ArgumentParser();ap.add_argument('--check',action='store_true');args=ap.parse_args()
root=Path(__file__).resolve().parents[1];dist=root/'dist';work=root/'.work/archmage'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
report=json.loads((work/'staged/validation.json').read_text(encoding='utf-8'))
hks=json.loads((work/'hks-validation.json').read_text(encoding='utf-8'))
texts=json.loads((work/'staged/texts/validation.json').read_text(encoding='utf-8'))
assert report['allPreexistingRowsUnchanged'] and report['allOtherTablesUnchanged']
assert report['abilityCount']==19 and report['orderedChoices']==5814
assert hks['inverseRestoresInput'] and hks['originalBranchesUnchanged'] and hks['independentDecompilerVerified']
assert hks['gateCases']==1360 and len(hks['functions'])==34
updates={}
def add(relative,source,before=None,after=None):
 assert source.is_file()
 if before:assert sha(dist/relative).upper()==before.upper(),f'Input changed: {relative}'
 if after:assert sha(source).upper()==after.upper(),f'Staged file changed: {relative}'
 updates[relative]=source
add('mod/regulation.bin',work/'staged/regulation.bin',report['sourceSha256'],report['outputSha256'])
add('mod/action/script/c0000.hks',work/'c0000.hks',hks['inputSha256'],hks['outputSha256'])
for name,key in [('source','sourceSha256'),('englishSource','englishSourceSha256'),('statusSource','statusSourceSha256')]:
 assert sha(root/texts[name]).upper()==texts[key].upper(),f'Text source changed: {texts[name]}'
for entry in texts['reports']:
 assert entry['unrelatedEntriesAndMembersUnchanged']
 if entry['inputSha256']!=entry['outputSha256']:
  relative=Path(entry['language'])/entry['file']
  add((Path('mod/msg')/relative).as_posix(),work/'staged/texts'/relative,entry['inputSha256'],entry['outputSha256'])
assert (root/'src/hex_pool.rs').read_bytes()==(work/'staged/hex_pool.rs').read_bytes()
add('mod/natives/NRHex.dll',root/'target/release/NRHex.dll')
assert sha(root/'target/release/NRHex.dll')!=sha(dist/'mod/natives/NRHex.dll')
if args.check:
 print(f'PASS: {version} staged inputs, scope and all {len(updates)} installation files verified')
 raise SystemExit(0)
process=subprocess.run(['powershell','-NoProfile','-Command','Get-Process nightreign -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id'],capture_output=True,text=True)
assert not process.stdout.strip(),'Exit the current game before replacing its DLL'
baseline={p.relative_to(dist).as_posix():sha(p) for folder in ['mod','testmods'] for p in (dist/folder).rglob('*') if p.is_file() and p.suffix!='.log'}
baseline.update({p.name:sha(p) for p in dist.glob('*.me3')})
backup=work/'before-install';backup.mkdir(parents=True,exist_ok=True)
for relative in updates:
 target=backup/relative;target.parent.mkdir(parents=True,exist_ok=True)
 if not target.exists():shutil.copy2(dist/relative,target)
try:
 for relative,source in updates.items():
  target=dist/relative;target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(source,target);assert sha(target)==sha(source)
 assert all(sha(dist/relative)==digest for relative,digest in baseline.items() if relative not in updates)
except BaseException:
 for relative in updates:shutil.copy2(backup/relative,dist/relative)
 raise
installed=dict(version=version,changedAssets={relative:sha(dist/relative) for relative in updates},otherAssetsAndSettingsPreserved=True,parameterValidation='archmage-parameter-validation.json',actionValidation='archmage-hks-validation.json',textValidation='archmage-text-validation.json',nativeValidation='archmage-native-validation.json',gameplayVerified=False,multiplayerVerified=False,gameStarted=False)
for source,name in [(work/'staged/validation.json','archmage-parameter-validation.json'),(work/'hks-validation.json','archmage-hks-validation.json'),(work/'staged/texts/validation.json','archmage-text-validation.json'),(work/'native-validation.json','archmage-native-validation.json')]:shutil.copy2(source,dist/name)
(dist/'archmage-validation.json').write_text(json.dumps(installed,indent=2),encoding='utf-8')
print(f'PASS: {version} Archmage installed; all {len(updates)} files verified; unrelated assets/settings preserved')
