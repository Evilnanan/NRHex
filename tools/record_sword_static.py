"""Verify the compiled user table and the scope of the DLL-only update."""
import csv
import hashlib
import io
import json
import os
import re
import struct
import subprocess
import sys
import tomllib
from local_paths import ROOT

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def assets():
    paths = [p for folder in ['mod', 'testmods']
             for p in (ROOT / 'dist' / folder).rglob('*')]
    paths += list((ROOT / 'dist').glob('*.me3'))
    return {p.relative_to(ROOT / 'dist').as_posix(): digest(p)
            for p in paths if p.is_file() and p.suffix != '.log'}

def generated_values(path):
    return {int(mid): float(value) for mid, value in
            re.findall(r'\((\d+), ([0-9.eE+\-]+)f32\)', path.read_text(encoding='utf8'))}

def f32(value):
    return struct.pack('<f', float(value))

work = ROOT / '.work/sword-static'
work.mkdir(parents=True, exist_ok=True)
baseline = work / 'package-baseline.json'
csv_path = ROOT / 'data/balance/sword.csv'
if '--before' in sys.argv:
    baseline.write_text(json.dumps(dict(assets=assets(), csvSha256=digest(csv_path)), indent=2), encoding='utf8')
    print('Captured package assets and user configuration before static sword table update.')
    sys.exit(0)

captured = json.loads(baseline.read_text())
before = captured['assets']
after = assets()
changed = sorted(p for p in before.keys() | after.keys() if before.get(p) != after.get(p))
assert set(changed) <= {'mod/natives/NRHex.dll', 'mod/natives/sword.csv'}, changed
assert 'mod/natives/sword.csv' not in after
assert digest(csv_path) == captured['csvSha256'], 'User CSV changed during the update'
with csv_path.open(encoding='utf-8-sig', newline='') as stream:
    reader = csv.DictReader(stream)
    header = reader.fieldnames
    percentage_col, = [c for c in ['加成', '吸血百分比', '效果百分比', '增伤百分比'] if c in header]
    rows = list(reader)
expected = {int(row['MagicID']): float(row[percentage_col] or 0) for row in rows}
assert len(expected) == len(rows) == 160
snapshot = max((ROOT / 'target/release/build').glob('nrhex-*/out/sword_bonuses.rs'), key=lambda p: p.stat().st_mtime_ns)
actual = generated_values(snapshot)
assert actual.keys() == expected.keys()
assert all(f32(actual[mid]) == f32(value) for mid, value in expected.items())

# Exercise the actual compiled build script with isolated inputs, leaving the
# user CSV untouched. This verifies validation happens before DLL compilation.
builder = max((ROOT / 'target/release/build').glob('nrhex-*/build-script-build.exe'), key=lambda p: p.stat().st_mtime_ns)
cases = []
for name, value, success in [('fresh_fractional_value', '17.25', True),
                             ('invalid_percentage', '101', False),
                             ('missing_csv', None, False)]:
    fixture = work / name
    output = fixture / 'out'
    output.mkdir(parents=True, exist_ok=True)
    if value is not None:
        fixture_csv = fixture / 'data/balance/sword.csv'
        fixture_csv.parent.mkdir(parents=True, exist_ok=True)
        buffer = io.StringIO(newline='')
        writer = csv.DictWriter(buffer, fieldnames=header)
        writer.writeheader()
        writer.writerows([dict(row, **{percentage_col: value}) if i == 0 else row
                          for i, row in enumerate(rows)])
        fixture_csv.write_text(buffer.getvalue(), encoding='utf8', newline='')
    env = dict(os.environ, CARGO_MANIFEST_DIR=str(fixture), OUT_DIR=str(output))
    result = subprocess.run([str(builder)], env=env, cwd=ROOT, capture_output=True, text=True)
    assert (result.returncode == 0) == success, (name, result.stderr)
    if success:
        assert generated_values(output / 'sword_bonuses.rs')[int(rows[0]['MagicID'])] == 17.25
    cases.append(dict(case=name, expectedSuccess=success, passed=True))

assert 'dir.join("sword.csv")' not in (ROOT / 'src/lib.rs').read_text(encoding='utf8')
dll_hash = digest(ROOT / 'target/release/NRHex.dll')
assert dll_hash == after['mod/natives/NRHex.dll']
version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
tests = int(sys.argv[sys.argv.index('--rust-tests') + 1])
report = dict(version=version, spellCount=len(expected), csvPath='data/balance/sword.csv', csvSha256=digest(csv_path),
              compiledSnapshotSha256=digest(snapshot), allUserValuesPreserved=True,
              runtimeCsvRequired=False, csvEditsRequireRecompile=True,
              isolatedBuildScriptCases=cases, rustTestsPassed=tests,
              changedAssets=changed, unrelatedAssetsAndUserConfigsUnchanged=True,
              gameplayVerified=False, multiplayerVerified=False)
(ROOT / 'dist/reports/sword-static-validation.json').write_text(json.dumps(report, indent=2), encoding='utf8')
build_path = ROOT / 'dist/reports/build-validation.json'
build = json.loads(build_path.read_text())
build.update(version=version, scope='Compile data/balance/sword.csv into DLL; CSV excluded from dist',
             changedAssets=changed, dllSha256=dll_hash, installedDllSha256=dll_hash,
             dllInstalled=True, rustTestsPassed=tests, regulationSha256=after['mod/regulation.bin'],
             unrelatedAssetsAndUserConfigsUnchanged=True, gameplayVerified=False, multiplayerVerified=False)
build['functionChecks'] = list(dict.fromkeys(build.get('functionChecks', []) + ['sword-static-validation.json']))
build_path.write_text(json.dumps(build, indent=2), encoding='utf8')
runtime_path = ROOT / 'dist/reports/runtime-validation.json'
runtime = json.loads(runtime_path.read_text())
runtime.update(version=version, active=None, pid=None, allFourNativeDllsLoaded=False,
               dllSha256=dll_hash, dllInstalled=True, gameplayVerified=False, multiplayerVerified=False,
               notes='Sword bonuses compiled into DLL; current game loading and gameplay remain unverified.')
runtime_path.write_text(json.dumps(runtime, indent=2), encoding='utf8')
print('PASS: all 160 bonuses embedded from data/balance/sword.csv; CSV excluded from dist; user values and unrelated assets preserved.')
