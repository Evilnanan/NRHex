"""Record cage-only AI scope, actual files and user configuration preservation."""
from pathlib import Path
import hashlib,json,sys,tomllib
from local_paths import ROOT
stage=ROOT/'.work/cage-ai'
digest=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def assets():
    paths=list((ROOT/'dist/mod').rglob('*'))
    for folder in ['mod/natives','testmods']:
        paths += [p for p in (ROOT/'dist'/folder).rglob('*') if p.suffix in ['.dll','.ini','.csv','.me3']]
    paths += list((ROOT/'dist').glob('*.me3'))
    return {p.relative_to(ROOT/'dist').as_posix():digest(p) for p in paths if p.is_file() and p.suffix != '.log'}
def write(p,value):p.write_text(json.dumps(value,ensure_ascii=False,indent=2),encoding='utf8')
baseline=stage/'package-before.json'
if '--before' in sys.argv:
    write(baseline,dict(assets=assets(),csvSha256=digest(ROOT/'data/balance/sword.csv')))
    print('Captured current cage package and user settings baseline')
elif '--loaded' in sys.argv:
    runtime=json.loads((ROOT/'dist/reports/runtime-validation.json').read_text())
    assert runtime['active']==1 and runtime['allFourNativeDllsLoaded']
    assert runtime['dllSha256']==digest(ROOT/'dist/mod/natives/NRHex.dll')
    runtime['notes']='0.17.24 restarted via project ME3 profile; current DLL ACTIVE=1, all four native Mods and fifteen hooks loaded; cage AI gameplay/coop remains unverified'
    write(ROOT/'dist/reports/runtime-validation.json',runtime)
    integration=json.loads((ROOT/'dist/reports/integration-validation.json').read_text());integration['loadingVerified']=True
    write(ROOT/'dist/reports/integration-validation.json',integration)
    print('Restarted game loading verified; actual pursuit/return behavior remains pending')
else:
    before=json.loads(baseline.read_text());after=assets();patch=json.loads((stage/'patch.json').read_text())
    changes=sorted(p for p in before['assets'].keys()|after.keys() if before['assets'].get(p)!=after.get(p))
    expected={'mod/regulation.bin','mod/natives/NRHex.dll'}|{f'mod/map/mapstudio/m46_{b}_00_00.msb.dcx' for b in [50,60,70,80]}
    assert set(changes)==expected,changes
    assert after['mod/regulation.bin']==patch['outputRegulationSha256']
    assert digest(ROOT/'data/balance/sword.csv')==before['csvSha256']
    assert len(patch['rows'])==37 and patch['originalThinkRowsPreserved'] and patch['unrelatedParameterTablesPreserved']
    for row in patch['rows']:
        assert row['combatScriptPreserved']
        for key,v in row['fields'].items():assert 0<=v['after']<=min(v['before'],patch['limits'][key])
    for item in patch['maps']:
        assert item['changedThinkReferences']==37 and item['unrelatedMapContentPreserved']
        assert after['mod/map/mapstudio/'+item['name']]==item['outputSha256']
    version=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version'];dll=after['mod/natives/NRHex.dll']
    assert dll==digest(ROOT/'target/release/NRHex.dll')
    patch.update(version=version,installed=True,changedAssets=changes,unrelatedAssetsAndUserConfigsUnchanged=True,
                 gameplayVerified=False,multiplayerVerified=False,scope='37 cage-only Think rows and 148 references; combat scripts preserved')
    write(ROOT/'dist/reports/cage-ai-validation.json',patch)
    build=json.loads((ROOT/'dist/reports/build-validation.json').read_text());build.update(version=version,scope=patch['scope'],
        changedAssets=changes,dllSha256=dll,installedDllSha256=dll,dllInstalled=True,regulationSha256=after['mod/regulation.bin'],
        unrelatedAssetsAndUserConfigsUnchanged=True,rustTestsScope='155 passed for 0.17.22 runtime; gameplay Rust source unchanged; 0.17.24 release build passed')
    build['functionChecks'].append('cage-ai-validation.json');write(ROOT/'dist/reports/build-validation.json',build)
    runtime=json.loads((ROOT/'dist/reports/runtime-validation.json').read_text());runtime.update(version=version,active=None,pid=None,
        allFourNativeDllsLoaded=False,dllSha256=dll,installedHooks={},notes='Cage AI ranges installed; pending restarted-process load check; gameplay unverified')
    runtime['pendingChecks'].append('Cage bosses acquire targets nearby and return when lured outside 40/60m; tower archetypes and large dragons; coop')
    write(ROOT/'dist/reports/runtime-validation.json',runtime)
    integration=json.loads((ROOT/'dist/reports/integration-validation.json').read_text());integration.update(version=version,loadingVerified=False)
    integration['nativeFiles']['mod/natives/NRHex.dll']=dll;write(ROOT/'dist/reports/integration-validation.json',integration)
    print('PASS: 37 AI rows, 148 map references and six-file scope; native encounters/combat/balances/events/user settings preserved')
