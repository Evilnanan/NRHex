"""Verify reread group assets and execute their actual serialized events offline."""
from collections import defaultdict
from pathlib import Path
import hashlib,itertools,json,os,struct,subprocess,sys,tomllib
from cage_selector_events import verify_selector
root=Path(__file__).resolve().parents[1]
read=lambda p:json.loads(p.read_text(encoding='utf8'))
catalog=read(root/'data/reference/world/cage-encounters.json')
built=read(Path(os.environ.get('NRHEX_GROUP_REPORT',root/'.work/cage-groups/built.json')))
validation_root=Path(os.environ.get('NRHEX_VALIDATION_ROOT',root/'.work/cage-groups'));validation_root.mkdir(parents=True,exist_ok=True)
stage=Path(built['output'])
schema={(o['bank'],o['id']):(o['name'],[t for t,_ in o['args']]) for o in read(root/'data/reference/world/cage-opcodes.json')['opcodes']}
schema.update({(2003,102):('LinkToBossHealthBar',['sbyte','int','uint']),
 (4,23):('IfCharacterRatioHPValue',['sbyte','uint','sbyte','int','sbyte','float']),
 (3,42):('IfEventFlagAndRandomCondition',['sbyte','uint','float']),
 (1003,13):('GotoIfPlayerIsInWorldType',['byte','byte']),
 (1003,14):('EndIfPlayerIsInWorldType',['byte','byte']),
 (2009,4):('ActivateMultiplayerdependantBuffs',['uint']),
 (2004,28):('SetNetworkUpdateAuthority',['uint','int'])})
for i in range(10):schema[1014,i]=(f'Label{i}',[])
fmt=dict(sbyte='b',byte='B',bool='B',int='i',uint='I',short='h',ushort='H',float='f')
def decode(i):
 name,types=schema[i['Bank'],i['ID']];raw=bytes.fromhex(i['hex']);at=0;args=[]
 for t in types:
  size=struct.calcsize(fmt[t]);at=(at+size-1)//size*size;args.append(struct.unpack_from('<'+fmt[t],raw,at)[0]);at+=size
 assert len(raw)==(at+3)//4*4 or i['Bank']==2000 and i['ID'] in [0,6]
 return name,args
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def prefix(r):return r['selector']['ID']//10000*10000
def entity(p,m):return p+9000+m['index']
def shown(p,m):return p+7750+m['index']
def dead(p,m):return p+7900+m['index']
def event(r,id):return [decode(i) for i in next(e for e in r['events'] if e['ID']==id)['instructions']]

class World:
 def __init__(self,p,g):
  self.p,self.g=p,g;self.flags={p+7403:True,p+7500+g['choice']:True};self.actors={entity(p,m):dict(hp=100,loaded=True,effects=set()) for m in g['members']};self.groups={p+5900+g['choice']:[entity(p,m) for m in g['members'] if m['combat']]};self.pending=[];self.draw=0;self.client=False;self.outputs=[]
 def members(self,id):return self.groups.get(id,[id])

class VM:
 def __init__(self,code,w,local=False):
  self.code,self.w,self.local=code,w,local;self.flags={};self.pc=0;self.groups=defaultdict(list);self.edges=defaultdict(set);self.compiled={};self.time=0;self.deadline=None;self.outputs=[];self.player_alive=True;self.distance=10
 def flag(self,id):return self.flags.get(id,False) if id in [5000,5001] or id%10000 in range(7750,7834) else self.w.flags.get(id,False)
 def group(self,g):return bool(self.groups[g]) and (any(f() for f in self.groups[g]) if g<0 else all(f() for f in self.groups[g]))
 def condition(self,g,f):
  self.groups[g].append(f)
  if g==0:
   if not self.group(0):self.groups[0].pop();return False
   visited=set()
   def snapshot(g):
    if g in visited:return
    visited.add(g)
    for child in self.edges[g]:snapshot(child)
   snapshot(0);self.compiled.update({g:self.group(g) for g in visited});self.groups.clear();self.edges.clear()
  return True
 def restart(self):self.pc=0;self.groups.clear();self.edges.clear();self.compiled.clear();self.deadline=None
 def step(self,t=0):
  self.time=t
  for _ in range(1000):
   if self.pc==len(self.code):return
   n,a=self.code[self.pc]
   if n=='EndIfPlayerIsInWorldType':
    assert a==[0,1]
    if self.w.client:self.pc=len(self.code);return
   elif n=='IfConditionGroup':
    self.edges[a[0]].add(a[2])
    if not self.condition(a[0],lambda a=a:self.group(a[2])==bool(a[1])):return
   elif n=='IfEventFlag':
    if not self.condition(a[0],lambda a=a:self.flag(a[3])==bool(a[1])):return
   elif n=='IfCharacterHPValue':
    assert a[4:]==[0,1]
    def hp(a=a):
     s=self.w.actors[a[1]]
     return s['loaded'] and ({0:s['hp']==a[3],2:s['hp']>a[3],5:s['hp']<=a[3]}[a[2]])
    if not self.condition(a[0],hp):return
   elif n=='IfCharacterRatioHPValue':
    assert a[2:]==[5,0,0,1]
    if not self.condition(a[0],lambda a=a:all(self.w.actors[id]['hp']<=0 for id in self.w.members(a[1]))):return
   elif n=='IfCharacterBackreadStatus':
    if not self.condition(a[0],lambda a=a:self.w.actors[a[1]]['loaded']==bool(a[2])):return
   elif n=='IfCharacterDeadalive':
    assert a[1]==20000
    if not self.condition(a[0],lambda a=a:self.player_alive==(a[2]==0)):return
   elif n=='IfEntityInoutsideRadiusOfEntity':
    if not self.condition(a[0],lambda a=a:(self.distance<=a[4])==bool(a[1])):return
   elif n=='IfCharacterHasSpEffect':
    if not self.condition(a[0],lambda a=a:(a[2] in self.w.actors[a[1]]['effects'])==bool(a[3])):return
   elif n=='IfEventFlagAndRandomCondition':
    if not self.condition(a[0],lambda a=a:self.flag(a[1]) and self.w.draw<a[2]):return
   elif n=='IfPlayAreaCurrentTimeInRange':
    if not self.condition(a[0],lambda:False):return
   elif n in ['GotoIfEventFlag','GotoIfConditionGroupStateUncompiled','GotoUnconditionally','GotoIfPlayerIsInWorldType']:
    go=self.flag(a[3])==bool(a[1]) if n=='GotoIfEventFlag' else self.group(a[2])==bool(a[1]) if n=='GotoIfConditionGroupStateUncompiled' else self.w.client if n=='GotoIfPlayerIsInWorldType' else True
    if go:
     self.pc=next(i for i in range(self.pc+1,len(self.code)) if self.code[i][0]==f'Label{a[0]}');continue
   elif n=='SkipUnconditionally':self.pc+=a[0]
   elif n=='SkipIfEventFlag':
    if self.flag(a[3])==bool(a[1]):self.pc+=a[0]
   elif n=='SkipIfConditionGroupStateCompiled':
    if self.compiled.get(a[2],False)==bool(a[1]):self.pc+=a[0]
   elif n=='SkipIfCharacterHasSpEffect':
    if (a[2] in self.w.actors[a[1]]['effects'])==bool(a[3]):self.pc+=a[0]
   elif n=='EndIfConditionGroupStateUncompiled':
    if self.group(a[2])==bool(a[1]):assert a[0]==1;self.restart();continue
   elif n=='EndUnconditionally':assert a==[1];self.restart();continue
   elif n in ['WaitFixedTimeSeconds','WaitFixedTimeFrames']:
    seconds=a[0] if n=='WaitFixedTimeSeconds' else a[0]/60
    if self.deadline is None:self.deadline=t+seconds
    if t+1e-8<self.deadline:return
    self.deadline=None
   elif n in ['SetEventFlag','SetNetworkconnectedEventFlag']:
    if n=='SetEventFlag':assert self.local;self.flags[a[1]]=bool(a[2])
    else:assert not self.local;self.w.flags[a[1]]=bool(a[2])
    self.outputs.append((t,n,a))
   elif n=='SetCharacterBackreadState':
    if not self.local:self.w.actors[a[0]]['loaded']=not bool(a[1])
    self.outputs.append((t,n,a))
   elif n=='InvokeEnemyGenerator':
    id=a[0]-3800+9000
    # Native MaxNum=1: preparing a loaded live actor does not duplicate it.
    if not self.local or self.w.actors[id]['hp']<=0:
     self.w.actors[id].update(loaded=False,hp=0);self.w.pending.append(id)
    self.outputs.append((t,n,a))
   elif n=='SetSpEffect':
    for id in self.w.members(a[0]):self.w.actors[id]['effects'].add(a[1])
    self.outputs.append((t,n,a))
   elif n=='SetNetworkUpdateAuthority':
    assert not self.w.client and a[1]==8192
    self.outputs.append((t,n,a))
   elif n in ['ActivateMultiplayerdependantBuffs','SetNetworkSyncState','SetCharacterAIState','SetCharacterInvincibility','SetCharacterImmortality','SetNetworkUpdateRate','ChangeCharacterCollisionState','ChangeCharacterEnableState','SpawnOneshotSFX','ForceAnimationPlayback','DisplayBossHealthBar','LinkToBossHealthBar','SetAreaWelcomeMessageState','SetFieldBattleBGMHeatUp','PlaySE','DisplayTextEffectId','IncrementTeamBossesKilled','IncrementNetworkconnectedEventValue','SetWorldMapPointFlag']:
    self.outputs.append((t,n,a))
   elif n.startswith('Label'):pass
   else:raise AssertionError((n,a))
   self.pc+=1
  raise AssertionError(('Event failed to yield',self.code[self.pc]))
 def calls(self,n):return [(t,a) for t,k,a in self.outputs if k==n]

def start_battle(code,w):
 """Run current peer generation or the historical host generation path."""
 births=[event(r,w.p+9900+m['index']) for m in w.g['members']]
 peer_generation=any(n=='InvokeEnemyGenerator' for n,_ in births[0])
 if peer_generation:
  for actor in w.actors.values():actor.update(loaded=False,hp=0)
 battle=VM(code,w);battle.step()
 if peer_generation:
  for birth in births:VM(birth,w,True).step()
 return battle

assert built['encounterCount']==50 and built['actorCount']==84
assert len(built['parameters'])==47 and all(built[k] for k in ['nativeTeamFightRewards','ridingRelationsRetained','allGroupMembersRetainNativeStats'])
assert 'NPC_COUNT:i32=84' in (root/'src/cage_mutation.rs').read_text()
cases=0
for r in built['reports']:
 p=prefix(r);assert digest(stage/'event'/f"{r['mapName']}.emevd.dcx")==r['eventSha256'];assert digest(stage/'map/mapstudio'/f"{r['mapName']}.msb.dcx")==r['mapSha256']
 state=dict(mapName=r["mapName"],request=p+7402,ready=p+7403,choices=p+7500,cooling=p+7550,encounterCount=50)
 verify_selector([decode(i) for i in r['selector']['instructions']],state,decode)
 assert len(r['events'])==114 and len(r['actorReports'])==47 and len(r['rewardReports'])==13
 for e in r['events']:
  code=[decode(i) for i in e['instructions']]
  for at,(n,a) in enumerate(code):
   if n.startswith('Goto'):assert any(k==f'Label{a[0]}' for k,_ in code[at+1:]),'Native goto only searches forward'
   if n.startswith('If') and n!='IfPlayAreaCurrentTimeInRange':assert -15<=a[0]<=15
 for g in catalog['groups']:
  combat=[m for m in g['members'] if m['combat']];battle=event(r,p+9710+g['choice']);hud=event(r,p+9800+g['choice']);reward=next(t for t in r['rewardReports'] if t['choice']==g['choice'])['copy']
  assert reward['TargetEntityGroupID']==p+5900+g['choice'] and reward['LeaderEntityID']==p+7850+g['choice']
  assert reward['SecondItemLot']==g['reward']['SecondItemLot']
  if g['map'].startswith(('m48_','m49_')):assert reward['FirstItemLot']==g['reward']['Unk18'] and reward['UnkItemLot']==reward['FirstItemLot'] and reward['Unk14']==reward['Unk1C']==0
  else:assert reward['FirstItemLot']==g['reward']['FirstItemLot'] and reward['UnkItemLot']==g['reward']['UnkItemLot']
  peer_generation=any(n=='InvokeEnemyGenerator' for n,_ in event(r,p+9900+g['members'][0]['index']))
  assert sum(n=='InvokeEnemyGenerator' for n,_ in battle)==(0 if peer_generation else len(g['members']))
  assert ('IfCharacterRatioHPValue',[0,p+5900+g['choice'],5,0,0,1]) in battle
  assert sum(n=='IncrementNetworkconnectedEventValue' for n,_ in battle)==1
  assert sum(n=='SetNetworkconnectedEventFlag' and a[1]==reward['LeaderEntityID'] for n,a in battle)==1
  client_world=World(p,g);client_world.client=True;before=dict(client_world.flags);client=VM(battle,client_world);client.step()
  assert client.pc==len(battle) and not client.calls('InvokeEnemyGenerator') and not client.calls('IncrementNetworkconnectedEventValue')
  assert client_world.flags==before and not client.calls('SetNetworkconnectedEventFlag')
  for m in combat:
   death_code=event(r,p+9100+m['index']);death_client=VM(death_code,client_world);death_client.step()
   assert death_client.pc==len(death_code) and not death_client.calls('SetNetworkconnectedEventFlag')
  # One host decision colors the entire encounter, including ridden bodies.
  for deep,draw,expected in [(False,0,False),(True,1,False),(True,0,True)]:
   ww=World(p,g);ww.draw=draw;ww.flags[7600]=True;bb=start_battle(battle,ww)
   for id in ww.pending:ww.actors[id].update(loaded=True,hp=100)
   if deep:
    effect=next(a[2] for n,a in battle if n=='IfCharacterHasSpEffect');ww.actors[entity(p,combat[0])]['effects'].add(effect)
   bb.step(0.1);bb.step(0.2)
   assert ww.flags[p+7700+g['choice']] and ww.flags[p+7650+g['choice']]==expected
   assert all((95001602 in s['effects'])==expected for s in ww.actors.values())
   cases+=1
  # Model delays cannot rearm the native reward flag while old dead actors remain.
  w=World(p,g);w.flags[reward['LeaderEntityID']]=True;b=start_battle(battle,w);assert w.pending and w.flags[reward['LeaderEntityID']]
  for id in w.pending[:-1]:w.actors[id].update(loaded=True,hp=100)
  b.step(1);assert w.flags[reward['LeaderEntityID']]
  w.actors[w.pending[-1]].update(loaded=True,hp=100);b.step(2);assert not w.flags[reward['LeaderEntityID']]
  b.step(2.1);b.step(6.1);assert w.flags[p+7404]
  death_vms={entity(p,m):VM(event(r,p+9100+m['index']),w) for m in combat}
  for vm in death_vms.values():vm.step(6.1)
  # Check every first/last victim pair, leaving support mounts alive.
  for first,last in itertools.permutations(combat,2):
   ww=World(p,g);bb=start_battle(battle,ww)
   for id in ww.pending:ww.actors[id].update(loaded=True,hp=100)
   bb.step(0.1);bb.step(0.2);bb.step(4.2)
   order=[first]+[m for m in combat if m not in [first,last]]+[last]
   for i,m in enumerate(order):
    ww.actors[entity(p,m)]['hp']=0;bb.step(5+i)
    assert not bb.calls('DisplayTextEffectId')
    if i<len(order)-1:assert ww.flags[p+7403] and ww.flags[p+7500+g['choice']]
   final=5+len(order)-1;bb.step(final+4.99);assert not bb.calls('DisplayTextEffectId');bb.step(final+5)
   assert bb.calls('DisplayTextEffectId')==[(final+5,[1030])]
   assert not ww.flags[p+7401] and ww.flags[p+7550+g['choice']]
   # Another encounter can start before this group's delayed cleanup ends.
   newer=p+7500+(g['choice']+1)%50;ww.flags.update({p+7401:True,p+7402:True,p+7403:True,p+7404:True,newer:True})
   bb.step(final+20)
   assert all(ww.flags[id] for id in [p+7401,p+7402,p+7403,p+7404,newer]) and not ww.flags[p+7550+g['choice']]
   assert all(not ww.actors[id]['loaded'] for id in ww.actors)
   cases+=1
  # Each peer reveals locally. Shared spawned does not bypass a missing live actor.
  for lag in [0,1,9]:
   ww=World(p,g);ww.flags[p+7404]=True;peer=VM(hud,ww,True)
   for m in combat:peer.flags[shown(p,m)]=True
   delayed=combat[-1];peer.flags[shown(p,delayed)]=False;peer.step();assert not peer.calls('DisplayBossHealthBar')
   peer.flags[shown(p,delayed)]=True;peer.step(lag)
   bars=[a for _,a in peer.calls('DisplayBossHealthBar') if a[0]==1];assert len(bars)==len(g['bars']) and {a[2] for a in bars}==set(range(len(g['bars'])))
   for i,m in enumerate(combat):ww.actors[entity(p,m)]['hp']=0;ww.flags[dead(p,m)]=True;peer.step(lag+1+i)
   end=lag+len(combat);peer.step(end+5)
   assert not peer.flag(5000) and not peer.flag(5001)
   cases+=1
  # A dead, unloaded member on rejoin cannot block surviving members' bars.
  ww=World(p,g);ww.flags[p+7404]=True;peer=VM(hud,ww,True)
  for m in combat:peer.flags[shown(p,m)]=True
  victim=combat[0];ww.actors[entity(p,victim)].update(hp=0,loaded=False);ww.flags[dead(p,victim)]=True;peer.flags[shown(p,victim)]=False;peer.step()
  bars=[a for _,a in peer.calls('DisplayBossHealthBar') if a[0]];assert len(bars)==sum(b['member']<0 or g['members'][b['member']]!=victim for b in g['bars'])
  # Shared groups use whichever native slot is free and release only their own.
  if len(g['bars'])==1:
   for occupied,slot in [(5000,1),(5001,0)]:
    one=VM(hud,ww,True);one.flags=peer.flags|{5000:False,5001:False,occupied:True};one.step()
    assert one.calls('DisplayBossHealthBar')[0][1][2]==slot
    one.distance=101;one.step(1);assert one.flag(occupied) and not one.flag(5000+slot)
    cases+=1
  # Cancellation while slots are busy must preserve their current owners.
  blocked=VM(hud,ww,True);blocked.flags=peer.flags|{5000:True,5001:True};blocked.step();assert not blocked.calls('DisplayBossHealthBar')
  ww.flags[p+7500+g['choice']]=False;blocked.step(1);blocked.step(1.1);assert blocked.flag(5000) and blocked.flag(5001) and not blocked.calls('DisplayBossHealthBar')
  # Births produce one local fog, skip replay when joining an active battle.
  for m in g['members']:
   code=event(r,p+9900+m['index']);fresh=World(p,g);birth=VM(code,fresh,True);birth.step();assert len(birth.calls('SpawnOneshotSFX'))==1;birth.step(4);assert birth.flag(shown(p,m))
   active=World(p,g);active.flags[p+7404]=True;birth=VM(code,active,True);birth.step();assert not birth.calls('SpawnOneshotSFX') and birth.flag(shown(p,m))
   # This peer observed the selection before the host's four-second intro,
   # but its model loads nine seconds late: its local fog must still play.
   slow=World(p,g);slow.actors[entity(p,m)]['loaded']=False;birth=VM(code,slow,True);birth.step()
   slow.flags[p+7404]=True;slow.actors[entity(p,m)]['loaded']=True;birth.step(9)
   assert birth.calls('SpawnOneshotSFX') and not birth.flag(shown(p,m))
   birth.step(13);assert birth.flag(shown(p,m))
   assert len(birth.calls('ActivateMultiplayerdependantBuffs'))==1
  cases+=2

# Keep the previous 37-actor interaction/birth/HUD/red regression suite active.
single=read(root/'.work/cage/built.json')
for r in single['reports']:
 group=next(g for g in built['reports'] if g['mapName']==r['mapName']);r.update(events=group['singleEvents'],encounterCount=50,outputSha256=group['eventSha256'],mapSha256=group['mapSha256'])
single_path=validation_root/'single-built.json';single_path.write_text(json.dumps(single,ensure_ascii=False,indent=2),encoding='utf8')
env=os.environ|{'NRHEX_CAGE_REPORT':str(single_path),'NRHEX_CAGE_MOD_ROOT':str(stage),'NRHEX_VALIDATION_ROOT':str(validation_root)}
subprocess.run([sys.executable,'-X','utf8',str(root/'tools/verify_cage_mutation.py')],env=env,check=True)
report=dict(version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version'],encounters=50,singleEncounters=37,groupEncounters=13,actors=84,combatMembers=41,supportMounts=6,
 eventCases=cases,opcodeAbiDecoded=True,forwardGotosOnly=True,allMembersDeadBeforeVictory=True,atomicHudReservation=True,deadMemberRejoinPassed=True,delayedLoadingPassed=True,
 cleanupPreservesNextEncounter=True,rewardClaimRearmedOnlyAfterAllNewActorsAlive=True,nativeGroupRewardsCopied=True,towerWeaponOverridesCopied=True,nativeNpcStatsAndScalingPreserved=True,
 gameplayVerified=False,multiplayerVerified=False,pendingChecks=['All new combinations spawn and move correctly','Repeated group reward drops, including day/map reload','Mounted enemies dismount and abilities','Host/client late loading and HUD','Deep/night/co-op final HP'])
(validation_root/'validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf8')
if stage.resolve()==(root/'dist/mod').resolve():
 for src,dest in [('validation.json','cage-groups-validation.json'),('cage-validation.json','cage-validation.json'),('cage-mutation-validation.json','cage-mutation-validation.json')]:
  (root/'dist'/dest).write_text((validation_root/src).read_text(encoding='utf8'),encoding='utf8')
print(f'PASS: {cases} group event cases; 13 groups, native reward flags, whole-group HUD reservation and cleanup; gameplay pending')
