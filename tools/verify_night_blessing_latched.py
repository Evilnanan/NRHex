"""Read back both latched Night's Blessing stages; no gameplay claims."""
import argparse,json,hashlib
from pathlib import Path

def verify(stage):
    report=json.loads((stage/'validation.json').read_text())
    assert report['unrelatedRowsAndTablesUnchanged'] and report['nativeRainUnchanged'] and report['legacyHealAttackBacklashNeutralized']
    assert hashlib.sha256((stage/'regulation.bin').read_bytes()).hexdigest().upper()==report['outputSha256']
    effects={r['ID']:r['fields'] for r in report['effects']}
    assert set(effects)==set(range(95002501,95002512))
    for r in effects.values():
        assert r['changeHpRate']==r['changeHpPoint']==0 and r['replaceSpEffectId']==-1
        assert all(r[f]==1 for f in ['physicsAttackRate','magicAttackRate','fireAttackRate','thunderAttackRate','darkAttackRate'])
        assert r['isDisableNetSync']==1
    receipt=effects[95002505]
    assert (receipt['saveCategory'],receipt['dontDeleteOnDead'],receipt['effectEndurance'])==(10,1,-1)
    assert effects[95002506]['effectEndurance']==-1
    assert (effects[95002504]['vfxId'],effects[95002504]['vfxId1'])==(1643100,1642000)
    assert effects[95002503]['iconId']==22206 and effects[95002503]['spEffectTextId_1']==95002503
    texts=json.loads((stage/'texts/validation.json').read_text())
    for r in texts['reports']:
        assert r['unrelatedEntriesAndMembersUnchanged']
        assert all(c['id'] in (95002500,95002503) for c in r['changes'])
        assert hashlib.sha256((stage/'texts'/r['language']/r['file']).read_bytes()).hexdigest().upper()==r['outputSha256']
    print('PASS:',stage,'neutral support, saved latch, native rain/unrelated data preserved, bilingual readback')

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('stages',nargs='+',type=Path)
    for stage in p.parse_args().stages: verify(stage)
