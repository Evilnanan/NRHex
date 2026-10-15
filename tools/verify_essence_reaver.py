"""Read serialized mechanics/resets and execute original FP/HUD predicates.

Native API allocation, FP recalculation, rendering and multiplayer are not run.
"""
from pathlib import Path
import ctypes as c,hashlib,json,struct,sys,math
import pefile
from local_paths import ROOT,GAME

work=ROOT/'.work/essence-reaver'
single='--single-staged' in sys.argv
installed='--installed' in sys.argv or single
d=json.loads(((ROOT/'.work/essence-single/parameters.json')if single else(work/('installed.json'if installed else 'staged/validation.json'))).read_text())
OWNER=95002900;PULSE=OWNER+1;LAYER=OWNER+2;RESET=OWNER+3;FIRST=95006000
effects={e['ID']:e['fields'] for e in d['effects']}
owner=effects[OWNER]
assert LAYER not in effects
assert (owner['stateInfo'],owner['applyIdOnGetSoul'],owner['applyIdOnKillSp'])==(199,PULSE,PULSE)
assert owner['iconId']==20292 and owner['displayIconForPermamentEffect']==1
assert owner['maxMpRate']==1 and owner['dispDartDebuffStacks']==0
ids={FIRST+digit*256+value for digit in range(4) for value in range(1,256)}
assert ids.issubset(effects) and len(ids)==1020
for id in ids|{RESET,95002829}:
 e=effects[id]
 assert (e['saveCategory'],e['dontDeleteOnDead'],e['isDisableNetSync'])==(10,1,1)
 assert (e['maxHpRate'],e['maxMpRate'],e['iconId'],e['stateInfo'])==(1,1,-1,0)
 assert e['effectEndurance']==-1 and e['displayIconForPermamentEffect']==0
if not installed:assert d['allExistingRowsAndRewardTriplesPreserved'] and d['onlyTwoGuardedEventsExtended']
events={e['ID']:e for e in d['events']}
previous={e['ID']:e for e in d['previousEvents']}if not installed else {}
for id,e in events.items():
 ins=e['instructions'];at=next(n for n,i in enumerate(ins) if (i['Bank'],i['ID'],i['hex'])==(2004,21,struct.pack('<Ii',20000,RESET).hex().upper()))
 assert [(i['Bank'],i['ID'],struct.unpack('<Ii',bytes.fromhex(i['hex'])))for i in ins[at:at+2]]==[(2004,21,(20000,RESET)),(2004,8,(20000,RESET))]
 assert sum((i['Bank'],i['ID'],i['hex'])==(2004,21,struct.pack('<Ii',20000,95002829).hex().upper())for i in ins)==1
 if not installed:assert ins[:at-1]+ins[at+2:]==previous[id]['instructions'] and e['Parameters']==previous[id]['Parameters']

def run(event,completed=False,ready=True,hub=False,world=False):
 marks={95002829};groups={};pc=0
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
   if id in [RESET,95002829]:assert player==20000
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
   reset,marks=run(1111,completed,ready,world=world);assert reset==(not completed and ready)
   assert (95002829 in marks)==(not reset)
   timelines.append(dict(event=1111,completed=completed,ready=ready,world=world,reset=reset))
for hub in [False,True]:
 reset,marks=run(9100941,hub=hub);assert reset==hub and (95002829 in marks)==(not reset)
 timelines.append(dict(event=9100941,hub=hub,reset=reset))

exe=GAME/'nightreign.exe';assert hashlib.sha256(exe.read_bytes()).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(str(exe),fast_load=True)
saved_limits=[struct.unpack_from('<ii',pe.get_data(0x2be6dc0,96),i*8)for i in range(12)]
assert saved_limits[9]==(13,18)and saved_limits[10]==(31,50)
k=c.WinDLL('kernel32',use_last_error=True);k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong,c.c_ulong];k.VirtualAlloc.restype=c.c_void_p;k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong];k.VirtualFree.restype=c.c_int
memory=k.VirtualAlloc(None,8192,0x3000,0x40);assert memory
cases=[];hud=[];fp=[]
try:
 code=pe.get_data(0x510b20,0x50);c.memmove(memory,code,len(code))
 predicate=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p,c.c_int)(memory)
 row=c.create_string_buffer(0x400);node=c.create_string_buffer(0x88)
 struct.pack_into('<Q',node,0,c.addressof(row));struct.pack_into('<i',node,8,LAYER)
 for category in [10,20]:
  struct.pack_into('<h',row,0x13e,category)
  for effect in [LAYER,LAYER+100]:
   result=predicate(c.addressof(node),effect);assert result==int(category==20 and effect==LAYER)
   cases.append(dict(category=category,sameId=effect==LAYER,reusesExistingNode=bool(result)))
 # Original ordinary/permanent icon aggregation, with private vectors keeping
 # all external allocation calls unreachable. This also checks the infinite-
 # lifetime visibility flag, not just a positive-duration approximation.
 code=pe.get_data(0x7deb30,0x2d4);c.memmove(memory+1024,code,len(code))
 aggregate=c.WINFUNCTYPE(c.c_void_p,c.c_void_p,c.c_void_p,c.c_void_p)(memory+1024)
 row[0x353]=8;vector=c.create_string_buffer(0x30);entries=c.create_string_buffer(1200*20);first=c.addressof(entries)
 # Exactly one owner entry, independent of accumulated kills.
 struct.pack_into('<QQQ',vector,0x10,first,first,first+len(entries))
 entry=c.create_string_buffer(struct.pack('<iiifi',OWNER,20292,0,-1.0,1))
 aggregate(c.addressof(vector),c.addressof(node),c.addressof(entry))
 end=struct.unpack_from('<Q',vector,0x18)[0];assert end-first==20
 shown=struct.unpack_from('<i',entries,16)[0];assert shown==1
 hud.append(dict(visibleIcons=1,displayedCount=shown))
 # Run the EXE's actual aggregate and per-node max-FP getter. Only relocate
 # code/data references into private memory; native behavior stays intact.
 import capstone
 md=capstone.Cs(capstone.CS_ARCH_X86,capstone.CS_MODE_64);md.detail=True
 regions=[(0x50c230,0x6f,2048),(0x5103d0,0xa1,2304)]
 struct.pack_into('<f',entries,0,1.0)
 for rva,length,offset in regions:
  code=bytearray(pe.get_data(rva,length))
  for ins in md.disasm(bytes(code),rva):
   if ins.mnemonic=='call':
    target=ins.operands[0].imm
    if target==0x5103d0:struct.pack_into('<i',code,ins.address-rva+ins.imm_offset,memory+2304-(memory+offset+ins.address-rva+ins.size))
    # Conditional scaling helper is unreachable (row flag 0x160 is zero).
   for op in ins.operands:
    if op.type==capstone.CS_OP_MEM and op.mem.base==capstone.x86_const.X86_REG_RIP:
     original=ins.address+ins.size+op.mem.disp
     data=pe.get_data(original,4);slot=4096+(original&255)*4
     c.memmove(memory+slot,data,4)
     struct.pack_into('<i',code,ins.address-rva+ins.disp_offset,memory+slot-(memory+offset+ins.address-rva+ins.size))
  c.memmove(memory+offset,bytes(code),len(code))
 getter=c.WINFUNCTYPE(c.c_float,c.c_void_p,c.c_ubyte)(memory+2048)
 manager=c.create_string_buffer(24);rows=[c.create_string_buffer(0x400)for _ in range(3)];nodes=[c.create_string_buffer(0x88)for _ in rows]
 for i,(r,n)in enumerate(zip(rows,nodes)):
  struct.pack_into('<f',r,0x14,[1.2,1.0,1.0][i]);struct.pack_into('<Q',n,0,c.addressof(r));struct.pack_into('<Q',n,0x38,c.addressof(nodes[i+1])if i<2 else 0)
 struct.pack_into('<Q',manager,8,c.addressof(nodes[0]))
 for flags in [0,0x40000,0x80000,0x80000000]:
  struct.pack_into('<I',nodes[0],0x68,flags)
  for mode in [0,1]:
   actual=getter(c.addressof(manager),mode);expected=1.2 if flags==0 else 1.0
   assert math.isclose(actual,expected,rel_tol=1e-6)
   fp.append(dict(flags=flags,mode=mode,nativeRate=actual))
finally:assert k.VirtualFree(memory,0,0x8000)
formula=[dict(kills=count,requestedPower=1.01**count)for count in [0,1,10,50,100,256,1000]]
report=dict(nativeCategoryCases=cases,nativeHudCases=hud,nativeFpCases=fp,serializedResetTimelines=timelines,formula=formula,compactSavedRecordLimit=4,nativeSavedCategoryCapacities=saved_limits,originalParameterRowsAndRewardTriplesPreserved=True if not installed else None,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f'PASS: {len(fp)} native FP aggregation cases, {len(hud)} single-icon case, {len(timelines)} reset timelines and compact records')
