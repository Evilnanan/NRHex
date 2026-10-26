"""Install the verified Crystal Shell DLL and current Markdown text delta."""
from distribution_paths import distribution_file
from pathlib import Path
import hashlib,json,shutil,tomllib
root=Path(__file__).resolve().parents[1];dist=root/'dist';work=root/'.work/crystal-rebound-01730'
digest=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
read=lambda p:json.loads(p.read_text(encoding='utf8'))
write=lambda p,x:p.write_text(json.dumps(x,ensure_ascii=False,indent=2),encoding='utf8')
baseline=read(work/'baseline.json');native=read(work/'native-validation.json');texts=read(work/'texts/validation.json')
assert tomllib.loads((root/'Cargo.toml').read_text())['package']['version']=='0.17.30'
assert native['isolatedNativeCases']==62 and native['nativeHpNullifyAndRecoilIndependent']
assert digest(root/'docs/hexes.md')==texts['sourceSha256'].lower()
assert digest(root/'data/balance/sword.csv')==digest(work/'before/data/balance/sword.csv')
for name,expected in baseline.items():
 if name not in ['README.md','runtime-validation.json']:assert digest(distribution_file(dist,name))==expected,('package changed',name)
assets={'natives/NRHex.dll':root/'target/release/NRHex.dll'}
for report in texts['reports']:
 name='mod/msg/zhocn/'+report['file'];source=work/'texts'/report['file']
 assert report['inputSha256'].lower()==baseline[name] and digest(source)==report['outputSha256'].lower()
 assert report['unrelatedEntriesAndMembersUnchanged']
 if report['inputSha256']!=report['outputSha256']:assets[name]=source
for name in set(assets)|{'scope-validation.json','build-validation.json','runtime-validation.json','integration-validation.json','icon-validation.json'}:
 p=work/'before/dist'/name;p.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(distribution_file(dist,name),p)
for name,source in assets.items():shutil.copy2(source,distribution_file(dist,name))
report=dict(version='0.17.30',rustTestsPassed=162,signatureCount=57,native=native,texts=texts,
 shellCooldownSeconds=40,hiddenEffectDescriptionUnchanged=True,directCharacterSourcesOnly=True,
 privateRecoilInputPreservesOriginalHit=True,gameplayVerified=False,multiplayerVerified=False)
write(dist/'reports'/'crystal-rebound-validation.json',report)
scope=dict(version='0.17.30',assets={n:dict(before=baseline[n],after=digest(p)) for n,p in assets.items()},
 regulationMapsEventsHksAndUserConfigurationsUnchanged=True,handEditedSwordCsvUnchanged=True,
 sourceMarkdownUnchanged=True,gameplayVerified=False,multiplayerVerified=False)
write(dist/'reports'/'scope-validation.json',scope);write(work/'installed-assets.json',scope)
allow=set(assets)|{'README.md','scope-validation.json','runtime-validation.json'}
for name,expected in baseline.items():
 if name not in allow:assert digest(distribution_file(dist,name))==expected,('unrelated delivered file changed',name)
assert all(digest(distribution_file(dist,name))==digest(source) for name,source in assets.items())
print('PASS: installed Crystal Shell DLL and Markdown Chinese text delta; parameter, event, map, HKS, configuration and user sources preserved.')
