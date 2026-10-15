"""Read back reworked native rates, timer/phase links and reset scope.

The game applies periodic HP changes and the replacement chain; these checks
verify the serialized instructions/parameters, not real gameplay.
"""
import hashlib,json
from pathlib import Path
from local_paths import ROOT

work=ROOT/'.work/night-rework';stage=work/'staged'
d=json.loads((stage/'validation.json').read_text())
effects={r['ID']:r['fields'] for r in d['effects']}
for heal,token,drain,flat in [(95002501,95002508,95002510,15),(95002502,95002509,95002511,30)]:
    h,t,b=effects[heal],effects[token],effects[drain]
    assert (h['changeHpRate'],h['changeHpPoint'],h['motionInterval'],h['effectEndurance'],h['replaceSpEffectId'])==(-4,-2*flat,1,15,token)
    assert h['bCurrHPIndependeMaxHP']==b['bCurrHPIndependeMaxHP']==0
    assert t['effectEndurance']==-1 and t['changeHpRate']==t['changeHpPoint']==0 and t['replaceSpEffectId']==-1
    assert (b['changeHpRate'],b['changeHpPoint'],b['motionInterval'],b['effectEndurance'],b['replaceSpEffectId'])==(2,flat,1,-1,-1)
    assert h['isDisableNetSync']==t['isDisableNetSync']==b['isDisableNetSync']==1
attack=effects[95002503]
assert all(abs(attack[f]-1.2)<1e-6 for f in ['physicsAttackRate','magicAttackRate','fireAttackRate','thunderAttackRate','darkAttackRate'])
assert all(attack[f]==1 for f in ['magParamChange','miracleParamChange','shamanParamChange'])
receipt=effects[95002505];grace=effects[95002506]
assert (receipt['saveCategory'],receipt['dontDeleteOnDead'],receipt['effectEndurance'],receipt['isDisableNetSync'])==(10,1,-1,1)
assert (grace['effectEndurance'],grace['isDisableNetSync'])==(-1,1)
assert effects[95002507]['effectEndurance']==0
assert (effects[95002504]['vfxId'],effects[95002504]['vfxId1'])==(1643100,1642000)
assert d['onlySupportEffectsAndReceiptResetsChanged'] and d['originalRowsAndOtherTablesPreserved'] and d['originalEventsAndMetadataPreserved']
assets=json.loads((ROOT/'.work/night-blessing/staged/assets-validation.json').read_text())
assert assets['retiredGestureClips']==[81400,81401] and assets['originalAnimationEventsPreserved'] and assets['originalGraceCommandsPreserved']
assert hashlib.sha256((ROOT/'.work/night-blessing/staged/c0000.anibnd.dcx').read_bytes()).hexdigest().upper()==assets['animationOutputSha256']
texts=json.loads((stage/'texts/validation.json').read_text())
assert all(r['unrelatedEntriesAndMembersUnchanged'] and all(c['id'] in (95002500,95002503) for c in r['changes']) for r in texts['reports'])
report=dict(checks=['two native rain-based rates','15-second native healing lifetime and two neutral replacement tokens','unlimited backlash only requested in rain','saved one-use receipt surviving death','persistent consumed grace notification','20-percent attack across all five types and spell kinds','obsolete gesture markers removed','only own support rows and two receipt resets changed','bilingual ability and attack status texts'],backlashSeconds=None,backlashEndsOnRainExit=True,gameplayVerified=False,multiplayerVerified=False)
(work/'checks.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print('PASS: native regeneration/backlash rates, lifetimes, phase links, local saved receipt, gesture removal and bilingual text read back')
