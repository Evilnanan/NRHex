"""Execute the serialized local EMEVD events against controlled timelines."""
from pathlib import Path
import json, struct
from local_paths import ROOT

stage=ROOT/'.work/contract-heart/staged'
events=json.loads((stage/'events.json').read_text())
data=json.loads((stage/'mechanics.json').read_text())
effects={r['ID']:r['fields'] for r in data['effects']}
lots={r['ID']:r['fields'] for r in data['lots']}
OWNER=95002700;RECEIPT=OWNER+1;USED=OWNER+2;COOL=OWNER+3;RETURN=OWNER+4;SECOND=OWNER+5
assert effects[COOL]['effectEndurance']==60 and effects[COOL]['replaceSpEffectId']==RETURN
assert effects[RECEIPT]['saveCategory']==10 and effects[COOL]['dontDeleteOnDead']==1
assert effects[USED]['isDisableNetSync']==1
assert effects[SECOND]['isDisableNetSync']==1 and effects[SECOND]['dontDeleteOnDead']==1 and effects[SECOND]['effectEndurance']==-1
assert effects[95002600]['stateInfo']==2103 and effects[95002600]['atkOccurrenceSpEffectId']==95002601
assert all(effects[95002600][f]==0 for f in ['magParamChange','miracleParamChange','shamanParamChange'])
pulse=effects[95002601]
assert pulse['changeHpRate']==0 and pulse['changeHpPoint']==0 and pulse['bCurrHPIndependeMaxHP']==0 and pulse['isDisableNetSync']==1
assert pulse['effectTargetAttacker']==1 and pulse['motionInterval']>pulse['effectEndurance']>0

class Local:
    def __init__(self):
        self.time=0.;self.effects={};self.inventory=0;self.grants=[]
        self.threads=[dict(pc=0,groups={}) for _ in events]
    def apply(self,e):
        life=effects[e]['effectEndurance'];self.effects[e]=None if life<0 else self.time+life
    def tick(self,dt):
        self.time+=dt
        for e,end in list(self.effects.items()):
            if end is not None and end<=self.time:
                del self.effects[e]
                nxt=effects[e]['replaceSpEffectId']
                if nxt>0:self.apply(nxt)
        for event,thread in zip(events,self.threads):
            # Restart always yields to the next update. Consume/clear requests
            # and receipts before a repeated notification can be observed.
            for _ in range(32):
                i=event['instructions'][thread['pc']];bank,ident=i['Bank'],i['ID'];b=bytes.fromhex(i['hex'])
                if (bank,ident)==(2000,2):assert b==bytes(4)
                elif (bank,ident)==(4,5):
                    group=b[0];entity,effect=struct.unpack_from('<Ii',b,4);should=b[12]
                    assert entity==20000 and b[13]==0 and struct.unpack_from('<f',b,16)[0]==1.
                    thread['groups'].setdefault(group,[]).append((effect,should))
                elif (bank,ident)==(0,0):
                    assert b[:3]==bytes([0,1,1]);conditions=thread['groups'][1]
                    if not all((e in self.effects)==bool(should) for e,should in conditions):break
                elif (bank,ident)==(2004,8):
                    entity,effect=struct.unpack('<Ii',b);assert entity==20000;self.apply(effect)
                elif (bank,ident)==(2004,21):
                    entity,effect=struct.unpack('<Ii',b);assert entity==20000;self.effects.pop(effect,None)
                elif (bank,ident)==(2003,4):
                    lot=lots[struct.unpack('<i',b)[0]]
                    assert lot['lotItemId01']==3350 and lot['lotItemCategory01']==1 and lot['lotItemBasePoint01']==1000 and lot['getItemFlagId']==0
                    n=lot['lotItemNum01'];self.inventory=min(2,self.inventory+n);self.grants.append((self.time,n))
                elif bank==1004 and ident in (0,2):
                    assert struct.unpack_from('<I',b,4)[0]==20000 and b[13]==0 and struct.unpack_from('<i',b,16)[0]==1
                    e=struct.unpack_from('<i',b,8)[0]
                    if (e in self.effects)==bool(b[12]):
                        if ident==2:
                            assert b[0]==1;thread.update(pc=0,groups={});break
                        thread['pc']+=b[0]
                elif (bank,ident)==(1000,4):
                    assert b==bytes([1,0,0,0]);thread.update(pc=0,groups={});break
                else:raise AssertionError((bank,ident))
                thread['pc']+=1
            else:raise AssertionError('event busy loop')
    def use(self):
        assert self.inventory>0;self.inventory-=1;self.apply(USED);self.tick(0)
    def acquire(self):self.apply(OWNER);self.tick(0)

cases=[]
a=Local();a.tick(5);assert not a.grants;a.acquire();assert a.inventory==2 and a.grants==[(5,2)]
for _ in range(120):a.tick(.1)
assert len(a.grants)==1;cases.append('initial pair awarded once while continuously owned')
a.use();start=a.time;a.tick(20);a.use();assert a.inventory==0 and SECOND in a.effects
a.tick(39.9);assert len(a.grants)==1;a.tick(.101);assert a.inventory==2 and len(a.grants)==3 and a.grants[-2:]==[(a.time,1),(a.time,1)]
assert abs(a.grants[-1][0]-(start+60))<.01 and SECOND not in a.effects;cases.append('second use returns two at the first deadline without extending it')
a.tick(120);assert len(a.grants)==3;a.use();a.tick(60);assert a.inventory==2 and len(a.grants)==4;cases.append('a later single use starts a fresh return with no stale second marker')

single=Local();single.acquire();single.use();single.tick(59.9);assert len(single.grants)==1;single.tick(.1)
assert single.grants==[(0,2),(60.,1)] and single.inventory==2;cases.append('one use returns exactly one after 60 seconds')

many=Local();many.acquire();many.use();deadline=many.effects[COOL];many.tick(5);many.use()
for _ in range(3):
    many.inventory=1;many.tick(5);many.use();assert many.effects[COOL]==deadline
many.inventory=0;many.tick(40);assert many.time==60 and many.inventory==2 and len(many.grants)==3
many.tick(120);assert len(many.grants)==3;cases.append('third and later uses neither exceed two returns nor refresh the deadline')

edge=Local();edge.acquire();edge.use();edge.tick(59.99);edge.use();edge.tick(.01)
assert edge.inventory==2 and len(edge.grants)==3;cases.append('second use just before expiry still returns two')

boundary=Local();boundary.acquire();boundary.use();boundary.time=60.;boundary.use()
assert USED in boundary.effects and len(boundary.grants)==2
boundary.tick(0);assert USED not in boundary.effects and boundary.effects[COOL]==120.
boundary.tick(60);assert len(boundary.grants)==3
cases.append('use arriving at settlement survives until it starts the next interval')
b=Local();b.acquire();b.use();b.tick(30)
# The pending native effect survives incapacitation; ownership checks prevent
# a second initial pair when the same ability is reconstructed after revival.
receipt=b.effects[RECEIPT];b.tick(30);assert len(b.grants)==2 and b.effects[RECEIPT]==receipt
cases.append('pending return and initial receipt survive a death/revival timeline')
revived=Local();revived.acquire();revived.use();revived.tick(10);revived.use();revived.tick(50)
assert revived.inventory==2 and len(revived.grants)==3 and SECOND not in revived.effects
cases.append('second-use marker survives death policy and clears on settlement')
c=Local();c.acquire();c.use();c.effects.pop(OWNER);c.tick(60);assert len(c.grants)==1 and RETURN in c.effects
c.acquire();assert len(c.grants)==2 and c.grants[-1][1]==1;cases.append('loss of owner cannot award a branch; pending return resumes after ownership returns')
d=Local();e=Local();d.acquire();d.use();d.tick(59);e.tick(59);assert not e.grants;e.acquire();e.use();d.tick(1);e.tick(1)
assert len(d.grants)==2 and len(e.grants)==1;e.tick(59);assert len(e.grants)==2
cases.append('two clients have independent initial receipts and return deadlines')
# Fresh deployment follows the serialized cleanup added to guarded event 1111.
for state in [RECEIPT,USED,COOL,RETURN,SECOND]:a.effects.pop(state,None)
a.effects.pop(OWNER,None);a.tick(0);a.acquire();assert a.grants[-1][1]==2
cases.append('new expedition clears old initial/pending state')
report=dict(eventTimelineCases=len(cases),cases=cases,serializedArgumentsVerified=True,nativeInventoryCapacityModelled=2,gameplayVerified=False,multiplayerVerified=False)
(ROOT/'.work/contract-heart/event-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f'PASS: {len(cases)} serialized event timelines; initial pair, capped two returns, unchanged deadline and local lifecycle verified')
