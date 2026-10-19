import json
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
report = json.loads((ROOT / '.work/cage-music/validation.json').read_text(encoding='utf-8'))
catalog = json.loads((ROOT / 'data/reference/world/cage-encounters.json').read_text(encoding='utf-8'))
groups = {group['choice']: group for group in catalog['groups']}


class Machine:
    def __init__(self, event, prefix, track, shown_flags):
        self.instructions = event['instructions']
        self.prefix = prefix
        self.track = track
        self.shown_flags = shown_flags
        self.members = [track['choice']] if track['choice'] < 37 else [member['index'] for member in groups[track['choice']]['members'] if member['combat']]
        self.choice = True
        self.spawned = True
        self.player_alive = True
        self.distance = 10
        self.priority = True
        self.shown = set(self.members)
        self.loaded = set(self.members)
        self.alive = set(self.members)
        self.playing = None
        self.actions = []
        self.conditions = {}
        self.compiled = {}
        self.pc = 0
        self.now = 0
        self.resume = 0

    def condition(self, group):
        values = [evaluate() for evaluate in self.conditions.get(group, [])]
        return (all(values) if group > 0 else any(values)) if values else False

    def flag(self, flag):
        if flag == self.prefix + 7404:
            return self.spawned
        if flag == self.prefix + 7500 + self.track['choice']:
            return self.choice
        return any(self.shown_flags[member] == flag for member in self.shown)

    def run(self, seconds=0):
        self.now += seconds
        for _ in range(500):
            if self.now < self.resume:
                return
            instruction = self.instructions[self.pc]
            bank, opcode = instruction['Bank'], instruction['ID']
            raw = bytes.fromhex(instruction['hex'])
            signed = lambda offset: struct.unpack_from('<b', raw, offset)[0]
            integer = lambda offset: struct.unpack_from('<i', raw, offset)[0]
            unsigned = lambda offset: struct.unpack_from('<I', raw, offset)[0]
            floating = lambda offset: struct.unpack_from('<f', raw, offset)[0]
            evaluate = None
            if bank == 3 and opcode == 0:
                flag, on = unsigned(4), bool(raw[1])
                evaluate = lambda flag=flag, on=on: self.flag(flag) == on
            elif bank == 4 and opcode == 0:
                assert unsigned(4) == 20000
                dead = bool(raw[8])
                evaluate = lambda dead=dead: self.player_alive != dead
            elif bank == 3 and opcode == 3:
                within, limit = bool(raw[1]), floating(12)
                evaluate = lambda within=within, limit=limit: (self.distance <= limit) == within
            elif bank == 4 and opcode == 7:
                member, loaded = unsigned(4) - self.prefix - 9000, bool(raw[8])
                evaluate = lambda member=member, loaded=loaded: (member in self.loaded) == loaded
            elif bank == 4 and opcode == 14:
                member, comparison = unsigned(4) - self.prefix - 9000, signed(8)
                assert comparison in [2, 5] and integer(12) == 0
                evaluate = lambda member=member, comparison=comparison: (member in self.alive) == (comparison == 2)
            elif bank == 10 and opcode == 3:
                assert signed(1) == 5 and integer(4) == self.track['bgm']
                evaluate = lambda: self.priority
            elif bank == 10 and opcode == 2:
                bgm, playing = integer(4), bool(raw[8])
                evaluate = lambda bgm=bgm, playing=playing: (self.playing == bgm) == playing
            elif bank == 0 and opcode == 0:
                source, expected = signed(2), bool(raw[1])
                if signed(0) == 0:
                    if self.condition(source) != expected:
                        return
                    self.compiled = {group: self.condition(group) for group in self.conditions}
                    self.conditions = {}
                else:
                    evaluate = lambda source=source, expected=expected: self.condition(source) == expected
            elif bank == 2000 and opcode == 2:
                assert raw == bytes(4)
            elif bank == 2010 and opcode == 10:
                bgm, state = struct.unpack('<ii', raw)
                assert bgm == self.track['bgm'] and state in [0, -1]
                self.actions.append((bgm, state))
                self.playing = bgm if state == 0 else None
            elif bank == 1001 and opcode in [0, 1]:
                self.resume = self.now + (floating(0) if opcode == 0 else integer(0) / 30)
            elif bank == 1000 and opcode == 7:
                if self.compiled.get(signed(2), False) == bool(raw[1]):
                    self.pc += raw[0]
            elif bank == 1010 and opcode == 4:
                if (self.playing == integer(4)) == bool(raw[8]):
                    self.pc += raw[0]
            elif bank == 1000 and opcode == 4:
                assert raw[0] == 1
                self.conditions = {}
                self.compiled = {}
                self.pc = 1
                continue
            else:
                raise AssertionError((bank, opcode))
            if evaluate is not None:
                self.conditions.setdefault(signed(0), []).append(evaluate)
            self.pc += 1
        raise AssertionError('Unbounded music event loop')


cases = 0
for staged in report['reports']:
    prefix = staged['prefix']
    events = {event['ID']: event for event in staged['events']}
    shown_flags = {}
    for birth in staged['birthEvents']:
        member = birth['ID'] - prefix - 9900
        instructions = birth['instructions']
        candidates = []
        for instruction in instructions:
            raw = bytes.fromhex(instruction['hex'])
            if (instruction['Bank'], instruction['ID']) == (2003, 66) and raw[8] == 1:
                candidates.append(struct.unpack_from('<I', raw, 4)[0])
        assert len(candidates) == 1, (birth['ID'], candidates)
        shown_flags[member] = candidates[0]
    assert len(shown_flags) == 84
    for track in report['tracks']:
        event = events[prefix + 9400 + track['choice']]
        assert not event['Parameters'] and event['RestBehavior'] == 1
        for key, value in [('choice', False), ('spawned', False), ('player_alive', False), ('priority', False), ('distance', 51), ('shown', set()), ('loaded', set()), ('alive', set())]:
            machine = Machine(event, prefix, track, shown_flags)
            setattr(machine, key, value)
            machine.run()
            assert machine.actions == [], key
            cases += 1
        machine = Machine(event, prefix, track, shown_flags)
        machine.run()
        assert machine.actions == [(track['bgm'], 0)]
        machine.run(1)
        if len(machine.members) > 1:
            machine.alive.remove(machine.members[0])
            machine.run(1)
            assert len(machine.actions) == 1
        machine.alive.clear()
        machine.run()
        assert len(machine.actions) == 1
        machine.run(5)
        assert machine.actions[-1] == (track['bgm'], -1)
        machine.spawned = False
        machine.run(1)
        machine.alive = set(machine.members)
        machine.spawned = True
        machine.run(1)
        assert machine.actions == [(track['bgm'], 0), (track['bgm'], -1), (track['bgm'], 0)]
        cases += 1
        for key, value in [('choice', False), ('spawned', False), ('player_alive', False), ('distance', 101), ('shown', set())]:
            machine = Machine(event, prefix, track, shown_flags)
            machine.run()
            setattr(machine, key, value)
            machine.run(1)
            assert machine.actions == [(track['bgm'], 0), (track['bgm'], -1)], key
            machine.run(1)
            assert len(machine.actions) == 2
            cases += 1
        machine = Machine(event, prefix, track, shown_flags)
        machine.run()
        machine.playing = 999999
        machine.priority = False
        machine.run(1)
        assert machine.playing == 999999 and machine.actions == [(track['bgm'], 0)]
        machine.run(1)
        assert len(machine.actions) == 1
        cases += 1
        if track['choice'] < 37:
            legacy = json.loads(json.dumps(event))
            for instruction in legacy['instructions']:
                raw = bytearray.fromhex(instruction['hex'])
                if (instruction['Bank'], instruction['ID']) == (3, 0) and struct.unpack_from('<I', raw, 4)[0] == shown_flags[track['choice']]:
                    struct.pack_into('<I', raw, 4, prefix + 7750 + track['choice'])
                    instruction['hex'] = raw.hex()
            machine = Machine(legacy, prefix, track, shown_flags)
            machine.run()
            assert machine.actions == [], 'Original single-boss regression must reproduce'
            cases += 1

output = dict(cases=cases, encounters=50, maps=4, distinctBgmIds=len({track['bgm'] for track in report['tracks']}), gameplayVerified=False, multiplayerVerified=False)
(ROOT / '.work/cage-music/state-validation.json').write_text(json.dumps(output, indent=2), encoding='utf-8')
print(f'PASS: {cases} emitted-event lifecycle cases; native priority/audio engine and multiplayer require game testing')
