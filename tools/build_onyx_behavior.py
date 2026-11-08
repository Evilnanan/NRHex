"""Append Onyx channel clips to the target's own Nightreign TAG0 graph.

TAE ImportsHKX supplies motion, but does not register a clip in Havok's
SwordArts selectors. Preserve the target TYPE/THSH schema and every existing
item; never deserialize NR custom nodes with the older Elden Ring registry.
"""
import argparse
import hashlib
import json
import struct
from pathlib import Path


class Graph:
    def __init__(self, raw):
        self.raw = raw
        self.chunks = {}
        self.sections(8, len(raw))
        assert raw[4:8] == b'TAG0'
        start, end = self.chunks[b'TSTR']
        strings = raw[start:end].split(b'\0')
        count, pos = self.packed(self.chunks[b'TNA1'][0])
        self.types = [None]
        for _ in range(count - 1):
            name, pos = self.packed(pos)
            n, pos = self.packed(pos)
            self.types.append(strings[name].decode())
            for _ in range(n):
                _, pos = self.packed(pos)
                _, pos = self.packed(pos)
        self.items = [struct.unpack_from('<III', raw, p)
                      for p in range(*self.chunks[b'ITEM'], 12)]
        self.data = bytearray(raw[slice(*self.chunks[b'DATA'])])
        self.patches = {}
        p, end = self.chunks[b'PTCH']
        while p < end:
            t, n = struct.unpack_from('<II', raw, p)
            p += 8
            self.patches[t] = list(struct.unpack_from('<' + 'I' * n, raw, p))
            p += 4 * n
        assert p == end
        self.patch_type = {o: t for t, offsets in self.patches.items() for o in offsets}

    def sections(self, begin, end):
        while begin < end:
            size = int.from_bytes(self.raw[begin:begin + 4], 'big') & 0x3fffffff
            tag = self.raw[begin + 4:begin + 8]
            assert size >= 8 and begin + size <= end
            self.chunks[tag] = (begin + 8, begin + size)
            if tag in (b'TYPE', b'INDX'):
                self.sections(begin + 8, begin + size)
            begin += size
        assert begin == end

    def packed(self, p):
        n = self.raw[p]
        for limit, mask, width in [(0x80, 0x7f, 0), (0xc0, 0x3f, 1),
                                   (0xe0, 0x1f, 2), (0xe8, 7, 3)]:
            if n < limit:
                return ((n & mask) << (8 * width)) | int.from_bytes(self.raw[p + 1:p + 1 + width], 'big'), p + 1 + width
        raise ValueError('Unsupported packed type')

    def q(self, o):
        return struct.unpack_from('<Q', self.data, o)[0]

    def text(self, i):
        if not i:
            return ''
        _, o, n = self.items[i]
        return bytes(self.data[o:o + n]).split(b'\0')[0].decode()

    def objects(self, typ):
        return [(i, o) for i, (k, o, n) in enumerate(self.items)
                if k and self.types[k & 0xffffff] == typ and n == 1]

    def refs(self, i):
        _, o, n = self.items[i]
        return list(struct.unpack_from('<' + 'Q' * n, self.data, o))

    def append(self, kind, data, count=1):
        self.data.extend(b'\0' * (-len(self.data) % 16))
        offset = len(self.data)
        self.data.extend(data)
        i = len(self.items)
        self.items.append((kind, offset, count))
        return i, offset

    def string(self, model, text):
        raw = text.encode() + b'\0'
        return self.append(self.items[model][0], raw, len(raw))[0]

    def array(self, model, refs):
        kind, old, count = self.items[model]
        t = self.patch_type[old]
        assert all(self.patch_type[old + 8 * i] == t for i in range(count))
        idx, offset = self.append(kind, struct.pack('<' + 'Q' * len(refs), *refs), len(refs))
        self.patches[t].extend(offset + i * 8 for i in range(len(refs)))
        return idx

    def chunk(self, tag, payload):
        start, _ = self.chunks[tag]
        flags = int.from_bytes(self.raw[start - 8:start - 4], 'big') & 0xc0000000
        return (flags | (len(payload) + 8)).to_bytes(4, 'big') + tag + payload

    def write(self):
        self.data.extend(b'\0' * (-len(self.data) % 16))
        item = b''.join(struct.pack('<III', *v) for v in self.items)
        patch = b''.join(struct.pack('<II', t, len(v)) + struct.pack('<' + 'I' * len(v), *v)
                         for t, v in self.patches.items())
        result = b''
        for tag in (b'SDKV', b'DATA', b'TYPE', b'INDX'):
            a, b = self.chunks[tag]
            if tag == b'DATA':
                result += self.chunk(tag, self.data)
            elif tag == b'INDX':
                result += self.chunk(tag, self.chunk(b'ITEM', item) + self.chunk(b'PTCH', patch))
            else:
                result += self.raw[a - 8:b]
        return (len(result) + 8).to_bytes(4, 'big') + b'TAG0' + result


def patch(raw):
    g = Graph(raw)
    old_data = bytes(g.data)
    old_items = g.items[:]
    old_patches = {k: v[:] for k, v in g.patches.items()}
    _, strings = g.objects('hkbBehaviorGraphStringData')[0]
    names_array = g.q(strings + 88)
    names = g.refs(names_array)
    clips = {g.text(g.q(o + 72)): (i, o) for i, o in g.objects('hkbClipGenerator')}
    allowed = set()
    changes = []
    for action, source in [(40003, 'a613_040003_hkx_AutoSet_00'), (40004, 'a613_040004_hkx_AutoSet_00')]:
        clip_name = f'a841_{action:06d}'
        assert not any(clip_name in g.text(i) for i in names), 'Onyx behavior already registered'
        model, old = clips[source]
        original_id = struct.unpack_from('<h', g.data, old + 198)[0]
        old_path = g.text(names[original_id])
        assert f'a613_{action:06d}.hkx' in old_path
        new_path = old_path.replace('a613', 'a841')
        names.append(g.string(names[original_id], new_path))
        assert len(names) <= 32768
        node, at = g.append(g.items[model][0], g.data[old:old + 352])
        for offset, typ in g.patch_type.items():
            if old <= offset < old + 352:
                g.patches[typ].append(at + offset - old)
        struct.pack_into('<Q', g.data, at + 72, g.string(g.q(old + 72), clip_name + '_hkx_Onyx'))
        struct.pack_into('<Q', g.data, at + 152, g.string(g.q(old + 152), clip_name))
        struct.pack_into('<h', g.data, at + 198, len(names) - 1)
        # A bounded hold, not a repeating cycle: keep the last frame if a
        # slow update crosses the finish marker instead of wrapping to zero.
        g.data[at + 196] = 0
        # All existing loop/end selectors (full body and upper-body blends)
        # get a new child. No fallback or existing clip is replaced.
        arrays = {}
        selected = []
        for _, offset in g.objects('CustomManualSelectorGenerator'):
            if struct.unpack_from('<ii', g.data, offset + 168) != (18, action):
                continue
            label = g.text(g.q(offset + 72))
            assert label.startswith('SwordArtsLoop')
            array = g.q(offset + 152)
            if array not in arrays:
                refs = g.refs(array)
                assert any(g.text(g.q(g.items[i][1] + 152)) == f'a613_{action:06d}' for i in refs)
                arrays[array] = g.array(array, refs + [node])
            struct.pack_into('<Q', g.data, offset + 152, arrays[array])
            allowed.update(range(offset + 152, offset + 160))
            selected.append(label)
        assert len(selected) == 3, selected
        changes.append(dict(animation=clip_name, selectors=selected, animationIndex=len(names) - 1))
    struct.pack_into('<Q', g.data, strings + 88, g.array(names_array, names))
    allowed.update(range(strings + 88, strings + 96))
    assert all(a == b or i in allowed for i, (a, b) in enumerate(zip(old_data, g.data)))
    assert g.items[:len(old_items)] == old_items
    assert all(g.patches[k][:len(v)] == v for k, v in old_patches.items())
    out = g.write()
    check = Graph(out)
    assert check.data == g.data and check.items == g.items and check.patches == g.patches
    assert out[slice(*check.chunks[b'TYPE'])] == raw[slice(*Graph(raw).chunks[b'TYPE'])]
    # Every added relocation references a valid item; array length lives in
    # ITEM, while serialized hkArray length/capacity fields remain untouched.
    for offsets in check.patches.values():
        for offset in offsets:
            assert check.q(offset) < len(check.items)
    return out, dict(addedClips=changes, existingItemsPreserved=True,
                     existingBytesChangedOnlyForSevenArrayReferences=True,
                     originalTypeSchemaPreserved=True, gameplayVerified=False)


if __name__ == '__main__':
    p = argparse.ArgumentParser()
    p.add_argument('source', type=Path)
    p.add_argument('output', type=Path)
    a = p.parse_args()
    raw = a.source.read_bytes()
    result, report = patch(raw)
    a.output.write_bytes(result)
    report.update(inputSha256=hashlib.sha256(raw).hexdigest(), outputSha256=hashlib.sha256(result).hexdigest())
    a.output.with_suffix('.json').write_text(json.dumps(report, indent=2) + '\n')
    print('PASS: six native selectors and two Onyx motion bindings appended; original nodes and schema preserved')
