import copy
import json
from pathlib import Path
import tempfile
import unittest
from mmv_fixes import with_package, checked_package, sha


class IndependentFixPackageTests(unittest.TestCase):
    def test_opt_in_and_removal_preserve_other_mods_and_settings(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);mod=root/'mod';mod.mkdir();f=mod/'example';f.write_bytes(b'fixed')
            (root/'manifest.json').write_text(json.dumps(dict(sourceFiles={},files={'mod/example':sha(f)})))
            initial=dict(profileVersion='v1',start_online=True,savefile='custom.save',natives=[dict(path='unchanged.dll')],
                         packages=[dict(id='mod',path='MMV/mod'),dict(id='nrhex-mmv',path='NRHex/mod',load_after=[dict(id='mod')])])
            before=copy.deepcopy(initial);enabled=with_package(initial,mod)
            self.assertEqual(initial,before)
            self.assertEqual(with_package(enabled,None),before)
            self.assertEqual(enabled['packages'][-1]['load_after'],[dict(id='mod',optional=False),dict(id='nrhex-mmv',optional=False)])
            self.assertEqual(with_package(enabled,mod),enabled)
            dll=root/'natives/MMVFixes.dll';dll.parent.mkdir();dll.write_bytes(b'test dll')
            (root/'manifest.json').write_text(json.dumps(dict(sourceFiles={},files={'mod/example':sha(f),'natives/MMVFixes.dll':sha(dll)})))
            native_enabled=with_package(enabled,mod)
            self.assertEqual(native_enabled['natives'][0],before['natives'][0])
            self.assertEqual(native_enabled['natives'][-1],dict(id='mmv-fixes',path=str(dll.resolve()),load_early=True))
            self.assertEqual(with_package(native_enabled,None),before)
            self.assertEqual(with_package(native_enabled,mod),native_enabled)
            f.write_bytes(b'manual edit')
            with self.assertRaisesRegex(ValueError,'output changed'):checked_package(mod)

    def test_upstream_update_rejects_stale_fix(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);mod=root/'mod';mod.mkdir();source=root/'source';source.write_bytes(b'old upstream')
            (root/'manifest.json').write_text(json.dumps(dict(sourceFiles={str(source):sha(source)},files={})))
            checked_package(mod);source.write_bytes(b'updated upstream')
            with self.assertRaisesRegex(ValueError,'source changed'):checked_package(mod)


if __name__=='__main__':unittest.main()
