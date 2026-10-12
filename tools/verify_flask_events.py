"""Independent control-flow checks against reopened EMEVD instruction dumps."""
from pathlib import Path
import json, struct
root=Path(__file__).resolve().parents[1]
original=json.loads((root/'data/reference/rewards/flask-events.json').read_text())
built=json.loads((root/'.work/rewards/flask-built-events.json').read_text())
before={(e['file'],e['ID']):e for e in original}
after={(e['file'],e['ID']):e for e in built}
for eid in [1145,1146,1303]:
    assert before['common',eid]['instructions']==after['common',eid]['instructions']
    assert before['common',eid]['Parameters']==after['common',eid]['Parameters']
def grants(e,owned,night_flag=False):
    pc=2 if e['ID']==1304 else 4
    charges=refills=0
    ins=e['instructions']
    while pc<len(ins):
        i=ins[pc]; b,n=i['Bank'],i['ID']; a=bytes.fromhex(i['hex'])
        if (b,n)==(1003,1):
            if night_flag: pc+=a[0]
        elif (b,n)==(1004,0):
            assert a[0]==6 and struct.unpack_from('<Ii',a,4)==(20000,95000500)
            assert a[12]==0 and struct.unpack_from('<i',a,16)[0]==1
            if not owned: pc+=a[0]
        elif (b,n)==(2004,86): charges+=1
        elif (b,n)==(2004,87): refills+=1
        elif (b,n)==(1000,4): break
        pc+=1
    return charges,refills
for key in [('common',1304)]:
    for flag in [False,True]:
        assert grants(after[key],False,flag)==(1,1)
        assert grants(after[key],True,flag)==(2,2)
assert before['common_func',90005919]['instructions']==after['common_func',90005919]['instructions']
assert before['common_func',90015208]['instructions']==after['common_func',90015208]['instructions']
churches=[('m41_00_00_00',41004700),('m41_01_00_00',41014700),('m41_30_00_00',41304700),('m51_00_00_00',51004700)]
for index,(name,flag) in enumerate(churches):
    rotted=name=='m41_30_00_00'
    eid=41302530 if rotted else flag-2180
    for key,e in before.items():
        if key[0]!=name: continue
        if key[1]==eid and not rotted:
            assert e['instructions']==after[key]['instructions'][:len(e['instructions'])]
            assert e['Parameters']==after[key]['Parameters']
        elif key[1]==0 and rotted:
            assert e['instructions']==[dict(i,index=i['index']-1) for i in after[key]['instructions'][1:]]
        else:assert e['instructions']==after[key]['instructions'] and e['Parameters']==after[key]['Parameters']
    ins=after[name,eid]['instructions']
    assert len(ins)==(10 if rotted else 12)
    assert eid//10000==flag//10000
    if not rotted:
        ctor=after[name,0]['instructions']
        assert any(i['Bank']==2000 and i['ID']==0 and i['values'][1]==eid for i in ctor)
    assert (ins[1]['Bank'],ins[1]['ID'],ins[1]['values'])==(1003,2,[256,flag])
    assert (ins[2]['Bank'],ins[2]['ID'],ins[2]['values'])==(3,0,[256,flag])
    start=3 if rotted else 5
    if not rotted:
        assert ins[3]['Bank']==2003 and ins[3]['ID']==18 and ins[3]['values'][:2]==[20000,60071]
        assert ins[4]['values']==[20000,99270]
    assert (ins[start]['Bank'],ins[start]['ID'])==(1004,2)
    assert struct.unpack_from('<Ii',bytes.fromhex(ins[start]['hex']),4)==(20000,95000500)
    assert bytes.fromhex(ins[start]['hex'])[12]==0
    assert ins[start+1]['values']==[20000,95000508] and ins[start+2]['values']==[3]
    assert (ins[start+3]['Bank'],ins[start+3]['ID'])==(2004,86)
    assert (ins[start+4]['Bank'],ins[start+4]['ID'])==(2004,87)
    assert ins[start+5]['values']==[1] and ins[start+6]['values']==[20000,95000507]
probe=after['common',9100901]['instructions']
assert (probe[5]['Bank'],probe[5]['ID'])==(2004,87)
assert probe[6]['values']==[1] and probe[7]['values']==[20000,95000506]
for count in range(21):
    e=after['common',9100910+count]; ins=e['instructions']
    assert sum(i['Bank']==2004 and i['ID']==86 for i in ins)==count
    assert [struct.unpack_from('<i',bytes.fromhex(i['hex']),8)[0] for i in ins[1:4]]==[95000500,95000502,95000520+count]
    assert [bytes.fromhex(i['hex'])[12] for i in ins[1:4]]==[1,0,1]
    marker=5+count+(2 if count>0 else 0)
    assert ins[marker]['values']==[20000,95000504]
    if count>0:
        assert (ins[marker-2]['Bank'],ins[marker-2]['ID'])==(2004,87)
        assert (ins[marker-1]['Bank'],ins[marker-1]['ID'],ins[marker-1]['values'])==(1001,1,[1])
report={'version':'0.6.6','secondNightBothBranchesVerified':True,'churchNativePickupTailsAndMapNamespaceVerified':churches,
        'alreadyOpenedChurchesAndNonOwnersSkipped':True,'originalChurchInstructionsPreserved':True,'firstNightAndStolenFlaskEventsUnchanged':True,
        'fullCapacityProbeBeforeAcquisitionVerified':True,
        'acquisitionCountsVerified':list(range(21)),'inGameFlaskBehaviorVerified':False}
(root/'dist/flask-event-validation.json').write_text(json.dumps(report,indent=2))
print('PASS: native church pickup tails and map namespace, owner/reload guards, capacity probe and preserved night/return flows verified.')
