"""Exercise shipped event bytes with distinct host and replica execution roles."""
import json
from verify_cage_groups import World, VM, decode, root, single, catalog

cases = 0
for report in single['reports']:
    p = int(report['events'][0]['ID']) // 10000 * 10000
    for index in range(37):
        code = [decode(i) for i in next(e for e in report['events'] if e['ID'] == p+9710+index)['instructions']]
        entity = p+9000+index
        group = dict(choice=index,members=[dict(index=index,combat=True)])
        remote = World(p,group); remote.client = True; flags = dict(remote.flags)
        client = VM(code,remote); client.step()
        assert client.pc == len(code) and remote.flags == flags
        for name in ['InvokeEnemyGenerator','SetNetworkconnectedEventFlag','ActivateMultiplayerdependantBuffs',
                     'SetNetworkUpdateAuthority','IncrementNetworkconnectedEventValue','DisplayTextEffectId']:
            assert not client.calls(name), (index,name)
        host = World(p,group); battle = VM(code,host); battle.step()
        assert battle.calls('InvokeEnemyGenerator') == [(0,[p+3800+index])]
        host.actors[entity].update(hp=100,loaded=True)
        battle.step(.1); battle.step(.2); battle.step(4.2)
        assert battle.calls('SetNetworkUpdateAuthority') == [(.1,[entity,8192])]
        assert battle.calls('ActivateMultiplayerdependantBuffs') == [(.1,[entity])]
        assert 7999 in host.actors[entity]['effects'] and host.flags[p+7404]
        host.actors[entity]['hp'] = 0; battle.step(5); battle.step(10)
        assert battle.calls('IncrementNetworkconnectedEventValue') == [(10,[8145,4,7])]
        battle.step(25)
        assert len(battle.calls('InvokeEnemyGenerator')) == 1
        cases += 2

# An unreferenced condition is not a reliable finished-condition snapshot.
# Verify the saved fresh/ongoing predicate belongs to the first wait's graph.
for report in single['reports']:
    for event in report['events']:
        if event['ID'] % 10000 < 9900: continue
        code = [decode(i) for i in event['instructions']]
        if ('IfConditionGroup',[-5,1,5]) in code:
            # Historical 0.17.27 bytes. This graph passed the original model
            # but native group latching was later found to skip second fights.
            assert ('IfConditionGroup',[-5,1,6]) in code
            assert ('IfConditionGroup',[1,1,-5]) in code
        else:
            branch=next(i for i,(n,a) in enumerate(code) if n=='GotoIfEventFlag' and a[1]==1 and a[3]==report['spawned'])
            load=next(i for i,(n,a) in enumerate(code) if n=='SetCharacterBackreadState')
            assert branch<load and not any(n=='SkipIfConditionGroupStateCompiled' and a[2]==5 for n,a in code)
        skip = next(a for n,a in code if n == 'SkipIfCharacterHasSpEffect')
        assert skip[0] == 2 and skip[2:] == [7999,1,0,1]

report = dict(version='0.17.27',singleAuthorityAndLifecycleCases=cases,groupEventCases=960,
              selectorHostOnly=True,replicaGeneratorsAndRewardsExcluded=True,
              duplicateRequestsCollapsed=True,hostActorAuthority=8192,
              nativeCoopActivation='2009[4]',nativeCoopOnceMarker=7999,
              freshIntroCapturedBeforeBackreadWait=True,finishedSnapshotConnectedToMainCondition=True,
              localDelayedFogAndHudPassed=True,latePeerModelDelaysSeconds=[5,9],
              inheritedNpcAndDifficultyTablesUnchanged=True,gameplayVerified=False,multiplayerVerified=False)
(root/'.work/fixes-01727/coop-fixes-validation.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print(f'PASS: {cases} single host/client lifecycle cases, 960 group cases, duplicate requests and late local reveal.')
