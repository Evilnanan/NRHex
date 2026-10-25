import copy
import json
import unittest
from pathlib import Path
from build_mmv_echo import ROOT, PROFILE, body, generate, ids


@unittest.skipUnless((ROOT / 'work/compat/mmv/sword-parameters.json').is_file(), 'requires exported MMV parameters')
class MmvEchoTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw = json.loads((ROOT / 'work/compat/mmv/sword-parameters.json').read_text(encoding='utf8'))
        cls.profile = json.loads(PROFILE.read_text(encoding='utf8'))
        cls.native = (ROOT / 'src/echo_cast_data.rs').read_text(encoding='utf8')
        cls.output, cls.report = generate(cls.raw, cls.profile, cls.native)

    def test_native_support_exclusions_release_and_visuals_are_preserved(self):
        self.assertTrue(self.report['nativeEntriesUnchanged'])
        for name in ('AREAS', 'WORLD_AREAS', 'RELEASES', 'VISUALS', 'EXCLUDED'):
            self.assertEqual(body(self.output, name), body(self.native, name))
        self.assertEqual(self.report['addedSpells'], 39)
        self.assertEqual(self.report['addedRoots'], 96)

    def test_new_helpers_personal_effects_and_melee_are_not_registered(self):
        selected = {p['magic']: p['roots'] for p in self.profile['spells']}
        for mid in (6610, 6620, 6907, 7300, 2006800, 3730100, 3730200, 3730300, 3730400):
            self.assertEqual(selected[mid], [])
        self.assertNotIn(10999999, [b for values in selected.values() for b in values])
        self.assertEqual(selected[2006400], [210640000])

    def test_streams_and_mixed_area_roots_are_distinct(self):
        self.assertEqual(self.report['streams'], [6906, 6908, 2007020, 2007300, 2007720, 2007740])
        self.assertIn((2007000, 210700000), self.report['worldRoots'])
        self.assertNotIn((2007000, 210700001), self.report['worldRoots'])
        self.assertTrue(set(ids(self.output, 'MMV_SPELLS')) <= set(ids(self.output, 'INDEPENDENT_HITS')))

    def test_new_spell_requires_review(self):
        raw = dict(self.raw, tables=dict(self.raw['tables']))
        raw['tables']['Magic'] = self.raw['tables']['Magic'] + [dict(ID=999987, fields=self.raw['tables']['Magic'][0]['fields'])]
        with self.assertRaisesRegex(ValueError, 'additions changed'):
            generate(raw, self.profile, self.native)

    def test_changed_bullet_chain_fails_instead_of_reusing_stale_review(self):
        raw = dict(self.raw, tables=dict(self.raw['tables']))
        raw['tables']['Bullet'] = [dict(row, fields=dict(row['fields'], HitBulletID=-1))
                                  if row['ID'] == 210450000 else row for row in self.raw['tables']['Bullet']]
        with self.assertRaisesRegex(ValueError, 'chain changed'):
            generate(raw, self.profile, self.native)

    def test_child_or_empty_helper_cannot_become_a_direct_root(self):
        for root, message in [(210450001, 'Non-root'), (10999999, 'Non-root')]:
            profile = copy.deepcopy(self.profile)
            next(p for p in profile['spells'] if p['magic'] == 2004500)['roots'] = [root]
            with self.assertRaisesRegex(ValueError, message):
                generate(self.raw, profile, self.native)
        profile = copy.deepcopy(self.profile)
        next(p for p in profile['spells'] if p['magic'] == 7540)['roots'] = [10999999]
        with self.assertRaisesRegex(ValueError, 'Non-damaging helper'):
            generate(self.raw, profile, self.native)

    def test_moving_side_of_mixed_spell_cannot_be_world_area(self):
        profile = copy.deepcopy(self.profile)
        next(p for p in profile['spells'] if p['magic'] == 2007000)['worldRoots'] = [210700001]
        with self.assertRaisesRegex(ValueError, 'World-area root'):
            generate(self.raw, profile, self.native)


if __name__ == '__main__':
    unittest.main()
