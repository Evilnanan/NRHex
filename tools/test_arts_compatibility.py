"""Check reviewed cross-weapon failures against exported native/MMV parameters."""
import json
import unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
NATIVE=ROOT/'.work/noisy/parameters.json'
MMV=ROOT/'work/compat/mmv/sword-parameters.json'
ARTS=ROOT/'work/compat/mmv/arts-parameters.json'


@unittest.skipUnless(all(p.exists() for p in [NATIVE,MMV,ARTS]),'requires original and target parameter exports')
class ArtsCompatibilityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        native=json.loads(NATIVE.read_text(encoding='utf8'))
        mmv=json.loads(MMV.read_text(encoding='utf8'))
        arts=json.loads(ARTS.read_text(encoding='utf8'))
        cls.native={r['ID']:r['fields'] for r in native['attacks']}
        cls.target={r['ID']:r['fields'] for r in mmv['tables']['AtkParam_Pc']}
        cls.bullets={r['ID']:r['fields'] for r in mmv['tables']['Bullet']}
        cls.behaviors={r['ID']:r['fields'] for r in arts['tables']['BehaviorParam_PC']}

    def test_elemental_damage_has_no_other_base_or_physical_component(self):
        cases={'Fire':[302106900,302106901,302106902,302106903,302106904,302106905,302106906,300314902,301202900,300214925,300214926],
               'Mag':[301612910,301612911,300214906,300214907,300214908,300214909,301302900,301302901,301302902,301302903,301302904,301302905],
               'Dark':[300317900,300317906,300317910,300317911,301604902]}
        for element,ids in cases.items():
            for i in ids:
                for table in [self.native,self.target]:
                    row=table[i]
                    for kind in ['Phys','Mag','Fire','Thun','Dark']:
                        self.assertEqual(row['atk'+kind],0,(i,kind))
                        if kind==element:self.assertGreater(row['atk'+kind+'Correction'],0)
                        else:self.assertEqual(row['atk'+kind+'Correction'],0,(i,kind))

    def test_ammunition_holes_and_native_projectile_targets(self):
        for judge in [870,875,886]:self.assertNotIn(105000000+judge,self.behaviors)
        for judge in [601,606,610,615,620,625,630,635,641,646]:
            self.assertNotIn(105100000+judge,self.behaviors)
            row=self.behaviors[105000000+judge]
            self.assertEqual(row['refType'],1)
            self.assertIn(row['refId'],self.bullets)
        for judge in [840,845]:
            for family in [105000,105100]:self.assertNotIn(family*1000+judge,self.behaviors)
        for judge in [870,875,881,886,851,856]:
            row=self.behaviors[105100000+judge]
            self.assertEqual(row['refType'],1)
            self.assertIn(row['refId'],self.bullets)
        self.assertEqual(self.bullets[20100070]['isPenetrateChr'],1)

    def test_rancor_uses_arrow_behavior_and_ember_has_its_own_chain(self):
        for judge,bullet in [(900,20003500),(901,20003505)]:
            self.assertNotIn(104100000+judge,self.behaviors)
            self.assertEqual(self.behaviors[105000000+judge]['refId'],bullet)
            self.assertEqual(self.bullets[bullet]['isEnableAutoHoming'],1)
            self.assertIn(self.bullets[bullet]['atkId_Bullet'],self.target)
        self.assertEqual(self.behaviors[300300994]['refId'],100063400)
        self.assertEqual(self.behaviors[300300995]['refId'],2514700)
        self.assertIn(self.bullets[100063400]['intervalCreateBulletId'],self.bullets)


if __name__=='__main__':unittest.main()
