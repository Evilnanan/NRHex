"""Install the checked 0.17.44 DLL and minimal corpse-cue parameter patch."""
from distribution_paths import distribution_file
from pathlib import Path
import hashlib,json,shutil,subprocess,tomllib
root=Path(__file__).resolve().parents[1];work=root/'.work/spirit-placement-01744';stage=work/'staged';dist=root/'dist'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest().upper()
assert tomllib.loads((root/'Cargo.toml').read_text(encoding='utf8'))['package']['version']=='0.17.44'
assert '204 passed; 0 failed' in (work/'tests.log').read_text(encoding='utf8')
assert 'PASS: current EXE hash' in (work/'signatures.log').read_text(encoding='utf8')
assert 'Finished' in (work/'build.log').read_text(encoding='utf8')
p=json.loads((stage/'parameter-validation.json').read_text(encoding='utf8'))
assert sha(stage/'regulation.bin')==p['outputSha256']
assert sha(dist/'mod/regulation.bin') in [p['inputSha256'],p['outputSha256']]
assert subprocess.run(['powershell','-NoProfile','-Command','if(Get-Process nightreign -ErrorAction SilentlyContinue){exit 1}else{exit 0}']).returncode==0,'Close game before installing'
files=[(root/'target/release/NRHex.dll',dist/'mod/natives/NRHex.dll'),(stage/'regulation.bin',dist/'mod/regulation.bin'),(stage/'README.md',dist/'README.md')]
for source,target in files:
 saved=work/'install-before'/target.relative_to(dist);saved.parent.mkdir(parents=True,exist_ok=True)
 if not saved.exists():shutil.copy2(target,saved)
 shutil.copy2(source,target);assert sha(source)==sha(target)
for source,name in [(stage/'parameter-validation.json','spirit-placement-parameter-validation.json'),(work/'native-validation.json','spirit-placement-native-validation.json')]:shutil.copy2(source,distribution_file(dist,name))
report=dict(version='0.17.44',rustTestsPassed=204,signatureChecksPassed=89,originalSweepWrapperCases=16,retryGameSeconds=.5,requestSurvivesPlacementFailure=True,requestCancelledOnDeath=True,consumeOnlyAfterCreation=True,captureCueTarget='defeated NPC handle',groundProbeRadius=.4,destinationRadius='unchanged NpcParam.hitRadius',searchDirections=8,searchRings=2,files={str(t.relative_to(dist)).replace('\\','/'):sha(t) for _,t in files},gameplayVerified=False,multiplayerVerified=False)
(dist/'reports'/'spirit-placement-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
v=json.loads((dist/'reports'/'validation.json').read_text(encoding='utf8'));v.update(codeVersion='0.17.44',gameTested=False,outputRegulationSha256=p['outputSha256'],currentIncrementalValidation='spirit-placement-validation.json')
(dist/'reports'/'validation.json').write_text(json.dumps(v,ensure_ascii=False,indent=2),encoding='utf8')
print('PASS: 0.17.44 installed with backups; all copied hashes matched')
