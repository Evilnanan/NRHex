"""Install the validated additive Night's Blessing package with rollback."""
from distribution_paths import distribution_file
from pathlib import Path
import hashlib
import json
import shutil
import subprocess
import tomllib

root = Path(__file__).resolve().parents[1]
dist = root / 'dist'
work = root / '.work/night-blessing'
stage = work / 'staged'

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest().upper()

def read(path):
    return json.loads(path.read_text(encoding='utf8'))

params = read(stage / 'validation.json')
assets = read(stage / 'assets-validation.json')
texts = read(stage / 'texts/validation.json')
assert params['originalRowsUnchanged'] and params['abilityCount'] == 23
assert assets['originalAnimationEventsPreserved'] and assets['originalGraceCommandsPreserved']
assert (root / 'src/hex_pool.rs').read_bytes() == (stage / 'hex_pool.rs').read_bytes()
for name, key in [('source', 'sourceSha256'), ('englishSource', 'englishSourceSha256'), ('statusSource', 'statusSourceSha256')]:
    assert sha(root / texts[name]) == texts[key].upper(), f'Text source changed: {name}'
process = subprocess.run(['powershell', '-NoProfile', '-Command', 'Get-Process nightreign -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id'], capture_output=True, text=True)
assert not process.stdout.strip(), 'Close the game before installing'
updates = {}

def add(rel, source, before=None, after=None):
    assert (distribution_file(dist,rel)).resolve().is_relative_to(dist.resolve())
    if before:
        assert sha(distribution_file(dist,rel)) == before.upper(), f'Input changed: {rel}'
    if after:
        assert sha(source) == after.upper(), f'Staged output changed: {rel}'
    updates[rel] = source

add('mod/regulation.bin', stage / 'regulation.bin', params['inputSha256'], params['outputSha256'])
add('mod/chr/c0000.anibnd.dcx', stage / 'c0000.anibnd.dcx', assets['animationInputSha256'], assets['animationOutputSha256'])
talk = 'mod/script/talk/m00_00_00_00.talkesdbnd.dcx'
assert sha(Path(assets['graceInput'])) == assets['graceInputSha256']
assert (distribution_file(dist,talk)).exists() == assets['graceInputInstalled']
add(talk, stage / 'm00_00_00_00.talkesdbnd.dcx', assets['graceInputSha256'] if assets['graceInputInstalled'] else None, assets['graceOutputSha256'])
for entry in texts['reports']:
    assert entry['unrelatedEntriesAndMembersUnchanged']
    rel = Path(entry['language']) / entry['file']
    add((Path('mod/msg') / rel).as_posix(), stage / 'texts' / rel, entry['inputSha256'], entry['outputSha256'])
add('natives/NRHex.dll', root / 'target/release/NRHex.dll')
baseline = {p.relative_to(dist).as_posix(): sha(p) for p in dist.rglob('*') if p.is_file() and p.suffix != '.log'}
version = tomllib.loads((root / 'Cargo.toml').read_text())['package']['version']
backup = work / f'before-install-{version}'
assert not backup.exists(), 'Inspect existing installation backup before repeating'
backup.mkdir(parents=True)
existed = {rel for rel in updates if (distribution_file(dist,rel)).exists()}
for rel in existed:
    target = backup / rel
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(distribution_file(dist,rel), target)
try:
    for rel, source in updates.items():
        target = distribution_file(dist,rel)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
        assert sha(target) == sha(source)
    assert all(sha(distribution_file(dist,rel)) == value for rel, value in baseline.items() if rel not in updates)
except BaseException:
    for rel in updates:
        if rel in existed:
            shutil.copy2(backup / rel, distribution_file(dist,rel))
        else:
            (distribution_file(dist,rel)).unlink(missing_ok=True)
    raise
for source, name in [(stage / 'validation.json', 'night-blessing-parameter-validation.json'), (stage / 'assets-validation.json', 'night-blessing-assets-validation.json'), (stage / 'texts/validation.json', 'night-blessing-text-validation.json')]:
    shutil.copy2(source, distribution_file(dist,name))
summary = read(dist/'reports'/'validation.json')
summary.update(codeVersion=version, hexes=read(stage / 'catalog.json'), orders=10626, currentIncrementalValidation='night-blessing-validation.json', currentStatusTextValidation='night-blessing-text-validation.json', gameTested=False, gameplayVerified=False)
(dist/'reports'/'validation.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding='utf8')
report = dict(version=version, changedAssets={rel: sha(distribution_file(dist,rel)) for rel in updates}, otherAssetsAndSettingsPreserved=True, gameStarted=False, gameplayVerified=False, multiplayerVerified=False)
(dist/'reports'/'night-blessing-validation.json').write_text(json.dumps(report, indent=2), encoding='utf8')
print(f'PASS: {version} installed; {len(updates)} assets verified; unrelated assets and settings preserved.')
