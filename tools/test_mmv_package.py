import tempfile
import hashlib
import json
import shutil
import tomllib
from pathlib import Path
import unittest

from mmv_package import profile, output_path, community_profile, COMMUNITY_PROFILE, source_profiles


class OutputTests(unittest.TestCase):
    def test_test_build_is_nested_and_community_requires_external_destination(self):
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder).resolve()
            root, mmv = base / 'repo', base / 'MMV'
            self.assertEqual(output_path(root, mmv, 'dist'), root / 'dist/mmv')
            for destination in (None, Path('relative'), root, root / 'dist/community', mmv, mmv / 'overlay', base):
                with self.subTest(destination=destination), self.assertRaises(ValueError):
                    output_path(root, mmv, 'community', destination)
            external = base / 'community'
            self.assertEqual(output_path(root, mmv, 'community', external), external)
            self.assertFalse(external.exists())


class ProfileTests(unittest.TestCase):
    def test_community_relocates_and_keeps_independent_fixes(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            base = root / 'MMV'
            (base / 'dll').mkdir(parents=True)
            (base / 'mod').mkdir()
            (base / 'dll/test.dll').write_bytes(b'mmv')
            (base / 'dll/test.ini').write_text('user=true')
            source = base / 'original.me3'
            content = '''profileVersion="v1"
savefile="NR0000.dbb"
start_online=true
disable_arxan=true
[[supports]]
game="nightreign"
[[packages]]
path="mod"
[[natives]]
path="dll/test.dll"
config="dll/test.ini"
load_early=true
'''
            source.write_text(content)
            fixes = root / 'fixes/mod'
            fixes.mkdir(parents=True)
            (fixes / 'fixed.bin').write_bytes(b'fixed')
            (fixes.parent / 'natives').mkdir()
            (fixes.parent / 'natives/MMVFixes.dll').write_bytes(b'fixdll')
            (fixes.parent / 'MMVFixes.log').write_text('private log')
            manifest = dict(sourceFiles={}, files={name: hashlib.sha256((fixes.parent / name).read_bytes()).hexdigest()
                for name in ['mod/fixed.bin', 'natives/MMVFixes.dll']})
            (fixes.parent / 'manifest.json').write_text(json.dumps(manifest))
            text, parsed, payload = community_profile(source, fixes)
            self.assertFalse(text.startswith('#'))
            self.assertTrue(parsed['start_online'])
            self.assertTrue(parsed['disable_arxan'])
            self.assertEqual(parsed['savefile'], 'NR0000.dbb')
            self.assertEqual(parsed['packages'][1]['path'], 'MayhemAugments/mod')
            self.assertEqual(parsed['packages'][1]['load_after'], [dict(id='mod', optional=False)])
            self.assertEqual(parsed['packages'][2]['load_after'], [dict(id=i, optional=False) for i in ['mod', 'nrhex-mmv']])
            self.assertEqual(set(payload), {'MMV-Fixes/mod/fixed.bin', 'MMV-Fixes/natives/MMVFixes.dll'})
            self.assertEqual(source.read_text(), content)
            self.assertEqual(tomllib.loads(text), parsed)
            # Both unrelated installation roots must resolve all dependencies;
            # no developer drive, log, manifest or original MMV file is shipped.
            for name in ['relocated one', 'relocated two']:
                target = root / name
                shutil.copytree(base, target)
                (target / 'MayhemAugments/mod').mkdir(parents=True)
                for relative, data in {**payload, 'MayhemAugments/natives/NRHex.dll': b'ma'}.items():
                    path = target / relative
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(data)
                (target / COMMUNITY_PROFILE).write_text(text)
                self.assertEqual(source_profiles(target), [target / 'original.me3'])
                regenerated, _, _ = community_profile(target / 'original.me3', fixes)
                self.assertEqual(regenerated, text)
                for kind in ['packages', 'natives']:
                    for entry in parsed[kind]:
                        for key in ['path', 'config']:
                            if key in entry:
                                self.assertFalse(Path(entry[key]).is_absolute())
                                self.assertTrue((target / entry[key]).exists(), entry)
            (base / 'MayhemAugments').mkdir()
            with self.assertRaisesRegex(ValueError, 'already exists'):
                community_profile(source)

    def test_community_rejects_external_dependency(self):
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder).resolve() / 'MMV'
            base.mkdir()
            external = base.parent / 'shared.dll'
            external.write_bytes(b'shared')
            source = base / 'source.me3'
            source.write_text('profileVersion="v1"\n[[packages]]\npath="mod"\n[[natives]]\npath="../shared.dll"\n')
            with self.assertRaisesRegex(ValueError, 'outside its directory'):
                community_profile(source)

    def test_external_base_and_explicit_overlay_precedence(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            base = root / 'MMV'
            base.mkdir()
            (base / 'dll').mkdir()
            (base / 'dll/example.dll').write_bytes(b'fixture')
            source = base / 'MMV.me3'
            content = '''profileVersion="v1"
savefile="NR0000.dbb"
start_online=true
disable_arxan=true
[[supports]]
game="nightreign"
[[packages]]
path="mod"
[[natives]]
path="dll/example.dll"
load_early=true
config="dll/example.ini"
[[natives]]
path="dll/UltrawideFix.dll"
'''
            source.write_text(content)
            text, result = profile(source, root / 'overlay')
            self.assertEqual(source.read_text(), content)
            self.assertEqual(tomllib.loads(text), result)
            self.assertEqual(result['savefile'], 'NR0000.dbb')
            self.assertTrue(result['start_online'])
            self.assertTrue(result['disable_arxan'])
            self.assertEqual(result['packages'][0]['path'], str((base / 'mod').resolve()))
            self.assertEqual(result['packages'][1]['load_after'], [{'id': 'mod', 'optional': False}])
            self.assertEqual(result['natives'][0], dict(path=str((base / 'dll/example.dll').resolve()),
                             load_early=True, config=str((base / 'dll/example.ini').resolve())))
            self.assertTrue(result['natives'][1]['optional'])
            self.assertEqual(len(result['natives']), 3)
            (base / 'dll/example.dll').unlink()
            with self.assertRaisesRegex(ValueError, 'Required MMV DLL is missing'):
                profile(source, root / 'overlay')


if __name__ == '__main__':
    unittest.main()
