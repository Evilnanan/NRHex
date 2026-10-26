"""Exercise MMV cage selection and generated lifecycle/reward wiring offline."""
import json
import struct
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / 'work/compat/mmv/staged/cages-validation.json'


def instructions(event):
    return [(i['Bank'], i['ID'], bytes.fromhex(i['hex'])) for i in event['instructions']]


def u32(data, offset=0):
    return struct.unpack_from('<I', data, offset)[0]


def select(report, draw, cooling=False, client=False):
    code = instructions(report['selector'])
    p = report['selector']['ID'] - 9700
    choices = list(range(p+7500, p+7550)) + [e['choice'] for e in report['encounters']]
    cool = list(range(p+7550, p+7600)) + [e['cooling'] for e in report['encounters']]
    flags = {p+7402: True, cool[draw]: cooling}
    labels = {op: index for index, (bank, op, _) in enumerate(code) if bank == 1014}
    outputs, pc = [], 0
    for _ in range(len(code)*2):
        bank, op, data = code[pc];pc += 1
        if bank == 2000 and op == 2: pass
        elif bank == 1003 and op == 14:
            if client:return outputs
        elif bank == 3 and op == 0:
            if bool(flags.get(u32(data,4),False)) != bool(data[1]):return outputs
        elif bank == 1003 and op == 101:
            if bool(flags.get(u32(data,4),False)) == bool(data[1]):pc = labels[data[0]] + 1
        elif bank == 1003 and op == 1:
            if bool(flags.get(u32(data,4),False)) == bool(data[1]):pc += data[0]
        elif bank == 2003 and op == 69:
            flag=u32(data,4);flags[flag]=bool(data[8]);outputs.append((flag,bool(data[8])))
            if flag == p+7403:return outputs
        elif bank == 2003 and op == 22:
            for flag in range(u32(data),u32(data,4)+1):flags[flag]=False
        elif bank == 2003 and op == 17:
            assert u32(data,4)-u32(data)+1 == len(choices)
            flags[u32(data)+draw]=True
        elif bank == 1001 and op == 1: pass
        elif bank == 1000 and op == 4:return outputs
        elif bank == 1014:pass
        else:raise AssertionError((bank,op,data.hex()))
    raise AssertionError('Selector failed to yield')


@unittest.skipUnless(REPORT.exists(), 'build MMV cage stage first')
class CageAdapterTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.report=json.loads(REPORT.read_text(encoding='utf8'))
        cls.catalog=json.loads((ROOT/'compat/mmv/cage-encounters.json').read_text(encoding='utf8'))

    def test_all_choices_publish_exactly_one_shared_on_and_cooling_rejects(self):
        for r in self.report['reports']:
            p=r['selector']['ID']-9700
            choices=list(range(p+7500,p+7550))+[e['choice'] for e in r['encounters']]
            for index,flag in enumerate(choices):
                writes=select(r,index)
                self.assertEqual([v for v in writes if v[0] in choices],[(flag,True)])
                self.assertIn((p+7403,True),writes)
                self.assertFalse(any(f in choices for f,_ in select(r,index,cooling=True)))
                self.assertEqual(select(r,index,client=True),[])

    def test_flags_fit_allocated_bank_and_only_mutation_is_shared(self):
        for r in self.report['reports']:
            p=r['selector']['ID']-9700
            flags=[r['red'],r['decided']]+list(range(r['draft'],r['draft']+r['encounterCount']))
            for e in r['encounters']:flags += [e['choice'],e['cooling'],e['reward']]
            for a in r['actors']:flags += [a['shown']]+([a['dead']] if a['combat'] else [])
            self.assertEqual(len(flags),len(set(flags)))
            self.assertTrue(all(p+7000<=f<p+8000 for f in flags))
            old=set(range(p+7500,p+7600))|set(range(p+7600,p+7637))|set(range(p+7650,p+7750))|set(range(p+7787,p+7834))|set(range(p+7887,p+7900))|set(range(p+7900,p+7984))
            self.assertFalse(old & set(flags))

    def test_every_actor_has_two_demand_creation_paths_and_local_correction(self):
        for r in self.report['reports']:
            for actor in r['actors']:
                matching=[e for e in r['events'] if any(b==2003 and op==54 and u32(d)==actor['generator'] for b,op,d in instructions(e))]
                self.assertEqual(len(matching),1)
                code=instructions(matching[0]);requests=[n for n,(b,op,d) in enumerate(code) if b==2003 and op==54 and u32(d)==actor['generator']]
                self.assertEqual(len(requests),2)
                for n in requests:
                    self.assertEqual(code[n+1][:2],(2004,34))
                    self.assertEqual(code[n+2][:2],(2004,29))
                    self.assertEqual(u32(code[n+1][2]),actor['actor'])
                self.assertTrue(any(b==2004 and op==8 and u32(d)==actor['actor'] and u32(d,4)==95001603 for b,op,d in code))

    def test_noncombat_helpers_never_qualify_for_boss_soul_hud(self):
        for e in self.catalog['encounters']:
            self.assertTrue(any(m['combat'] for m in e['members']))
            self.assertEqual(len(e['members']),len({m['entity'] for m in e['members']}))
            self.assertLessEqual(sum(m['combat'] for m in e['members']),9)
            if any(m['model']=='c4483' for m in e['members']):
                self.assertTrue(all(not m['combat'] for m in e['members'] if m['model']=='c4483'))
        forbidden={22400000,53001000,62000000,48000010,50301000,52100010,95150012}
        self.assertFalse(forbidden & {m['npc'] for e in self.catalog['encounters'] for m in e['members']})
        spirit=REPORT.parent/'spirit-validation.json'
        if spirit.exists():
            bosses={p['npc'] for p in json.loads(spirit.read_text(encoding='utf8'))['bossBindings']}
            for e in self.catalog['encounters']:
                for m in e['members']:
                    self.assertEqual(95300200+m['index'] in bosses,m['combat'])

    def test_reward_group_and_demand_npc_catalog_match_resource_ids(self):
        import re
        text=(REPORT.parent/'cage_data.rs').read_text(encoding='utf8')
        arrays={name:list(map(int,re.findall(r'\d+',body))) for name,body in re.findall(r'pub const (\w+): &\[\w+\] = &\[(.*?)\];',text)}
        for r in self.report['reports']:
            self.assertEqual(arrays['EXTRA_NPCS'],[a['clone'] for a in r['actors']])
            self.assertEqual(arrays['EXTRA_REWARD_GROUPS'],[e['target']%10000 for e in r['encounters']])
            for e in r['encounters']:
                code=[item for event in r['events'] for item in instructions(event)]
                claims=[d for b,op,d in code if b==2003 and op==69 and u32(d,4)==e['reward']]
                self.assertEqual(len(claims),1)
                self.assertEqual(claims[0][8],0) # only native settlement sets it ON


if __name__=='__main__':unittest.main()
