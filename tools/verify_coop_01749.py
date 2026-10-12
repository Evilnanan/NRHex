"""Check staged replica events with independent load/HP delivery, not a game test."""
from pathlib import Path
import hashlib,json,struct
from cage_local_events import Peer
root=Path(__file__).resolve().parents[1];work=root/'.work/coop-01749'
schema={(o['bank'],o['id']):(o['name'],[t for t,_ in o['args']]) for o in json.loads((root/'data/reference/world/cage-opcodes.json').read_text(encoding='utf-8'))['opcodes']}
schema.update({(2009,4):('ActivateMultiplayerdependantBuffs',['uint']), (2004,34):('SetNetworkUpdateRate',['uint','bool','sbyte']), (2004,39):('ChangeCharacterCollisionState',['uint','byte'])})
for i in range(10):schema[1014,i]=(f'Label{i}',[])
fmt=dict(sbyte='b',byte='B',bool='B',int='i',uint='I',short='h',ushort='H',float='f')
def decode(i):
 n,types=schema[i['Bank'],i['ID']];raw=bytes.fromhex(i['hex']);at=0;args=[]
 for t in types:
  size=struct.calcsize(fmt[t]);at=(at+size-1)//size*size;args.append(struct.unpack_from('<'+fmt[t],raw,at)[0]);at+=size
 assert len(raw)==(at+3)//4*4
 return n,args
cases=0;births=0
for report in json.loads((work/'cages.json').read_text(encoding='utf-8')):
 p=int(report['mapName'][1:3]+report['mapName'][4:6])*10000
 assert hashlib.sha256((work/'staged/mod/event'/f"{report['mapName']}.emevd.dcx").read_bytes()).hexdigest().upper()==report['outputSha256']
 before={e['ID']:e['before'] for e in report['changes']}
 for e in report['events']:
  actor=p+9000+e['ID']-p-9900;code=list(map(decode,e['instructions']));old=list(map(decode,before[e['ID']]['instructions']))
  choice=next(a[3] for n,a in code if n=='IfEventFlag' and a[0]==1 and a[3]!=p+7403)
  shown=next(a[1] for n,a in code if n=='SetEventFlag' and a[2]==1)
  assert not any(n=='IfCharacterHPValue' for n,a in code)
  assert not any(n in ['SetNetworkconnectedEventFlag','IncrementNetworkconnectedEventValue'] for n,a in code)
  preserved=['SpawnOneshotSFX','ForceAnimationPlayback','InvokeEnemyGenerator','ActivateMultiplayerdependantBuffs']
  assert [i for i in code if i[0] in preserved]==[i for i in old if i[0] in preserved]
  flags={p+7403:True,choice:True,p+7404:False}
  for delay in [0,.2,5,30,100]:
   for ongoing in [False,True]:
    peer=Peer(code,flags|{p+7404:ongoing},actor,loaded=False,hp=0)
    peer.step(0);assert len(peer.calls('InvokeEnemyGenerator'))==1
    assert peer.calls('SetNetworkUpdateRate')==[(0,[actor,1,0])]
    assert not peer.flags.get(shown)
    peer.flags[p+7404]=True
    peer.state['loaded']=True  # Deliberately no positive HP packet yet.
    peer.step(delay);peer.step(delay+4)
    assert peer.flags[shown] and peer.calls('ChangeCharacterEnableState')[-1][1]==[actor,1]
    assert len(peer.calls('SpawnOneshotSFX'))==int(not ongoing)
    assert peer.calls('SetCharacterAIState')[-1][1]==[actor,1]
    peer.state['loaded']=False;peer.step(delay+10)
    peer.state.update(loaded=True,hp=100);peer.step(delay+20)
    assert len(peer.calls('InvokeEnemyGenerator'))==1
    cases+=1
  dead=next((a[3] for n,a in code if n=='GotoIfEventFlag' and a[0]==3),None)
  if dead is not None:
   peer=Peer(code,flags|{p+7404:True,dead:True},actor,hp=0);peer.step(0);peer.step(100)
   assert not peer.calls('InvokeEnemyGenerator') and not peer.flags.get(shown);cases+=1
  # Ended encounters must release peers still waiting for their model.
  peer=Peer(code,flags,actor,loaded=False,hp=0);peer.step(0)
  peer.flags[choice]=False;peer.step(1);peer.step(2)
  assert peer.restarts==1 and not peer.calls('SpawnOneshotSFX');cases+=1
  births+=1
assert births==336
report=dict(version='0.17.49',birthEvents=births,independentReplicaCases=cases,
 hostSelectionDeathRewardsUnchanged=True,replicaPresentationDoesNotWaitForPositiveHp=True,
 gameplayVerified=False,multiplayerVerified=False)
(work/'event-validation.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print(f'PASS: {births} births; {cases} delayed model/HP, reconnect, death and cancellation cases (simulation only)')
