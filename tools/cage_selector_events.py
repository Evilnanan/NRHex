"""Exercise the serialized selector across consecutive externally requested rounds."""
import json
from pathlib import Path


class Selector:
    def __init__(self, code, report, client=False):
        self.code, self.report = code, report
        self.labels = {int(n[5:]): i for i, (n, _) in enumerate(code) if n.startswith('Label')}
        self.flags = {report['request']: True}
        self.pc, self.draw, self.commits = 0, 0, []
        self.ended = False
        self.missing_labels = []
        self.client = client
        self.random_calls = 0
        self.network_writes = []

    def step(self):
        if self.ended:
            return
        for _ in range(500):
            n, a = self.code[self.pc]
            if n == 'EndIfPlayerIsInWorldType':
                assert a == [0, 1]
                if self.client:
                    self.ended = True
                    return
            elif n == 'SetNetworkSyncState':
                assert a == [0]
            elif n == 'IfEventFlag':
                assert a[0] == 0
                if bool(self.flags.get(a[3], False)) != bool(a[1]):
                    return
            elif n == 'GotoIfEventFlag':
                if bool(self.flags.get(a[3], False)) == bool(a[1]):
                    # Current NR native 0x5943b4 starts at PC + 1. A label
                    # behind the instruction is not a valid jump target.
                    target = next((i for i in range(self.pc + 1, len(self.code))
                                   if self.code[i][0] == f'Label{a[0]}'), None)
                    if target is None:
                        self.missing_labels.append((self.pc, a[0]))
                        self.ended = True
                        return
                    self.pc = target
            elif n == 'SkipIfEventFlag':
                if bool(self.flags.get(a[3], False)) == bool(a[1]):
                    self.pc += a[0]
            elif n in ('BatchSetNetworkconnectedEventFlags', 'BatchSetEventFlags'):
                for flag in range(a[0], a[1] + 1):
                    self.flags[flag] = bool(a[2])
                    if n == 'BatchSetNetworkconnectedEventFlags':
                        self.network_writes.append((flag, bool(a[2])))
            elif n in ('RandomlySetNetworkconnectedEventFlagInRange', 'RandomlySetEventFlagInRange'):
                self.random_calls += 1
                self.flags[a[0] + self.draw] = True
                if n == 'RandomlySetNetworkconnectedEventFlagInRange':
                    self.network_writes.append((a[0] + self.draw, True))
            elif n == 'SetNetworkconnectedEventFlag':
                self.flags[a[1]] = bool(a[2])
                self.network_writes.append((a[1], bool(a[2])))
                if a[1] == self.report['ready'] and a[2]:
                    self.commits.append(self.draw)
            elif n == 'EndUnconditionally':
                assert a == [1]
                self.pc = 0
                continue
            elif n == 'WaitFixedTimeFrames':
                assert a == [1]
                self.pc += 1
                return
            elif n.startswith('Label'):
                pass
            else:
                raise AssertionError(n)
            self.pc += 1
        raise AssertionError('Selector failed to yield')

    def victory_and_reopen(self):
        # The encounter releases busy after clearing request/ready. The opener
        # can then set request before the selector observes its brief OFF state.
        self.flags[self.report['request']] = False
        self.flags[self.report['ready']] = False
        for flag in range(self.report['choices'], self.report['choices'] + self.report.get('encounterCount', 37)):
            self.flags[flag] = False
        self.flags[self.report['request']] = True


def verify_selector(code, report, decode):
    old = json.loads((Path(__file__).resolve().parents[1] / 'data/reference/world/cage-selector-0179.json').read_text())
    old_report = next(r for r in old['reports'] if r['mapName'] == report['mapName'])
    old_event = next(e for e in old_report['events'] if e['ID'] % 10000 == 9700)
    stalled = Selector([decode(i) for i in old_event['instructions']], old_report)
    stalled.step()
    stalled.step()
    assert stalled.commits == [0]
    stalled.victory_and_reopen()
    for _ in range(50):
        stalled.step()
    assert stalled.commits == [0], 'The old request-OFF wait must reproduce the missed edge'

    selector = Selector(code, report)
    selector.step()
    selector.step()
    assert selector.commits == [0]
    # Multiple opener notifications during one committed round cannot draw a
    # second encounter; remote selector instances must never draw at all.
    for _ in range(12):
        selector.flags[report['request']] = True
        selector.step()
    assert selector.commits == [0] and selector.random_calls == 1
    remote = Selector(code, report, client=True)
    remote.step()
    assert remote.ended and remote.random_calls == 0 and not remote.commits
    assert remote.flags == {report['request']: True}
    for index in range(1, 201):
        selector.draw = index % report.get("encounterCount",37)
        selector.victory_and_reopen()
        selector.step()
        selector.step()
        assert len(selector.commits) == index + 1
    # Reload with a committed boss must wait for that round instead of drawing.
    loaded = Selector(code, report)
    loaded.flags[report['ready']] = True
    loaded.step()
    assert loaded.commits == []
    for chosen in range(report.get("encounterCount",37)):
        retry = Selector(code, report)
        retry.draw = chosen
        retry.flags[report['cooling'] + chosen] = True
        for _ in range(32):
            retry.step()
        assert not retry.ended and not retry.commits, ('Cooling retry ended', chosen)
        retry.flags[report['cooling'] + chosen] = False
        for _ in range(3):
            retry.step()
        assert retry.commits == [chosen] and not retry.ended
