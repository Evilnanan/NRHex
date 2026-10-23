"""Install validated Ultimate-triggered Necromancy assets without launching game."""
from distribution_paths import distribution_file
from pathlib import Path
import hashlib,json,shutil,subprocess
root=Path(__file__).resolve().parents[1];stage=root/'.work/spirit-ultimate-01739';dist=root/'dist'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest().upper()
assert subprocess.run(['powershell','-NoProfile','-Command','if(Get-Process nightreign -ErrorAction SilentlyContinue){exit 1}else{exit 0}'],check=False).returncode==0,'Close the game before installing'
parameter=json.loads((stage/'parameter-validation.json').read_text(encoding='utf8'))
hks=json.loads((stage/'hks-validation.json').read_text(encoding='utf8'))
texts=json.loads((stage/'texts/validation.json').read_text(encoding='utf8'))
native=json.loads((stage/'native-validation.json').read_text(encoding='utf8'))
assert sha(root/'docs/hexes.md')==texts['sourceSha256']
assert sha(stage/'regulation.bin')==parameter['outputSha256']
assert sha(dist/'mod/regulation.bin') in [parameter['inputSha256'],parameter['outputSha256']]
assert sha(stage/'c0000.hks')==hks['outputSha256'].upper()
assert sha(dist/'mod/action/script/c0000.hks') in [hks['inputSha256'].upper(),hks['outputSha256'].upper()]
files=[(root/'target/release/NRHex.dll',dist/'mod/natives/NRHex.dll'),(stage/'regulation.bin',dist/'mod/regulation.bin'),(stage/'c0000.hks',dist/'mod/action/script/c0000.hks')]
for r in texts['reports']:
 source=stage/'texts'/r['language']/r['file'];target=dist/'mod/msg'/r['language']/r['file']
 assert sha(source)==r['outputSha256'] and sha(target) in [r['inputSha256'],r['outputSha256']]
 if r['changes']:files.append((source,target))
backup=stage/'before';backup.mkdir(exist_ok=True)
for source,target in files:
 saved=backup/target.relative_to(dist);saved.parent.mkdir(parents=True,exist_ok=True)
 if not saved.exists():shutil.copy2(target,saved)
 shutil.copy2(source,target);assert sha(source)==sha(target)
report=dict(version='0.17.39',rustTests=192,signatureChecks=83,nativeIsolatedCases=native['cases'],mechanics=dict(triggerEffect=6999615,requiresEmptyFamily=False,consumesSoulOnSuccess=True,forwardMetres=5,periodicHpLoss=False,lifetimeSeconds=15,usesGameTime=True,forcedDeath='3D5010(actor, false, false)',rewardsDisabled=True),files={str(t.relative_to(dist)).replace('\\','/'):sha(t) for _,t in files},nativeValidation='spirit-expiry-native-validation.json',parameterValidation='spirit-ultimate-parameter-validation.json',hksValidation='spirit-ultimate-hks-validation.json',textValidation='spirit-ultimate-text-validation.json',loadingVerified=False,gameplayVerified=False,multiplayerVerified=False,gameLaunched=False)
for source,target in [('parameter-validation.json','spirit-ultimate-parameter-validation.json'),('hks-validation.json','spirit-ultimate-hks-validation.json'),('texts/validation.json','spirit-ultimate-text-validation.json'),('native-validation.json','spirit-expiry-native-validation.json')]:shutil.copy2(stage/source,distribution_file(dist,target))
(dist/'reports'/'spirit-ultimate-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
# Preserve the historical full-build report while recording current metadata.
p=dist/'reports'/'validation.json';v=json.loads(p.read_text(encoding='utf8'))
hex=next(h for h in v['hexes'] if h['Id']==95001800)
for r in texts['reports']:
 for change in r['changes']:
  if change['id']!=95001800:continue
  if change['fmg']=='PermanentBuffName.fmg':hex['Zh' if r['language']=='zhocn' else 'En']=change['after']
  if change['fmg']=='PermanentBuffInfo.fmg':hex['ZhInfo' if r['language']=='zhocn' else 'EnInfo']=change['after']
v.update(codeVersion='0.17.39',gameTested=False,outputRegulationSha256=parameter['outputSha256'],currentIncrementalValidation='spirit-ultimate-validation.json')
p.write_text(json.dumps(v,ensure_ascii=False,indent=2),encoding='utf8')
print('PASS: 0.17.39 DLL, parameters, HKS and bilingual texts installed; game not launched')
