"""Install the verified combat fixes and one-second text; retain other assets."""
from pathlib import Path
from datetime import datetime
import argparse, hashlib, json, re, shutil, tomllib

ROOT = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser()
p.add_argument('--dll', type=Path, default=ROOT/'target/release/NRHex.dll')
args = p.parse_args()
dist = ROOT/'dist'
work = ROOT/'work/combat-protection'
texts = work/'texts'

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))

def assets():
    return {p.relative_to(dist).as_posix(): sha(p) for p in (dist/'mod').rglob('*')
            if p.is_file() and p.suffix != '.log'}

native = {name: read(ROOT/f'work/{name}/native-validation.json')
          for name in ('death-dance', 'erosion')}
for report in native.values():
    assert report['nativeExecutionPassed']
    for path, digest in report['sources'].items():
        assert sha(ROOT/path) == digest, 'Source changed after verification: '+path
assert native['death-dance']['debitSeconds'] == 1
assert native['erosion']['bodyContactsExcluded']
test_log = (work/'rust-tests.log').read_text(errors='replace')
tests = re.search(r'test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;', test_log)
assert tests and int(tests[1]) > 0, 'Rust suite did not pass'
signature_count = int(re.search(r"fn signatures\(\)->\[\(&'static str,u32\);(\d+)\]",
                                (ROOT/'src/lib.rs').read_text())[1])
assert 'PASS: current EXE hash and runtime signatures are unique at the researched RVAs.' in (work/'scancheck.log').read_text(), 'Signature check missing'
text_report = read(texts/'validation.json')
for field, digest in [('source', 'sourceSha256'), ('englishSource', 'englishSourceSha256'),
                      ('statusSource', 'statusSourceSha256')]:
    assert sha(ROOT/text_report[field]) == text_report[digest].lower()
before = assets()
sources = {'mod/natives/NRHex.dll': args.dll}
for report in text_report['reports']:
    relative = f"mod/msg/{report['language']}/{report['file']}"
    source = texts/report['language']/report['file']
    assert report['unrelatedEntriesAndMembersUnchanged']
    assert all(c['id'] == 95003000 for c in report['changes']), 'Unrelated text change'
    assert before[relative] == report['inputSha256'].lower()
    assert sha(source) == report['outputSha256'].lower()
    sources[relative] = source
assert len(sources) == 5
backup = work/'before'/datetime.now().strftime('%Y%m%d-%H%M%S-%f')
for relative in sources:
    target = backup/relative
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(dist/relative, target)
try:
    for relative, source in sources.items():
        shutil.copyfile(source, dist/relative)
        assert sha(dist/relative) == sha(source)
    after = assets()
    changed = sorted(p for p in before.keys() | after.keys() if before.get(p) != after.get(p))
    assert set(changed) <= sources.keys(), 'Unrelated asset changed'
except Exception:
    for relative in sources:
        shutil.copy2(backup/relative, dist/relative)
    raise
version = tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version']
report = dict(version=version, changedAssets=changed, files={p: after[p] for p in sources},
              native=native, texts=text_report, rustPassed=int(tests[1]), rustIgnored=int(tests[2]),
              signatureCount=signature_count, debitSeconds=1, perFrameDebit=True,
              allOtherResourcesAndUserSettingsPreserved=True, backup=str(backup),
              gameplayVerified=False, multiplayerVerified=False)
(dist/'combat-protection-validation.json').write_text(json.dumps(report, indent=2), encoding='utf8')
(dist/'build-validation.json').write_text(json.dumps(dict(
    version=version, buildPassed=True, scope='Smooth one-second Death Dance with native damage immunity/HP floors; Erosion body-contact filtering',
    dllSha256=after['mod/natives/NRHex.dll'], signatureCount=signature_count,
    validation='combat-protection-validation.json', gameplayVerified=False, multiplayerVerified=False), indent=2), encoding='utf8')
runtime = read(dist/'runtime-validation.json')
pending = runtime.get('pendingChecks', [])
pending += ['One-second smooth Death Dance during roll and Ultimate Art immunity in game',
            'Revenant HP lock during Death Dance debt in game',
            'Erosion roll contact versus weapon follow-up and simultaneous projectiles in game']
runtime.update(version=version, active=None, pid=None, dllSha256=after['mod/natives/NRHex.dll'],
               build='build-validation.json', gameplayVerified=False, multiplayerVerified=False,
               pendingChecks=list(dict.fromkeys(pending)))
(dist/'runtime-validation.json').write_text(json.dumps(runtime, indent=2), encoding='utf8')
print('PASS: combat DLL and bilingual one-second text installed; all other assets/settings preserved')
