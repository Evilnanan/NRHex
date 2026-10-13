"""Check the reopened cage event assets, local presentation and private quotas."""
import hashlib,json,struct,tomllib
from cage_usage_events import ROOT,decode,verify_open,opening
from cage_local_events import Peer

work=ROOT/'.work/cage-01795'
data=json.loads((work/'events.json').read_text(encoding='utf8'))
opener=next(e for e in data['reports'][0]['events'] if e['ID']==9100961)
prelude=not data.get('preludeRemoved',False)
duration=6.5 if prelude else 4
cases=0;birth_cases=0;initializers=0
for report in data['reports']:
    target=work/'staged/mod/event'/f"{report['name']}.emevd.dcx"
    assert hashlib.sha256(target.read_bytes()).hexdigest().upper()==report['outputSha256']
    if report['name']=='common_func':continue
    p=(4600+int(report['name'][4:6]))*10000
    ctor=next(e for e in report['events'] if e['ID']==0)
    parameters=None
    for i in ctor['instructions']:
        raw=bytes.fromhex(i['hex'])
        if (i['Bank'],i['ID'])==(2000,6) and struct.unpack_from('<I',raw,4)[0]==9100961:
            assert len(raw)==40 and struct.unpack_from('<I',raw,36)[0]==p+5700
            parameters=raw[8:];cases+=len(verify_open(opener,parameters));initializers+=1
    assert parameters
    # Separate local storage for each player and physical gaol. Restarting an
    # event reuses this storage; only a successful custom opening increments it.
    counters={}
    for player,gaol in [(1,0)]*5+[(2,0)]*5+[(1,1)]*5:
        count=counters.get((player,gaol),0)
        r=opening(opener,parameters,count=count,player=player)
        assert r['request'];counters[player,gaol]=r['count']
    assert counters=={(1,0):5,(2,0):5,(1,1):5}
    for (player,gaol),count in counters.items():
        r=opening(opener,parameters,count=count,player=player,keys=1)
        assert not r['request'] and r['removed']==1 and r['count']==5
    for e in report['events']:
        if p+9710<=e['ID']<p+9760:
            waits=[struct.unpack('<f',bytes.fromhex(i['hex']))[0] for i in e['instructions'] if (i['Bank'],i['ID'])==(1001,0)]
            assert not any(i['Bank']==2010 and i['ID']==2 and struct.unpack_from('<i',bytes.fromhex(i['hex']),8)[0]==999997300 for i in e['instructions'])
            assert duration in waits and (4 if prelude else 6.5) not in waits
        if not p+9900<=e['ID']<p+9984:continue
        code=[decode(i) for i in e['instructions']]
        entity=p+9000+e['ID']-(p+9900)
        effects=[a for n,a in code if n=='SpawnOneshotSFX']
        assert effects==([[2,entity,900,690050],[2,entity,900,690047]] if prelude else [[2,entity,900,690047]])
        # Extract the actual selection flag (group members share a choice).
        choice=next(a[3] for n,a in code if n=='IfEventFlag' and p+7500<=a[3]<p+7550)
        for delay in (0,0.5,9):
            peer=Peer(code,{p+7403:True,choice:True},entity,loaded=delay==0)
            peer.step()
            if delay:
                assert not peer.calls('SpawnOneshotSFX')
                peer.flags[p+7404]=True;peer.state['loaded']=True;peer.step(delay)
            assert peer.calls('SpawnOneshotSFX')==[(delay,effects[0])]
            if prelude:peer.step(delay+2.5)
            if prelude:assert peer.calls('SpawnOneshotSFX')==[(delay,effects[0]),(delay+2.5,effects[1])]
            peer.step(delay+duration-.01)
            assert peer.calls('ChangeCharacterEnableState')[-1][1]==[entity,0]
            peer.step(delay+duration)
            assert peer.calls('ChangeCharacterEnableState')[-1]==(delay+duration,[entity,1])
            birth_cases+=1
        peer=Peer(code,{p+7403:True,choice:True,p+7404:True},entity)
        peer.step();assert not peer.calls('SpawnOneshotSFX')
        assert peer.calls('ChangeCharacterEnableState')[-1][1]==[entity,1]
        birth_cases+=1

report=dict(version=tomllib.loads((ROOT/'Cargo.toml').read_text(encoding='utf8'))['package']['version'],
 personalLimit=5,counterCategory=5,openingBranchCases=cases,openingInitializers=initializers,
 localBirthCases=birth_cases,bossIntroEffects=[690050,690047] if prelude else [690047],introSeconds=duration,
 originalOtherEventsPreserved=True,gameplayVerified=False,multiplayerVerified=False,
 pendingChecks=['Each player and each physical gaol retains five independent opens',
 'Sixth interaction uses native keys/conditions; move away and return',
 'Native opening popup and main fog on both peers; no prelude'])
(work/'validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f'PASS: {cases} opening branches, {birth_cases} local birth cases, private player/gaol quota simulation; real coop pending')
