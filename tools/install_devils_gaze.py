"""Install the checked incremental Demon’s Gaze assets without unrelated edits."""
from distribution_paths import distribution_file
from pathlib import Path
import hashlib,json,shutil,subprocess,tomllib

root=Path(__file__).resolve().parents[1];dist=root/'dist';work=root/'.work/devils-gaze';stage=work/'staged'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def report(name):return json.loads((stage/name).read_text(encoding='utf-8'))
params=report('validation.json');resources=report('resources-validation.json');texts=json.loads((stage/'texts/validation.json').read_text())
native=json.loads((work/'native-validation.json').read_text())
assert params['originalRowsUnchanged'] and params['abilityCount']==20
assert resources['existingResourcesPreserved'] and native['nativeNetworkFieldChecksPassed']
assert (root/'src/hex_pool.rs').read_bytes()==(stage/'hex_pool.rs').read_bytes()
version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
updates={}
def add(relative,source,before=None,after=None):
 if before:assert sha(distribution_file(dist,relative)).upper()==before.upper(),f'Input changed: {relative}'
 if after:assert sha(source).upper()==after.upper(),f'Stage changed: {relative}'
 updates[relative]=source
add('mod/regulation.bin',stage/'regulation.bin',params['inputSha256'],params['outputSha256'])
add('mod/sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx',stage/'sfxbnd_commoneffects_dlc01.ffxbnd.dcx',resources['inputSha256'],resources['outputSha256'])
for name,key in [('source','sourceSha256'),('englishSource','englishSourceSha256'),('statusSource','statusSourceSha256')]:
 assert sha(root/texts[name]).upper()==texts[key].upper(),f'Text edited since staging: {texts[name]}'
for entry in texts['reports']:
 assert entry['unrelatedEntriesAndMembersUnchanged']
 rel=Path(entry['language'])/entry['file']
 add((Path('mod/msg')/rel).as_posix(),stage/'texts'/rel,entry['inputSha256'],entry['outputSha256'])
add('mod/natives/NRHex.dll',root/'target/release/NRHex.dll')
process=subprocess.run(['powershell','-NoProfile','-Command','Get-Process nightreign -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id'],capture_output=True,text=True)
assert not process.stdout.strip(),'Close the running game before installation'
baseline={p.relative_to(dist).as_posix():sha(p) for p in (dist/'mod').rglob('*') if p.is_file() and p.suffix!='.log'}
backup=work/'before-install'
for rel in updates:
 dest=backup/rel;dest.parent.mkdir(parents=True,exist_ok=True)
 assert not dest.exists(),'Previous installation backup exists; inspect it before reinstalling'
 shutil.copy2(distribution_file(dist,rel),dest)
try:
 for rel,source in updates.items():shutil.copy2(source,distribution_file(dist,rel));assert sha(distribution_file(dist,rel))==sha(source)
 assert all(sha(distribution_file(dist,rel))==digest for rel,digest in baseline.items() if rel not in updates)
except BaseException:
 for rel in updates:shutil.copy2(backup/rel,distribution_file(dist,rel))
 raise
for source,name in [(stage/'validation.json','devils-gaze-parameter-validation.json'),(stage/'resources-validation.json','devils-gaze-resource-validation.json'),(stage/'texts/validation.json','devils-gaze-text-validation.json'),(work/'native-validation.json','devils-gaze-native-validation.json')]:shutil.copy2(source,distribution_file(dist,name))
installed=dict(version=version,changedAssets={rel:sha(distribution_file(dist,rel)) for rel in updates},otherAssetsAndSettingsPreserved=True,gameStarted=False,gameplayVerified=False,multiplayerVerified=False)
(dist/'reports'/'devils-gaze-validation.json').write_text(json.dumps(installed,indent=2),encoding='utf-8')
print(f'PASS: {version} installed, {len(updates)} assets verified; unrelated assets/settings preserved')
