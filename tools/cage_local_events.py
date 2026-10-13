"""Exercise decoded local events with independent peer state, without the game."""
from collections import defaultdict


class Peer:
    def __init__(self, decoded, flags, entity, **state):
        self.code = decoded
        self.flags = dict(flags)
        self.entity = entity
        self.state = dict(loaded=True, hp=100, player_alive=True, distance=10, ai=3)
        self.state.update(state)
        self.pc = 0
        self.groups = defaultdict(list)
        self.edges = defaultdict(set)
        self.compiled = {}
        self.outputs = []
        self.deadline = None
        self.time = 0
        self.restarts = 0
        self.effects = set()
        self.labels = {int(n[5:]): i for i, (n, _) in enumerate(decoded) if n.startswith('Label')}

    def group(self, g):
        predicates = self.groups.get(g, [])
        return bool(predicates) and (any(p() for p in predicates) if g < 0 else all(p() for p in predicates))

    def step(self, time=None):
        if time is not None:
            self.time = time
        for _ in range(500):
            n, a = self.code[self.pc]
            if n == 'IfConditionGroup':
                if a[0] == 0:
                    if self.group(a[2]) != bool(a[1]):
                        return
                    visited = set()
                    def snapshot(g):
                        if g in visited: return
                        visited.add(g)
                        for child in self.edges[g]: snapshot(child)
                    snapshot(a[2])
                    self.compiled.update({g: self.group(g) for g in visited})
                    self.groups.clear()
                    self.edges.clear()
                else:
                    self.edges[a[0]].add(a[2])
                    self.groups[a[0]].append(lambda a=a: self.group(a[2]) == bool(a[1]))
            elif n == 'IfEventFlag':
                if a[0] == 0:
                    if bool(self.flags.get(a[3], False)) != bool(a[1]): return
                    self.groups.clear();self.edges.clear();self.compiled.clear()
                    self.pc += 1
                    continue
                self.groups[a[0]].append(lambda a=a: bool(self.flags.get(a[3], False)) == bool(a[1]))
            elif n == 'IfCharacterBackreadStatus':
                self.groups[a[0]].append(lambda a=a: self.state['loaded'] == bool(a[2]))
            elif n == 'IfCharacterHPValue':
                assert a[2:] == [2, 0, 0, 1]
                self.groups[a[0]].append(lambda: self.state['hp'] > 0)
            elif n == 'IfCharacterDeadalive':
                self.groups[a[0]].append(lambda a=a: (self.state['hp'] > 0 if a[1] == self.entity else self.state['player_alive']) == (a[2] == 0))
            elif n == 'IfEntityInoutsideRadiusOfEntity':
                self.groups[a[0]].append(lambda a=a: (self.state['distance'] <= a[4]) == bool(a[1]))
            elif n == 'WaitFixedTimeSeconds' or n == 'WaitFixedTimeFrames':
                duration = a[0] if n == 'WaitFixedTimeSeconds' else a[0] / 60
                if self.deadline is None:
                    self.deadline = self.time + duration
                if self.time + 1e-9 < self.deadline:
                    return
                self.deadline = None
            elif n == 'GotoIfEventFlag':
                if bool(self.flags.get(a[3], False)) == bool(a[1]):
                    self.pc = self.labels[a[0]]
            elif n == 'GotoUnconditionally':
                self.pc = self.labels[a[0]]
            elif n == 'EndIfEventFlag':
                if bool(self.flags.get(a[3], False)) == bool(a[1]):
                    assert a[0] == 1
                    self.restart()
                    continue
            elif n == 'EndIfConditionGroupStateUncompiled':
                if self.group(a[2]) == bool(a[1]):
                    assert a[0] == 1
                    self.restart()
                    continue
            elif n == 'SkipIfConditionGroupStateCompiled':
                if self.compiled.get(a[2], False) == bool(a[1]):
                    self.pc += a[0]
            elif n == 'SkipUnconditionally':
                self.pc += a[0]
            elif n == 'SkipIfCharacterHasSpEffect':
                if (a[2] in self.effects) == bool(a[3]):
                    self.pc += a[0]
            elif n == 'SetSpEffect':
                self.effects.add(a[1])
                self.outputs.append((self.time, n, a))
            elif n == 'SetEventFlag':
                assert a[0] == 0 and (a[1] in (5000, 5001) or a[1] % 10000 in range(7600, 7637) or a[1] % 10000 in range(7750, 7834))
                self.flags[a[1]] = bool(a[2])
                self.outputs.append((self.time, n, a))
            elif n in ('SpawnOneshotSFX', 'ForceAnimationPlayback', 'ChangeCharacterEnableState', 'DisplayBossHealthBar', 'LinkToBossHealthBar', 'SetCharacterBackreadState', 'SetCharacterAIState', 'SetNetworkUpdateRate', 'ChangeCharacterCollisionState', 'ActivateMultiplayerdependantBuffs', 'InvokeEnemyGenerator'):
                self.outputs.append((self.time, n, a))
            elif n == 'EndUnconditionally':
                assert a == [1]
                self.restart()
                continue
            elif n == 'SetNetworkSyncState':
                assert a == [0]
            elif n.startswith('Label'):
                pass
            else:
                raise AssertionError(('Unexpected local mutation', n, a))
            self.pc += 1
        raise AssertionError('Local event failed to yield')

    def restart(self):
        self.pc = 0
        self.groups.clear()
        self.edges.clear()
        self.compiled.clear()
        self.restarts += 1

    def calls(self, name):
        return [(t, a) for t, n, a in self.outputs if n == name]


def verify_hud(decoded, report, entity, name, choice):
    assert decoded[0] == ('SetNetworkSyncState', [0])
    assert not any(n == 'IfCharacterAIState' for n, _ in decoded)
    bars = [a for n, a in decoded if n == 'DisplayBossHealthBar']
    assert bars == [[1, entity, 0, name], [1, entity, 1, name], [0, entity, 0, name], [0, entity, 1, name]]
    two_phase=sum(n=='SpawnOneshotSFX' for n,_ in birth_code)==2
    duration=6.5 if two_phase else 4
    shown = report['shown'] + entity % 10000 - 9000
    flags = {report['spawned']: True, choice: True, shown: True}
    assert ('LinkToBossHealthBar', [1, name, entity]) in decoded
    # Shared birth completion cannot bypass this peer's local reveal.
    delayed = Peer(decoded, flags | {shown: False}, entity)
    delayed.step()
    assert not delayed.calls('DisplayBossHealthBar')
    delayed.flags[shown] = True
    delayed.step(4.5)
    assert delayed.calls('DisplayBossHealthBar') == [(4.5, [1, entity, 0, name])]

    # Host and both clients may have completely different replica AI values.
    for ai in range(7):
        for occupied, slot in [(False, 0), (True, 1)]:
            peer = Peer(decoded, flags, entity)
            peer.state['ai'] = ai
            peer.flags[5000] = occupied
            peer.step()
            assert peer.calls('DisplayBossHealthBar') == [(0, [1, entity, slot, name])]
            peer.state['distance'] = 60
            peer.step(1)
            assert len(peer.calls('DisplayBossHealthBar')) == 1
            peer.state['distance'] = 101
            peer.step(2)
            peer.step(3)
            assert peer.calls('DisplayBossHealthBar')[-1] == (3, [0, entity, slot, name])
            assert not peer.flags[5000 + slot]
            assert slot == 0 or peer.flags[5000], 'Another boss must retain its reservation'
            peer.state['distance'] = 10
            peer.step(4)
            assert peer.calls('DisplayBossHealthBar')[-1] == (4, [1, entity, slot, name])
            peer.state['hp'] = 0
            peer.step(5)
            peer.step(9.9)
            assert peer.calls('DisplayBossHealthBar')[-1][1][0] == 1
            peer.step(10)
            assert peer.calls('DisplayBossHealthBar')[-1] == (10, [0, entity, slot, name])
            peer.step(11)
            peer.state['hp'] = 100
            peer.step(12)
            assert peer.calls('DisplayBossHealthBar')[-1] == (12, [1, entity, slot, name])

    for changed in [dict(player_alive=False), dict(loaded=False), dict(hp=0), dict(distance=51)]:
        peer = Peer(decoded, flags, entity)
        peer.state.update(changed)
        peer.step()
        assert not peer.calls('DisplayBossHealthBar')
    for state_change, flag_change in [(dict(player_alive=False), {}), (dict(loaded=False), {}), ({}, {choice: False}), ({}, {report['spawned']: False})]:
        peer = Peer(decoded, flags, entity)
        peer.step()
        peer.state.update(state_change)
        peer.flags.update(flag_change)
        peer.step(1)
        peer.step(2)
        assert peer.calls('DisplayBossHealthBar')[-1][1] == [0, entity, 0, name]

    # Both slots full: yield without overwriting either native bar. An ended
    # fight cancels the wait even if neither reservation ever becomes free.
    peer = Peer(decoded, flags | {5000: True, 5001: True}, entity)
    peer.step()
    assert not peer.calls('DisplayBossHealthBar')
    peer.flags[5001] = False
    peer.step(1)
    assert peer.calls('DisplayBossHealthBar') == [(1, [1, entity, 1, name])]
    peer = Peer(decoded, flags | {5000: True, 5001: True}, entity)
    peer.step()
    peer.flags[choice] = False
    peer.step(1)
    peer.flags[5000] = False
    peer.step(2)
    assert not peer.calls('DisplayBossHealthBar') and not peer.flags[5000]


def verify_birth(decoded, report, entity, choice, animation, fog):
    assert decoded[0] == ('SetNetworkSyncState', [0])
    flags = {report['ready']: True, choice: True}
    effects=[a for n,a in decoded if n=='SpawnOneshotSFX']
    two_phase=len(effects)==2
    duration=6.5 if two_phase else 4
    assert effects == ([[2,entity,900,690050],[2,entity,900,690047]] if two_phase else [[2,entity,900,fog]])
    assert [a[1:3] for n, a in decoded if n == 'ForceAnimationPlayback'] == animation
    assert not any(n in ('SetNetworkconnectedEventFlag', 'SetCharacterAIState', 'IncrementNetworkconnectedEventValue') for n, _ in decoded)
    generators=[a for n,a in decoded if n=='InvokeEnemyGenerator']
    assert generators in ([],[[entity-5200],[entity-5200]])
    # Each peer runs its own event. A delayed local backread delays that peer's
    # SFX until its model exists, without spawning an extra enemy or reward.
    for delay in [0, .2, .5, 5, 9]:
        peer = Peer(decoded, flags, entity)
        peer.state['loaded'] = delay == 0
        peer.step()
        if delay:
            assert not peer.calls('SpawnOneshotSFX')
            if delay >= 5:
                peer.flags[report['spawned']] = True
            peer.state['loaded'] = True
            peer.step(delay)
        assert peer.calls('SpawnOneshotSFX') == [(delay,effects[0])]
        if two_phase:peer.step(delay+2.5)
        assert peer.calls('ChangeCharacterEnableState') == [(delay, [entity, 0])]
        peer.step(delay + duration - .01)
        assert not peer.calls('ForceAnimationPlayback')
        peer.step(delay + duration)
        assert peer.calls('ChangeCharacterEnableState')[-1] == (delay + duration, [entity, 1])
        assert len(peer.calls('ForceAnimationPlayback')) == len(animation)
        peer.flags[report['spawned']] = True
        peer.step(delay+duration+1)
        assert len(peer.calls('SpawnOneshotSFX')) == len(effects)
        assert len(peer.calls('ActivateMultiplayerdependantBuffs')) == 1
        peer.flags[choice] = False
        peer.step(delay+duration+2)
        peer.step(delay+duration+3)
        peer.flags[choice] = True
        peer.flags[report['spawned']] = False
        peer.effects.clear()  # A new generated actor has a new native marker.
        peer.step(delay+duration+4)
        assert len(peer.calls('SpawnOneshotSFX')) == len(effects)+1
        assert len(peer.calls('ActivateMultiplayerdependantBuffs')) == 2
    # Loading an already active fight reveals its actor without replaying an
    # intro or disabling its AI. This includes unload/reload with choice held.
    peer = Peer(decoded, flags | {report['spawned']: True}, entity)
    peer.step()
    assert not peer.calls('SpawnOneshotSFX') and not peer.calls('ForceAnimationPlayback')
    assert peer.calls('ChangeCharacterEnableState') == [(0, [entity, 1])]
    peer.state['loaded'] = False
    peer.step(1)
    peer.step(2)
    peer.state['loaded'] = True
    peer.step(3)
    assert not peer.calls('SpawnOneshotSFX')
    assert peer.calls('ChangeCharacterEnableState')[-1] == (3, [entity, 1])
    assert len(peer.calls('ActivateMultiplayerdependantBuffs')) == 1


def verify_peer_reveal(birth_code, hud_code, report, entity, name, choice):
    two_phase=sum(n=='SpawnOneshotSFX' for n,_ in birth_code)==2
    duration=6.5 if two_phase else 4
    shown = report['shown'] + entity % 10000 - 9000
    for delay in (0.25, 2.0, 5.0):
        flags = {report['ready']: True, choice: True}
        birth = Peer(birth_code, flags, entity, loaded=False)
        hud = Peer(hud_code, flags, entity, loaded=False)
        birth.flags = hud.flags = flags
        birth.step(0)
        hud.step(0)
        if delay >= 5:
            flags[report['spawned']] = True
        birth.state['loaded'] = hud.state['loaded'] = True
        birth.step(delay)
        flags[report['spawned']] = True
        hud.step(delay)
        assert not hud.calls('DisplayBossHealthBar')
        # Shared completion arrives while this peer is still in its own fog.
        hud.step(delay + duration - .01)
        assert not hud.calls('DisplayBossHealthBar')
        if two_phase:birth.step(delay+2.5)
        birth.step(delay + duration)
        assert flags[shown]
        hud.step(delay + duration)
        assert hud.calls('DisplayBossHealthBar') == [(delay + duration, [1, entity, 0, name])]
        assert hud.calls('LinkToBossHealthBar') == [(delay + duration, [1, name, entity])]
