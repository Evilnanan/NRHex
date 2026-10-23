"""Read-only binding checks and isolated original getter/HP-condition execution."""
from pathlib import Path
import ctypes as c,hashlib,json,struct
import pefile
root=Path(__file__).resolve().parents[1]
raw=Path(r'D:/Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game/nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
def check(rva,hex):assert pe.get_data(rva,len(bytes.fromhex(hex)))==bytes.fromhex(hex)
check(0x3ff060,'f30f104118c3')
check(0x405205,'488d8f00180000e84f9effff')
check(0x3ff5a8,'c743180000803f')
check(0x2bfa02c,'0ad7233c')
branch=bytes.fromhex('33f60f28c1410f2fc8f3410f59c10f93c184c9740732d20f2fc67202b201')
check(0x50f808,branch.hex())
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong,c.c_ulong];k.VirtualAlloc.restype=c.c_void_p;k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_ulong]
memory=k.VirtualAlloc(None,4096,0x3000,0x40);assert memory
cases=[]
try:
 c.memmove(memory,pe.get_data(0x3ff060,6),6)
 getter=c.WINFUNCTYPE(c.c_float,c.c_void_p)(memory)
 controller=c.create_string_buffer(0x20)
 for rate in [0.0,0.5,1.0,2.0,3.0]:
  struct.pack_into('<f',controller,0x18,rate);assert getter(c.addressof(controller))==rate
 # Save every nonvolatile register touched by the original condition fragment.
 prefix=bytes.fromhex('56 4883ec30 f30f7f3424 f3440f7f442410 f3440f7f4c2420 0f28f0 450f57c0 b900002042 660f6ec9 b80ad7233c 66440f6ec8')
 suffix=bytes.fromhex('0fb6c2 f30f6f3424 f3440f6f442410 f3440f6f4c2420 4883c430 5e c3')
 code=prefix+branch+suffix;c.memmove(memory+128,code,len(code))
 predicate=c.WINFUNCTYPE(c.c_int,c.c_float)(memory+128)
 def f32(v):return struct.unpack('<f',struct.pack('<f',v))[0]
 for hp,maxhp in [(399,1000),(400,1000),(401,1000),(1,1000),(500,1000),(800,2000)]:
  fraction=f32(f32(hp)/f32(maxhp));expected=fraction<=f32(f32(40)*f32(.01));actual=bool(predicate(fraction));assert actual==expected
  cases.append(dict(hp=hp,maxHp=maxhp,native=actual))
finally:k.VirtualFree(memory,0,0x8000)
rows={r['ID']:r['fields'] for r in json.loads((root/'.work/effects/native-low-hp.json').read_text())}
assert all(rows[id]['conditionHp']==40 for id in [340800,7012300,8660100,8660000])
assert rows[700600]['displayIconForPermamentEffect']==1
serialized={r['id']:r['fields'] for r in json.loads((root/'.work/effects/escape-serialized.json').read_text())}
assert serialized[95001201]['effectEndurance']==10 and serialized[95001202]['effectEndurance']==60
assert serialized[95001203]['iconId']==20222 and serialized[95001203]['displayIconForPermamentEffect']==1
assert serialized[95001201]['conditionHp']==-1 and serialized[95001201]['stateInfo']==0
assert all(serialized[95001201][field]==.4 for field in ['slashDamageCutRate','blowDamageCutRate','thrustDamageCutRate','neutralDamageCutRate','magicDamageCutRate','fireDamageCutRate','thunderDamageCutRate','darkDamageCutRate'])
source=(root/'src/lib.rs').read_text();getter=source[source.index('unsafe extern "system" fn speed_detour'):source.index('unsafe fn update_escape')]
assert 'read_ptr(controller+8)!=player' in getter and 'read_ptr(modules+0x28).checked_add(0x1800)' in getter
assert 'ptr::write' not in getter and '2.0}else{rate}' in getter
assert 'speed_prepared.stolen!=5' in source
report=dict(version='0.13.0',originalSpeedGetterIsolatedExecutionPassed=True,originalLowHpComparisonIsolatedExecutionPassed=True,hpCases=cases,ctPointerChain='player+1B8 -> module+28 -> controller+1800 -> rate+18',onlyLocalActiveOwner=True,noSpeedFieldWrites=True,readyHudIcon=20222,ultimateReadyHudIcon=20297,gameplayVerified=False,multiplayerVerified=False)
(root/'dist/reports/escape-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print('PASS: original speed getter and low-HP predicate execution; local-only 2x rate, ready HUD and serialized buff scope.')
