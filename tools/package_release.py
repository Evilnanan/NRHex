"""Produce a clean Mayhem Augments standalone folder and verified ZIP."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import zipfile

import pefile

ROOT = Path(__file__).resolve().parents[1]
NAME = 'Mayhem Augments'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-directory', type=Path, default=ROOT / 'releases')
    args = parser.parse_args()
    output = args.output_directory.resolve()
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text(encoding='utf8'))['package']['version']
    folder = output / f'{NAME}-{version}'
    archive = folder.with_name(folder.name + '.zip')
    if folder.exists() or archive.exists():
        raise ValueError('Release already exists; choose another output directory or update the version')
    dll = (ROOT / '.work/no-hotkeys-target/release/NRHex.dll').read_bytes()
    with pefile.PE(data=dll, fast_load=True) as image:
        image.parse_data_directories(directories=[1])
        imports = {i.name for entry in image.DIRECTORY_ENTRY_IMPORT for i in entry.imports}
    assert b'GetAsyncKeyState' not in imports, 'Test hotkeys must not ship'
    assert b'CageDemand prepared:' in dll, 'Required cage-demand runtime missing'
    assert b'CageProbe prepared:' not in dll, 'Diagnostic build must not ship'
    assert b'MMV compatibility: verified Legendary Stones' not in dll, 'MMV runtime must not ship'
    # Never copy dist itself, its testmods, nested MMV, profiles or logs.
    payload = {}
    inputs = {}
    for p in sorted((ROOT / 'dist/mod').rglob('*')):
        if not p.is_file() or 'natives' in p.relative_to(ROOT / 'dist/mod').parts:
            continue
        if p.suffix == '.log':
            continue
        if p.suffix not in ('.bin', '.dcx', '.hks') or p.resolve() != p.absolute():
            raise ValueError(f'Unexpected release asset: {p}')
        data = p.read_bytes()
        payload[p.relative_to(ROOT / 'dist').as_posix()] = data
        inputs[p] = sha(data)
    assert 'mod/regulation.bin' in payload and 'mod/event/common.emevd.dcx' in payload
    payload['natives/NRHex.dll'] = dll
    payload['natives/NRHex.ini'] = b'; Mayhem Augments release. Test hotkeys are disabled at compile time.\n[Settings]\n'
    profile = f'{NAME}.me3'
    payload[profile] = b'''profileVersion = "v1"

[[supports]]
game = "nightreign"

[[natives]]
path = "natives/NRHex.dll"

[[packages]]
id = "nrhex"
source = "mod"
'''
    me3 = Path(os.environ.get('NRHEX_ME3', str(Path(ROOT.anchor) / 'Program Files/me3/bin/me3.exe')))
    reports = ROOT / '.work/release'
    reports.mkdir(parents=True, exist_ok=True)
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.mayhem-stage-', dir=output) as temporary:
        stage = Path(temporary) / folder.name
        for name, data in payload.items():
            p = stage / name
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_bytes(data)
            assert p.read_bytes() == data, name
        with (reports / 'profile-validation.log').open('w', encoding='utf8') as log:
            subprocess.run([str(me3), 'profile', 'show', '--file', str(stage / profile)],
                           check=True, stdout=log, stderr=subprocess.STDOUT)
        staged_zip = Path(temporary) / archive.name
        with zipfile.ZipFile(staged_zip, 'w', zipfile.ZIP_DEFLATED, compresslevel=6) as z:
            for name in payload:
                z.write(stage / name, folder.name + '/' + name)
        with zipfile.ZipFile(staged_zip) as z:
            assert z.testzip() is None
            assert set(z.namelist()) == {folder.name + '/' + name for name in payload}
            for name, data in payload.items():
                assert sha(z.read(folder.name + '/' + name)) == sha(data), name
        assert all(sha(p.read_bytes()) == h for p, h in inputs.items()), 'Resources changed while packaging'
        assert (ROOT / '.work/no-hotkeys-target/release/NRHex.dll').read_bytes() == dll
        stage.rename(folder)
        staged_zip.rename(archive)
    report = dict(name=NAME, internalName='NRHex', version=version, folder=str(folder), archive=str(archive),
                  archiveSha256=sha(archive.read_bytes()), files={name: sha(data) for name, data in payload.items()},
                  testHotkeys=False, mmv=False, bundledOtherMods=False, profileValidated=True,
                  archiveReadbackVerified=True, gameplayVerified=False)
    (reports / f'{version}.json').write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf8')
    print(f'PASS: {NAME} {version}; {len(payload)} verified files; {archive}')


if __name__ == '__main__':
    main()
