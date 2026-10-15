"""Check serialized exposure markers and execute their guarded reset prefixes."""
from pathlib import Path
import json,struct
from local_paths import ROOT

work=ROOT/'.work/reward-cycle';d=json.loads((work/'staged/validation.json').read_text())
FIRST=95002800;RESET=FIRST+25
effects={e['ID']:e['fields'] for e in d['effects']};assert set(effects)==set(range(FIRST,RESET+1))
for e in effects.values():
 assert (e['saveCategory'],e['dontDeleteOnDead'],e['isDisableNetSync'],e['effectEndurance'])==(10,1,1,-1)
 assert (e['maxHpRate'],e['iconId'],e['spEffectTextId_1'],e['stateInfo'])==(1,-1,-1,0)
 assert e['vfxId']==-1 and e['vfxId1']==-1 and e['changeHpPoint']==0 and e['changeMpPoint']==0
assert d['onlyNeutralSavedEffectsAndTwoResetsAdded'] and d['originalRowsTablesEventsAndMetadataPreserved']
events={e['ID']:e for e in d['events']};old={e['ID']:e for e in d['previousEvents']};assert set(events)=={1111,9100941}
for id,e in events.items():
 ins=e['instructions'];before=old[id]['instructions'];assert len(ins)==len(before)+27
 start=next(n for n,i in enumerate(ins) if (i['Bank'],i['ID'])==(2004,21) and struct.unpack('<Ii',bytes.fromhex(i['hex']))==(20000,FIRST))
 block=ins[start:start+27]
 assert [(i['Bank'],i['ID'],struct.unpack('<Ii',bytes.fromhex(i['hex']))) for i in block]==[(2004,21,(20000,i)) for i in range(FIRST,RESET+1)]+[(2004,8,(20000,RESET))]
 assert ins[:start]+ins[start+27:]==before and e['Parameters']==old[id]['Parameters']

def until_notice(event,completed=False,ready=True,hub=False,world=False):
 effects=set(range(FIRST,RESET))|{95002600};groups={};pc=0;reset=False
 for _ in range(256):
  i=events[event]['instructions'][pc];op=i['Bank'],i['ID'];a=bytes.fromhex(i['hex']);pc+=1
  if op==(2000,2):assert a[0]==0
  elif op==(1003,1):
   skip,want,typ,flag=struct.unpack('<BBBxI',a);assert flag==9011 and typ==0
   if completed==bool(want):pc+=skip
  elif op==(1000,4):return reset,effects
  elif op==(3,41):groups[-1]=world
  elif op==(1000,1):
   skip,want,group=struct.unpack('<BBbx',a)
   if groups[group]==bool(want):pc+=skip
  elif op==(3,0):
   group,want,typ,flag=struct.unpack('<bBBxI',a);assert group==0 and flag==7500
   if not ready:return reset,effects
  elif op==(3,8):
   group,inside,area,block,region,index=struct.unpack('<bBbbbbxx',a);assert group==-1
   groups[group]=groups.get(group,False) or (hub==bool(inside))
  elif op==(0,0):
   _,want,group=struct.unpack('<bBbx',a)
   if groups.get(group,False)!=bool(want):return reset,effects
  elif op==(2003,66):pass
  elif op in [(2004,8),(2004,21)]:
   player,id=struct.unpack('<Ii',a)
   if op==(2004,21):effects.discard(id)
   else:
    effects.add(id)
    if id==RESET:return True,effects
  else:raise AssertionError((event,pc-1,op))
 raise AssertionError('reset prefix failed to settle')

cases=[]
for completed in [False,True]:
 for ready in [False,True]:
  for world in [False,True]:
   reset,marks=until_notice(1111,completed,ready,world=world);wanted=not completed and ready
   assert reset==wanted
   assert marks.intersection(range(FIRST,RESET))==(set() if wanted else set(range(FIRST,RESET)))
   assert 95002600 in marks;cases.append(dict(event=1111,completed=completed,playerReady=ready,worldTypeBranch=world,reset=reset))
for hub in [False,True]:
 reset,marks=until_notice(9100941,hub=hub);assert reset==hub and 95002600 in marks
 cases.append(dict(event=9100941,inHub=hub,reset=reset))
report=dict(markerCount=26,resetInstructionCountPerEvent=27,serializedTimelineCases=len(cases),cases=cases,localSavedMarkersSurviveDeathPolicy=True,freshDeploymentGuardPreserved=True,gameplayVerified=False,multiplayerVerified=False)
(work/'event-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f'PASS: 26 local saved markers and {len(cases)} serialized reset timelines; revival cannot enter fresh-deployment cleanup')
