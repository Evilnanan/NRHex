import tempfile
import tomllib
from pathlib import Path
import unittest

from mmv_package import profile, output_path


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
