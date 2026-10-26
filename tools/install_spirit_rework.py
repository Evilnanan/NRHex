"""Install validated spirit-only parameter/text changes and current DLL."""
from distribution_paths import distribution_file
from pathlib import Path
import hashlib,json,shutil,subprocess,tomllib
root=Path(__file__).resolve().parents[1]
stage=root/'.work/spirit-rework-01743/staged';dist=root/'dist'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest().upper()
assert tomllib.loads((root/'Cargo.toml').read_text(encoding='utf8'))['package']['version']=='0.17.43'
assert subprocess.run(['powershell','-NoProfile','-Command','if(Get-Process nightreign -ErrorAction SilentlyContinue){exit 1}else{exit 0}']).returncode==0,'Close game before install'
parameters=json.loads((stage/'parameter-validation.json').read_text(encoding='utf8'))
texts=json.loads((stage/'texts/validation.json').read_text(encoding='utf8'))
assert sha(root/'docs/hexes.md')==texts['sourceSha256']
assert sha(stage/'regulation.bin')==parameters['outputSha256']
assert sha(dist/'mod/regulation.bin') in [parameters['inputSha256'],parameters['outputSha256']]
assert '198 passed; 0 failed' in (stage.parent/'tests.log').read_text(encoding='utf8')
assert 'PASS: current EXE hash' in (stage.parent/'scancheck.log').read_text(encoding='utf8')
files=[(root/'target/release/NRHex.dll',dist/'natives/NRHex.dll'),(stage/'regulation.bin',dist/'mod/regulation.bin')]
for r in texts['reports']:
    source=stage/'texts'/r['language']/r['file'];target=dist/'mod/msg'/r['language']/r['file']
    assert sha(source)==r['outputSha256'] and sha(target) in [r['inputSha256'],r['outputSha256']]
    if r['changes']:files.append((source,target))
backup=stage.parent/'install-before'
for source,target in files:
    saved=backup/target.relative_to(dist);saved.parent.mkdir(parents=True,exist_ok=True)
    if not saved.exists():shutil.copy2(target,saved)
    shutil.copy2(source,target);assert sha(source)==sha(target)
for source,target in [('parameter-validation.json','spirit-rework-parameter-validation.json'),('texts/validation.json','spirit-rework-text-validation.json')]:shutil.copy2(stage/source,distribution_file(dist,target))
report=dict(version='0.17.43',rustTestsPassed=198,signatureChecksPassed=88,
            mechanics=dict(trigger='Ultimate Art',nativeDrainMaxHpFractionPerSecond=1/60,damageMultiplier=.1,forcedExpiry=False,captureSfx=461351,summonSfx=460801,placementHelpers=['3886A0','385410'],nativeSlotSelector='3922C0',failedPlacementPreservesSoul=True),
            files={str(t.relative_to(dist)).replace('\\','/'):sha(t) for _,t in files},gameplayVerified=False,multiplayerVerified=False,gameLaunched=False,
            limitations='Placement ABI tested with controlled geometry callbacks; full geometry, visible cues and damage need in-game testing. Native HP loss obeys native healing/immunity rules.')
(dist/'reports'/'spirit-rework-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
p=dist/'reports'/'validation.json';v=json.loads(p.read_text(encoding='utf8'));h=next(h for h in v['hexes'] if h['Id']==95001800)
for r in texts['reports']:
    for change in r['changes']:
        if change['id']!=95001800:continue
        if change['fmg']=='PermanentBuffName.fmg':h['Zh' if r['language']=='zhocn' else 'En']=change['after']
        if change['fmg']=='PermanentBuffInfo.fmg':h['ZhInfo' if r['language']=='zhocn' else 'EnInfo']=change['after']
v.update(codeVersion='0.17.43',gameTested=False,outputRegulationSha256=parameters['outputSha256'],currentIncrementalValidation='spirit-rework-validation.json')
p.write_text(json.dumps(v,ensure_ascii=False,indent=2),encoding='utf8')
print('PASS: 0.17.43 DLL, spirit parameters and bilingual text installed with backup; game not launched')
