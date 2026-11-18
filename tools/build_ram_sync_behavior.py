"""Append private, unbound typed variables; preserve all original graph indices."""
import struct
from build_onyx_behavior import Graph

VARIABLES = [('MARamSyncV1', 3), ('MARamGeneration', 3), ('MARamCycle', 3),
             ('MARamSample', 3), ('MARamRate', 4), ('MARamPhase', 4),
             ('MAArtsActionV1', 3), ('MAArtsWeapon', 3), ('MAArtsCast', 3), ('MAArtsSample', 3)]


def variable_layout(raw):
    graph = Graph(raw)
    _, data = graph.objects('hkbBehaviorGraphData')[0]
    _, strings = graph.objects('hkbBehaviorGraphStringData')[0]
    names = [graph.text(item) for item in graph.refs(graph.q(strings + 56))]
    _, start, count = graph.items[graph.q(data + 40)]
    if count != len(names):
        raise ValueError('Ram graph variable and type counts differ')
    return [(name, graph.data[start + 6 * i + 4]) for i, name in enumerate(names)]


def verify(raw, baseline):
    expected = variable_layout(baseline) + VARIABLES
    if variable_layout(raw) != expected:
        raise ValueError('Packaged graph variable names, order or types differ from the target graph plus Ram variables')


def patch(raw):
    g = Graph(raw)
    old = bytes(g.data)
    items = g.items[:]
    relocations = {k: v[:] for k, v in g.patches.items()}
    _, data = g.objects('hkbBehaviorGraphData')[0]
    _, strings = g.objects('hkbBehaviorGraphStringData')[0]
    _, values = g.objects('hkbVariableValueSet')[0]
    names_item = g.q(strings + 56)
    names = g.refs(names_item)
    original_names = [g.text(i) for i in names]
    assert not any(n in original_names for n, _ in VARIABLES), 'Ram sync variables already registered'
    info_item = g.q(data + 40)
    value_item = g.q(values + 24)
    bounds_item = g.q(data + 88)
    assert g.items[info_item][2] == g.items[value_item][2] == len(names)
    # Variable bounds are optional. Extend an existing full array, otherwise
    # leave the original empty reference untouched.
    assert not bounds_item or g.items[bounds_item][2] in (0, len(names))
    changed = set()

    def replace(at, item):
        struct.pack_into('<Q', g.data, at, item)
        changed.update(range(at, at + 8))

    def extend(item, stride, suffix):
        kind, at, count = g.items[item]
        # No pointers in these scalar records.
        assert not any(at <= p < at + count * stride for p in g.patch_type)
        return g.append(kind, g.data[at:at + count * stride] + suffix,
                        count + len(VARIABLES))[0]

    for name, _ in VARIABLES:
        names.append(g.string(names[0], name))
    replace(strings + 56, g.array(names_item, names))
    replace(data + 40, extend(info_item, 6, b''.join(struct.pack('<HHbB', 0, 0, t, 0) for _, t in VARIABLES)))
    replace(values + 24, extend(value_item, 4, b'\0' * (4 * len(VARIABLES))))
    if bounds_item and g.items[bounds_item][2]:
        replace(data + 88, extend(bounds_item, 8, b'\0' * (8 * len(VARIABLES))))
    assert all(a == b or i in changed for i, (a, b) in enumerate(zip(old, g.data)))
    assert g.items[:len(items)] == items
    assert all(g.patches[k][:len(v)] == v for k, v in relocations.items())
    out = g.write()
    check = Graph(out)
    assert check.data == g.data and check.items == g.items and check.patches == g.patches
    assert out[slice(*check.chunks[b'TYPE'])] == raw[slice(*Graph(raw).chunks[b'TYPE'])]
    for offsets in check.patches.values():
        for at in offsets:
            assert check.q(at) < len(check.items)
    return out, dict(variables=[dict(name=n, type=t, localIndex=len(original_names)+i)
                               for i, (n, t) in enumerate(VARIABLES)],
                     existingIndicesPreserved=True, nativeGlobalIdsResolvedAtRuntime=True,
                     originalTypeSchemaPreserved=True, gameplayVerified=False)

if __name__ == '__main__':
    import argparse, json, hashlib
    from pathlib import Path
    parser = argparse.ArgumentParser()
    parser.add_argument('source', type=Path)
    parser.add_argument('output', type=Path, nargs='?')
    parser.add_argument('--verify-against', type=Path)
    args = parser.parse_args()
    source = args.source.read_bytes()
    if args.verify_against:
        verify(source, args.verify_against.read_bytes())
        raise SystemExit(0)
    if args.output is None:
        parser.error('output is required when patching')
    result, report = patch(source)
    args.output.write_bytes(result)
    report.update(inputSha256=hashlib.sha256(source).hexdigest(), outputSha256=hashlib.sha256(result).hexdigest())
    args.output.with_suffix('.json').write_text(json.dumps(report, indent=2))
    print('PASS: typed Ram and Arts sync variables appended; original graph indices and nodes preserved')
