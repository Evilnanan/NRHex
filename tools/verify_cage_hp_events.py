"""Check serialized cage births and host gates; independent peer timelines.

This is a control-flow simulation, not a native networking acceptance test.
"""
import hashlib,json,struct
from pathlib import Path
from local_paths import ROOT
from cage_local_events import Peer

work=ROOT/'.work/cage-hp-122';data=json.loads((work/'validation.json').read_text())
schema={(o['bank'],o['id']):(o['name'],[t for t,_ in o['args']]) for o in json.loads((ROOT/'data/reference/world/cage-opcodes.json').read_text())['opcodes']}
schema[2009,4]=('ActivateMultiplayerdependantBuffs',['uint'])
schema.update({(1003,14):('EndIfPlayerIsInWorldType',['byte','byte']),
 (1003,13):('GotoIfPlayerIsInWorldType',['byte','byte']),
 (2004,28):('SetNetworkUpdateAuthority',['uint','int']),
 (3,42):('IfEventFlagAndRandomCondition',['sbyte','uint','float']),
 (4,23):('IfCharacterRatioHPValue',['sbyte','uint','sbyte','int','sbyte','float'])})
for i in range(10):schema[1014,i]=(f'Label{i}',[])
fmt=dict(sbyte='b',byte='B',bool='B',int='i',uint='I',short='h',ushort='H',float='f')
def decode(i):
    name,types=schema[i['Bank'],i['ID']];raw=bytes.fromhex(i['hex']);at=0;args=[]
    for t in types:
        size=struct.calcsize(fmt[t]);at=(at+size-1)//size*size;args.append(struct.unpack_from('<'+fmt[t],raw,at)[0]);at+=size
    assert len(raw)==(at+3)//4*4
    return name,args
cases=births=battles=0
def shown_flag(p,actor):
    index=actor-(p+9000)
    return p+(7600 if index<37 else 7750)+index
for report in data['reports']:
    p=report['prefix'];path=work/'staged/mod/event'/f"{report['name']}.emevd.dcx"
    assert hashlib.sha256(path.read_bytes()).hexdigest().upper()==report['outputSha256']
    for event in report['events']:
        code=[decode(i) for i in event['instructions']]
        if event['ID']<p+9900:
            battles+=1;assert not any(n=='ActivateMultiplayerdependantBuffs' for n,_ in code)
            at=next(i for i,(n,a) in enumerate(code) if n=='WaitFixedTimeSeconds' and a==[4.])
            guards=[];i=at+1
            while code[i][0]=='IfEventFlag' and code[i][1][0]==4:
                a=code[i][1];assert a[1:3]==[1,0];guards.append(a[3]);i+=1
            assert guards and len(guards)==len(set(guards)) and code[i]==('IfConditionGroup',[0,1,4])
            actors={a[0] for n,a in code if n=='SetNetworkUpdateAuthority'}
            assert set(guards)=={shown_flag(p,actor) for actor in actors}
            continue
        births+=1;actor=p+9000+event['ID']-(p+9900)
        choice=next(a[3] for n,a in code if n=='IfEventFlag' and a[0]==1 and a[3]!=p+7403)
        activations=[i for i,(n,a) in enumerate(code) if n=='ActivateMultiplayerdependantBuffs']
        assert len(activations)==1;at=activations[0]
        assert code[at-1]==('ChangeCharacterEnableState',[actor,1])
        assert code[at+1]==('SetSpEffect',[actor,95001603])
        shown=shown_flag(p,actor)
        shown_at=next(i for i,(n,a) in enumerate(code) if n=='SetEventFlag' and a==[0,shown,1])
        assert shown_at>at
        for ongoing in (False,True):
            for delay in (0.,.2,5.,30.):
                peer=Peer(code,{p+7403:True,choice:True,p+7404:ongoing},actor,loaded=False,hp=0)
                peer.step(0);assert not peer.calls('ActivateMultiplayerdependantBuffs')
                peer.state.update(loaded=True,hp=100);peer.step(delay);peer.step(delay+4.)
                assert len(peer.calls('ActivateMultiplayerdependantBuffs'))==1
                assert bool(peer.calls('SpawnOneshotSFX'))==(not ongoing)
                assert peer.flags.get(shown) and 95001603 in peer.effects
                outputs=[n for _,n,_ in peer.outputs];assert outputs.index('ActivateMultiplayerdependantBuffs')>max(i for i,(_,n,a) in enumerate(peer.outputs) if n=='ChangeCharacterEnableState' and a==[actor,1])
                peer.step(delay+40);assert len(peer.calls('ActivateMultiplayerdependantBuffs'))==1
                cases+=1
assert births==336 and battles==200
report=dict(serializedBirthEvents=births,serializedBattleEvents=battles,independentPeerTimelines=cases,activationAfterEnableAndBeforeLocalShown=True,hostWaitsForAllLocalMembers=True,nativeMultiplierSelectionDelegated=True,unchangedFogAndNoOngoingReplay=True,gameplayVerified=False,multiplayerVerified=False)
(work/'event-validation.json').write_text(json.dumps(report,indent=2))
print(f'PASS: {cases} independent peer timelines; 336 births and 200 host gates; generation/enable precede native correction and HUD readiness')
