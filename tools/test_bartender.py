"""Run the actual native switch block plus its two adapters, and the MMV Lua path."""
import json
import os
from pathlib import Path
import struct
import sys
import unittest
from build_bartender_hks import OWNER, QUERY, MARKERS, FID, patch, check, reader, ROOT
from hks_bytecode import decode
from mmv_hks_stage import compose, PLAN, runtime
from mmv_hks_fragments import functions


def execute(code, constants, env, act, hand, roll):
    words = [w[0] for w in struct.iter_unpack('>I', code)]
    globals_ = dict(env=env,act=act,TRUE=1,FALSE=0,RELIC_WEAPONCHANGE_ENCHANT=hand,
                    SetAIActionState=lambda:None,math={'random':lambda lo,hi:roll})
    regs = {}; pc = 0
    while pc < len(words):
        d=decode(words[pc]);pc+=1
        op,a,b,c,bx=[d[k] for k in ('op','a','b','c','bx')]
        rk=lambda i:constants[i-256] if i>=256 else regs[i]
        if op==91:regs[a]=globals_[constants[bx]]
        elif op==76:pass
        elif op==25:regs[a]=constants[bx]
        elif op==73:regs[a]=regs[b][constants[c]]
        elif op in (2,69):
            result=regs[a](*(regs[i] for i in range(a+1,a+b)))
            if c==2:regs[a]=result
        elif op==4:
            if (rk(b)==rk(c))!=bool(a):pc+=1
        elif op==59:
            if (constants[b]<=rk(c))!=bool(a):pc+=1
        elif op==28:pc+=bx-0x10000+1
        else:raise AssertionError((pc-1,d))


class BartenderTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.native=(ROOT/'assets/original/action/script/c0000.hks').read_bytes()
        cls.target,cls.ranges=patch(cls.native)
        cls.f=reader(cls.target).functions[FID]
        cls.added=sum(hi-lo for lo,hi in cls.ranges.values())
        cls.code=cls.f['code'][:(154+cls.added)*4]
        cls.modes=('bytecode',)
        mmv=Path(os.environ.get('NRHEX_MMV',str(ROOT.parent/'MMV')))
        if not mmv.exists(): return
        cls.modes=('bytecode','lua');cls.lua=runtime()
        source=(mmv/'mod/action/script/c0000.hks').read_bytes().decode('utf-8-sig')
        merged,report=compose(source,reader(cls.native),reader((ROOT/'dist/mod/action/script/c0000.hks').read_bytes()),json.loads(PLAN.read_text()))
        span=functions(merged)['WeaponChangeCommonFunction']
        helpers=next(e['after'] for e in report['edits'] if e['reason']=='WeaponChangeCommonFunction:helpers')
        cls.lua.execute(helpers+merged[span['start']:span['end']])
        cls.lua.execute('TRUE=1; FALSE=0; GetSpEffectID=1116; AddSpEffect=2002; FALL_TYPE_DEFAULT=0; SetAIActionState=function() end; ExecPassiveAction=function() return TRUE end')

    def run_chain(self,mode,effects,hand=0,roll=1,deferred=False,eligible=True):
        calls=[];pending=[];buffs=[]
        def env(code,effect=None):
            if code==QUERY:return int(OWNER in effects)
            self.assertEqual(code,1116);return int(effect in effects)
        def apply(code,effect):
            effect=int(effect);calls.append((code,effect))
            def update():
                if code==9001:effects.discard(effect)
                else:
                    self.assertEqual(code,2002)
                    effects.add(effect)
                    if 7035702<=effect<=7035716 and effect%2==0 and eligible:buffs.append(effect+1)
            if deferred:pending.append(update)
            else:update()
        if mode=='bytecode':execute(self.code,self.f['constants'],env,apply,hand,roll)
        else:
            g=self.lua.globals();g.env=env;g.act=apply;g.RELIC_WEAPONCHANGE_ENCHANT=hand
            g.math.random=lambda lo,hi:roll
            g.WeaponChangeCommonFunction(0)
        for update in pending:update()
        return calls,buffs

    def test_installed_patch_and_f32_precision(self):
        check((ROOT/'dist/mod/action/script/c0000.hks').read_bytes())
        self.assertNotIn(float(OWNER),self.f['constants'])
        for effect in MARKERS:self.assertEqual(struct.unpack('>f',struct.pack('>f',effect))[0],effect)

    def test_rotation_hands_debounce_coownership_and_deferred_actions(self):
        for mode in self.modes:
            for original_relic in (False,True):
                for deferred in (False,True):
                    effects={OWNER,7035701}|({7035700} if original_relic else set())
                    for i in range(12):
                        hand=i%2
                        calls,buffs=self.run_chain(mode,effects,hand,(i*31)%100+1,deferred)
                        expected=7035702+4*(i%4)+2*hand
                        self.assertEqual(buffs,[expected+1],(mode,i,original_relic,deferred))
                        self.assertEqual([e for c,e in calls if c==2002 and 7035702<=e<=7035716],[expected])
                        self.assertEqual(effects.intersection(MARKERS),{MARKERS[i%4]})
                        # Every update while the native 0.1 s guard is present is ignored.
                        for _ in range(4):self.assertEqual(self.run_chain(mode,effects,hand,deferred=deferred),([],[]))
                        effects.difference_update(range(7035718,7035722))
                        effects.discard(7035701)
                        self.assertEqual(self.run_chain(mode,effects,hand,deferred=deferred),([],[]))
                        effects.add(7035701)
                    # Expedition cleanup removes the saved sequence; starts at Magic again.
                    effects.difference_update(MARKERS);effects.difference_update(range(7035718,7035722))
                    self.assertEqual(self.run_chain(mode,effects)[1],[7035703])

    def test_native_random_relic_and_unowned_path_unchanged(self):
        for roll in range(1,101):
            for hand in (0,1):
                expected=7035702+(0 if roll>=75 else 4 if roll>=50 else 8 if roll>=25 else 12)+hand*2
                for mode in self.modes:
                    calls,buffs=self.run_chain(mode,{7035700,7035701},hand,roll)
                    self.assertEqual(buffs,[expected+1])
                    self.assertEqual(len(calls),2)
                    self.assertEqual(self.run_chain(mode,{7035701},hand,roll),([],[]))

    def test_native_eligibility_remains_downstream_of_selection(self):
        for mode in self.modes:
            effects={OWNER,7035701}
            calls,buffs=self.run_chain(mode,effects,eligible=False)
            self.assertEqual(buffs,[])
            self.assertIn((2002,7035702),calls)
            effects.difference_update(range(7035718,7035722))
            self.assertEqual(self.run_chain(mode,effects)[1],[7035707])


if __name__=='__main__':unittest.main()
