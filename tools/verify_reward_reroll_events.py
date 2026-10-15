"""Read back new saved-use markers and execute the serialized run reset guards."""
from pathlib import Path
import json,struct
from local_paths import ROOT
work=ROOT/'.work/hex-reroll';d=json.loads((work/'staged/validation.json').read_text())
FIRST=95002826;RESET=95002825
assert d['onlyThreeLocalSavedMarkersAndGuardedResetsAdded'] and d['originalRowsTablesEventsAndTextPreserved']
effects={e['ID']:e['fields'] for e in d['effects']};assert set(effects)==set(range(FIRST,FIRST+3))
for e in effects.values():
 assert (e['saveCategory'],e['dontDeleteOnDead'],e['isDisableNetSync'],e['effectEndurance'])==(10,1,1,-1)
 assert (e['maxHpRate'],e['iconId'],e['spEffectTextId_1'],e['stateInfo'])==(1,-1,-1,0)
 assert e['vfxId']==-1 and e['vfxId1']==-1 and e['changeHpPoint']==0 and e['changeMpPoint']==0
events={e['ID']:e for e in d['events']};old={e['ID']:e for e in d['previousEvents']}
for id,e in events.items():
 ins=e['instructions'];before=old[id]['instructions'];assert len(ins)==len(before)+3
 start=next(n for n,i in enumerate(ins) if (i['Bank'],i['ID'])==(2004,21) and struct.unpack('<Ii',bytes.fromhex(i['hex']))==(20000,FIRST))
 assert [(i['Bank'],i['ID'],struct.unpack('<Ii',bytes.fromhex(i['hex'])))for i in ins[start:start+3]]==[(2004,21,(20000,i))for i in range(FIRST,FIRST+3)]
 assert ins[:start]+ins[start+3:]==before and e['Parameters']==old[id]['Parameters']

def run(event,completed=False,ready=True,hub=False,world=False):
 marks=set(range(95002800,FIRST+3))|{95002600};groups={};pc=0
 for _ in range(256):
  i=events[event]['instructions'][pc];op=i['Bank'],i['ID'];a=bytes.fromhex(i['hex']);pc+=1
  if op==(2000,2):assert a[0]==0
  elif op==(1003,1):
   skip,want,typ,flag=struct.unpack('<BBBxI',a);assert flag==9011 and typ==0
   if completed==bool(want):pc+=skip
  elif op==(1000,4):return False,marks
  elif op==(3,41):groups[-1]=world
  elif op==(1000,1):
   skip,want,group=struct.unpack('<BBbx',a)
   if groups[group]==bool(want):pc+=skip
  elif op==(3,0):
   group,want,typ,flag=struct.unpack('<bBBxI',a);assert group==0 and flag==7500
   if not ready:return False,marks
  elif op==(3,8):
   group,inside,area,block,region,index=struct.unpack('<bBbbbbxx',a);assert group==-1
   groups[group]=groups.get(group,False) or (hub==bool(inside))
  elif op==(0,0):
   _,want,group=struct.unpack('<bBbx',a)
   if groups.get(group,False)!=bool(want):return False,marks
  elif op==(2003,66):pass
  elif op in [(2004,8),(2004,21)]:
   player,id=struct.unpack('<Ii',a)
   if op==(2004,21):marks.discard(id)
   else:
    marks.add(id)
    if id==RESET:return True,marks
  else:raise AssertionError((event,pc-1,op))
 raise AssertionError('Run reset did not settle')
cases=[]
for completed in [False,True]:
 for ready in [False,True]:
  for world in [False,True]:
   reset,marks=run(1111,completed,ready,world=world);wanted=not completed and ready;assert reset==wanted
   assert marks.intersection(range(FIRST,FIRST+3))==(set() if wanted else set(range(FIRST,FIRST+3)))
   assert 95002600 in marks;cases.append(dict(event=1111,completed=completed,ready=ready,world=world,reset=reset))
for hub in [False,True]:
 reset,marks=run(9100941,hub=hub);assert reset==hub and 95002600 in marks
 assert marks.intersection(range(FIRST,FIRST+3))==(set()if hub else set(range(FIRST,FIRST+3)))
 cases.append(dict(event=9100941,hub=hub,reset=reset))
report=dict(markerCount=3,serializedTimelineCases=len(cases),cases=cases,existingGuardsAndTextPreserved=True,gameplayVerified=False,multiplayerVerified=False)
(work/'event-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f'PASS: three saved local uses and {len(cases)} serialized reset timelines; revival preserves reroll use')
