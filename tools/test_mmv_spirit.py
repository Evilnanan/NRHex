import copy
import json
import struct
import unittest
from build_mmv_spirit import ROOT, REPORTS, FIRST, generate, pairs, boss_bindings


class HudBindingTests(unittest.TestCase):
    def test_parameterized_health_bar_does_not_promote_unrelated_helpers(self):
        def instruction(bank, op, data):
            return dict(Bank=bank, ID=op, hex=data.hex())
        entity, helper = 12340800, 12340801
        events = {'common_func': [dict(ID=100, Parameters=[dict(InstructionIndex=0, SourceStartByte=0,
                    TargetStartByte=4, ByteCount=4)], instructions=[instruction(2003, 11, bytes(12))])],
                  'm_test': [dict(ID=0, Parameters=[], instructions=[
                      instruction(2000, 6, struct.pack('<III', 0, 100, entity))])]}
        maps = {'m_test': [dict(NpcParamId=npc, entity=e, groups=[0], ModelName='c5000')
                           for npc, e in [(50000100, entity), (50000101, helper)]]}
        proof = boss_bindings(maps, events)
        self.assertEqual([p['npc'] for p in proof], [50000100])
        events['m_test'][0]['instructions'].clear()
        self.assertEqual(boss_bindings(maps, events), [])


@unittest.skipUnless((REPORTS/'spirit-parameters.json').exists(), 'requires MMV NPC/event/map export')
class SpiritTargetTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = json.loads((REPORTS/'spirit-parameters.json').read_text(encoding='utf8'))
        cls.output, cls.plan, cls.report = generate(cls.data)

    def test_native_private_ids_are_preserved_and_new_ids_are_unique(self):
        native = pairs(ROOT/'src/spirit_npcs.rs')
        self.assertEqual(self.plan['pairs'][:len(native)], native)
        private = [p for _, p in self.plan['pairs']]
        self.assertEqual(private, list(range(FIRST, FIRST+len(private))))
        self.assertGreater(self.report['newPrivateRows'], 5000)

    def test_actual_models_override_numeric_prefix(self):
        mapping = dict(self.plan['pairs'])
        for source, model in [(62000001, 6201), (901300011, 9013), (901200010, 9012)]:
            self.assertIn(f' ({mapping[source]},{model}),', self.output)
        self.assertNotIn(600030520, mapping)  # PC initialization is not an NPC descriptor.
        self.assertNotIn(524070090, mapping)

    def test_boss_type_is_not_misclassified_as_a_player_descriptor(self):
        mapping=dict(self.plan['pairs'])
        for npc,model in [(46400960,4640),(46500070,4650),(53001000,5300),(20310050,2031)]:
            self.assertIn(npc,mapping)
            self.assertIn(f' ({mapping[npc]},{model}),',self.output)
            self.assertIn(npc,{p['npc'] for p in self.report['bossBindings']})
        self.assertNotIn(10006050,mapping) # Unplaced c1000 Talk Dummy still needs PC initialization.
        registry=json.loads((ROOT/'compat/mmv/spirit-npc-ids.json').read_text(encoding='utf8'))['pairs']
        for npc,private in registry:self.assertEqual(mapping[npc],private)

    def test_boss_flags_alone_do_not_qualify_helpers(self):
        bosses = {p['npc'] for p in self.report['bossBindings']}
        self.assertIn(50000100, bosses)  # Gaius night boss: actual health bar binding.
        self.assertIn(53001000, bosses)  # Rellana.
        self.assertNotIn(51400099, bosses)  # Messmer's auxiliary serpent, despite boss flag.
        self.assertNotIn(31600020, bosses)  # Horse is not promoted by reward flags.

    def test_duplicate_sources_and_private_collisions_fail_closed(self):
        data = copy.deepcopy(self.data)
        data['rows'].append(copy.deepcopy(data['rows'][0]))
        with self.assertRaisesRegex(ValueError, 'Ambiguous'): generate(data)
        data = copy.deepcopy(self.data)
        row = copy.deepcopy(data['rows'][0]);row['ID'] = FIRST + len(pairs(ROOT/'src/spirit_npcs.rs'))
        data['rows'].append(row)
        with self.assertRaisesRegex(ValueError, 'collision'): generate(data)

    def test_map_order_does_not_change_network_private_ids(self):
        data = dict(self.data, maps=dict(reversed(list(self.data['maps'].items()))))
        output, plan, _ = generate(data)
        self.assertEqual(output, self.output)
        self.assertEqual(plan, self.plan)


if __name__ == '__main__':
    unittest.main()
