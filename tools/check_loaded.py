"""Read-only current-game DLL/status/hook check; no game writes or archives."""
from pathlib import Path
import ctypes as c,json,subprocess,hashlib
import pefile,capstone
root=Path(__file__).resolve().parents[1]
digest=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
command="$g=Get-Process nightreign -ErrorAction Stop; @{pid=$g.Id;modules=@($g.Modules | Where-Object {$_.ModuleName -in @('nightreign.exe','NRHex.dll','derandomizer.dll','StormControl_no_gui.dll','DeepUnlocker.dll')} | ForEach-Object {@{name=$_.ModuleName;base=$_.BaseAddress.ToInt64();path=$_.FileName}})} | ConvertTo-Json -Depth 4"
info=json.loads(subprocess.run(['powershell','-NoProfile','-Command',command],capture_output=True,text=True,check=True).stdout)
modules={m['name']:m for m in info['modules']};assert len(modules)==5
dll=root/'dist/mod/natives/NRHex.dll';assert digest(dll)==json.loads((root/'dist/build-validation.json').read_text())['installedDllSha256']
pe=pefile.PE(str(dll));exports={s.name:s.address for s in pe.DIRECTORY_ENTRY_EXPORT.symbols}
k=c.WinDLL('kernel32');k.OpenProcess.argtypes=[c.c_uint,c.c_int,c.c_uint];k.OpenProcess.restype=c.c_void_p
k.ReadProcessMemory.argtypes=[c.c_void_p,c.c_void_p,c.c_void_p,c.c_size_t,c.POINTER(c.c_size_t)];k.CloseHandle.argtypes=[c.c_void_p]
h=k.OpenProcess(0x10,False,info['pid']);assert h
def read(p,n):
 b=c.create_string_buffer(n);g=c.c_size_t();assert k.ReadProcessMemory(h,p,b,n,c.byref(g)) and g.value==n;return b.raw
try:
 address=modules['NRHex.dll']['base']+exports[b'NRHex_status']
 md=capstone.Cs(capstone.CS_ARCH_X86,capstone.CS_MODE_64);md.detail=True
 load=next(i for i in md.disasm(read(address,24),address) if i.mnemonic=='movzx' and '[rip' in i.op_str)
 assert read(load.address+load.size+load.operands[1].mem.disp,1)==b'\x01'
 hooks={hex(r):read(modules['nightreign.exe']['base']+r,5).hex() for r in [0x6a20b0,0x6a2180,0x6a2240,0x668b30,0x7e91a0,0x6c16f0,0x570410,0x57ca60,0x57b490,0x3975d0,0xb9ea10,0x353590,0x355d20,0xed60c0,0x353cb0,0x82ba40,0x7df110,0xa89a93,0xe94990,0x33c120,0x66ab90,0x6c5ec0]}
 assert all(v.startswith('e9') for v in hooks.values())
 # This HUD function already starts with a native Arxan JMP on disk. An E9
 # alone would falsely report it as patched; require a different destination.
 assert hooks['0x7df110'] != 'e99176f5ff'
finally:k.CloseHandle(h)

report=json.loads((root/'dist/runtime-validation.json').read_text())
report.update(active=1,pid=info['pid'],allFourNativeDllsLoaded=True,installedHooks=hooks)
(root/'dist/runtime-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print('PASS: current installed DLL ACTIVE=1; twenty-two sampled hook entries including spirit model mapping, detailed status routing and both comparators; four native Mods loaded. Gameplay not inferred.')
