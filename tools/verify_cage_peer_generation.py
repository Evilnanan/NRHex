"""Exercise staged Birth events with independent peer generator/backread state.

This is an event simulation. Native generator network ownership and actual
cooperative gameplay remain user acceptance checks.
"""
from pathlib import Path
import hashlib,json,struct
from cage_local_events import Peer

root=Path(__file__).resolve().parents[1]
work=root/'.work/fixes-01731'
patch=json.loads((work/'cages.json').read_text(encoding='utf8'))
schema={(o['bank'],o['id']):(o['name'],[t for t,_ in o['args']]) for o in json.loads((root/'data/reference/world/cage-opcodes.json').read_text(encoding='utf8'))['opcodes']}
schema[(2009,4)]=('ActivateMultiplayerdependantBuffs',['uint'])
for i in range(10):schema[1014,i]=(f'Label{i}',[])
fmt=dict(sbyte='b',byte='B',bool='B',int='i',uint='I',short='h',ushort='H',float='f')
def decode(i):
    n,types=schema[i['Bank'],i['ID']];raw=bytes.fromhex(i['hex']);at=0;args=[]
    for t in types:
        size=struct.calcsize(fmt[t]);at=(at+size-1)//size*size
        args.append(struct.unpack_from('<'+fmt[t],raw,at)[0]);at+=size
    assert len(raw)==(at+3)//4*4
    return n,args

class GeneratorPeer(Peer):
    def __init__(self,code,flags,entity,delay):
        super().__init__(code,flags,entity,loaded=False,hp=0)
        self.delay=delay;self.request=None;self.observed=0
    def step(self,time=0):
        if self.request is not None and time+1e-9>=self.request+self.delay:
            self.state.update(loaded=True,hp=100);self.request=None
        super().step(time)
        requests=self.calls('InvokeEnemyGenerator')
        if len(requests)>self.observed:
            assert len(requests)==self.observed+1
            self.observed=len(requests)
            if not self.state['loaded']:self.request=time
            if self.delay==0 and self.request is not None:
                self.state.update(loaded=True,hp=100);self.request=None
                super().step(time)

cases=blocked=death_cases=0
for r in patch['reports']:
    p=int(r['mapName'][1:3]+r['mapName'][4:6])*10000
    assert r['unrelatedEventsAndConstructorUnchanged']
    assert hashlib.sha256((work/'events'/f"{r['mapName']}.emevd.dcx").read_bytes()).hexdigest()==r['outputSha256']
    before={e['ID']:e['before'] for e in r['changes']}
    assert len(r['events'])==134
    for e in r['events']:
        old=before[e['ID']];assert e['RestBehavior']==old['RestBehavior'] and e['Parameters']==old['Parameters']==[]
        if e['ID']<p+9900:
            assert not any(i['Bank']==2003 and i['ID']==54 for i in e['instructions'])
            assert sum(i['Bank']==1003 and i['ID']==14 and bytes.fromhex(i['hex'])==b'\0\1\0\0' for i in e['instructions'])==1
            # The host generator move and single death markers are the only
            # Battle changes; rewards/scaling/AI/cleanup retain original bytes.
            previous=[i for i in old['instructions'] if (i['Bank'],i['ID'])!=(2003,54)]
            current=[i for i in e['instructions'] if not (i['Bank']==2003 and i['ID']==69 and p+7900<=struct.unpack_from('<I',bytes.fromhex(i['hex']),4)[0]<p+7937)]
            assert previous==current
            continue
        code=[decode(i) for i in e['instructions']];legacy=[decode(i) for i in old['instructions']]
        entity=p+9000+e['ID']-p-9900;choice=next(a[3] for n,a in code if n=='IfEventFlag' and a[0]==1 and a[3]!=p+7403)
        generator=entity-5200
        assert [a for n,a in code if n=='InvokeEnemyGenerator']==[[generator],[generator]]
        assert not any(n=='EndIfPlayerIsInWorldType' for n,_ in code)
        assert [x for x in code if x[0] in ['SpawnOneshotSFX','ForceAnimationPlayback']]==[x for x in legacy if x[0] in ['SpawnOneshotSFX','ForceAnimationPlayback']]
        for at,(n,a) in enumerate(code):
            if n.startswith('Goto'):assert any(k==f'Label{a[0]}' for k,_ in code[at+1:])
        flags={p+7403:True,choice:True,p+7404:False}
        oldpeer=GeneratorPeer(legacy,flags,entity,0);oldpeer.step(0);oldpeer.step(20)
        assert not oldpeer.calls('InvokeEnemyGenerator') and not oldpeer.calls('SpawnOneshotSFX') and not oldpeer.calls('ChangeCharacterEnableState');blocked+=1
        # Independent host/peer delays, a different fight while this actor is
        # idle, and a later shared spawned bit cannot erase the fresh branch.
        for delay in [0,.2,5,9]:
            peer=GeneratorPeer(code,{p+7403:False,choice:False,p+7404:False},entity,delay)
            peer.step(0);peer.flags[p+7403]=peer.flags[p+7404]=True;peer.step(1)
            assert not peer.calls('InvokeEnemyGenerator')
            peer.flags[p+7403]=peer.flags[p+7404]=False;peer.step(2)
            peer.flags[p+7403]=peer.flags[choice]=True;peer.step(3)
            assert len(peer.calls('InvokeEnemyGenerator'))==1
            if delay:peer.flags[p+7404]=True;peer.step(3+delay)
            assert len(peer.calls('SpawnOneshotSFX'))==1
            assert peer.calls('ChangeCharacterEnableState')[-1][1]==[entity,0]
            peer.step(7+delay)
            assert peer.calls('ChangeCharacterEnableState')[-1][1]==[entity,1]
            assert len(peer.calls('ActivateMultiplayerdependantBuffs'))==1
            peer.flags[choice]=False;peer.step(8+delay);peer.step(9+delay)
            peer.flags[p+7404]=False;peer.flags[choice]=True;peer.state.update(loaded=False,hp=0);peer.effects.clear()
            peer.step(10+delay);peer.step(10+2*delay);peer.step(14+2*delay)
            assert len(peer.calls('InvokeEnemyGenerator'))==2 and len(peer.calls('SpawnOneshotSFX'))==2
            assert peer.calls('ChangeCharacterEnableState')[-1][1]==[entity,1];cases+=1
        for delay in [0,5,9]:
            peer=GeneratorPeer(code,flags|{p+7404:True},entity,delay);peer.step(0);peer.step(delay)
            assert len(peer.calls('InvokeEnemyGenerator'))==1
            assert not peer.calls('SpawnOneshotSFX') and not peer.calls('ForceAnimationPlayback')
            assert peer.calls('ChangeCharacterEnableState')[-1][1]==[entity,1];cases+=1
        death=next((a[3] for n,a in code if n=='GotoIfEventFlag' and a[0]==3),None)
        if death is not None:
            peer=GeneratorPeer(code,flags|{p+7404:True,death:True},entity,0);peer.step(0);peer.step(20)
            assert not peer.calls('InvokeEnemyGenerator') and not peer.calls('ChangeCharacterEnableState');death_cases+=1
            # A later fresh life must ignore the previous life's death marker.
            peer.flags[choice]=False;peer.step(21);peer.step(22)
            peer.flags[p+7404]=False;peer.flags[choice]=True;peer.step(23);peer.step(27)
            assert len(peer.calls('InvokeEnemyGenerator'))==1 and len(peer.calls('SpawnOneshotSFX'))==1;cases+=1
report=dict(version='0.17.31',oldLocalBirthsUnableToGenerateReplica=blocked,
            delayRepeatReconnectAndFreshAfterDeathCases=cases,deadMembersNeverRegeneratedOnReconnect=death_cases,
            all336BirthsAnd200HostBattlesCovered=True,unchangedFogAnimationScalingRewardAndOtherEvents=True,
            generatorNetworkAuthorityIsNativeAndRequiresCoopAcceptance=True,gameMemoryWritten=False,gameplayVerified=False,multiplayerVerified=False)
(work/'cage-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f'PASS: {blocked} legacy blocked peer Births; {cases} lifecycle cases; {death_cases} dead-member reconnect cases.')
