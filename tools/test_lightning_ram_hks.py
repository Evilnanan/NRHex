"""Execute real inserted HKS instructions and their MMV Lua translation."""
import struct,unittest
from pathlib import Path
from build_lightning_ram_hks import patch,QUERY,NAMES
from hks_bytecode import Reader,decode
from mmv_hks_fragments import Fragment
from mmv_hks_stage import runtime

RECOVERY=['ExecFallStart','ExecWeaponChange','ExecEvasion','ExecItem','ExecAttack','ExecMagic','ExecQuickTurnOnCancelTiming','ExecJump','ExecHandChange','ExecGuardOnCancelTiming','MoveStartonCancelTiming','ExecGesture']
def recovery_globals():
 g={n:lambda *a:0 for n in RECOVERY+['SetAIActionState','ClearAttackQueue','SetArtsGeneratorTransitionIndex','GetAttackRequest','CalcDamageCount']}
 g.update({n:0 for n in ['ALLBODY','FALL_TYPE_DEFAULT','ESTEP_NONE','QUICKTYPE_ATTACK','HAND_RIGHT','Event_Move']})
 g.update({'SWORDARTS_REQUEST_'+n:100+i for i,n in enumerate(['LEFT_NORMAL','BOTH_NORMAL','RIGHT_NORMAL','RIGHT_STEP','RIGHT_COMBO_1','RIGHT_COMBO_2','RIGHT_STANCE'])})
 g['DAMAGE_FLAG_PUSH']=5
 return g

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
   for mode in range(7):
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
      if code in (336,339,990062):return 0
      self.fail(str(code))
     globals_={'env':env,'act':lambda *a:acts.append(a),'c_SwordArtsID':113,'c_SwordArtsHand':1,'ACTION_ARM_L2':42,'GetVariable':lambda n:25 if n=='TurnAngle' else self.fail(n), 'DAMAGE_TYPE_WALL_RIGHT':17,'DAMAGE_TYPE_WALL_LEFT':18,'DAMAGE_DIR_FRONT':0,'DAMAGE_FLAG_SMALL_BLOW':10,'SetVariable':lambda *a:None,'ResetDamageCount':lambda:None,'Replanning':lambda:None,'ExecEventAllBody':events.append}
     globals_.update(recovery_globals())
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
     expected=[] if mode in (0,1,3) or forced else [{2:'W_SwordArtsOneShotComboEnd',4:'W_DamageLv7_SmallBlow',5:'W_Repelled_Wall',6:'W_DamageLv5_Push'}[mode]]
     self.assertEqual(returned,mode!=0);self.assertEqual(events,expected);self.assertEqual(bool(stops),bool(mode and forced))
     observed=(events[:],acts[:],stops[:]);events.clear();acts.clear();stops.clear()
     for k,v in globals_.items():lua.globals()[k]=v
     self.assertEqual(lua.globals().probe(),-99 if mode==0 else None)
     self.assertEqual((events,acts,stops),observed)
 def test_wall_finish_crash_and_forced_exit_priority(self):
  data,report=patch(Path('assets/original/action/script/c0000.hks').read_bytes());r=Reader(data);r.function()
  for f in r.functions:
   if f['name'] not in NAMES:continue
   lua=runtime();lua.execute('function probe()\n'+Fragment(f['code'][:report[f['name']]*4],f['constants']).lua()+'return -99\nend')
   for mode in [3,4,6]:
    for ended in [0,1]:
     for wall in [0,17,18]:
      for lethal in [False,True]:
       events=[];calls=[];variables=[];acts=[]
       def env(code,*args):
        calls.append(code)
        return {QUERY:mode,1108:1,1116:1,336:wall,339:ended}.get(code,0)
       def death():return int(lethal and 990062 in calls)
       g=dict(env=env,act=lambda *a:acts.append(a),GetVariable=lambda n:35,c_SwordArtsID=113,c_SwordArtsHand=1,ACTION_ARM_L2=42,
        ExecEventAllBody=events.append,DAMAGE_TYPE_WALL_RIGHT=17,DAMAGE_TYPE_WALL_LEFT=18,DAMAGE_DIR_FRONT=0,DAMAGE_FLAG_SMALL_BLOW=10,
        SetVariable=lambda *a:variables.append(a),ResetDamageCount=lambda:None,Replanning=lambda:None,ExecNearDeath=death,ExecDeath=death)
       g.update(recovery_globals())
       for name in ['ExecTalk','ExecMovableEventAnim','ExecDamage','ExecBirdAct','ExecFookShot']:g[name]=lambda *a:0
       for k,v in g.items():lua.globals()[k]=v
       lua.globals().probe()
       expected=['W_Idle']if mode==3 and ended else ['W_DamageLv7_SmallBlow']if mode==4 and not lethal else ['W_DamageLv5_Push']if mode==6 else []
       self.assertEqual(events,expected);self.assertEqual(calls.count(990062),int(mode==4))
       if mode==4 and not lethal:self.assertIn((141,10),acts);self.assertEqual(variables,[('DamageDirection',0)])
       if mode==4 and lethal:self.assertIn(990061,calls)
 def test_recovery_cancels_at_native_acceptance_and_held_skill_cannot_loop(self):
  data,report=patch(Path('assets/original/action/script/c0000.hks').read_bytes());r=Reader(data);r.function()
  for f in r.functions:
   if f['name'] not in NAMES:continue
   lua=runtime();lua.execute('function probe()\n'+Fragment(f['code'][:report[f['name']]*4],f['constants']).lua()+'return -99\nend')
   for accepted in [None]+RECOVERY:
    for skill in [False,True]:
     calls=[];stops=[];events=[]
     g=recovery_globals()
     g.update(env=lambda code,*args:3 if code==QUERY else stops.append(code) or 0,act=lambda *a:None,GetVariable=lambda n:0,c_SwordArtsID=113,c_SwordArtsHand=1,ACTION_ARM_L2=42,ExecEventAllBody=events.append,DAMAGE_TYPE_WALL_RIGHT=17,DAMAGE_TYPE_WALL_LEFT=18)
     for n in ['ExecNearDeath','ExecDeath','ExecTalk','ExecMovableEventAnim','ExecDamage','ExecBirdAct','ExecFookShot']:g[n]=lambda *a:0
     for n in RECOVERY:g[n]=lambda *a,n=n:calls.append(n) or int(n==accepted)
     g['GetAttackRequest']=lambda *a:102 if skill else 0
     for k,v in g.items():lua.globals()[k]=v
     lua.globals().probe()
     self.assertEqual(990061 in stops,accepted is not None and not(skill and accepted=='ExecAttack'))
     self.assertEqual(events,[])
     if skill:self.assertNotIn('ExecAttack',calls)
     if accepted=='MoveStartonCancelTiming':self.assertNotIn(339,stops)
if __name__=='__main__':unittest.main()
