"""Execute both bytecode-derived MMV Lua and the intended native fallthrough."""
import struct, unittest
from pathlib import Path
from build_onyx_dash_hks import patch, QUERY, FLAG, SCALE
from hks_bytecode import Reader, decode
from mmv_hks_fragments import Fragment, functions
from mmv_hks_stage import runtime

class DashTests(unittest.TestCase):
    def test_prefix_matrix_preserves_inputs_and_only_short_circuits_the_window(self):
        source=Path('assets/original/action/script/c0000.hks').read_bytes()
        result,report=patch(source);r=Reader(result);r.function()
        f=next(f for f in r.functions if f['name']=='ExecTravelMagnification')
        n=report['prefixInstructions'];words=[decode(w[0]) for w in struct.iter_unpack('>I',f['code'][:n*4])]
        lua=runtime();prefix=Fragment(f['code'][:n*4],f['constants'],inputs={0:'attack_type'}).lua()
        lua.execute('function probe(attack_type)\n'+prefix+'return -99\nend')
        for attack in [12,13]:
            for eligible in [0,1]:
                for flag in [0,1]:
                    calls=[];regs={i:('original',i)for i in range(report['originalRegisters'])};regs[0]=attack;saved=regs.copy()
                    def env(code,*args):
                        if code==QUERY:return eligible
                        self.assertEqual((code,args),(301,(FLAG,)));return flag
                    globals_={'ATTACK_RIGHT_HEAVYDASH':12,'env':env,'act':lambda *args:calls.append(args),'TRUE':1}
                    pc=0;returned=False
                    while pc<n:
                        w=words[pc];pc+=1;op,a,b,c,bx=[w[k]for k in ['op','a','b','c','bx']]
                        if op==91:regs[a]=globals_[f['constants'][bx]]
                        elif op==76:pass
                        elif op==25:regs[a]=f['constants'][bx]
                        elif op==2:
                            v=regs[a](*(regs[i]for i in range(a+1,a+b)))
                            if c==2:regs[a]=v
                        elif op==4:
                            if (regs[b]==regs[c])!=bool(a):pc+=1
                        elif op==28:pc+=bx-0x10000+1
                        elif op==9:returned=True;self.assertEqual(regs[a],1);break
                        else:self.fail(str(w))
                    expected=attack==12 and eligible==1 and flag==1
                    self.assertEqual(returned,expected);self.assertEqual(calls,[(2001,SCALE)]if expected else[])
                    self.assertEqual({k:regs[k]for k in saved},saved)
                    for k,v in globals_.items():lua.globals()[k]=v
                    calls.clear();self.assertEqual(lua.globals().probe(attack),1 if expected else -99)
                    self.assertEqual(calls,[(2001,SCALE)]if expected else[])

    def test_patch_is_idempotent_and_keeps_all_native_branches(self):
        source=Path('dist/mod/action/script/c0000.hks').read_bytes()
        target,_=patch(source);self.assertEqual(patch(target)[0],target)
        a=Reader(source);a.function();b=Reader(target);b.function()
        for old,new in zip(a.functions,b.functions):
            self.assertTrue(new['code'].endswith(old['code']))
            if old['name']!='ExecTravelMagnification':self.assertEqual(old['code'],new['code'])

    def test_travel_prefix_does_not_bypass_mmv_fallback(self):
        source=Path('dist/mmv/mod/action/script/c0000.hks').read_text(encoding='utf-8-sig')
        f=functions(source)['ExecTravelMagnification'];body=source[f['start']:f['end']]
        # Test the existing native MMV multiplier with a disabled dash query.
        lua=runtime();calls=[];lua.globals()['env']=lambda *_:0;lua.globals()['act']=lambda *args:calls.append(args)
        for name,value in [('TRUE',1),('SetMovementScaleMult',2001),('ATTACK_RIGHT_DASH',11),('ATTACK_RIGHT_HEAVYDASH',12),('ATTACK_BOTH_DASH',21),('ATTACK_BOTH_HEAVYDASH',22)]:lua.globals()[name]=value
        lua.execute(body)
        for stick,locked,expected in [(1,False,1.25),(0,False,1),(0,True,1.25)]:
            lua.globals()['GetVariable']=lambda name:locked if name=='IsLockon' else stick
            calls.clear();lua.globals().ExecTravelMagnification(12);self.assertEqual(calls,[(2001,expected)])

if __name__=='__main__':unittest.main()
