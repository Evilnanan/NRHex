"""Build/enable a removable MMV bug-fix package, independent of the NRHex overlay."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
from mmv_package import value, OPTIONAL_MMV_NATIVES, source_profiles

ROOT = Path(__file__).resolve().parents[1]
PACKAGE_ID = 'mmv-fixes'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def checked_package(mod):
    manifest = json.loads((mod.parent / 'manifest.json').read_text(encoding='utf8'))
    for path, expected in manifest['sourceFiles'].items():
        if not Path(path).is_file() or sha(Path(path)) != expected:
            raise ValueError('MMV source changed. Review or remove MMV-Fixes: ' + path)
    for relative, expected in manifest['files'].items():
        path = mod.parent / relative
        if not path.resolve().is_relative_to(mod.parent.resolve()) or sha(path) != expected:
            raise ValueError('MMV-Fixes output changed: ' + relative)


def with_package(profile, mod):
    result = copy.deepcopy(profile)
    result['packages'] = [p for p in result.get('packages', []) if p.get('id') != PACKAGE_ID]
    if 'natives' in result:
        result['natives'] = [n for n in result['natives'] if n.get('id') != PACKAGE_ID]
    if mod is not None:
        checked_package(mod)
        ids = [p.get('id', p.get('path', p.get('source'))) for p in result['packages']]
        if not ids or any(not i for i in ids):
            raise ValueError('Existing package identifiers are required')
        result['packages'].append(dict(id=PACKAGE_ID, path=str(mod.resolve()),
                                       load_after=[dict(id=i, optional=False) for i in ids]))
        manifest = json.loads((mod.parent / 'manifest.json').read_text(encoding='utf8'))
        native = 'natives/MMVFixes.dll'
        if native in manifest['files']:
            result.setdefault('natives', []).append(dict(id=PACKAGE_ID, path=str((mod.parent/native).resolve()), load_early=True))
    return result


def write_profile(path, parsed):
    text = '\n'.join(f'{json.dumps(k)} = {value(v)}' for k, v in parsed.items()) + '\n'
    assert tomllib.loads(text) == parsed
    temporary = path.with_name(path.stem + '.preview.me3')
    temporary.write_text(text, encoding='utf8')
    me3 = Path(ROOT.anchor) / 'Program Files/me3/bin/me3.exe'
    try:
        subprocess.run([str(me3), 'profile', 'show', '--file', str(temporary)], check=True,
                       stdout=subprocess.DEVNULL)
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def configure_profile(path, mod):
    original = tomllib.loads(path.read_text(encoding='utf-8-sig'))
    updated = with_package(original, mod)
    # Only this independently removable package and its own DLL may change.
    assert {k:v for k,v in updated.items() if k not in ('packages','natives')} == {k:v for k,v in original.items() if k not in ('packages','natives')}
    assert [n for n in updated.get('natives',[]) if n.get('id') != PACKAGE_ID] == [n for n in original.get('natives',[]) if n.get('id') != PACKAGE_ID]
    backup = ROOT / 'work/compat/mmv-fixes/profile-before.me3'
    backup.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(path, backup)
    write_profile(path, updated)
    # Keep the existing MMV packaging edit guard in sync with this explicit opt-in.
    if path.resolve() == (ROOT/'dist/mmv/NRHex-MMV.me3').resolve():
        state_path = ROOT/'work/compat/mmv/package-state.json'
        state = json.loads(state_path.read_text(encoding='utf8'))
        state['NRHex-MMV.me3'] = sha(path)
        state_path.write_text(json.dumps(state, indent=2), encoding='utf8')


def build(mmv, output, original):
    output = output.resolve();mmv = mmv.resolve()
    if output == mmv or output.is_relative_to(mmv) or mmv.is_relative_to(output):
        raise ValueError('The independent fix package must be outside the MMV directory')
    work = ROOT/'work/compat/mmv-fixes';work.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='build-', dir=work) as directory:
        staged = Path(directory)
        subprocess.run(['dotnet','run','--project',str(ROOT/'tools/MmvFixes'),'--',str(ROOT),str(mmv),
                        str(original.resolve()),str(staged)], check=True, cwd=ROOT)
        manifest = json.loads((staged/'manifest.json').read_text(encoding='utf8'))
        subprocess.run(['cargo','build','--release','--locked','--offline','--manifest-path',str(ROOT/'tools/MmvFixesNative/Cargo.toml'),
                        '--target-dir',str(work/'native-target')], check=True, cwd=ROOT)
        native = 'natives/MMVFixes.dll'
        (staged/'natives').mkdir()
        shutil.copy2(work/'native-target/release/MMVFixes.dll',staged/native)
        manifest['files'][native] = sha(staged/native)
        manifest['fixes'].append('dlc-map-icon-definition')
        manifest['nativeMapFix'] = dict(scope='icon-cache definition lookup fallback only',
                                      maps=[*range(5000,5007),5010,5011],gameplayVerified=False)
        (staged/'manifest.json').write_text(json.dumps(manifest,indent=2),encoding='utf8')
        previous_path = output/'manifest.json'
        previous = json.loads(previous_path.read_text(encoding='utf8')) if previous_path.exists() else {'files':{}}
        for relative, digest in previous['files'].items():
            path=output/relative
            if not path.resolve().is_relative_to(output) or not path.is_file() or sha(path)!=digest:
                raise ValueError('Existing fix file was manually changed: '+str(path))
        for relative in manifest['files']:
            if (output/relative).exists() and relative not in previous['files']:
                raise ValueError('Unmanaged file at fix destination: '+relative)
        output.mkdir(parents=True,exist_ok=True)
        for relative in manifest['files']:
            target=output/relative;target.parent.mkdir(parents=True,exist_ok=True)
            shutil.copy2(staged/relative,target)
        for relative in previous['files'].keys()-manifest['files'].keys():
            (output/relative).unlink()  # Exact previously managed file, hash/path checked above.
        shutil.copy2(staged/'manifest.json',previous_path)
    shutil.copy2(ROOT/'compat/mmv-fixes/README.md',output/'README.md')
    profiles=source_profiles(mmv)
    if len(profiles)!=1:raise ValueError('Expected one source MMV profile')
    base=tomllib.loads(profiles[0].read_text(encoding='utf-8-sig'))
    for kind in ['packages','natives']:
        for entry in base.get(kind,[]):
            for key in ['path','source','config']:
                if key in entry:
                    p=Path(entry[key]);entry[key]=str(p if p.is_absolute() else (mmv/p).resolve())
            if kind=='natives' and Path(entry.get('path','')).name in OPTIONAL_MMV_NATIVES and not Path(entry['path']).exists():
                entry['optional']=True
    for entry in base['packages']:
        entry.setdefault('id',entry.get('path',entry.get('source')))
    write_profile(output/'MMV-Fixes.me3',with_package(base,output/'mod'))
    print('Built independent package:',output)


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--mmv',type=Path)
    parser.add_argument('--output',type=Path)
    parser.add_argument('--original',type=Path,default=ROOT/'work/compat/mmv-fixes/original')
    parser.add_argument('--enable-profile',type=Path)
    parser.add_argument('--disable-profile',type=Path)
    args=parser.parse_args()
    if args.disable_profile:
        configure_profile(args.disable_profile,None)
        print('Removed only the mmv-fixes package/native entries; other settings preserved.')
        return
    if not args.mmv or not args.output:parser.error('--mmv and --output are required to build')
    build(args.mmv,args.output,args.original)
    if args.enable_profile:configure_profile(args.enable_profile,args.output/'mod')


if __name__=='__main__':main()
