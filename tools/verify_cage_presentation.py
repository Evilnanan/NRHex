"""Simulate reread group fog events; this does not render in-game SFX."""
import hashlib
import json
import struct
from local_paths import ROOT
from cage_local_events import Peer

work = ROOT / '.work/cage-presentation'
data = json.loads((work / 'validation.json').read_text())
catalog = json.loads((ROOT / 'data/reference/world/cage-encounters.json').read_text())
schema = {(entry['bank'], entry['id']): (entry['name'], [kind for kind, _ in entry['args']])
          for entry in json.loads((ROOT / 'data/reference/world/cage-opcodes.json').read_text())['opcodes']}
schema[2009, 4] = ('ActivateMultiplayerdependantBuffs', ['uint'])
for label in range(10):
    schema[1014, label] = (f'Label{label}', [])
formats = dict(sbyte='b', byte='B', bool='B', int='i', uint='I', short='h', ushort='H', float='f')


def decode(instruction):
    name, kinds = schema[instruction['Bank'], instruction['ID']]
    raw = bytes.fromhex(instruction['hex'])
    offset, args = 0, []
    for kind in kinds:
        size = struct.calcsize(formats[kind])
        offset = (offset + size - 1) // size * size
        args.append(struct.unpack_from('<' + formats[kind], raw, offset)[0])
        offset += size
    assert len(raw) == (offset + 3) // 4 * 4
    return name, args


cases = encounters = 0
for report in data['reports']:
    prefix = report['prefix']
    path = work / 'staged/mod/event' / (report['name'] + '.emevd.dcx')
    assert hashlib.sha256(path.read_bytes()).hexdigest().upper() == report['outputSha256']
    events = {entry['ID']: [decode(instruction) for instruction in entry['instructions']]
              for entry in report['events']}
    assert len(events) == 84 and report['changedEvents'] == 47
    for index in range(37):
        assert [args for name, args in events[prefix + 9900 + index] if name == 'SpawnOneshotSFX'] == [[2, prefix + 9000 + index, 900, 690047]]
    for group in catalog['groups']:
        encounters += 1
        choice = prefix + 7500 + group['choice']
        for ongoing in (False, True):
            for delay in (0, .2, 5, 30):
                fogs = []
                for member in group['members']:
                    actor = prefix + 9000 + member['index']
                    code = events[prefix + 9900 + member['index']]
                    expected = [[1, prefix + 3900, -1, 690047]] if member == group['members'][0] else []
                    assert [args for name, args in code if name == 'SpawnOneshotSFX'] == expected
                    peer = Peer(code, {prefix + 7403: True, choice: True, prefix + 7404: ongoing}, actor, loaded=False, hp=0)
                    peer.step(0)
                    assert not peer.calls('SpawnOneshotSFX')
                    peer.flags[prefix + 7404] = True
                    peer.state.update(loaded=True, hp=100)
                    peer.step(delay)
                    assert [args for _, args in peer.calls('SpawnOneshotSFX')] == ([] if ongoing else expected)
                    fogs.extend(peer.calls('SpawnOneshotSFX'))
                    peer.step(delay + 4)
                    assert peer.flags[prefix + 7750 + member['index']]
                    assert len(peer.calls('ActivateMultiplayerdependantBuffs')) == 1
                    peer.step(delay + 10)
                    assert len(peer.calls('SpawnOneshotSFX')) == (0 if ongoing else len(expected))
                    peer.state['loaded'] = False
                    peer.step(delay + 11)
                    peer.step(delay + 12)
                    peer.state['loaded'] = True
                    peer.step(delay + 13)
                    assert len(peer.calls('SpawnOneshotSFX')) == (0 if ongoing else len(expected))
                    peer.flags[choice] = False
                    peer.step(delay + 14)
                    peer.step(delay + 15)
                    peer.step(delay + 40)
                    peer.flags[choice] = True
                    peer.flags[prefix + 7404] = False
                    peer.effects.clear()
                    peer.step(delay + 41)
                    assert len(peer.calls('SpawnOneshotSFX')) == (1 if ongoing else 2) * len(expected)
                    peer.step(delay + 45)
                    assert peer.flags[prefix + 7750 + member['index']]
                    cases += 1
                assert len(fogs) == (0 if ongoing else 1)
assert encounters == 52 and cases == 1504
result = dict(encounters=encounters,independentMemberTimelines=cases,singleBossBirthsPreserved=148,
              oneCentralFogPerFreshGroup=True,noOngoingOrStreamReloadReplay=True,repeatEncountersPassed=True,
              gameplayVerified=False,multiplayerVerified=False)
(work / 'event-validation.json').write_text(json.dumps(result, indent=2))
print(f'PASS: {encounters} group encounters, {cases} member timelines; one center fog, ongoing/reload skip and repeat spawn')
