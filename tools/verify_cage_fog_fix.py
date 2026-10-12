"""Reproduce the native pending-condition latch and verify staged birth bytes."""
from pathlib import Path
from collections import defaultdict
import hashlib,json,struct
from cage_local_events import Peer

root=Path(__file__).resolve().parents[1]
work=root/'.work/cage-fog-01729'
patch=json.loads((work/'patch.json').read_text())
proof=json.loads((work/'cage-condition-latch.json').read_text())
assert proof['group5CompiledMasks']==[0,16,16] and proof['mainConditionRemainedFalse']
schema={(o['bank'],o['id']):(o['name'],[t for t,_ in o['args']])
        for o in json.loads((root/'data/reference/world/cage-opcodes.json').read_text())['opcodes']}
schema.update({(2009,4):('ActivateMultiplayerdependantBuffs',['uint'])})
for i in range(10):schema[1014,i]=(f'Label{i}',[])
fmt=dict(sbyte='b',byte='B',bool='B',int='i',uint='I',short='h',ushort='H',float='f')
def decode(i):
    n,types=schema[i['Bank'],i['ID']];raw=bytes.fromhex(i['hex']);offset=0;args=[]
    for t in types:
        size=struct.calcsize(fmt[t]);offset=(offset+size-1)//size*size
        args.append(struct.unpack_from('<'+fmt[t],raw,offset)[0]);offset+=size
    assert len(raw)==(offset+3)//4*4
    return n,args

class NativeLatchPeer(Peer):
    # Native 5AA3B0 records true condition groups while the main wait is false;
    # 5ABD40 reads the saved bit, 5ABCF0 clears it when the event restarts.
    def group(self,g):
        if self.compiled.get(g,False):return True
        return super().group(g)
    def step(self,time=None):
        for g in tuple(self.groups):
            if self.group(g):self.compiled[g]=True
        return super().step(time)

cases=0;reproduced=0
for report in patch['reports']:
    assert report['unrelatedEventsAndConstructorUnchanged']
    before={e['ID']:e['before'] for e in report['changes']}
    assert len(report['events'])==len(before)==84
    for event in report['events']:
        code=[decode(i) for i in event['instructions']]
        old=[decode(i) for i in before[event['ID']]['instructions']]
        p=event['ID']//10000*10000
        entity=next(a[0] for n,a in code if n=='SetCharacterBackreadState')
        choice=next(a[3] for n,a in code if n=='IfEventFlag' and a[0]==1 and a[3]!=p+7403)
        for at,(n,a) in enumerate(code):
            if n.startswith('Goto'):assert any(k==f'Label{a[0]}' for k,_ in code[at+1:])
        assert not any(n=='SkipIfConditionGroupStateCompiled' and a[2]==5 for n,a in code)
        assert [i for i in code if i[0] in ['SpawnOneshotSFX','ForceAnimationPlayback']] == [i for i in old if i[0] in ['SpawnOneshotSFX','ForceAnimationPlayback']]
        base={p+7403:False,choice:False,p+7404:False}
        def pending_other_fight(decoded):
            peer=NativeLatchPeer(decoded,base,entity,loaded=False)
            peer.step(0)
            peer.flags[p+7403]=peer.flags[p+7404]=True
            peer.step(1)  # Another encounter fights while this Birth waits.
            peer.flags[p+7403]=peer.flags[p+7404]=False
            peer.step(2)
            peer.flags[p+7403]=peer.flags[choice]=True
            peer.step(3)
            return peer
        legacy=pending_other_fight(old);assert legacy.compiled[5]
        legacy.state['loaded']=True;legacy.step(3)
        assert not legacy.calls('SpawnOneshotSFX')
        assert legacy.calls('ChangeCharacterEnableState')[-1]==(3,[entity,1])
        reproduced+=1
        for delay in [0,.2,5,9]:
            peer=pending_other_fight(code)
            if delay>=5:peer.flags[p+7404]=True
            peer.state['loaded']=True;peer.step(3+delay)
            assert len(peer.calls('SpawnOneshotSFX'))==1
            assert peer.calls('ChangeCharacterEnableState')[-1]==(3+delay,[entity,0])
            peer.step(6.99+delay);assert len(peer.calls('ChangeCharacterEnableState'))==1
            peer.step(7+delay);assert peer.calls('ChangeCharacterEnableState')[-1]==(7+delay,[entity,1])
            assert len(peer.calls('ActivateMultiplayerdependantBuffs'))==1
            # Subsequent reuse of this actor clears native condition latches.
            peer.flags[choice]=False;peer.step(8+delay);peer.step(9+delay)
            peer.effects.clear();peer.flags[p+7404]=False;peer.flags[choice]=True
            peer.step(10+delay);assert len(peer.calls('SpawnOneshotSFX'))==2
            cases+=1
        for delay in [0,5,9]:
            peer=NativeLatchPeer(code,base|{p+7403:True,choice:True,p+7404:True},entity,loaded=False)
            peer.step(0);peer.state['loaded']=True;peer.step(delay)
            assert not peer.calls('SpawnOneshotSFX') and not peer.calls('ForceAnimationPlayback')
            assert peer.calls('ChangeCharacterEnableState')==[(delay,[entity,1])]
            assert len(peer.calls('ActivateMultiplayerdependantBuffs'))==1
            cases+=1
    expected=report['outputSha256'].lower()
    assert hashlib.sha256((work/'events'/f"{report['mapName']}.emevd.dcx").read_bytes()).hexdigest()==expected
report=dict(version='0.17.29',oldPendingSecondEncounterSkipReproduced=reproduced,
            freshDelayRepeatAndReconnectCases=cases,nativeLatchProof=proof,
            all336SingleGroupAndMountBirthEventsCovered=True,
            originalFogAnimationsAndCoopActivationPreserved=True,
            generationRewardsHudAndAllOtherEventsUnchanged=True,
            gameMemoryWritten=False,gameplayVerified=False,multiplayerVerified=False)
(work/'validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f'PASS: {reproduced} old second-encounter skips reproduced; {cases} fresh/delay/reuse/reconnect cases passed.')
