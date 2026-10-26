import copy
import json
import re
import unittest
from build_mmv_archmage import ROOT, generate


class CatalogueRules(unittest.TestCase):
    def fixture(self):
        magic = [dict(ID=i, fields=dict(ezStateBehaviorType=k, isEnchant=e, isShieldEnchant=s))
                 for i, k, e, s in [(1,0,0,0),(2,1,0,0),(3,0,1,0),(4,1,0,1),(5,0,0,0),(6,2,0,0)]]
        lottery = [dict(ID=i, fields=dict(magicId=i, chanceWeight=w)) for i,w in [(1,1),(2,10),(3,1),(4,1),(5,0),(6,1)]]
        return dict(tables=dict(Magic=magic, MagicTableParam=lottery))

    def test_weights_types_and_both_enchantment_flags(self):
        _, r = generate(self.fixture())
        self.assertEqual(r['classified'], [1,2,3,4,5])
        self.assertEqual(r['randomSpells'], [1,2])
        self.assertEqual(r['excludedEnchantments'], [3,4])
        self.assertEqual(r['excludedSpecial'], [6])

    def test_duplicate_lottery_rows_do_not_weight_random_pool(self):
        raw = self.fixture(); raw['tables']['MagicTableParam'] *= 3
        self.assertEqual(generate(raw)[1]['randomSpells'], [1,2])

    def test_missing_magic_is_excluded_and_ambiguous_classification_fails(self):
        raw=self.fixture();raw['tables']['Magic']=raw['tables']['Magic'][:-1]
        self.assertEqual(generate(raw)[1]['missingMagic'],[6])
        raw=self.fixture();row=copy.deepcopy(raw['tables']['Magic'][0]);row['fields']['ezStateBehaviorType']=1
        raw['tables']['Magic'].append(row)
        with self.assertRaisesRegex(ValueError,'Ambiguous Magic'):generate(raw)

    @unittest.skipUnless((ROOT/'work/compat/mmv/archmage-parameters.json').exists(), 'requires target export')
    def test_current_mmv_additions_and_native_preservation(self):
        raw=json.loads((ROOT/'work/compat/mmv/archmage-parameters.json').read_text(encoding='utf8'))
        _, r=generate(raw)
        native=(ROOT/'src/archmage_data.rs').read_text(encoding='utf8')
        old_known={int(i):int(k) for i,k in re.findall(r'\((\d+),(\d+)\)',native)}
        old_random=set(map(int,re.findall(r'\d+',native.split('RANDOM_SPELLS:&[i32]=&[')[1])))
        current={row['ID']:row['fields']['ezStateBehaviorType'] for row in raw['tables']['Magic']}
        self.assertTrue(all(current[i]==k for i,k in old_known.items()))
        self.assertTrue(old_random <= set(r['randomSpells']))
        self.assertEqual(len(set(r['classified'])-old_known.keys()),48)
        self.assertEqual(len(set(r['randomSpells'])-old_random),41)
        for i in [2007740,2006800,2004500,6907]: self.assertIn(i,r['randomSpells'])
        for i in [6610,6620,7300,3730100,3730200,3730300,3730400]:self.assertNotIn(i,r['randomSpells'])

    @unittest.skipUnless((ROOT/'dist/mmv/mod/action/script/c0000.hks').exists(), 'requires MMV HKS')
    def test_installed_mmv_preserves_all_spell_slot_notifications(self):
        from hks_bytecode import Reader
        from mmv_hks_fragments import functions
        reader=Reader((ROOT/'dist/mod/action/script/c0000.hks').read_bytes());reader.function()
        callbacks=[f for f in reader.functions if 6999936 in f['constants']]
        self.assertEqual(len(callbacks),34)
        source=(ROOT/'dist/mmv/mod/action/script/c0000.hks').read_text(encoding='utf8')
        parsed=functions(source)
        for f in callbacks:
            location=parsed[f['name']];body=source[location['body']:location['end']]
            self.assertIn('IndexMagicSlot',body)
            for token in set(f['constants']) & set(range(6999936,6999942)):
                self.assertIn(str(int(token)),body,f['name'])


if __name__=='__main__': unittest.main()
