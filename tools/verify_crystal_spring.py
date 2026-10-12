"""Read back the current original crystal/pseudo-stream launch path."""
from pathlib import Path
import hashlib,json,re
from hks_bytecode import Reader
root=Path(__file__).resolve().parents[1]
original=root/'assets/original/action/script/c0000.hks'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(original)=='2ec9f8ad913a8a6f1cf4a6f4794692a59e1fecda156bc9a87b3068f226c0dc1d'
r=Reader(original.read_bytes());r.function();jump=next(f for f in r.functions if f['name']=='ExecJump')
d=Reader((root/'dist/mod/action/script/c0000.hks').read_bytes());d.function();deployed=next(f for f in d.functions if f['name']=='ExecJump')
assert jump['code']==deployed['code'] and jump['constants']==deployed['constants']
text=(root/'data/reference/actions/exec-jump.lua').read_text(encoding='utf-8-sig')
assert 'if highJumpHeight > 0 or env(1116, 102501) == TRUE or env(1116, 98215) == TRUE then' in text
assert re.search(r'elseif env\(1116, 98215\) == TRUE then\s+SetVariable\("IndexSuperJump", 7\)',text)
data=json.loads((root/'data/reference/actions/spiritspring.json').read_text())
region=next(r for r in data['mapRegions'] if r['Common_EntityID']==1047402502)
assert region['type'].endswith('+Other') and 'JumpHeight' not in region['fields']
event=next(e for e in data['crystalEvents'] if e['ID']==1047402505)
assert any(i['Bank']==2004 and i['ID']==8 and i['values']==[20000,98215] for i in event['instructions'])
assert any(p['SourceStartByte']==0 and p['TargetStartByte']==8 for p in event['Parameters'])
report=dict(version='0.10.10',originalHksSha256=sha(original),originalRegulationSha256=data['sourceRegulationSha256'],nativeExecJumpUnchanged=True,nativeEligibilityEffect=98215,nativeSuperJumpVariant=7,region=region,event=event,interpretation='Great Hollow pseudo-stream eligibility is independent of env366 region height; track the flight apex from this launch hint.',centralCrystalGameplayVerified=False,ordinarySpiritspringUserConfirmedForVersion='0.10.9',multiplayerVerified=False)
(root/'dist/crystal-spring-validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
print('PASS: current original ExecJump and Great Hollow pseudo-stream event use 98215; deployed jump bytecode unchanged.')
