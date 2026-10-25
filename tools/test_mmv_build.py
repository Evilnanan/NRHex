"""MMV preflight safety checks using small, disposable inputs only."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import mmv_build as build


class MmvPreflightTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.folder = Path(self.temp.name)
        self.mmv = self.folder / 'MMV'
        (self.mmv / 'mod').mkdir(parents=True)
        (self.mmv / 'dll').mkdir()
        (self.mmv / 'mod/regulation.bin').write_bytes(b'params')
        (self.mmv / 'dll/example.dll').write_bytes(b'library')
        (self.mmv / 'MMV.me3').write_text('profileVersion = "v1"\n')

    def test_resource_or_profile_drift_rejected(self):
        current = build.inventory(self.mmv)
        lock = build.lock_data(current)
        build.validate_lock(lock, current)
        for file in ['mod/regulation.bin', 'dll/example.dll', 'MMV.me3']:
            path = self.mmv / file
            original = path.read_bytes()
            path.write_bytes(original + b'changed')
            with self.assertRaisesRegex(ValueError, 'baseline changed'):
                build.validate_lock(lock, build.inventory(self.mmv))
            path.write_bytes(original)

    def test_added_and_removed_resources_rejected(self):
        lock = build.lock_data(build.inventory(self.mmv))
        added = self.mmv / 'mod/new.bin'
        added.write_bytes(b'new')
        with self.assertRaisesRegex(ValueError, 'baseline changed'):
            build.validate_lock(lock, build.inventory(self.mmv))
        added.unlink()
        (self.mmv / 'dll/example.dll').unlink()
        with self.assertRaisesRegex(ValueError, 'baseline changed'):
            build.validate_lock(lock, build.inventory(self.mmv))

    def test_logs_do_not_invalidate_baseline(self):
        before = build.inventory(self.mmv)
        (self.mmv / 'dll/game.log').write_bytes(b'log output')
        (self.mmv / 'logs').mkdir()
        (self.mmv / 'logs/session.txt').write_text('not a build input')
        (self.mmv / 'dll/target/release').mkdir(parents=True)
        (self.mmv / 'dll/target/release/0987.txt').write_text('nraio > runtime log')
        self.assertEqual(before, build.inventory(self.mmv))
        (self.mmv / 'dll/settings.txt').write_text('real input')
        self.assertNotEqual(before, build.inventory(self.mmv))

    def test_no_release_on_successful_inspection(self):
        repo = self.folder / 'repo'
        repo.mkdir()
        config = repo / 'target.json'
        config.write_text(json.dumps(dict(name='NRHex-MMV', output='dist/mmv', requiredAdapters=['HKS'])))
        (repo / 'Cargo.toml').write_text('[package]\nversion="test"\n')
        reports = repo / 'work'
        lock = repo / 'baseline.lock.json'
        build.write_json(lock, build.lock_data(build.inventory(self.mmv)))
        before = build.inventory(self.mmv)

        def fake_dotnet(*args, **kwargs):
            build.write_json(reports / 'audit.json', dict(records=[], conflicts=[]))
            build.write_json(reports / 'staged/manifest.json', dict(releaseReady=False, stage='partial-preview', files={}))
            build.write_json(reports / 'staged/events-validation.json', dict(files=[]))

        with patch.multiple(build, ROOT=repo, CONFIG=config, LOCK=lock, REPORTS=reports), \
             patch.object(build, 'original_assets'), \
             patch.object(build, 'shared_inputs', return_value={'resource': 'hash'}), \
             patch.object(build.subprocess, 'check_output', return_value='revision'), \
             patch.object(build.subprocess, 'run', side_effect=fake_dotnet), \
             patch.object(build, 'stage_hks', return_value=dict(functions=['example'], inverseRestoresMmvSource=True,
                          lua51SyntaxVerified=True, outputSha256='hash')), \
             patch.object(build, 'audit_hks', return_value=dict(prefixFunctions=1, interiorFunctions=1)):
            self.assertEqual(build.main(['--mmv', str(self.mmv)]), 1)
            self.assertFalse((repo / 'dist/mmv').exists())
            report = json.loads((reports / 'build-report.json').read_text())
            self.assertEqual(report['status'], 'failed')
            self.assertFalse(report['releaseReady'])
            self.assertEqual(build.main(['--mmv', str(self.mmv), '--audit-only']), 0)
            self.assertFalse(json.loads((reports / 'build-report.json').read_text())['releaseReady'])
            # A previously successful report cannot disguise a new failed preflight.
            (self.mmv / 'mod/regulation.bin').write_bytes(b'drift')
            self.assertEqual(build.main(['--mmv', str(self.mmv), '--audit-only']), 1)
            self.assertEqual(json.loads((reports / 'build-report.json').read_text())['status'], 'failed')
            (self.mmv / 'mod/regulation.bin').write_bytes(b'params')
        self.assertEqual(before, build.inventory(self.mmv))


if __name__ == '__main__':
    unittest.main()
