"""Read-only Parry ABI/data checks; execute untouched native getters in private memory."""
import ctypes as c,hashlib,json,struct
from pathlib import Path
import pefile
from local_paths import ROOT,GAME
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
p=pefile.PE(data=raw,fast_load=True);p.parse_data_directories(directories=[5])
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p;k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
m=k.VirtualAlloc(None,p.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert m
cases=[]
try:
 c.memmove(m,raw[:p.OPTIONAL_HEADER.SizeOfHeaders],p.OPTIONAL_HEADER.SizeOfHeaders)
 for s in p.sections:c.memmove(m+s.VirtualAddress,s.get_data(),s.SizeOfRawData)
 for block in p.DIRECTORY_ENTRY_BASERELOC:
  for r in block.entries:
   if r.type==10:
    q=c.c_uint64.from_address(m+r.rva);q.value=(q.value+m-p.OPTIONAL_HEADER.ImageBase)&0xffffffffffffffff
   else:assert r.type==0
 for off,rva in [(0x388,0x66b910),(0x118,0x4d2f00),(0x250,0x66cde0),(0x260,0x66b8c0),(0x278,0x66b060),(0x328,0x668b30),(0x4d0,0x3c9660),(0x4a0,0x66c400)]:
  assert p.get_data(0x2c307f8+off,8)==struct.pack('<Q',p.OPTIONAL_HEADER.ImageBase+rva)
 # HKS's c_Style comes from env(207). Verify its actual dispatch and getter,
 # then execute that getter on synthetic equipment for both hands and rejects.
 index=p.get_data(0x3fad88+207-105,1)[0]
 assert struct.unpack('<I',p.get_data(0x3fad38+index*4,4))[0]==0x3f7f97
 assert p.get_data(0x3f7f97,12)==bytes.fromhex('48 8b 0e 48 8b 01 ff 90 78 02 00 00')
 assert p.get_data(0x66b060,10)==bytes.fromhex('48 8b 81 70 06 00 00 8b 00 c3')
 style_actor=c.create_string_buffer(0x678);equipment=c.c_int()
 c.c_void_p.from_buffer(style_actor,0x670).value=c.addressof(equipment)
 style_get=c.WINFUNCTYPE(c.c_int,c.c_void_p)(m+0x66b060)
 for style in [-1,0,1,2,3,4,5,2147483647]:
  equipment.value=style;actual=style_get(c.addressof(style_actor));assert actual==style
  cases.append(dict(equipmentStyle=actual,twoHanded=actual in (2,3)))
 # The native airborne query includes controller/action takeoff even while
 # collision contact is still set, covering jumping skills and edge frames.
 assert p.get_data(0x4678e0,25)==bytes.fromhex('40 57 48 83 ec 20 80 b9 92 00 00 00 00 48 8b f9 74 43 48 81 c1 00 04 00 00')
 assert p.get_data(0x467919,13)==bytes.fromhex('48 8b 4a 58 48 c1 e9 37 80 e1 01 0a cb')
 ground_actor=c.create_string_buffer(0x200);ground_modules=c.create_string_buffer(0x90)
 ground_action=c.create_string_buffer(0x60);physics=c.create_string_buffer(0x450)
 c.c_void_p.from_buffer(ground_actor,0x1b8).value=c.addressof(ground_modules)
 c.c_void_p.from_buffer(ground_modules,8).value=c.addressof(ground_action)
 c.c_void_p.from_buffer(ground_modules,0x68).value=c.addressof(physics)
 c.c_void_p.from_buffer(physics,8).value=c.addressof(ground_actor)
 c.c_void_p.from_buffer(physics,0x420).value=c.addressof(ground_actor)
 airborne=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p)(m+0x4678e0)
 for contact in [0,1]:
  for action_air in [0,1]:
   for jump in [0.,.1]:
    c.c_ubyte.from_buffer(physics,0x92).value=contact
    c.c_uint64.from_buffer(ground_action,0x58).value=action_air<<55
    c.c_float.from_buffer(physics,0x440).value=jump
    actual=airborne(c.addressof(physics));assert actual==int(not contact or action_air or jump>0)
    cases.append(dict(contact=contact,actionAir=action_air,jumpController=jump,nativeAirborne=actual))
 # Native guardability/facing gates precede the cost hook. Its continuation
 # preserves the native no-stamina flag and state-606 handling, but skips the
 # <= current-stamina guard-break comparison (including zero stamina).
 assert p.get_data(0x43a0da,8)==bytes.fromhex('41 c6 86 b4 02 00 00 01')
 assert p.get_data(0x43a0fd,9)==bytes.fromhex('f3 41 0f 10 b6 4c 02 00 00')
 assert p.get_data(0x43a18e,8)==bytes.fromhex('41 f6 86 60 01 00 00 02')
 # Final reaction notification (four integer-register args) happens downstream
 # of the crystal proc and death/nullification gates; native GUARD is kind 3.
 assert p.get_data(0x4381f0,19)==bytes.fromhex('48 89 5c 24 08 55 56 57 41 56 41 57 48 83 ec 40 45 8b f0')
 assert p.get_data(0x438198,5)==bytes.fromhex('bf 03 00 00 00')
 assert p.get_data(0x4381ae,14)==bytes.fromhex('4c 8b 4c 24 20 44 8b c7 48 8b d6 49 8b ce')
 code=p.get_data(0x4381bc,5)
 assert code[0]==0xe8 and 0x4381c1+struct.unpack('<i',code[1:])[0]==0x4381f0
 # Native one-shot FFX (TAE 96) and conditional Wwise sound (TAE 129) both
 # gate on their u16 stateInfo at +10 through the existing queried manager.
 assert p.get_data(0x415d77,4)==bytes.fromhex('0f b7 53 10')
 assert p.get_data(0x40f0d7,4)==bytes.fromhex('0f b7 56 10')
 for site in [0x415d8d,0x40f0eb]:
  code=p.get_data(site,5);assert code[0]==0xe8 and site+5+struct.unpack('<i',code[1:])[0]==0x508e20
 assert p.get_data(0x438305,4)==bytes.fromhex('44 89 73 38') # original action notification
 # HKS env(237): actual resolved impact + weapon guard-motion class, not
 # stamina cost. Zero impact suppresses the guard reaction on every weapon.
 assert p.get_data(0x3f84a9,7)==bytes.fromhex('8b 50 20 0f b6 48 1c')
 code=p.get_data(0x3f84b0,5)
 assert code[0]==0xe8 and 0x3f84b5+struct.unpack('<i',code[1:])[0]==0x6c8970
 motion=c.WINFUNCTYPE(c.c_int,c.c_ubyte,c.c_int)(m+0x6c8970)
 for cls in range(5):
  for impact in range(12):
   native=motion(impact,cls)
   table=struct.unpack('<i',p.get_data(0x39a9110+(cls*12+impact)*4,4))[0]
   assert native==table
   if impact==0:assert native==0
   if cls>0 and impact==1:assert native==1
   cases.append(dict(guardClass=cls,impact=impact,nativeGuardMotion=native))
 code=p.get_data(0x43a0cd,5)
 assert code[0]==0xe8 and 0x43a0d2+struct.unpack('<i',code[1:])[0]==0x43cbf0
 # The tag is consumed by the existing native resolver path, not cached.
 assert p.get_data(0x439d95,12)==bytes.fromhex('8b 93 74 02 00 00 ff 90 88 03 00 00')
 # Source getter is leaf code reading the current attack hand, not equipment weight.
 actor=c.create_string_buffer(0x720)
 source=c.WINFUNCTYPE(c.c_int,c.c_void_p)(m+0x66b8c0)
 for mode,wanted in [(0,-2),(1,-1),(2,6),(3,7),(4,8),(5,9),(6,-2)]:
  c.c_int.from_buffer(actor,0x704).value=mode
  actual=source(c.addressof(actor));assert actual==wanted,(mode,actual)
  cases.append(dict(mode=mode,nativeSource=actual))
 # Original TAE attack and guard event handlers both consume +0E stateInfo.
 for site in [0x40ca88,0x40d3ff]:
  code=p.get_data(site,5);assert code[0]==0xe8 and site+5+struct.unpack('<i',code[1:])[0]==0x508e20
 assert p.get_data(0x502c5d,4)==bytes.fromhex('4d8b4010')
 assert p.get_data(0x508e20,7)==bytes.fromhex('440fb7d24c8bd9')
 query=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p,c.c_uint16)(m+0x508e20)
 manager=c.create_string_buffer(0x40);node=c.create_string_buffer(0x70);effect=c.create_string_buffer(0x160)
 c.c_void_p.from_buffer(manager,8).value=c.addressof(node)
 c.c_void_p.from_buffer(manager,0x10).value=c.addressof(actor)
 c.c_uint64.from_buffer(manager,0x28).value=0xffffffffffffffff
 c.c_void_p.from_buffer(node).value=c.addressof(effect)
 for state in [3,434,14000,17761,25013]:
  c.c_uint16.from_buffer(effect,0x156).value=state
  for flags in [0,1,0x40000,0x80000000]:
   c.c_uint32.from_buffer(node,0x68).value=flags
   assert query(c.addressof(manager),state)==int(flags==0)
   assert query(c.addressof(manager),state+1)==0
   cases.append(dict(state=state,flags=flags))
finally:assert k.VirtualFree(m,0,0x8000)
work=ROOT/'.work/parry';stage=work/'staged'
a=json.loads((stage/'actions-validation.json').read_text())
assert a['guardJudge']==95002400 and a['nativeGuardJudge']==460
assert a['endState']<=24000 and a['originalEventsPreserved'] and a['originalGuardClipsUntouched']
assert a['minTier']==2 and a['tiers']==7 and a['timeUnit']=='milliseconds'
assert a['totalMilliseconds']==[67,100,133,167,200,233,267]
assert a['beforeMilliseconds']==[45,67,89,111,133,155,178]
assert a['afterMilliseconds']==[22,33,44,56,67,78,89]
assert [before+after for before,after in zip(a['beforeMilliseconds'],a['afterMilliseconds'])]==a['totalMilliseconds']
assert a['endState']==a['firstState']+a['conditionCount']*7 and a['verifiedWindows']==a['added']
assert (ROOT/'src/parry_data.rs').read_bytes()==(stage/'parry_data.rs').read_bytes()
report=dict(nativeCases=len(cases),cases=cases,playerVirtualMethodsVerified=True,managerOwnerVerified=True,nativeStateQueryUsesUnsigned16Bit=True,nativeAttackAndGuardTaeGatesVerified=True,finalGuardNotificationVerified=True,nativeZeroImpactGuardMotionVerified=True,nativeFeedbackTaeConditionsVerified=True,guardDirectionRemainsNative=True,attackGuardStaminaWaived=True,gameMemoryWritten=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f'PASS: {len(cases)} native source/state cases; canonical owner and virtual methods verified; no game writes')
