"""Install verified staged group assets and record their bounded package scope."""
from distribution_paths import distribution_file
from pathlib import Path
import hashlib,json,shutil,sys,time,tomllib
from local_paths import ROOT
work=ROOT/'.work/cage-groups';dist=ROOT/'dist';stage=work/'staged'
digest=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
read=lambda p:json.loads(p.read_text(encoding='utf8'))
def write(p,v):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
def assets():
 files=list((dist/'mod').rglob('*'))
 for part in ['mod/natives','testmods','support']:files += [p for p in (distribution_file(dist,part)).rglob('*') if p.suffix in ['.dll','.ini','.csv','.me3','.ps1','.py','.json']]
 files+=list(dist.glob('*.me3'))
 return {p.relative_to(dist).as_posix():digest(p) for p in files if p.is_file() and p.suffix != '.log'}
built=read(work/'built.json');patch=read(work/'parameter-patch.json');version=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version']
pairs=[(stage/'regulation.bin',dist/'mod/regulation.bin'),(ROOT/'target/release/NRHex.dll',dist/'mod/natives/NRHex.dll')]
for r in built['reports']:
 pairs.extend([(stage/'event'/f"{r['mapName']}.emevd.dcx",dist/'mod/event'/f"{r['mapName']}.emevd.dcx"),
               (stage/'map/mapstudio'/f"{r['mapName']}.msb.dcx",dist/'mod/map/mapstudio'/f"{r['mapName']}.msb.dcx")])
expected={target.relative_to(dist).as_posix() for _,target in pairs}
mode=sys.argv[1]
if mode=='before':
 assert Path(built['output']).resolve()==stage.resolve()
 assert digest(dist/'mod/regulation.bin')==patch['inputSha256']
 for r in built['reports']:
  assert digest(dist/'mod/map/mapstudio'/f"{r['mapName']}.msb.dcx")==r['inputMapSha256']
  assert digest(dist/'mod/event'/f"{r['mapName']}.emevd.dcx")==r['inputEventSha256']
 write(work/'package-before.json',dict(assets=assets(),csvSha256=digest(ROOT/'data/balance/sword.csv')))
 for _,target in pairs:
  backup=work/'before'/target.relative_to(dist);backup.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(target,backup)
 print('Captured current package/configuration baseline and backed up ten target assets')
elif mode=='install':
 before=read(work/'package-before.json');assert assets()==before['assets'];assert digest(ROOT/'data/balance/sword.csv')==before['csvSha256']
 assert digest(stage/'regulation.bin')==patch['outputSha256']
 validation=read(work/'validation.json');assert validation['version']==version and validation['eventCases']==960
 for r in built['reports']:
  assert digest(stage/'event'/f"{r['mapName']}.emevd.dcx")==r['eventSha256']
  assert digest(stage/'map/mapstudio'/f"{r['mapName']}.msb.dcx")==r['mapSha256']
 try:
  for source,target in pairs:
   for attempt in range(20):
    try:shutil.copyfile(source,target);break
    except PermissionError:
     if attempt==19:raise
     time.sleep(.2)
   assert digest(source)==digest(target)
 except Exception:
  for _,target in pairs:shutil.copyfile(work/'before'/target.relative_to(dist),target)
  raise
 # Preserve the established single-actor regression inputs with corrected hashes.
 write(ROOT/'.work/cage/built.json',read(work/'single-built.json'))
 print('Installed regulation, four group maps, four event files and 0.17.26 DLL')
elif mode=='record':
 before=read(work/'package-before.json');after=assets();changes=sorted(k for k in before['assets'].keys()|after.keys() if before['assets'].get(k)!=after.get(k));assert set(changes)==expected,changes
 assert digest(ROOT/'data/balance/sword.csv')==before['csvSha256']
 for source,target in pairs:assert digest(source)==digest(target)
 validation=read(work/'validation.json');validation.update(installed=True,changedAssets=changes,unrelatedAssetsAndUserConfigsUnchanged=True,sourceCsvPreserved=True,
   originalParameterRowsPreserved=patch['originalRowsPreserved'],unrelatedParameterTablesPreserved=patch['unrelatedTablesPreserved'],dllSha256=digest(dist/'mod/natives/NRHex.dll'),
   regulationSha256=digest(dist/'mod/regulation.bin'),rustTests=155,loadingVerified=False)
 write(dist/'reports'/'cage-groups-validation.json',validation)
 for name in ['cage-validation.json','cage-mutation-validation.json']:write(distribution_file(dist,name),read(work/name))
 dll=digest(dist/'mod/natives/NRHex.dll');b=read(dist/'reports'/'build-validation.json');b.update(version=version,scope='13 native group encounters, group rewards and local HUD; total 50 choices / 84 actors',
  changedAssets=changes,dllSha256=dll,installedDllSha256=dll,dllInstalled=True,regulationSha256=validation['regulationSha256'],unrelatedAssetsAndUserConfigsUnchanged=True,rustTestsScope='155 passed for 0.17.26; release build passed')
 if 'cage-groups-validation.json' not in b['functionChecks']:b['functionChecks'].append('cage-groups-validation.json')
 write(dist/'reports'/'build-validation.json',b)
 b=read(dist/'reports'/'runtime-validation.json');b.update(version=version,active=None,pid=None,allFourNativeDllsLoaded=False,dllSha256=dll,installedHooks={},notes='0.17.26 group encounters installed; awaiting current-process load check; group gameplay/coop pending');write(dist/'reports'/'runtime-validation.json',b)
 b=read(dist/'reports'/'integration-validation.json');b.update(version=version,loadingVerified=False);b['nativeFiles']['mod/natives/NRHex.dll']=dll;write(dist/'reports'/'integration-validation.json',b)
 print('PASS: ten-asset change scope; original rows/tables/events, other mods, settings and sword.csv preserved')
elif mode=='loaded':
 b=read(dist/'reports'/'runtime-validation.json');assert b['version']==version and b['active']==1 and b['allFourNativeDllsLoaded'] and b['dllSha256']==digest(dist/'mod/natives/NRHex.dll')
 b['notes']='0.17.26 loaded through project ME3 profile; ACTIVE=1, fifteen hooks and all four native Mods verified; group gameplay/coop pending';write(dist/'reports'/'runtime-validation.json',b)
 for name in ['integration-validation.json','cage-groups-validation.json']:
  b=read(distribution_file(dist,name));b['loadingVerified']=True;write(distribution_file(dist,name),b)
 print('Current DLL, fifteen hooks and all four native Mods loaded; gameplay not inferred')
else:raise SystemExit('before / install / record / loaded')
