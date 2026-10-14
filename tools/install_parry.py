"""Install additive Parry package after parameter, text, animation and native checks."""
from pathlib import Path
import hashlib,json,shutil,subprocess,tomllib,sys
root=Path(__file__).resolve().parents[1];dist=root/'dist';work=root/'.work/parry';stage=work/'staged'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def read(p):return json.loads(p.read_text(encoding='utf8'))
if '--counter-update' in sys.argv:
 texts=read(root/'.work/texts/validation.json');support=read(stage/'counter-parameter-validation.json')
 assert not subprocess.run(['powershell','-NoProfile','-Command','Get-Process nightreign -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id'],capture_output=True,text=True).stdout.strip(),'Close the game before replacing assets'
 for name,key in [('source','sourceSha256'),('englishSource','englishSourceSha256'),('statusSource','statusSourceSha256')]:assert sha(root/texts[name]).upper()==texts[key].upper()
 updates={'mod/natives/NRHex.dll':root/'target/release/NRHex.dll'}
 assert support['originalRowsUnchanged'] and support['nativeFiltersUnchanged'] and support['attackCategory']==103 and support['multiplier']==2
 assert sha(dist/'mod/regulation.bin').upper()==support['inputSha256'] and sha(stage/'counter-regulation.bin').upper()==support['outputSha256']
 updates['mod/regulation.bin']=stage/'counter-regulation.bin'
 for e in texts['reports']:
  rel=(Path('mod/msg')/e['language']/e['file']).as_posix();src=root/'.work/texts'/e['language']/e['file']
  assert e['unrelatedEntriesAndMembersUnchanged'] and sha(dist/rel).upper()==e['inputSha256'] and sha(src).upper()==e['outputSha256']
  updates[rel]=src
 version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
 backup=work/f'before-counter-update-{version}'
 baseline={p.relative_to(dist).as_posix():sha(p) for p in (dist/'mod').rglob('*') if p.is_file() and p.suffix!='.log'}
 for rel in updates:
  p=backup/rel;p.parent.mkdir(parents=True,exist_ok=True)
  assert not p.exists(),'Inspect existing backup before repeating installation'
  shutil.copy2(dist/rel,p)
 try:
  for rel,src in updates.items():shutil.copy2(src,dist/rel);assert sha(dist/rel)==sha(src)
  assert all(sha(dist/rel)==value for rel,value in baseline.items() if rel not in updates)
 except BaseException:
  for rel in updates:shutil.copy2(backup/rel,dist/rel)
  raise
 for src,name in [(stage/'counter-parameter-validation.json','parry-counter-parameter-validation.json'),(root/'.work/texts/validation.json','parry-text-validation.json')]:shutil.copy2(src,dist/name)
 summary=read(dist/'validation.json');summary.update(codeVersion=version,currentIncrementalValidation='parry-counter-validation.json',currentStatusTextValidation='parry-text-validation.json',gameTested=False,gameplayVerified=False)
 (dist/'validation.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2),encoding='utf8')
 (dist/'parry-counter-validation.json').write_text(json.dumps(dict(version=version,changedAssets={rel:sha(dist/rel) for rel in updates},otherAssetsAndSettingsPreserved=True,gameStarted=False,gameplayVerified=False,multiplayerVerified=False),indent=2),encoding='utf8')
 print(f'PASS: {version} guard counter update installed; {len(updates)} assets verified; unrelated assets/settings preserved.')
 sys.exit(0)
if '--guard-update' in sys.argv:
 actions=read(stage/'actions-validation.json');texts=read(root/'.work/texts/validation.json');native=read(work/'native-validation.json');support=read(stage/'guard-parameter-validation.json')
 assert actions['guardJudge']==95002400 and actions['originalEventsPreserved'] and actions['originalGuardClipsUntouched']
 assert native['nativeCases']==27 and native['attackGuardStaminaWaived']
 assert (root/'src/parry_data.rs').read_bytes()==(stage/'parry_data.rs').read_bytes()
 assert not subprocess.run(['powershell','-NoProfile','-Command','Get-Process nightreign -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id'],capture_output=True,text=True).stdout.strip(),'Close the game before replacing assets'
 for name,key in [('source','sourceSha256'),('englishSource','englishSourceSha256'),('statusSource','statusSourceSha256')]:assert sha(root/texts[name]).upper()==texts[key].upper()
 updates={'mod/chr/c0000.anibnd.dcx':stage/'c0000.anibnd.dcx','mod/natives/NRHex.dll':root/'target/release/NRHex.dll'}
 assert support['originalRowsUnchanged'] and support['nativeCommonResource']
 assert sha(dist/'mod/regulation.bin').upper()==support['inputSha256'] and sha(stage/'guard-regulation.bin').upper()==support['outputSha256']
 updates['mod/regulation.bin']=stage/'guard-regulation.bin'
 assert sha(dist/'mod/chr/c0000.anibnd.dcx').upper()==actions['inputSha256']
 assert sha(stage/'c0000.anibnd.dcx').upper()==actions['outputSha256']
 for e in texts['reports']:
  rel=(Path('mod/msg')/e['language']/e['file']).as_posix();src=root/'.work/texts'/e['language']/e['file']
  assert e['unrelatedEntriesAndMembersUnchanged'] and sha(dist/rel).upper()==e['inputSha256'] and sha(src).upper()==e['outputSha256']
  updates[rel]=src
 version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
 backup=work/f'before-guard-update-{version}'
 baseline={p.relative_to(dist).as_posix():sha(p) for p in (dist/'mod').rglob('*') if p.is_file() and p.suffix!='.log'}
 for rel in updates:
  p=backup/rel;p.parent.mkdir(parents=True,exist_ok=True)
  assert not p.exists(),'Inspect existing backup before repeating installation'
  shutil.copy2(dist/rel,p)
 try:
  for rel,src in updates.items():shutil.copy2(src,dist/rel);assert sha(dist/rel)==sha(src)
  assert all(sha(dist/rel)==value for rel,value in baseline.items() if rel not in updates)
 except BaseException:
  for rel in updates:shutil.copy2(backup/rel,dist/rel)
  raise
 for src,name in [(stage/'actions-validation.json','parry-actions-validation.json'),(stage/'guard-parameter-validation.json','parry-support-validation.json'),(root/'.work/texts/validation.json','parry-text-validation.json'),(work/'native-validation.json','parry-native-validation.json')]:shutil.copy2(src,dist/name)
 summary=read(dist/'validation.json');summary.update(codeVersion=version,currentIncrementalValidation='parry-guard-validation.json',currentStatusTextValidation='parry-text-validation.json',gameTested=False,gameplayVerified=False)
 (dist/'validation.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2),encoding='utf8')
 (dist/'parry-guard-validation.json').write_text(json.dumps(dict(version=version,changedAssets={rel:sha(dist/rel) for rel in updates},otherAssetsAndSettingsPreserved=True,gameStarted=False,gameplayVerified=False,multiplayerVerified=False),indent=2),encoding='utf8')
 print(f'PASS: {version} attack guard update installed; {len(updates)} assets verified; unrelated assets/settings preserved.')
 sys.exit(0)
params=read(stage/'validation.json');texts=read(stage/'texts/validation.json');actions=read(stage/'actions-validation.json');native=read(work/'native-validation.json')
assert params['originalRowsUnchanged'] and params['abilityCount']==22
assert actions['originalEventsPreserved'] and actions['originalGuardClipsUntouched'] and native['nativeCases']==27
for name in ['hex_pool.rs','parry_data.rs']:assert (root/'src'/name).read_bytes()==(stage/name).read_bytes()
for name,key in [('source','sourceSha256'),('englishSource','englishSourceSha256'),('statusSource','statusSourceSha256')]:assert sha(root/texts[name]).upper()==texts[key].upper()
updates={}
def add(rel,source,before=None,after=None):
 if before:assert sha(dist/rel).upper()==before.upper(),f'Input changed: {rel}'
 if after:assert sha(source).upper()==after.upper(),f'Staged output changed: {rel}'
 updates[rel]=source
add('mod/regulation.bin',stage/'regulation.bin',params['inputSha256'],params['outputSha256'])
add('mod/chr/c0000.anibnd.dcx',stage/'c0000.anibnd.dcx',actions['inputSha256'],actions['outputSha256'])
for e in texts['reports']:
 assert e['unrelatedEntriesAndMembersUnchanged'];rel=Path(e['language'])/e['file']
 add((Path('mod/msg')/rel).as_posix(),stage/'texts'/rel,e['inputSha256'],e['outputSha256'])
add('mod/natives/NRHex.dll',root/'target/release/NRHex.dll')
process=subprocess.run(['powershell','-NoProfile','-Command','Get-Process nightreign -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id'],capture_output=True,text=True)
assert not process.stdout.strip(),'Close the game before replacing assets'
baseline={p.relative_to(dist).as_posix():sha(p) for p in (dist/'mod').rglob('*') if p.is_file() and p.suffix!='.log'}
backup=work/'before-install'
for rel in updates:
 p=backup/rel;p.parent.mkdir(parents=True,exist_ok=True);assert not p.exists(),'Inspect existing backup before reinstalling';shutil.copy2(dist/rel,p)
try:
 for rel,src in updates.items():shutil.copy2(src,dist/rel);assert sha(dist/rel)==sha(src)
 assert all(sha(dist/rel)==value for rel,value in baseline.items() if rel not in updates)
except BaseException:
 for rel in updates:shutil.copy2(backup/rel,dist/rel)
 raise
for src,name in [(stage/'validation.json','parry-parameter-validation.json'),(stage/'actions-validation.json','parry-actions-validation.json'),(stage/'texts/validation.json','parry-text-validation.json'),(work/'native-validation.json','parry-native-validation.json')]:shutil.copy2(src,dist/name)
version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
summary=read(dist/'validation.json');summary.update(codeVersion=version,hexes=read(stage/'catalog.json'),orders=9240,currentIncrementalValidation='parry-parameter-validation.json',currentStatusTextValidation='parry-text-validation.json',gameTested=False,gameplayVerified=False)
(dist/'validation.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2),encoding='utf8')
report=dict(version=version,changedAssets={rel:sha(dist/rel) for rel in updates},otherAssetsAndSettingsPreserved=True,gameStarted=False,gameplayVerified=False,multiplayerVerified=False)
(dist/'parry-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f'PASS: {version} installed; {len(updates)} assets verified, other assets/settings preserved.')
