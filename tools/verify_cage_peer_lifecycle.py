"""Check 0.17.34 event changes and simulate independent client lifetimes.

Simulation does not establish native networking behavior or gameplay success.
"""
from pathlib import Path
import hashlib,json,struct
from cage_local_events import Peer

root=Path(__file__).resolve().parents[1]
work=root/'.work/audit-01734'
schema={(o['bank'],o['id']):(o['name'],[t for t,_ in o['args']]) for o in json.loads((root/'data/reference/world/cage-opcodes.json').read_text(encoding='utf8'))['opcodes']}
schema[2009,4]=('ActivateMultiplayerdependantBuffs',['uint'])
for i in range(10):schema[1014,i]=(f'Label{i}',[])
fmt=dict(sbyte='b',byte='B',bool='B',int='i',uint='I',short='h',ushort='H',float='f')
def decode(i):
    n,types=schema[i['Bank'],i['ID']];raw=bytes.fromhex(i['hex']);at=0;args=[]
    for t in types:
        size=struct.calcsize(fmt[t]);at=(at+size-1)//size*size
        args.append(struct.unpack_from('<'+fmt[t],raw,at)[0]);at+=size
    assert len(raw)==(at+3)//4*4
    return n,args

LocalPeer=Peer

cases=births=battles=0
for report in json.loads((work/'cages.json').read_text(encoding='utf8')):
    p=int(report['mapName'][1:3]+report['mapName'][4:6])*10000
    assert hashlib.sha256((work/'events'/f"{report['mapName']}.emevd.dcx").read_bytes()).hexdigest()==report['outputSha256']
    before={e['ID']:e['before'] for e in report['changes']}
    assert len(report['events'])==134
    for event in report['events']:
        old=before[event['ID']]
        assert event['Parameters']==old['Parameters']==[] and event['RestBehavior']==old['RestBehavior']
        if event['ID']<p+9900:
            code=event['instructions'];legacy=old['instructions']
            host=next(i for i,x in enumerate(legacy) if (x['Bank'],x['ID'])==(1003,14))
            assert code==legacy[:1]+legacy[host:]
            assert (code[1]['Bank'],code[1]['ID'])==(1003,14)
            assert not any((x['Bank'],x['ID'])==(2003,54) for x in code)
            battles+=1;continue
        code=[decode(i) for i in event['instructions']]
        legacy=[decode(i) for i in old['instructions']]
        births+=1
        actor=p+9000+event['ID']-p-9900
        choice=next(a[3] for n,a in code if n=='IfEventFlag' and a[0]==1 and a[3]!=p+7403)
        assert code[1:4]==[('SetCharacterBackreadState',[actor,1]),('ChangeCharacterEnableState',[actor,0]),('SetCharacterAIState',[actor,0])]
        assert not any(n=='IfCharacterBackreadStatus' and a[0]==-2 for n,a in code)
        preserved=['SpawnOneshotSFX','ForceAnimationPlayback','ActivateMultiplayerdependantBuffs','InvokeEnemyGenerator']
        assert [x for x in code if x[0] in preserved]==[x for x in legacy if x[0] in preserved]
        flags={p+7403:True,choice:True,p+7404:False}
        for delay in [0,.2,5,30]:
            peer=LocalPeer(code,flags,actor,loaded=False,hp=0)
            peer.step(0)
            assert len(peer.calls('InvokeEnemyGenerator'))==1 and not peer.calls('SpawnOneshotSFX')
            # A shared ready bit changing while a peer loads cannot select the
            # reconnect branch retroactively or suppress its fresh entrance.
            peer.flags[p+7404]=True
            peer.state.update(loaded=True,hp=100);peer.step(delay);peer.step(delay+4)
            assert len(peer.calls('SpawnOneshotSFX'))==1
            assert peer.calls('ChangeCharacterEnableState')[-1][1]==[actor,1]
            # Streaming out/in during the same fight never asks for a second
            # generation, even when a killed replica has no positive HP.
            peer.state.update(loaded=False,hp=0);peer.step(delay+5);peer.step(delay+40)
            assert len(peer.calls('InvokeEnemyGenerator'))==1
            peer.state.update(loaded=True,hp=100);peer.step(delay+41)
            assert len(peer.calls('SpawnOneshotSFX'))==1
            peer.flags[choice]=False;peer.step(delay+42);peer.step(delay+43);peer.step(delay+58)
            peer.flags[p+7404]=False;peer.flags[choice]=True;peer.step(delay+60);peer.step(delay+64)
            assert len(peer.calls('InvokeEnemyGenerator'))==2 and len(peer.calls('SpawnOneshotSFX'))==2
            cases+=1
        # Both fresh and reconnect load waits cancel when their encounter ends.
        for reconnect in [False,True]:
            peer=LocalPeer(code,flags|{p+7404:reconnect},actor,loaded=False,hp=0)
            peer.step(0);peer.flags[choice]=False;peer.step(1);peer.step(2)
            assert peer.restarts==1 and not peer.calls('SpawnOneshotSFX')
            peer.flags[choice]=True;peer.flags[p+7404]=False;peer.state.update(loaded=True,hp=100)
            peer.step(3);peer.step(7)
            assert len(peer.calls('InvokeEnemyGenerator'))==2 and len(peer.calls('SpawnOneshotSFX'))==1
            cases+=1
        dead=next((a[3] for n,a in code if n=='GotoIfEventFlag' and a[0]==3),None)
        if dead is not None:
            peer=LocalPeer(code,flags|{p+7404:True,dead:True},actor)
            peer.step(0);peer.step(30)
            assert not peer.calls('InvokeEnemyGenerator') and not peer.calls('SpawnOneshotSFX')
            cases+=1
assert (births,battles)==(336,200)
result=dict(version='0.17.34',birthEvents=births,hostBattleEvents=battles,lifecycleSimulationCases=cases,
            rewardScalingAndEntranceInstructionsUnchanged=True,gameplayVerified=False,multiplayerVerified=False)
(work/'cage-validation.json').write_text(json.dumps(result,indent=2),encoding='utf8')
print(f'PASS: {births} births, {battles} host battles, {cases} peer lifecycle cases')
