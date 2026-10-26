"""Build a separate ME3 overlay; MMV files remain external, read-only inputs."""
from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import tomllib

# The pinned MMV profile labels these as Optional, but omits optional=true.
# Correct only the generated overlay profile when the corresponding DLL is absent.
OPTIONAL_MMV_NATIVES = {'NightreignFPSFOV.dll', 'UnlockTheFps.dll', 'UltrawideFix.dll'}
COMMUNITY_DIRECTORY = 'MayhemAugments'
COMMUNITY_PROFILE = 'Mayhem Augments.me3'


def installed_community(mmv: Path) -> dict | None:
    path = mmv / COMMUNITY_PROFILE
    if not path.exists():
        return None
    parsed = tomllib.loads(path.read_text(encoding='utf-8-sig'))
    packages = [p for p in parsed.get('packages', []) if p.get('id') == 'nrhex-mmv']
    natives = [n for n in parsed.get('natives', []) if n.get('id') == 'nrhex-mmv']
    if (len(packages) != 1 or packages[0].get('path') != COMMUNITY_DIRECTORY + '/mod'
            or len(natives) != 1 or natives[0].get('path') not in
            (COMMUNITY_DIRECTORY + '/natives/NRHex.dll', COMMUNITY_DIRECTORY + '/mod/natives/NRHex.dll')):
        raise ValueError('Unrecognized file occupies the MA community profile name')
    return parsed


def source_profiles(mmv: Path) -> list[Path]:
    installed = installed_community(mmv)
    return sorted(p for p in mmv.glob('*.me3') if not (installed and p.name == COMMUNITY_PROFILE))


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def value(v):
    if isinstance(v, bool):
        return str(v).lower()
    if isinstance(v, str):
        return json.dumps(v, ensure_ascii=False)
    if isinstance(v, (int, float)):
        return repr(v)
    if isinstance(v, list):
        return '[' + ', '.join(value(x) for x in v) + ']'
    if isinstance(v, dict):
        return '{ ' + ', '.join(f'{json.dumps(k)} = {value(x)}' for k, x in v.items()) + ' }'
    raise ValueError(f'Unsupported profile field: {v!r}')


def profile(source: Path, overlay: Path, preserve_fixes=True) -> tuple[str, dict]:
    original = tomllib.loads(source.read_text(encoding='utf-8-sig'))
    merged = copy.deepcopy(original)
    if original.get('profileVersion') != 'v1' or len(original.get('packages', [])) != 1:
        raise ValueError('MMV profile structure changed; review package references')
    # Keep every original option, including missing optional DLL references.
    for kind in ('packages', 'natives'):
        for entry in merged.get(kind, []):
            key = 'path' if 'path' in entry else 'source'
            path = Path(entry[key])
            entry[key] = str(path if path.is_absolute() else (source.parent / path).resolve())
            if 'config' in entry:
                path = Path(entry['config'])
                entry['config'] = str(path if path.is_absolute() else (source.parent / path).resolve())
            if kind == 'natives' and not Path(entry[key]).is_file():
                if Path(entry[key]).name in OPTIONAL_MMV_NATIVES:
                    entry['optional'] = True
                elif not entry.get('optional', False):
                    raise ValueError(f'Required MMV DLL is missing: {entry[key]}')
    base = merged['packages'][0]
    base_id = original['packages'][0].get('id', original['packages'][0].get('path', original['packages'][0].get('source')))
    base['id'] = base_id
    if any(n.get('id') == 'nrhex-mmv' for n in merged.get('natives', [])):
        raise ValueError('NRHex native ID already present in MMV')
    merged['packages'].append(dict(id='nrhex-mmv', path=str((overlay / 'mod').resolve()),
                                  load_after=[dict(id=base_id, optional=False)]))
    merged.setdefault('natives', []).append(dict(id='nrhex-mmv', path=str((overlay / 'natives/NRHex.dll').resolve())))
    # Independent MMV fixes are opt-in. Preserve an explicitly enabled package;
    # deleting this entry disables it permanently, without rebuilding either mod.
    current_profile = overlay / 'NRHex-MMV.me3'
    if preserve_fixes and current_profile.exists():
        current = tomllib.loads(current_profile.read_text(encoding='utf-8-sig'))
        fixes = [p for p in current.get('packages', []) if p.get('id') == 'mmv-fixes']
        if len(fixes) > 1:
            raise ValueError('Duplicate independent MMV fix package')
        if fixes:
            from mmv_fixes import with_package
            path = Path(fixes[0]['path'])
            merged = with_package(merged, path if path.is_absolute() else overlay / path)
    # Arrays and inline tables preserve unknown options instead of discarding them.
    text = '# Generated NRHex-MMV overlay; original MMV profile is unchanged.\n'
    text += '\n'.join(f'{json.dumps(k)} = {value(v)}' for k, v in merged.items()) + '\n'
    if tomllib.loads(text) != merged:
        raise ValueError('Profile serialization did not round-trip')
    return text, merged


def readable_profile(parsed: dict) -> str:
    lines = []
    for key, item in parsed.items():
        if key in ('supports', 'packages', 'natives'):
            continue
        lines.append(f'{key} = {value(item)}')
    for key in ('supports', 'packages', 'natives'):
        for entry in parsed.get(key, []):
            lines += ['', f'[[{key}]]']
            lines += [f'{k} = {value(v)}' for k, v in entry.items()]
    text = '\n'.join(lines) + '\n'
    if tomllib.loads(text) != parsed:
        raise ValueError('Portable profile serialization did not round-trip')
    return text


def community_profile(source: Path, fixes: Path | None = None) -> tuple[str, dict, dict]:
    # Reuse the required-DLL and source-profile checks, but never inherit a local
    # overlay profile or its absolute installation paths into a distributable.
    _, parsed = profile(source, source.parent / COMMUNITY_DIRECTORY, preserve_fixes=False)
    parsed['packages'] = [p for p in parsed['packages'] if p.get('id') != 'mmv-fixes']
    parsed['natives'] = [p for p in parsed['natives'] if p.get('id') != 'mmv-fixes']
    for kind in ('packages', 'natives'):
        for entry in parsed[kind]:
            for key in ('path', 'source', 'config'):
                if key not in entry:
                    continue
                resolved = Path(entry[key]).resolve()
                if not resolved.is_relative_to(source.parent.resolve()):
                    raise ValueError(f'Non-portable MMV reference outside its directory: {entry[key]}')
                entry[key] = resolved.relative_to(source.parent.resolve()).as_posix()
    # MMV community follows MMV's online/Arxan settings, unlike the standalone
    # public release. The source entry remains untouched.
    payload = {}
    if fixes is not None:
        from mmv_fixes import checked_package
        checked_package(fixes)
        manifest = json.loads((fixes.parent / 'manifest.json').read_text(encoding='utf8'))
        for name in manifest['files']:
            payload['MMV-Fixes/' + name] = (fixes.parent / name).read_bytes()
        parsed['packages'].append(dict(id='mmv-fixes', path='MMV-Fixes/mod',
            load_after=[dict(id=p['id'], optional=False) for p in parsed['packages']]))
        if 'natives/MMVFixes.dll' in manifest['files']:
            parsed['natives'].append(dict(id='mmv-fixes', path='MMV-Fixes/natives/MMVFixes.dll', load_early=True))
    # Only our namespace and optional independent fix namespace may be copied
    # into MMV. Never overwrite an upstream file, even on a future MMV release.
    installed = installed_community(source.parent)
    owned = {COMMUNITY_DIRECTORY, COMMUNITY_PROFILE} if installed else set()
    if installed and any(p.get('id') == 'mmv-fixes' and p.get('path') == 'MMV-Fixes/mod' for p in installed.get('packages', [])):
        owned.add('MMV-Fixes')
    for name in (COMMUNITY_DIRECTORY, COMMUNITY_PROFILE, *(['MMV-Fixes'] if fixes else [])):
        if (source.parent / name).exists() and name not in owned:
            raise ValueError(f'Portable package name already exists in MMV: {name}')
    return readable_profile(parsed), parsed, payload


def enabled_fixes(root: Path) -> Path | None:
    """Carry the explicitly enabled independent fixes from the tested profile."""
    path = root / 'dist/mmv/NRHex-MMV.me3'
    if not path.exists():
        return None
    entries = [p for p in tomllib.loads(path.read_text(encoding='utf-8-sig')).get('packages', [])
               if p.get('id') == 'mmv-fixes']
    if len(entries) > 1:
        raise ValueError('Duplicate independent MMV fix package')
    if not entries:
        return None
    package = Path(entries[0]['path'])
    return package if package.is_absolute() else (path.parent / package).resolve()


def output_path(root: Path, mmv: Path, variant: str, community_destination: Path | None = None) -> Path:
    if variant == 'dist':
        output = root / 'dist/mmv'
    elif variant == 'community':
        if community_destination is None or not community_destination.is_absolute():
            raise ValueError('MMV community requires an explicit absolute external --community-destination')
        output = community_destination
        resolved = output.resolve()
        for protected in (root.resolve(), mmv.resolve()):
            if resolved.is_relative_to(protected) or protected.is_relative_to(resolved):
                raise ValueError('MMV community destination must be outside the repository and original MMV tree')
    else:
        raise ValueError('Unknown MMV variant')
    if output.resolve() != output.absolute():
        raise ValueError('MMV output must not be a linked directory')
    return output


def build(root: Path, mmv: Path, files: dict, me3: Path, variant='dist', install=True,
          community_destination: Path | None = None) -> dict:
    import pefile
    if variant not in ('dist', 'community'):
        raise ValueError('Unknown MMV variant')
    hotkeys = variant == 'dist'
    staging = root / 'work/compat/mmv/staged'
    output = output_path(root, mmv, variant, community_destination)
    sources = source_profiles(mmv)
    if len(sources) != 1:
        raise ValueError('Select an MMV launch profile explicitly before packaging')
    dll = root / f'work/compat/mmv/targets/{variant}/release/NRHex.dll'
    image = pefile.PE(str(dll), fast_load=True)
    image.parse_data_directories(directories=[1])
    imports = {i.name for entry in image.DIRECTORY_ENTRY_IMPORT for i in entry.imports}
    if (b'GetAsyncKeyState' in imports) != hotkeys or b'CageDemand prepared:' not in dll.read_bytes() or b'CageProbe prepared:' in dll.read_bytes() or b'MMV compatibility: verified Legendary Stones' not in dll.read_bytes():
        raise ValueError('MMV DLL feature check failed')
    image.close()
    prefix = 'mod' if hotkeys else COMMUNITY_DIRECTORY + '/mod'
    native_prefix = 'natives' if hotkeys else COMMUNITY_DIRECTORY + '/natives'
    profile_name = 'NRHex-MMV.me3' if hotkeys else COMMUNITY_PROFILE
    payload = {}
    for name, digest in files.items():
        path = staging / 'mod' / name
        if not path.resolve().is_relative_to((staging / 'mod').resolve()) or sha(path).lower() != digest.lower():
            raise ValueError(f'Staged file missing or changed: {name}')
        payload[prefix + '/' + name] = path.read_bytes()
    payload[native_prefix + '/NRHex.dll'] = dll.read_bytes()
    ini = output / native_prefix / 'NRHex.ini'
    fallback_ini = root / ('dist/natives/NRHex.ini' if hotkeys else 'dist/mmv/natives/NRHex.ini')
    if not fallback_ini.exists():
        fallback_ini = root / 'dist/natives/NRHex.ini'
    config = (ini if ini.exists() else fallback_ini).read_text(encoding='utf-8-sig')
    retired = {'VirtualKey', 'TestVirtualKey', 'NewHexVirtualKey', 'ChantVirtualKey', 'GazeVirtualKey', 'ContractVirtualKey',
               'AngelChantVirtualKey', 'StormTestVirtualKey', 'RequireControl', 'CooldownMs', 'SwordNearDistance'}
    if not hotkeys:
        config = '\n'.join(line for line in config.splitlines() if line.split('=', 1)[0].strip() not in retired and not line.startswith('; Ctrl+'))
        config = config.replace('[TestSpawn]', '[Settings]')
    payload[native_prefix + '/NRHex.ini'] = (config + '\n').encode('utf8')
    if hotkeys:
        text, parsed = profile(sources[0], output)
    else:
        text, parsed, fixes_payload = community_profile(sources[0], enabled_fixes(root))
        payload.update(fixes_payload)
    payload[profile_name] = text.encode('utf8')
    # Validate the concrete profile before publishing any new files.
    preview = root / f'work/compat/mmv/profile-preview-{variant}.me3'
    preview.write_text(text, encoding='utf8')
    with (preview.parent / f'profile-validation-{variant}.log').open('w', encoding='utf8') as log:
        subprocess.run([str(me3), 'profile', 'show', '--file', str(preview)], check=True, stdout=log, stderr=subprocess.STDOUT)
    state = root / ('work/compat/mmv/package-state.json' if hotkeys else 'work/compat/mmv/package-state-community.json')
    previous = json.loads(state.read_text(encoding='utf8')) if state.exists() else {}
    stale = [name for name in previous.keys() - payload.keys() if (output / name).exists()]
    if stale:
        raise ValueError('Retired generated files require cleanup before rebuilding: ' + ', '.join(stale))
    # Do not silently replace manual profile edits or unrelated files on rebuild.
    for name in payload:
        p = output / name
        if p.resolve() != p.absolute():
            raise ValueError(f'MMV output contains a linked path: {p}')
        if p.exists() and name != native_prefix + '/NRHex.ini' and sha(p) != previous.get(name):
            raise ValueError(f'Output was edited outside the MMV build: {p}')
    # Fresh build-only directories cannot retain obsolete files from an older
    # layout (notably mod/natives or the previous community root mod/).
    destination = output if install else Path(tempfile.mkdtemp(prefix=f'ready-{variant}-', dir=root / 'work/compat/mmv'))
    if install:
        # Detect a loaded DLL before replacing any resources. A build-only result
        # can be prepared while the user keeps the current expedition running.
        current_dll = output / native_prefix / 'NRHex.dll'
        if current_dll.exists():
            with current_dll.open('r+b'):
                pass
    for name, data in payload.items():
        p = destination / name
        p.parent.mkdir(parents=True, exist_ok=True)
        temporary = p.with_name(p.name + '.nrhex-new')
        temporary.write_bytes(data)
        temporary.replace(p)
    hashes = {name: sha(destination / name) for name in payload}
    if install:
        state.write_text(json.dumps(hashes, indent=2), encoding='utf8')
    return dict(output=str(output), files=hashes, profile=str(output / profile_name),
                variant=variant, installed=install, stagedPackage=str(destination),
                portableOverlay=not hotkeys, independentFixesIncluded=any(n.startswith('MMV-Fixes/') for n in payload),
                mmvProfile=str(sources[0]), mmvProfileSha256=sha(sources[0]),
                testHotkeys=hotkeys, profileValidated=True, gameplayVerified=False, nativeCoexistenceVerified=False,
                packageOrder=[p['id'] for p in parsed['packages']])
