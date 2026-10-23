"""Check the binary-preserving Marking pulse patch against its input TAE."""
import argparse
import struct
from pathlib import Path


def verify(before, after):
    targets = {520000, 520003, 525000}
    allowed = set(range(12, 16))

    def value(data, offset, fmt='Q'):
        return struct.unpack_from('<' + fmt, data, offset)[0]

    assert len(after) > len(before)
    assert value(after, 12, 'I') == len(after)
    headers = value(before, 0x58)
    checked = set()
    for index in range(value(before, 0x54, 'I')):
        identity = value(before, headers + index * 16)
        if identity not in targets:
            continue
        checked.add(identity)
        body = value(before, headers + index * 16 + 8)
        allowed.update(range(body, body + 24))
        allowed.update(range(body + 32, body + 44))
        old_events, new_events = value(before, body), value(after, body)
        old_groups, new_groups = value(before, body + 8), value(after, body + 8)
        count, groups = value(before, body + 32, 'I'), value(before, body + 36, 'I')
        assert value(after, body + 32, 'I') == count + 1
        assert value(after, body + 36, 'I') == groups + 1
        assert min(new_events, new_groups, value(after, body + 16)) >= len(before)
        for event in range(count):
            old_header, new_header = old_events + event * 24, new_events + (event + 1) * 24
            for offset in (0, 8):
                assert value(before, value(before, old_header + offset), 'I') == value(after, value(after, new_header + offset), 'I')
            old_data, new_data = value(before, old_header + 16), value(after, new_header + 16)
            old_end = value(before, old_header + 40) if event + 1 < count else old_groups
            new_end = value(after, new_header + 40) if event + 1 < count else new_groups
            assert before[old_data:old_data + 8] == after[new_data:new_data + 8]
            assert value(after, new_data + 8) == new_data + 16
            assert before[old_data + 16:old_end] == after[new_data + 16:new_end]
        for group in range(groups):
            old_header, new_header = old_groups + group * 32, new_groups + group * 32
            amount = value(before, old_header)
            assert amount == value(after, new_header)
            assert before[old_header + 16:old_header + 32] == after[new_header + 16:new_header + 32]
            for item in range(amount):
                old_event = value(before, value(before, old_header + 8) + item * 4, 'I')
                new_event = value(after, value(after, new_header + 8) + item * 4, 'I')
                assert new_event - new_events == old_event - old_events + 24
        added = value(after, new_events + 16)
        assert value(after, added, 'I') == 302
        assert value(after, added + 16, 'I') == 95004204
        assert value(after, value(after, new_events), 'f') == 0
        assert abs(value(after, value(after, new_events + 8), 'f') - 1 / 30) < 1e-8
    assert checked == targets
    assert all(old == new or offset in allowed for offset, (old, new) in enumerate(zip(before, after)))
    return len(checked)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('before', type=Path)
    parser.add_argument('after', type=Path)
    args = parser.parse_args()
    amount = verify(args.before.read_bytes(), args.after.read_bytes())
    print(f'PASS: {amount} pulse additions; original bytes, payloads and group membership preserved')
