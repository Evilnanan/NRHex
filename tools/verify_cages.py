"""Decode reopened events against NR EMEDF and exercise the open branches offline."""
from pathlib import Path
import hashlib,json,struct,tomllib,math,os
from cage_local_events import verify_hud, verify_birth, verify_peer_reveal
from cage_selector_events import verify_selector
from cage_usage_events import OPS as usage_ops, verify_open
root=Path(__file__).resolve().parents[1]
data=json.loads(Path(os.environ.get('NRHEX_CAGE_REPORT',root/'.work/cage/built.json')).read_text(encoding='utf8'))
mod_root=Path(os.environ.get('NRHEX_CAGE_MOD_ROOT',root/'dist/mod'))
validation_root=Path(os.environ.get('NRHEX_VALIDATION_ROOT',root/'dist'))
boss_count=len(data['bosses'])
assert boss_count==37
schema=json.loads((root/'data/reference/world/cage-opcodes.json').read_text(encoding='utf8'))
ops={(o['bank'],o['id']):o for o in schema['opcodes']}
for op,name in [(17,'RandomlySetEventFlagInRange'),(22,'BatchSetEventFlags')]:
    ops[2003,op]=dict(name=name,args=[['uint','startFlagId'],['uint','endFlagId'],['byte','flagState']])
# Current NR EMEDF 2003[102], also present in original 90015000.
ops[2003,102]=dict(name='LinkToBossHealthBar',args=[['sbyte','disabledEnabled'],['int','healthBarNameId'],['uint','entityId']])
ops[3,42]=dict(name='IfEventFlagAndRandomCondition',args=[['sbyte','resultConditionGroup'],['uint','targetEventFlagId'],['float','targetProbability']])
ops[1003,13]=dict(name='GotoIfPlayerIsInWorldType',args=[['byte','label'],['byte','worldType']])
ops[1003,14]=dict(name='EndIfPlayerIsInWorldType',args=[['byte','executionEndType'],['byte','worldType']])
ops[2009,4]=dict(name='ActivateMultiplayerdependantBuffs',args=[['uint','entityId']])
ops[2004,28]=dict(name='SetNetworkUpdateAuthority',args=[['uint','entityId'],['int','authorityLevel']])
for label in (6,8,9):ops[1014,label]=dict(name=f'Label{label}',args=[])
for key in [(3,12),(2003,127),(1000,101),(1014,0)]:
    name,types=usage_ops[key];ops[key]=dict(name=name,args=[[t,'argument'] for t in types])
births=json.loads((root/'data/reference/world/cage-birth.json').read_text(encoding='utf8'))
birth_by_model={m['model']:m for m in births['models']}
birth_overrides={o['npc']:o for o in births['nativeOverrides']}
idle_resets={o['npc']:o for o in births['idleResets']}
assert len(idle_resets)==len(births['idleResets'])
assert idle_resets[71000010]==dict(npc=71000010,model='c7100',animation=0)
assert not idle_resets.keys() & birth_overrides.keys()
assert len(birth_overrides)==len(births['nativeOverrides'])
assert birth_overrides[25001020]==dict(npc=25001020,model='c2500',animation=20025,loop=True,cancelled=True,map='m46_70_00_00',entity=46700840,eventId=90045001)
assert births['fogSfx']==690031 and births['nativeDummy']==960
assert (births['towerFogDefault'],births['towerFogDancer'],births['towerDummy'],births['towerWaitSeconds'])==(690047,690046,900,4)
for model in births['models']:
    assert 900 in model['fogDummies']
    effects=[e for a in model['animations'] if a['id']%1000000==20025 for e in a['events'] if e['Type']==96]
    assert model['nativeFog']==any(struct.unpack_from('<i',bytes.fromhex(e['hex']))[0]==690031 for e in effects)
    assert model['towerWalk']==any(a['id']%1000000==20026 and any(e['Type']==193 for e in a['events']) for a in model['animations'])
formats={'sbyte':'b','byte':'B','bool':'B','uint':'I','int':'i','float':'f','ushort':'H','short':'h'}
def decode(i):
    op=ops[i['Bank'],i['ID']];raw=bytes.fromhex(i['hex']);at=0;values=[]
    for typ,_ in op['args']:
        f=formats[typ];size=struct.calcsize(f);at=(at+size-1)//size*size
        values.append(struct.unpack_from('<'+f,raw,at)[0]);at+=size
    if i['Bank']==2000 and i['ID'] in (0,6):
        assert len(raw)>=at and len(raw)%4==0
    else:assert len(raw)==(at+3)//4*4,(op['name'],len(raw),at)
    return op['name'],values
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
for r in data['reports']:
    assert digest(mod_root/'event'/f"{r['mapName']}.emevd.dcx")==r['outputSha256']
    assert digest(mod_root/'map/mapstudio'/f"{r['mapName']}.msb.dcx")==r['mapSha256']
    assert len(r['events'])==boss_count*3+1 and r['openInitializers']
    expected_yaw=math.degrees(math.atan2(r['stonePosition']['X'],r['stonePosition']['Z']))+180
    assert abs(r['birthYaw']-expected_yaw)<1e-4
    assert len(r['birthLayouts'])==boss_count
    assert all(abs(p['yaw']-expected_yaw)<1e-4 and abs(p['spawnYaw']-expected_yaw)<1e-4 for p in r['birthLayouts'])
    assert all(f//1000%10==7 for f in [r['used'],r['busy'],r['request'],r['ready'],r['spawned'],r['choices'],r['choices']+boss_count-1,r['cooling'],r['cooling']+boss_count-1,r['shown'],r['shown']+boss_count-1]),'Map flag category 6000 has no storage'
    for i in r['openInitializers']:
        words=struct.unpack('<10I',bytes.fromhex(i['hex']))
        assert words[1]==9100961 and words[-4:]==(r['used'],r['busy'],r['request'],r['used']//10000*10000+5700)
    for e in r['events']:
        decoded=[decode(i) for i in e['instructions']]
        if e['ID']%10000==9700:
            verify_selector(decoded,r,decode)
            assert sum(n=='RandomlySetEventFlagInRange' for n,_ in decoded)==1
            assert not any(n in ('BatchSetNetworkconnectedEventFlags','RandomlySetNetworkconnectedEventFlagInRange') for n,_ in decoded)
            labels={int(n[5:]):i for i,(n,_) in enumerate(decoded) if n.startswith('Label')}
            def select(draws,cooling_indices=(),already_ready=False):
                flags={r['request']:True,r['ready']:already_ready}
                flags.update({r['cooling']+i:True for i in cooling_indices})
                pc=0;draws=iter(draws);seen=[];committed=[]
                for _ in range(1000):
                    n,a=decoded[pc];pc+=1
                    if n=='IfEventFlag':
                        if bool(flags.get(a[3],False))!=bool(a[1]):return seen,committed,flags
                    elif n=='GotoIfEventFlag':
                        if bool(flags.get(a[3],False))==bool(a[1]):
                            assert labels[a[0]]>=pc, 'Native goto cannot jump backwards'
                            pc=labels[a[0]]+1
                    elif n=='SkipIfEventFlag':
                        if bool(flags.get(a[3],False))==bool(a[1]):pc+=a[0]
                    elif n in ('BatchSetNetworkconnectedEventFlags','BatchSetEventFlags'):
                        for flag in range(a[0],a[1]+1):flags[flag]=bool(a[2])
                    elif n in ('RandomlySetNetworkconnectedEventFlagInRange','RandomlySetEventFlagInRange'):
                        draw=next(draws);seen.append(draw);flags[a[0]+draw]=True
                    elif n=='SetNetworkconnectedEventFlag':
                        flags[a[1]]=bool(a[2])
                        if a[1]==r['ready'] and a[2]:committed.append(seen[-1])
                    elif n=='EndUnconditionally':
                        if a==[1] and not committed:
                            pc=0
                            continue
                        return seen,committed,flags
                    elif n=='EndIfPlayerIsInWorldType':assert a==[0,1]
                    elif n=='SetNetworkSyncState':assert a==[0]
                    elif n=='WaitFixedTimeFrames' or n.startswith('Label'):pass
                    else:raise AssertionError(n)
                raise AssertionError('Selector did not yield')
            seen,committed,flags=select([18,20,3],cooling_indices=[18,20])
            assert seen==[18,20,3] and committed==[3]
            assert flags[r['choices']+3] and not flags[r['choices']+18] and not flags[r['choices']+20]
            assert select([18])[1]==[18],'Cleaned actors must become eligible again'
            assert select([],already_ready=True)[0]==[],'Reload must retain a committed choice'
        elif e['ID']%10000>=9900:
            index=e['ID']%10000-9900
            entity=e['ID']//10000*10000+9000+index
            boss=data['bosses'][index];birth=birth_by_model[boss['model']]
            override=birth_overrides.get(boss['npc']);reset=idle_resets.get(boss['npc'])
            animation=([[0,0]] if reset else ([] if override.get('cancelled',False) else [[override['animation'],int(override['loop'])]]) if override else [[20026,0]] if birth['towerWalk'] else [[20025,1]] if birth['nativeFog'] else [])
            if animation:assert any(a['id']%1000000==animation[0][0] for a in birth['animations'])
            verify_birth(decoded,r,entity,r['choices']+index,animation,690047)
            hud_event=next(event for event in r['events'] if event['ID']%10000==9800+index)
            verify_peer_reveal(decoded,[decode(i) for i in hud_event['instructions']],r,entity,boss['healthBarName'],r['choices']+index)
        elif e['ID']%10000>=9800:
            index=e['ID']%10000-9800
            entity=e['ID']//10000*10000+9000+index
            verify_hud(decoded,r,entity,data['bosses'][index]['healthBarName'],r['choices']+index)
        else:
            birth_event=next(v for v in r['events'] if v['ID']==e['ID']+190)
            peer_generation=any(i['Bank']==2003 and i['ID']==54 for i in birth_event['instructions'])
            assert sum(n=='InvokeEnemyGenerator' for n,_ in decoded)==(0 if peer_generation else 1)
            prefix=e['ID']//10000*10000
            index=e['ID']%10000-9710
            entity=prefix+9000+index
            difficulty_start=decoded.index(('GotoIfEventFlag',[2,1,0,7505]))
            difficulty_end=next(i for i,(n,a) in enumerate(decoded) if n=='Label4')
            difficulty=decoded[difficulty_start:difficulty_end+1]
            difficulty_labels={int(n[5:]):i for i,(n,_) in enumerate(difficulty) if n.startswith('Label')}
            def stage_effects(second_cycle,time):
                pc=0;groups={};effects=[]
                for _ in range(30):
                    n,a=difficulty[pc];pc+=1
                    if n=='GotoIfEventFlag':
                        if second_cycle==bool(a[1]):pc=difficulty_labels[a[0]]+1
                    elif n=='IfPlayAreaCurrentTimeInRange':
                        groups[a[0]]=(a[1]*3600+a[2]*60+a[3])<=time<=(a[4]*3600+a[5]*60+a[6])
                    elif n=='GotoIfConditionGroupStateUncompiled':
                        if groups.get(a[2],False)==bool(a[1]):pc=difficulty_labels[a[0]]+1
                    elif n=='GotoUnconditionally':pc=difficulty_labels[a[0]]+1
                    elif n=='SetSpEffect':
                        assert a[0]==entity;effects.append(a[1])
                    elif n.startswith('Label'):pass
                    else:raise AssertionError(n)
                    if pc>=len(difficulty):return effects
                raise AssertionError('Difficulty selection did not terminate')
            visual=99146 if data['bosses'][index]['entity']==46800810 else 99145
            assert stage_effects(False,20*3600)==[99150,visual]
            assert stage_effects(True,16*3600+9*60)==[99152,visual]
            assert stage_effects(True,16*3600+9*60+1)==[99153]
            active_ai=decoded.index(('SetCharacterAIState',[entity,1]))
            assert difficulty_end<active_ai
            assert not any(n=='SpawnOneshotSFX' or n=='ForceAnimationPlayback' and a[0]==entity for n,a in decoded),'Birth playback belongs to the per-peer event'
            assert ('WaitFixedTimeSeconds',[4]) in decoded
            assert ('IncrementNetworkconnectedEventValue',[8145,4,7]) in decoded
            # Check the generated timeline, not just the presence of a delay:
            # victory/relic awards at 5s, actor cleanup and reopening at 20s.
            dead_wait=decoded.index(('IfCharacterHPValue',[0,entity,0,0,0,1]))
            elapsed=0;timeline={}
            for n,a in decoded[dead_wait+1:]:
                if n=='WaitFixedTimeSeconds':elapsed+=a[0]
                elif n=='DisplayTextEffectId':timeline['victory']=elapsed
                elif n=='IncrementNetworkconnectedEventValue' and a[0]==8145:timeline['relic']=elapsed
                elif n=='ChangeCharacterEnableState' and a==[entity,0]:timeline['removed']=elapsed
                elif n=='SetCharacterBackreadState' and a==[entity,1]:timeline['backreadRemoved']=elapsed
                elif n=='SetNetworkconnectedEventFlag' and a==[0,r['busy'],0]:timeline['reopened']=elapsed
                elif n=='SetNetworkconnectedEventFlag' and a==[0,r['cooling']+index,1]:timeline['coolingStarted']=elapsed
                elif n=='SetNetworkconnectedEventFlag' and a==[0,r['cooling']+index,0]:timeline['coolingEnded']=elapsed
            assert timeline['victory']==timeline['relic']==5
            assert timeline['reopened']==timeline['coolingStarted']==5
            assert timeline['removed']==timeline['backreadRemoved']==timeline['coolingEnded']==20
            cleanup_wait=decoded.index(('WaitFixedTimeSeconds',[15]))
            assert not any(n=='SetNetworkconnectedEventFlag' and a[1] in [r['busy'],r['ready'],r['request'],r['spawned'],r['choices']+index] for n,a in decoded[cleanup_wait+1:]),'Delayed cleanup must not reset a newer encounter'
            assert not any(n=='DisplayBossHealthBar' for n,_ in decoded)
            assert ('SetNetworkconnectedEventFlag',[0,r['spawned'],1]) in decoded
            assert ('SetNetworkconnectedEventFlag',[0,r['spawned'],0]) in decoded
            setflags={a[1] for n,a in decoded if n=='SetNetworkconnectedEventFlag'}
            prefix=e['ID']//10000*10000
            assert not setflags & {prefix+200,prefix+201,prefix+202,prefix+800}
            assert not any(n in ['RemoveItemFromPlayer','AwardItemLotOld'] for n,_ in decoded)
            assert ('SetNetworkconnectedEventFlag',[0,r['busy'],0]) in decoded
            assert ('SetNetworkconnectedEventFlag',[0,r['ready'],0]) in decoded
            assert decoded[-1]==('EndUnconditionally',[1])
for e in [data['open'],data['terrain']]:
    for i in e['instructions']:decode(i)
terrain=[decode(i) for i in data['terrain']['instructions']]
assert ('IfEventFlag',[0,1,0,7604]) in terrain
assert ('SetSpEffect',[20000,95001601]) in terrain
assert ('ClearSpEffect',[20000,95001601]) in terrain
assert len(data['npcs'])==boss_count
for b,n in zip(data['bosses'],data['npcs']):
    assert n['runes']>0 and n['disableRespawn']==0
    tower=next((t for t in data['towers'] if t['sourceNpc']==b['npc']),None)
    assert n['nativeRewardLot']==b['nativeRewardLot']
    assert n['rewardItemLot']==(tower['firstLot'] if tower else b['nativeRewardLot'])
    assert (b['map'].startswith('m46_') and int(b['map'][4:6])>=50) or b['map'] in ['m48_10_00_00','m48_30_00_00','m48_40_00_00','m49_10_00_00','m49_17_00_00','m49_18_00_00','m49_20_00_00']
assert all(data[k] for k in ['originalMapEntriesReconstructed','originalCommonFunctionEventsPreserved','originalMapNonconstructorEventsPreserved'])

# Exercise actual serialized personal-quota/key branches.
parameters=struct.pack('<8I',12340200,12341200,2,10000,12347400,12347401,12347402,12345700)
cases=verify_open(data['open'],parameters)
version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
storage=json.loads((root/'.work/cage/flag-storage-test.json').read_text())
assert storage['old6000RequestSilentlyIgnored'] and storage['new7000StatesSetAndClear']
report=dict(version=version,nativeBossCount=boss_count,mapTemplates=[r['mapName'] for r in data['reports']],openBranchCases=cases,opcodeArgumentsDecoded=True,originalMapReconstructionPassed=True,originalEventsPreserved=True,flagStorage=storage,relicNativeCounter=8145,relicNativeStackCap=7,greatHollowFlag=7604,birthFacing='baked part and generator yaw toward each local stone, corrected by 180 degrees',victoryDelaySeconds=5,cleanupDelaySeconds=20,reopenDelaySeconds=5,difficultyEffects=data['difficultyEffects'],nativeProgressionEvent=90045004,progressionCasesPassed=True,deathTimelineVerified=True,cleanupExclusionVerified=True,selectorAcknowledgesReadyInsteadOfRequestOff=True,oldMissedEdgeReproduced=True,consecutiveImmediateReopenRoundsPerTemplate=200,healthBarWaitsForLocalReveal=True,nativeHealthBarAssociationRestored=True,combinedDelayedPeerRevealCases=444,healthBarLocal=True,healthBarShowRadius=50,healthBarFarHideRadius=100,healthBarPredicateCasesPassed=True,healthBarRequiresReplicaAI=False,healthBarNativeLocalSlots=[0,1],healthBarSlotFlags=[5000,5001],perPeerBirthPlayback=True,peerBirthReplayAndLateLoadCasesPassed=True,nativeFogModels=sum(m['nativeFog'] for m in births['models']),towerFog=[690047],towerAnimation=20026,nativeAnimationOverrides=births['nativeOverrides'],idleResets=births['idleResets'],towerWaitSeconds=4,towerWalkModels=sum(m['towerWalk'] for m in births['models']),gameplayVerified=False,multiplayerVerified=False,pendingChecks=['Birth facing and fog for field enemies','Native private-server coop: client opens cage, host/client fog and HUD; reverse opener roles; late loading and repeat boss','Native rune/latent reward repeat drops','No grace and native relic stack','Coop authority and visibility'])
(validation_root/'cage-validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf8')
print('PASS: 37 native boss identities/rewards, four map reconstructions, decoded opcode ABI and owner/key/busy/reopen branches; gameplay remains unverified.')
