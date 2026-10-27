"""Build an independent NRHex-MMV overlay against pinned, read-only MMV inputs.

Successful construction certifies static checks, not gameplay or native coexistence.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib

from mmv_hks import audit as audit_hks
from mmv_hks_stage import stage as stage_hks
from mmv_package import build as package, source_profiles

ROOT = Path(__file__).resolve().parents[1]
CONFIG = ROOT / 'compat/mmv/target.json'
LOCK = ROOT / 'compat/mmv/baseline.lock.json'
REPORTS = ROOT / 'work/compat/mmv'


def digest(path: Path) -> str:
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def write_json(path: Path, data) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + '.tmp')
    temporary.write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
    temporary.replace(path)


def files_under(root: Path):
    """Reject linked resource trees; all audited inputs must belong to this pack."""
    for path in root.rglob('*'):
        if path.is_symlink() or path.is_junction():
            raise ValueError(f'Linked MMV resource is not supported: {path}')
        relative = path.relative_to(root)
        mmv_runtime_log = root.name == 'dll' and relative.as_posix() == 'target/release/0987.txt'
        if path.is_file() and path.suffix.lower() != '.log' and 'logs' not in relative.parts and not mmv_runtime_log:
            yield path


def inventory(mmv: Path) -> dict:
    for name in ('mod', 'dll'):
        folder = mmv / name
        if not folder.is_dir() or folder.is_symlink() or folder.is_junction():
            raise ValueError(f'Missing or linked MMV input directory: {folder}')
    profiles = source_profiles(mmv)
    if not profiles:
        raise ValueError('MMV profile is missing')
    paths = [*files_under(mmv / 'mod'), *files_under(mmv / 'dll'), *profiles]
    files = {}
    for path in sorted(paths, key=lambda p: p.relative_to(mmv).as_posix()):
        if path.is_symlink():
            raise ValueError(f'Linked MMV profile is not supported: {path}')
        files[path.relative_to(mmv).as_posix()] = dict(bytes=path.stat().st_size, sha256=digest(path))
    fingerprint = hashlib.sha256(json.dumps(files, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    return dict(fileCount=len(files), totalBytes=sum(f['bytes'] for f in files.values()), sha256=fingerprint, files=files)


def lock_data(current: dict) -> dict:
    # Detailed inventory stays in work/; pin the whole pack and diagnostically useful files in Git.
    critical = {name: entry for name, entry in current['files'].items()
                if name.endswith(('.me3', '.dll', 'regulation.bin', 'c0000.hks'))}
    return dict(schemaVersion=1, scope='mod/**, dll/** and root *.me3; excluding *.log, logs/** and dll/target/release/0987.txt',
                fileCount=current['fileCount'], totalBytes=current['totalBytes'], sha256=current['sha256'],
                criticalFiles=critical, compatibilityVerified=False)


def validate_lock(lock: dict, current: dict) -> None:
    if lock.get('schemaVersion') != 1:
        raise ValueError('Unsupported MMV baseline lock schema')
    if any(lock.get(k) != current[k] for k in ('sha256', 'fileCount', 'totalBytes')):
        changed = [name for name, expected in lock.get('criticalFiles', {}).items()
                   if current['files'].get(name) != expected]
        detail = ', '.join(changed) if changed else 'other MMV resources or configuration'
        raise ValueError(f'MMV baseline changed: {detail}. Review the new inputs and update the lock explicitly.')


def original_assets() -> dict:
    manifest = json.loads((ROOT / 'assets/original/manifest.json').read_text(encoding='utf-8-sig'))
    failures = []
    # Verify the originals used by the current shared package, not unrelated research fixtures.
    for name, expected in manifest['files'].items():
        if not (ROOT / 'dist/mod' / name).is_file():
            continue
        path = ROOT / 'assets/original' / name
        if not path.is_file() or digest(path) != expected.lower():
            failures.append(name)
    if failures:
        raise ValueError('Original input mismatch: ' + ', '.join(failures))
    return manifest


def shared_inputs() -> dict:
    paths = [p for p in (ROOT / 'dist/mod').rglob('*') if p.is_file()
             and 'natives' not in p.relative_to(ROOT / 'dist/mod').parts and p.suffix != '.log']
    if not paths:
        raise ValueError('Build standalone resources first; dist/mod is empty')
    return {p.relative_to(ROOT / 'dist/mod').as_posix(): digest(p) for p in sorted(paths)}


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--mmv', required=True, type=Path)
    parser.add_argument('--original-sfx', type=Path, help='Explicit original common DLC SFX binder for detailed inspection')
    parser.add_argument('--variant', choices=['dist', 'community', 'both'], default='dist')
    parser.add_argument('--community-destination', type=Path, help='Explicit external directory for the community variant')
    parser.add_argument('--build-only', action='store_true', help='Prepare packages without replacing installed outputs')
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--record-baseline', action='store_true')
    mode.add_argument('--audit-only', action='store_true')
    mode.add_argument('--stage-only', action='store_true')
    args = parser.parse_args(argv)
    config = json.loads(CONFIG.read_text(encoding='utf8'))
    report = dict(schemaVersion=1, target=config['name'], status='preflight', releaseReady=False,
                  gameplayVerified=False, output=str(ROOT / config['output']))
    try:
        mmv = args.mmv.resolve(strict=True)
        if args.variant in ('community', 'both'):
            from mmv_package import output_path
            output_path(ROOT, mmv, 'community', args.community_destination)
        # Reports themselves must never be written into the input pack.
        if mmv == ROOT or mmv in ROOT.parents or ROOT in mmv.parents:
            raise ValueError('MMV input must be separate from the NRHex repository')
        print('Fingerprinting MMV resources, DLLs and profiles...', flush=True)
        current = inventory(mmv)
        write_json(REPORTS / 'inventory.json', current)
        report['mmvFingerprint'] = current['sha256']
        if args.record_baseline:
            if LOCK.exists():
                raise ValueError('Baseline lock already exists; review changes before explicitly removing and re-recording it')
            write_json(LOCK, lock_data(current))
            report['status'] = 'baseline-recorded'
            print(f'Recorded {current["fileCount"]} MMV inputs. This does not certify compatibility.')
            return 0
        if not LOCK.is_file():
            raise ValueError('MMV baseline is not pinned. Use -RecordBaseline once after reviewing the MMV inputs.')
        validate_lock(json.loads(LOCK.read_text(encoding='utf8')), current)
        original_assets()
        before = shared_inputs()
        report['sharedResources'] = before
        report['nrhexVersion'] = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
        try:
            report['gitRevision'] = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
        except subprocess.CalledProcessError:
            report['gitRevision'] = None
        command = ['dotnet', 'run', '--project', str(ROOT / 'tools/HexBuild'), '--', str(ROOT), '--mmv-audit', str(mmv)]
        if args.original_sfx:
            source = args.original_sfx.resolve(strict=True)
            report['extraOriginalSfx'] = dict(path=str(source), sha256=digest(source))
            if report['extraOriginalSfx']['sha256'] != config['originalSfxSha256']:
                raise ValueError('Original DLC SFX baseline hash mismatch')
            command += ['--original-sfx', str(source)]
        print('Inspecting parameters, events, animation, text, maps and effects...', flush=True)
        with (REPORTS / 'audit.log').open('w', encoding='utf8') as log:
            subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
        assets = json.loads((REPORTS / 'audit.json').read_text(encoding='utf8'))
        hks = audit_hks(mmv, REPORTS / 'hks-audit.json')
        # A concurrent update invalidates the whole inspection instead of mixing versions.
        if before != shared_inputs():
            raise ValueError('NRHex resources changed during audit; rerun against a stable shared build')
        validate_lock(json.loads(LOCK.read_text(encoding='utf8')), inventory(mmv))
        report.update(status='adapters-required', assetFindings=dict(Counter(c['reason'] for c in assets['conflicts'])),
                      hksChanges=dict(prefix=hks['prefixFunctions'], interior=hks['interiorFunctions']),
                      requiredAdapters=config['requiredAdapters'])
        print(f'Audit complete: {len(assets["records"])} NRHex resources; '
              f'{hks["prefixFunctions"] + hks["interiorFunctions"]} changed HKS functions inspected.')
        print(f'Report: {REPORTS / "build-report.json"}')
        if args.audit_only:
            print('Inspection succeeded. MMV compatibility and game startup have NOT been verified.')
            return 0
        if not args.original_sfx:
            raise ValueError('Full staging requires --original-sfx with the verified original common DLC SFX binder')
        print('Staging merged parameters and text...', flush=True)
        with (REPORTS / 'stage.log').open('w', encoding='utf8') as log:
            subprocess.run(['dotnet', 'run', '--project', str(ROOT / 'tools/HexBuild'), '--', str(ROOT),
                            '--mmv-stage-safe', str(mmv)], cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
        report['partialPreview'] = json.loads((REPORTS / 'staged/manifest.json').read_text(encoding='utf8'))
        print('Composing and checking MMV HKS source...', flush=True)
        hks_stage = stage_hks(mmv, REPORTS / 'staged')
        with (REPORTS / 'hks-tests.log').open('w', encoding='utf8') as log:
            subprocess.run([sys.executable, '-m', 'unittest', 'discover', '-s', 'tools', '-p', 'test_mmv_hks.py'],
                           cwd=ROOT, env={**os.environ, 'NRHEX_MMV': str(mmv)}, stdout=log, stderr=subprocess.STDOUT, check=True)
        report['hksAdapter'] = dict(functions=len(hks_stage['functions']),
                                   inverseRestoresMmvSource=hks_stage['inverseRestoresMmvSource'],
                                   lua51SyntaxVerified=hks_stage['lua51SyntaxVerified'], differentialTestsPassed=True)
        report['partialPreview']['files']['action/script/c0000.hks'] = hks_stage['outputSha256']
        print('Merging shared events and the MMV gaol opener...', flush=True)
        with (REPORTS / 'events.log').open('w', encoding='utf8') as log:
            subprocess.run(['dotnet', 'run', '--project', str(ROOT / 'tools/HexBuild'), '--', str(ROOT),
                            '--mmv-stage-events', str(mmv)], cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
        events = json.loads((REPORTS / 'staged/events-validation.json').read_text(encoding='utf8'))
        report['eventAdapter'] = events
        report['partialPreview']['files'].update({'event/' + f['file']: f['sha256'] for f in events['files']})
        for name, flag in [('maps', '--mmv-stage-maps'), ('binders', '--mmv-stage-binders'), ('tae', '--mmv-stage-tae')]:
            print(f'Staging and validating MMV {name}...', flush=True)
            command = ['dotnet', 'run', '--project', str(ROOT / 'tools/HexBuild'), '--', str(ROOT), flag, str(mmv)]
            if name == 'binders':
                command += ['--original-sfx', str(args.original_sfx.resolve(strict=True))]
            with (REPORTS / f'{name}.log').open('w', encoding='utf8') as log:
                subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
            result = json.loads((REPORTS / f'staged/{name}-validation.json').read_text(encoding='utf8'))
            report[name + 'Adapter'] = result
            entries = [result] if name == 'tae' else result['files']
            report['partialPreview']['files'].update({('map/mapstudio/' if name == 'maps' else '') + f['file']: f['sha256'] for f in entries})
        print('Adding reviewed MMV field bosses to the demand-created cage pool...', flush=True)
        with (REPORTS / 'cages.log').open('w', encoding='utf8') as log:
            subprocess.run(['dotnet', 'run', '--project', str(ROOT / 'tools/HexBuild'), '--', str(ROOT),
                            '--mmv-stage-cages', str(mmv)], cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
        cages = json.loads((REPORTS / 'staged/cages-validation.json').read_text(encoding='utf8'))
        report['cageAdapter'] = cages
        report['partialPreview']['files'].update(cages['files'])
        print('Applying personal gaol escalation and native boss buff stacks...', flush=True)
        from build_cage_escalation import install as install_cage_escalation
        escalation = install_cage_escalation(REPORTS / 'staged/mod', REPORTS / 'cage-escalation')
        report['partialPreview']['files'].update({name: digest(REPORTS / 'staged/mod' / name) for name in escalation})
        report['cageEscalation'] = json.loads((REPORTS / 'cage-escalation/validation.json').read_text(encoding='utf8'))
        print('Rebasing spirit summons on MMV NPCs and resolving boss HUD bindings...', flush=True)
        with (REPORTS / 'spirit-catalog.log').open('w', encoding='utf8') as log:
            subprocess.run(['dotnet', 'run', '--project', str(ROOT / 'tools/HexBuild'), '--', str(ROOT),
                            '--spirit-target-export', str(REPORTS / 'staged'), str(mmv)], cwd=ROOT,
                           stdout=log, stderr=subprocess.STDOUT, check=True)
            subprocess.run([sys.executable, str(ROOT / 'tools/build_mmv_spirit.py'), '--overlay', str(REPORTS / 'staged')],
                           cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
            subprocess.run(['dotnet', 'run', '--project', str(ROOT / 'tools/HexBuild'), '--', str(ROOT),
                            '--spirit-target-apply', str(REPORTS / 'staged'), str(REPORTS / 'staged/spirit-plan.json')],
                           cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
        report['spiritAdapter'] = json.loads((REPORTS / 'staged/spirit-validation.json').read_text(encoding='utf8'))
        report['partialPreview']['files']['regulation.bin'] = digest(REPORTS / 'staged/mod/regulation.bin')

        if set(report['partialPreview']['files']) != set(before):
            raise ValueError('Adapters do not cover exactly the current shared resource set; review added/removed files')
        report['partialPreview'].update(stage='merged-resources', releaseReady=False)
        write_json(REPORTS / 'staged/manifest.json', report['partialPreview'])
        if before != shared_inputs():
            raise ValueError('NRHex resources changed during staging; discard preview and rerun')
        validate_lock(json.loads(LOCK.read_text(encoding='utf8')), inventory(mmv))
        report.update(status='resources-staged', requiredAdapters=[])
        if args.stage_only:
            return 0
        with (REPORTS / 'sword-auto.log').open('w', encoding='utf8') as log:
            subprocess.run(['dotnet', 'run', '--project', str(ROOT / 'tools/HexBuild'), '--', str(ROOT),
                            '--mmv-sword-range-catalog', str(REPORTS / 'staged')], cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
            subprocess.run([sys.executable, str(ROOT / 'tools/build_sword_auto.py'), '--mmv', '--overlay', str(REPORTS / 'staged')],
                           cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
        with (REPORTS / 'echo-catalog.log').open('w', encoding='utf8') as log:
            subprocess.run([sys.executable, str(ROOT / 'tools/build_echo_cast.py'), '--check'],
                           cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
            subprocess.run([sys.executable, str(ROOT / 'tools/build_mmv_echo.py'), '--overlay', str(REPORTS / 'staged')],
                           cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
        report['echoAdapter'] = json.loads((REPORTS / 'staged/echo-validation.json').read_text(encoding='utf8'))
        with (REPORTS / 'archmage-catalog.log').open('w', encoding='utf8') as log:
            subprocess.run(['dotnet', 'run', '--project', str(ROOT / 'tools/HexBuild'), '--', str(ROOT),
                            '--archmage-target-export', str(REPORTS / 'staged')], cwd=ROOT,
                           stdout=log, stderr=subprocess.STDOUT, check=True)
            subprocess.run([sys.executable, str(ROOT / 'tools/build_mmv_archmage.py'), '--overlay', str(REPORTS / 'staged')],
                           cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
        report['archmageAdapter'] = json.loads((REPORTS / 'staged/archmage-validation.json').read_text(encoding='utf8'))
        with (REPORTS / 'arts-catalog.log').open('w', encoding='utf8') as log:
            subprocess.run(['dotnet', 'run', '--project', str(ROOT / 'tools/HexBuild'), '--', str(ROOT),
                            '--arts-target-export', str(REPORTS / 'staged')], cwd=ROOT,
                           stdout=log, stderr=subprocess.STDOUT, check=True)
            subprocess.run([sys.executable, str(ROOT / 'tools/build_mmv_arts.py'), '--overlay', str(REPORTS / 'staged')],
                           cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
        report['artsAdapter'] = json.loads((REPORTS / 'staged/arts-validation.json').read_text(encoding='utf8'))
        runtime_paths = [*ROOT.joinpath('src').rglob('*.rs'), ROOT / 'build.rs', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock',
                         ROOT / 'tools/build_sword_auto.py', ROOT / 'tools/sword_range_estimate.py', REPORTS / 'staged/sword_map.rs',
                         ROOT / 'tools/build_mmv_echo.py', ROOT / 'compat/mmv/echo-spells.json', REPORTS / 'staged/echo_cast_data.rs',
                         ROOT / 'tools/build_mmv_archmage.py', REPORTS / 'staged/archmage_data.rs',
                         ROOT / 'tools/build_mmv_arts.py', ROOT / 'compat/mmv/arts.json', REPORTS / 'staged/arts_data.rs',
                         ROOT / 'tools/build_mmv_spirit.py', ROOT / 'compat/mmv/spirit-npc-ids.json', REPORTS / 'staged/spirit_target_data.rs',
                         ROOT / 'compat/mmv/cage-encounters.json', REPORTS / 'staged/cage_data.rs']
        runtime_inputs = {str(p.relative_to(ROOT)): digest(p) for p in runtime_paths}
        print('Compiling shared runtime with MMV-generated attack data...', flush=True)
        variants = ['dist', 'community'] if args.variant == 'both' else [args.variant]
        for variant in variants:
            features = 'cage-demand,mmv-compat' + (',test-hotkeys' if variant == 'dist' else '')
            with (REPORTS / f'cargo-{variant}.log').open('w', encoding='utf8') as log:
                subprocess.run(['cargo', 'build', '--release', '--lib', '--no-default-features', '--features', features,
                                '--target-dir', str(REPORTS / 'targets' / variant)], cwd=ROOT,
                               env={**os.environ, 'NRHEX_PARRY_DATA': str(REPORTS / 'staged/parry_data.rs'),
                                    'NRHEX_SWORD_DATA': str(REPORTS / 'staged/sword_map.rs'),
                                    'NRHEX_ECHO_DATA': str(REPORTS / 'staged/echo_cast_data.rs'),
                                    'NRHEX_ARCHMAGE_DATA': str(REPORTS / 'staged/archmage_data.rs'),
                                    'NRHEX_ARTS_DATA': str(REPORTS / 'staged/arts_data.rs'),
                                    'NRHEX_SPIRIT_DATA': str(REPORTS / 'staged/spirit_target_data.rs'),
                                    'NRHEX_CAGE_DATA': str(REPORTS / 'staged/cage_data.rs')},
                               stdout=log, stderr=subprocess.STDOUT, check=True)
        if runtime_inputs != {str(p.relative_to(ROOT)): digest(p) for p in runtime_paths} or before != shared_inputs():
            raise ValueError('NRHex inputs changed during compilation; rerun before packaging')
        validate_lock(json.loads(LOCK.read_text(encoding='utf8')), inventory(mmv))
        if digest(args.original_sfx) != report['extraOriginalSfx']['sha256']:
            raise ValueError('Original SFX input changed during build')
        me3 = Path(os.environ.get('NRHEX_ME3', str(Path(ROOT.anchor) / 'Program Files/me3/bin/me3.exe')))
        report['packages'] = [package(ROOT, mmv, report['partialPreview']['files'], me3, v, not args.build_only, args.community_destination) for v in variants]
        report.update(status='built-awaiting-install' if args.build_only else 'built-awaiting-game-test', runtimeInputs=runtime_inputs,
                      parryDataSha256=digest(REPORTS / 'staged/parry_data.rs'),
                      nativeCoexistenceVerified=False, releaseReady=False)
        print(f'Built MMV {", ".join(variants)}. Installed: {not args.build_only}. Game startup and native DLL coexistence still require testing.')
        return 0
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        report.update(status='failed', error=str(exc))
        print(f'MMV preflight failed: {exc}', file=sys.stderr)
        return 1
    finally:
        write_json(REPORTS / 'build-report.json', report)


if __name__ == '__main__':
    sys.exit(main())
