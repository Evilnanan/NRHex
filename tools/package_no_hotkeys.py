"""Package the compiled no-hotkeys DLL with current mod assets and destination settings."""
from pathlib import Path
import argparse,hashlib,json,re,shutil,tomllib
import pefile

root=Path(__file__).resolve().parents[1]
args=argparse.ArgumentParser();args.add_argument('--destination',required=True);args=args.parse_args()
destination=Path(args.destination).resolve();dist=root/'dist'
assert destination!=Path(destination.anchor) and destination!=dist and not destination.is_relative_to(dist)
assert not dist.is_relative_to(destination), 'Destination must not contain the project distribution'
assert not (destination/'natives').exists(), 'Migrate existing natives to mod/natives before packaging'
version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
dll=root/'.work/no-hotkeys-target/release/NRHex.dll';assert dll.is_file()
pe=pefile.PE(str(dll),fast_load=True);pe.parse_data_directories(directories=[1])
imports={i.name for entry in pe.DIRECTORY_ENTRY_IMPORT for i in entry.imports}
assert b'GetAsyncKeyState' not in imports, 'DLL still imports test keyboard polling'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
baseline_dll=sha(dist/'mod/natives/NRHex.dll')
assert sha(dll)!=baseline_dll,'Production and test DLL unexpectedly identical'
backup=root/'.work/no-hotkeys-package-before'/version
files={}
def write(rel,data):
 p=destination/rel;assert p.resolve().is_relative_to(destination)
 if p.exists():
  b=backup/rel;b.parent.mkdir(parents=True,exist_ok=True)
  if not b.exists():shutil.copy2(p,b)
 p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(data)
 assert p.read_bytes()==data
 files[rel]=sha(p)
for p in (dist/'mod').rglob('*'):
 if not p.is_file() or p.is_relative_to(dist/'mod/natives') or p.suffix=='.log':continue
 write(p.relative_to(dist).as_posix(),p.read_bytes())
write('mod/natives/NRHex.dll',dll.read_bytes())
ini=destination/'mod/natives/NRHex.ini'
config=(ini if ini.exists() else dist/'mod/natives/NRHex.ini').read_text(encoding='utf-8-sig')
retired={'VirtualKey','TestVirtualKey','NewHexVirtualKey','ChantVirtualKey','AngelChantVirtualKey','StormTestVirtualKey','RequireControl','CooldownMs','SwordNearDistance'}
config='\n'.join(line for line in config.splitlines() if not line.startswith(('; Ctrl+','; Test reward hotkeys are disabled')) and line.split('=',1)[0].strip() not in retired)
config=config.replace('[TestSpawn]','[Settings]')
write('mod/natives/NRHex.ini',('; Test reward hotkeys are disabled at compile time.\n'+config+'\n').encode('utf-8'))
profile=destination/'NRHex.me3'
if profile.exists():
 text=profile.read_text(encoding='utf-8-sig').replace('"natives/NRHex.dll"','"mod/natives/NRHex.dll"')
 parsed=tomllib.loads(text)
 assert any(n.get('path')=='mod/natives/NRHex.dll' for n in parsed.get('natives',[])), 'Existing profile has no NRHex DLL entry'
 assert any(p.get('source',p.get('path'))=='mod' for p in parsed.get('packages',[])), 'Existing profile has no mod package'
else:
 text='profileVersion = "v1"\nstart_online = false\ndisable_arxan = true\n\n[[supports]]\ngame = "nightreign"\n\n[[natives]]\npath = "mod/natives/NRHex.dll"\n\n[[packages]]\nid = "nrhex"\nsource = "mod"\n'
write('NRHex.me3',text.encode('utf-8'))
write('NRHex-README.md',f'# NRHex {version} 群友测试版\n\nDLL与配置位于mod/natives。编译时移除全部测试快捷键发球入口，修改INI无法开启。保留正常升级发球及海克斯机制。亮出你的剑近身判定固定4米，INI不能修改。保留目标ME3联机／其他Mod配置。未启动游戏，当前目录实际加载待验证。\n'.encode('utf-8'))
assert sha(dist/'mod/natives/NRHex.dll')==baseline_dll
write('NRHex-build.json',json.dumps(dict(version=version,testHotkeys=False,keyboardPollingImportAbsent=True,swordNearDistance=4.0,swordRangeConfigurable=False,dllSha256=sha(dll),files=files.copy(),gameplayVerified=False,loadingVerified=False),ensure_ascii=False,indent=2).encode('utf-8'))
print(f'PASS: {version} no-hotkeys package installed in {destination}; {len(files)} files verified; existing non-test settings preserved')
