"""Execute real inserted HKS instructions and their MMV Lua translation."""
import struct,unittest
from pathlib import Path
from build_lightning_ram_hks import patch,QUERY,NAMES
from hks_bytecode import Reader,decode
from mmv_hks_fragments import Fragment
from mmv_hks_stage import runtime
class RamTests(unittest.TestCase):
 def test_native_fallback_forced_exits_and_loop_control(self):
  original=Path('assets/original/action/script/c0000.hks').read_bytes();data,report=patch(original)
  self.assertEqual(patch(data)[0],data)
  r=Reader(data);r.function()
  guards=['throw','ExecNearDeath','ExecDeath','ExecTalk','ExecMovableEventAnim','event','ExecDamage','ExecBirdAct','ExecFookShot']
  for f in r.functions:
   if f['name'] not in NAMES:continue
   n=report[f['name']];prefix=f['code'][:n*4];words=[decode(x[0])for x in struct.iter_unpack('>I',prefix)]
   lua=runtime();lua.execute('function probe()\n'+Fragment(prefix,f['constants']).lua()+'return -99\nend')
   for mode in range(4):
    for forced in [None]+guards:
     events=[];acts=[];stops=[]
     def env(code,*args):
      if code==QUERY:
       self.assertEqual(args,(113,1,0.7,1,25));return mode
      if code==1108:return .7
      if code==1116:return 1
      if code==1112:return int(forced=='throw')
      if code==346:return int(forced=='event')
      if code==990061:stops.append(True);return 0
      self.fail(str(code))
     globals_={'env':env,'act':lambda *a:acts.append(a),'c_SwordArtsID':113,'c_SwordArtsHand':1,'ACTION_ARM_L2':42,'GetVariable':lambda n:25,'ExecEventAllBody':events.append}
     globals_.update({k:(lambda *a,k=k:int(forced==k))for k in guards if k not in ['throw','event']})
     regs={};pc=0;returned=False
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
      elif op==9:returned=True;break
      else:self.fail(str(w))
     expected=[] if mode==0 or forced or mode==1 else ['W_SwordArtsOneShotComboEnd'if mode==2 else'W_Repelled_Wall']
     self.assertEqual(returned,mode!=0);self.assertEqual(events,expected);self.assertEqual(bool(stops),bool(mode and forced))
     observed=(events[:],acts[:],stops[:]);events.clear();acts.clear();stops.clear()
     for k,v in globals_.items():lua.globals()[k]=v
     self.assertEqual(lua.globals().probe(),-99 if mode==0 else None)
     self.assertEqual((events,acts,stops),observed)
if __name__=='__main__':unittest.main()
