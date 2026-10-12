"""Install the two staged hexes; never launch or terminate the game."""
from pathlib import Path
import hashlib,json,shutil,subprocess,re
root=Path(__file__).resolve().parents[1];stage=root/'.work/angel-chant-01740/staged';dist=root/'dist'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest().upper()
assert subprocess.run(['powershell','-NoProfile','-Command','if(Get-Process nightreign -ErrorAction SilentlyContinue){exit 1}else{exit 0}'],check=False).returncode==0,'Game is running; do not overwrite its package'
parameter=json.loads((stage/'validation.json').read_text(encoding='utf8'))
hks=json.loads((stage/'hks-validation.json').read_text(encoding='utf8'))
texts=json.loads((stage/'texts/validation.json').read_text(encoding='utf8'))
native=json.loads((stage/'native-validation.json').read_text(encoding='utf8'))
assert parameter['orders']==5814 and parameter['preservedOriginalTriples']==4080
assert hks['independentDecompilerMatchesAllOriginalBodies']
assert sha(root/'docs/hexes.md')==texts['sourceSha256']
assert sha(stage/'regulation.bin')==parameter['outputSha256']
assert sha(dist/'mod/regulation.bin') in [parameter['inputSha256'],parameter['outputSha256']]
assert sha(stage/'c0000.hks')==hks['outputSha256'].upper()
assert sha(dist/'mod/action/script/c0000.hks') in [hks['inputSha256'].upper(),hks['outputSha256'].upper()]
assert sha(root/'src/hex_pool.rs')==sha(stage/'hex_pool.rs')
files=[(root/'target/release/NRHex.dll',dist/'mod/natives/NRHex.dll'),(stage/'regulation.bin',dist/'mod/regulation.bin'),(stage/'c0000.hks',dist/'mod/action/script/c0000.hks')]
for r in texts['reports']:
 source=stage/'texts'/r['language']/r['file'];target=dist/'mod/msg'/r['language']/r['file']
 assert sha(source)==r['outputSha256'] and sha(target) in [r['inputSha256'],r['outputSha256']]
 if r['changes']:files.append((source,target))
backup=stage.parent/'before';backup.mkdir(exist_ok=True)
for source,target in files:
 saved=backup/target.relative_to(dist);saved.parent.mkdir(parents=True,exist_ok=True)
 if not saved.exists():shutil.copy2(target,saved)
 shutil.copy2(source,target);assert sha(source)==sha(target)
report=dict(version='0.17.40',rustTests=199,signatureChecks=85,nativeIsolatedCases=native['cases'],hksGateCases=hks['cases'],mechanics=dict(angelWindowSeconds=1/15,angelBeforeCrystal=True,nativeNullifyVfx=True,chantHeroes=[4,6,7],chantStaminaPerSecond=69,chantOnlyHeldCharge=True,nativeInterruptedReleaseChecksPreserved=True),files={str(t.relative_to(dist)).replace('\\','/'):sha(t) for _,t in files},loadingVerified=False,gameplayVerified=False,multiplayerVerified=False,gameLaunched=False)
for source,target in [('validation.json','angel-chant-parameter-validation.json'),('hks-validation.json','angel-chant-hks-validation.json'),('texts/validation.json','angel-chant-text-validation.json'),('native-validation.json','angel-chant-native-validation.json')]:shutil.copy2(stage/source,dist/target)
(dist/'angel-chant-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
p=dist/'validation.json';v=json.loads(p.read_text(encoding='utf8'))
v.update(hexes=parameter['hexes'],orders=5814,codeVersion='0.17.40',gameTested=False,outputRegulationSha256=parameter['outputSha256'],currentIncrementalValidation='angel-chant-validation.json')
p.write_text(json.dumps(v,ensure_ascii=False,indent=2),encoding='utf8')
print('PASS: 0.17.40 DLL, 19-hex pool, HKS and bilingual texts installed; game not launched')
