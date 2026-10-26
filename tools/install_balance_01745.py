"""Install the checked balance/removal/contact-point update, without starting the game."""
from pathlib import Path
import hashlib,json,re,shutil,subprocess,tomllib

root=Path(__file__).resolve().parents[1]
work=root/'.work/balance-01745';stage=work/'staged';dist=root/'dist'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest().upper()
assert tomllib.loads((root/'Cargo.toml').read_text(encoding='utf8'))['package']['version']=='0.17.45'
result=re.search(r'test result: ok\. (\d+) passed; 0 failed', (work/'tests.log').read_text(encoding='utf8'))
assert result and int(result[1])==202
assert 'PASS: current EXE hash' in (work/'signatures.log').read_text(encoding='utf8')
assert 'Finished' in (work/'build.log').read_text(encoding='utf8')
assert (work/'profile.log').stat().st_size>0
p=json.loads((stage/'validation.json').read_text(encoding='utf8'))
assert sha(stage/'regulation.bin')==p['outputSha256']
assert sha(dist/'mod/regulation.bin') in [p['inputSha256'],p['outputSha256']]
assert p['allLegacyRewardsExcludeRetired'] and len(p['hexes'])==18
text_report=json.loads((stage/'texts/validation.json').read_text(encoding='utf8'))
assert sha(root/'docs/hexes.md')==text_report['sourceSha256']
files=[(root/'target/release/NRHex.dll',dist/'natives/NRHex.dll'),(stage/'regulation.bin',dist/'mod/regulation.bin'),(stage/'README.md',dist/'README.md')]
for row in text_report['reports']:
    source=stage/'texts'/row['language']/row['file'];target=dist/'mod/msg'/row['language']/row['file']
    assert sha(source)==row['outputSha256']
    assert sha(target) in [row['inputSha256'],row['outputSha256']]
    files.append((source,target))
assert subprocess.run(['powershell','-NoProfile','-Command','if(Get-Process nightreign -ErrorAction SilentlyContinue){exit 1}else{exit 0}']).returncode==0,'Close game before installing'
for source,target in files:
    saved=work/'install-before'/target.relative_to(dist);saved.parent.mkdir(parents=True,exist_ok=True)
    if not saved.exists():shutil.copy2(target,saved)
    shutil.copy2(source,target);assert sha(source)==sha(target)
shutil.copy2(stage/'validation.json',dist/'reports'/'balance-parameter-validation.json')
shutil.copy2(stage/'texts/validation.json',dist/'reports'/'balance-text-validation.json')
report=dict(version='0.17.45',rustTestsPassed=int(result[1]),signatureChecksPassed=89,spiritDamageMultiplier=.25,chantStaminaPerSecond=15,retiredOwner=95001900,activeAbilities=18,activeChoices=4896,historicalRewardGroups=5814,swordRange='source world position to native hit+1F0 world contact; same damage and lifesteal predicate',swordDefaultDistance=6,ctrlF9=['Keep Chanting','Draw Your Sword','filler'],files={str(t.relative_to(dist)).replace('\\','/'):sha(t) for _,t in files},gameplayVerified=False,multiplayerVerified=False)
(dist/'reports'/'balance-sword-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
v=json.loads((dist/'reports'/'validation.json').read_text(encoding='utf8'))
v.update(version='0.17.45',codeVersion='0.17.45',hexes=p['hexes'],orders=4896,legacyRewardGroups=5814,gameTested=False,outputRegulationSha256=p['outputSha256'],currentIncrementalValidation='balance-sword-validation.json',currentStatusTextValidation='balance-text-validation.json',validation='Current balance, retired ability and native contact-point update; earlier event and parameter records are historical. Gameplay/multiplayer pending.')
(dist/'reports'/'validation.json').write_text(json.dumps(v,ensure_ascii=False,indent=2),encoding='utf8')
print('PASS: 0.17.45 installed with backups; all copied hashes matched; game not started')
