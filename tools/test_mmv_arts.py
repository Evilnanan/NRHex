import copy
import json
import unittest
from build_mmv_arts import ROOT, PROFILE, generate, table, numbers


@unittest.skipUnless((ROOT/'work/compat/mmv/arts-parameters.json').exists(),'requires target export')
class ArtsTargetTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw=json.loads((ROOT/'work/compat/mmv/arts-parameters.json').read_text(encoding='utf8'))
        cls.profile=json.loads(PROFILE.read_text(encoding='utf8'))
        cls.native=(ROOT/'src/arts_data.rs').read_text(encoding='utf8')
        cls.output,cls.report=generate(cls.raw,cls.profile,cls.native)

    def test_all_weapon_defaults_bows_and_elemental_variants(self):
        weapons=table(self.raw,'EquipParamWeapon')
        defaults=numbers(self.output,'DEFAULTS')
        self.assertEqual(dict(zip(defaults[::2],defaults[1::2])),{i:w['swordArtsParamId'] for i,w in weapons.items()})
        self.assertEqual(self.report['addedWeapons'],302)
        self.assertEqual(self.report['addedBowWeapons'],[7230000,41071000,41080000,41081000])
        for name,field in [('FIRE_WEAPONS','attackBaseFire'),('MAGIC_WEAPONS','attackBaseMagic')]:
            self.assertEqual(numbers(self.output,name),sorted(i for i,w in weapons.items() if w[field]>0))

    def test_only_reviewed_portable_skills_and_native_bow_split(self):
        self.assertEqual(len(self.report['addedSkills']),37)
        self.assertEqual(self.report['ordinaryPool'],212)
        self.assertEqual(numbers(self.output,'BOW_POOL'),numbers(self.native,'BOW_POOL'))
        self.assertTrue(set(numbers(self.native,'POOL'))<=set(numbers(self.output,'POOL')))
        self.assertTrue({1007,146,1201,4040,4050,5170,5270,5340,5360,8000}.isdisjoint(numbers(self.output,'POOL')))

    def test_changed_attack_or_animation_cannot_silently_enable(self):
        raw=copy.deepcopy(self.raw)
        raw['animations']['626'][0]['ID']+=1
        with self.assertRaisesRegex(ValueError,'changed: 126'):generate(raw,self.profile,self.native)
        raw=copy.deepcopy(self.raw)
        next(r for r in raw['tables']['BehaviorParam_PC'] if r['ID']==700000569)['fields']['refId']=-1
        with self.assertRaisesRegex(ValueError,'changed: 126'):generate(raw,self.profile,self.native)

    def test_new_skill_requires_review_and_conflicting_weapon_rows_fail(self):
        raw=copy.deepcopy(self.raw);row=copy.deepcopy(raw['tables']['SwordArtsParam'][0]);row['ID']=777777
        raw['tables']['SwordArtsParam'].append(row)
        with self.assertRaisesRegex(ValueError,'require review'):generate(raw,self.profile,self.native)
        raw=copy.deepcopy(self.raw);row=copy.deepcopy(raw['tables']['EquipParamWeapon'][0]);row['fields']['swordArtsParamId']+=1
        raw['tables']['EquipParamWeapon'].append(row)
        with self.assertRaisesRegex(ValueError,'Ambiguous'):generate(raw,self.profile,self.native)

    def test_mmv_notification_owner_hand_and_busy_boundaries(self):
        from mmv_hks_stage import runtime,arts_prefix
        lua=runtime();seen=[];lua.globals().TRUE=1;lua.globals().HAND_LEFT=0
        lua.globals().act=lambda *args:seen.append(args)
        for mode in ['used','busy']:
            f=lua.eval('function()\n'+arts_prefix(mode)+'return 123 end')
            for owned in [0,1]:
                lua.globals().env=lambda *args:owned if args==(1116,6999800) else 0
                for hand in [0,1]:
                    lua.globals().c_SwordArtsHand=hand;seen.clear()
                    self.assertEqual(f(),123)
                    self.assertEqual(seen,[(2002,6999801 if mode=='busy' else 6999802+hand)] if owned else [])
        self.assertEqual(self.profile['callbacks']['SwordArtsRolling_Sub_onUpdate']['mode'],'busy')
        self.assertEqual(self.profile['callbacks']['SwordArtsHalfBothLoopEnd_Upper_onUpdate']['mode'],'busy')


if __name__=='__main__':unittest.main()
