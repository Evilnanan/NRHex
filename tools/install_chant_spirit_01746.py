"""Install checked 0.17.46 files only after the game has exited; do not launch."""
from pathlib import Path
import hashlib,json,shutil,subprocess,tomllib
root=Path(__file__).resolve().parents[1];work=root/'.work/chant-spirit-01746';dist=root/'dist'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest().upper()
assert tomllib.loads((root/'Cargo.toml').read_text(encoding='utf8'))['package']['version']=='0.17.46'
assert '202 passed; 0 failed' in (work/'tests.log').read_text(encoding='utf8')
assert 'PASS: current EXE hash' in (work/'signatures.log').read_text(encoding='utf8')
assert 'Finished' in (work/'build.log').read_text(encoding='utf8')
n=json.loads((work/'native-validation.json').read_text());assert n['cases']==516
p=json.loads((work/'parameter-validation.json').read_text())
assert sha(work/'regulation.bin')==p['outputSha256']
assert sha(dist/'mod/regulation.bin') in [p['inputSha256'],p['outputSha256']]
t=json.loads((work/'texts/validation.json').read_text())
assert sha(root/'docs/hexes.md')==t['sourceSha256']
files=[(work/'NRHex.dll',dist/'mod/natives/NRHex.dll'),(work/'regulation.bin',dist/'mod/regulation.bin'),(work/'README.md',dist/'README.md')]
for row in t['reports']:
 a=work/'texts'/row['language']/row['file'];b=dist/'mod/msg'/row['language']/row['file']
 assert sha(a)==row['outputSha256'] and sha(b) in [row['inputSha256'],row['outputSha256']]
 files.append((a,b))
assert sha(work/'NRHex.dll')==sha(root/'target/release/NRHex.dll'),'Build changed since staging'
assert subprocess.run(['powershell','-NoProfile','-Command','if(Get-Process nightreign -ErrorAction SilentlyContinue){exit 1}else{exit 0}']).returncode==0,'Game still running; preserve current test'
for a,b in files:
 saved=work/'install-before'/b.relative_to(dist);saved.parent.mkdir(parents=True,exist_ok=True)
 if not saved.exists():shutil.copy2(b,saved)
 shutil.copy2(a,b);assert sha(a)==sha(b)
for name,target in [('parameter-validation.json','spirit-damage-parameter-validation.json'),('native-validation.json','spirit-damage-native-validation.json'),('texts/validation.json','chant-text-validation.json')]:shutil.copy2(work/name,dist/target)
report=dict(version='0.17.46',rustTestsPassed=202,signatureChecksPassed=89,nativeClassificationCases=516,chantStaminaPerSecond=30,spiritDamageMultiplier=.25,spiritClassification='native family/player-side damage',privateNpcRows=p['privateNpcRows'],files={str(b.relative_to(dist)).replace('\\','/'):sha(b) for _,b in files},gameplayVerified=False,multiplayerVerified=False)
(dist/'chant-spirit-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
v=json.loads((dist/'validation.json').read_text(encoding='utf8'))
v.update(version='0.17.46',codeVersion='0.17.46',gameTested=False,outputRegulationSha256=p['outputSha256'],currentIncrementalValidation='chant-spirit-validation.json',currentStatusTextValidation='chant-text-validation.json',validation='Chant 30 stamina/s and private spirit damage classification; previous records remain historical. Gameplay and multiplayer pending.')
for h in v['hexes']:
 if h['Id']==95002000:
  h['ZhInfo']=h['ZhInfo'].replace('每秒回复15点精力值','每秒回复30点精力值');h['EnInfo']=h['EnInfo'].replace('Restore 15 stamina','Restore 30 stamina')
(dist/'validation.json').write_text(json.dumps(v,ensure_ascii=False,indent=2),encoding='utf8')
print('PASS: 0.17.46 installed with backups; all file hashes matched; game not launched')
