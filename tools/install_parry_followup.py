"""Install verified Parry final-notification and animation follow-up assets."""
from pathlib import Path
import hashlib,json,shutil,subprocess,tomllib
root=Path(__file__).resolve().parents[1];dist=root/'dist';stage=root/'.work/parry-fix/staged'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest().upper()
def read(p):return json.loads(p.read_text(encoding='utf8'))
params=read(stage/'parameters.json');actions=read(stage/'actions.json');native=read(root/'.work/parry/native-validation.json')
assert params['originalRowsUnchanged'] and actions['originalEventsPreserved'] and native['finalGuardNotificationVerified']
assert sum(map(len,actions['targets'].values()))==57
updates={'mod/regulation.bin':stage/'regulation.bin','mod/chr/c0000.anibnd.dcx':stage/'c0000.anibnd.dcx','mod/natives/NRHex.dll':root/'target/release/NRHex.dll'}
for rel,report in [('mod/regulation.bin',params),('mod/chr/c0000.anibnd.dcx',actions)]:
 assert sha(dist/rel)==report['inputSha256'] and sha(updates[rel])==report['outputSha256']
proc=subprocess.run(['powershell','-NoProfile','-Command','Get-Process nightreign -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id'],capture_output=True,text=True)
assert not proc.stdout.strip(),'Game must be closed before installation'
version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
backup=root/'.work/parry-fix'/('before-install-'+version);assert not backup.exists(),'Inspect existing backup first'
baseline={p.relative_to(dist).as_posix():sha(p) for p in dist.rglob('*') if p.is_file() and p.suffix!='.log'}
for rel in updates:
 p=backup/rel;p.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(dist/rel,p)
try:
 for rel,src in updates.items():shutil.copy2(src,dist/rel);assert sha(src)==sha(dist/rel)
 assert all(sha(dist/rel)==value for rel,value in baseline.items() if rel not in updates)
except BaseException:
 for rel in updates:shutil.copy2(backup/rel,dist/rel)
 raise
summary=read(dist/'validation.json');summary.update(codeVersion=version,currentIncrementalValidation='parry-followup-validation.json',gameTested=False,gameplayVerified=False)
(dist/'validation.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2),encoding='utf8')
report=dict(version=version,parameters=params,actions=actions,native=native,changedAssets={rel:sha(dist/rel) for rel in updates},otherAssetsAndSettingsPreserved=True,gameplayVerified=False,multiplayerVerified=False)
(dist/'parry-followup-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print('PASS: '+version+' installed; DLL, one neutral effect and reaction markers updated; other assets/settings preserved.')
