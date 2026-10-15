"""Check serialized guarded resets; native reward routing is checked separately."""
from pathlib import Path
import json,struct
root=Path(__file__).resolve().parents[1]
work=root/'.work/red-envelope-rewards'
d=json.loads((work/'staged/validation.json').read_text())
assert d['allExistingRowsUnchanged'] and d['nativeOutdoorLotteriesPreserved']
assert d['added']=={'SpEffectParam':[95000103,95000104,95000105,95000106],'ItemLotParam_enemy':[95410000,95410001,95410002,95410010,95410011,95410012]}
RESET=95000106
receipts={95000103,95000104,95000105}
events={e['ID']:e for e in d['events']}
previous={e['ID']:e for e in d['previousEvents']}
for id,e in events.items():
 ins=e['instructions'];at=next(n for n,i in enumerate(ins)if (i['Bank'],i['ID'],i['hex'])==(2004,21,struct.pack('<Ii',20000,95000103).hex().upper()))
 expected=[(2004,21,(20000,i)) for i in [95000103,95000104,95000105,RESET]]+[(2004,8,(20000,RESET))]
 assert [(i['Bank'],i['ID'],struct.unpack('<Ii',bytes.fromhex(i['hex'])))for i in ins[at:at+5]]==expected
 assert ins[:at]+ins[at+5:]==previous[id]['instructions'] and e['Parameters']==previous[id]['Parameters']
def run(event,completed=False,ready=True,hub=False,world=False):
 marks=set(receipts);groups={};pc=0
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
   group,want,typ,flag=struct.unpack('<bBBxI',a);assert flag==7500
   if not ready:return False,marks
  elif op==(3,8):
   group,inside,area,block,region,index=struct.unpack('<bBbbbbxx',a)
   groups[group]=groups.get(group,False) or hub==bool(inside)
  elif op==(0,0):
   _,want,group=struct.unpack('<bBbx',a)
   if groups.get(group,False)!=bool(want):return False,marks
  elif op==(2003,66):pass
  elif op in [(2004,8),(2004,21)]:
   player,id=struct.unpack('<Ii',a)
   if id in receipts|{RESET}:assert player==20000
   if op==(2004,21):marks.discard(id)
   else:
    marks.add(id)
    if id==RESET:return True,marks
  else:raise AssertionError((event,pc-1,op))
 raise AssertionError('reset prefix did not settle')

timelines=[]
for completed in [False,True]:
 for ready in [False,True]:
  for world in [False,True]:
   reset,marks=run(1111,completed,ready,world=world)
   assert reset==(not completed and ready)
   assert (marks&receipts)==(set()if reset else receipts)
   timelines.append(dict(event=1111,completed=completed,ready=ready,world=world,reset=reset))
for hub in [False,True]:
 reset,marks=run(9100941,hub=hub);assert reset==hub
 assert (marks&receipts)==(set()if reset else receipts)
 timelines.append(dict(event=9100941,hub=hub,reset=reset))
(work/'event-validation.json').write_text(json.dumps(dict(timelines=timelines,unrelatedInstructionsAndMetadataPreserved=True,gameplayVerified=False),indent=2))
print('PASS: 10 native event timelines: fresh deployment/hub reset receipts; revival/completed deployment preserve receipts; unrelated instructions unchanged')
