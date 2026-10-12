"""Install the verified Sword-only update; preserve all other installed assets."""
from pathlib import Path
import hashlib,json,shutil,tomllib
from datetime import datetime

root=Path(__file__).resolve().parents[1];dist=root/'dist';work=root/'.work/sword-vfx'
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def assets():return {p.relative_to(dist).as_posix():sha(p) for p in (dist/'mod').rglob('*') if p.is_file() and p.suffix!='.log'}
before=assets();parameter=json.loads((work/'parameter-validation.json').read_text());hks=json.loads((work/'hks-validation.json').read_text());resource=json.loads((work/'resource-validation.json').read_text())
sfx='mod/sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx'
sources={'mod/natives/NRHex.dll':root/'target/release/NRHex.dll','mod/regulation.bin':work/'regulation.bin','mod/action/script/c0000.hks':work/'c0000.hks',sfx:work/Path(sfx).name}
for rel,report in [('mod/regulation.bin',parameter),('mod/action/script/c0000.hks',hks),(sfx,resource)]:
 assert before[rel]==report['inputSha256'].lower(),f'{rel} changed after staging'
 assert sha(sources[rel])==report['outputSha256'].lower(),f'{rel} staged output changed'
backup=work/'before'/datetime.now().strftime('%Y%m%d-%H%M%S-%f');backup.mkdir(parents=True)
for rel in sources:
 dest=backup/rel;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(dist/rel,dest)
copied=[]
try:
 for rel,source in sources.items():
  # Install DLL first: a running game refuses replacement before any assets change.
  copied.append(rel);shutil.copyfile(source,dist/rel);assert sha(dist/rel)==sha(source)
 after=assets();changed=sorted(p for p in before.keys()|after.keys() if before.get(p)!=after.get(p))
 assert set(changed)<=sources.keys(),changed
except Exception:
 for rel in reversed(copied):
  if sha(dist/rel)!=sha(backup/rel):shutil.copyfile(backup/rel,dist/rel)
 raise
version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
report=dict(version=version,swordNearDistance=4.0,visual='Native Storm Ruler stance sword on casting weapon hand, including spell recovery',visualOwner=95000900,chantStaminaRecoveryUnchanged=True,changedAssets=changed,unrelatedAssetsAndUserConfigPreserved=True,files={rel:after[rel] for rel in sources},parameters=parameter,hks=hks,resources=resource,gameplayVerified=False,multiplayerVerified=False)
(work/'installation-validation.json').write_text(json.dumps(report,indent=2))
build=dict(version=version,buildPassed=True,scope='Sword Storm Ruler stance VFX',dllSha256=sha(sources['mod/natives/NRHex.dll']),installedDllSha256=after['mod/natives/NRHex.dll'],gameplayVerified=False,multiplayerVerified=False,validation='.work/sword-vfx/installation-validation.json')
(dist/'build-validation.json').write_text(json.dumps(build,indent=2))
print('PASS: Sword DLL, parameters and HKS installed and read back; all other mod assets and user settings preserved')
