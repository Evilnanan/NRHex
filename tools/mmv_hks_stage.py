"""Compose NRHex additions into an independent copy of MMV's text HKS."""
import hashlib
import json
from pathlib import Path
import re
import struct
import sys

from hks_bytecode import Reader
from mmv_hks_fragments import Fragment, functions

ROOT = Path(__file__).resolve().parents[1]
PLAN = ROOT / 'compat/mmv/hks-interior.json'


def hash_function(f):
    return hashlib.sha256(f['code'] + json.dumps(f['constants'], ensure_ascii=False).encode()).hexdigest()


def runtime():
    sys.path.insert(0, str(ROOT / '.work/mmv-python'))
    try:
        from lupa.lua51 import LuaRuntime
    except ImportError as exc:
        raise ValueError('Install compat/mmv/requirements.txt into .work/mmv-python before HKS staging') from exc
    return LuaRuntime(unpack_returned_tuples=True)


def compose(source, before, after, plan):
    named = functions(source)
    if 'nrhex_r' in source or 'NRHexMMV_' in source:
        raise ValueError('MMV HKS already contains an adapter or its reserved local names')
    edits = []; adapted = []; fragments = []

    def add(start, end, text, reason):
        edits.append(dict(start=start, end=end, before=source[start:end], after=text, reason=reason))

    def find(f, needle, count=1):
        matches = list(re.finditer(re.escape(needle), source[named[f]['body']:named[f]['end']]))
        if len(matches) != count:
            raise ValueError(f'{f}: expected {count} source anchors for {needle!r}, got {len(matches)}')
        return [(named[f]['body'] + m.start(), named[f]['body'] + m.end()) for m in matches]

    def render(f, start, end, inputs=None, outputs=None):
        code = f['code'][start * 4:end * 4]
        fragment = Fragment(code, f['constants'], inputs, outputs)
        fragments.append(dict(function=f['name'], start=start, end=end, inputs=inputs or {}, outputs=outputs or {}))
        return fragment.lua()

    if len(before.functions) != len(after.functions):
        raise ValueError('Shared HKS function topology changed')
    for old, new in zip(before.functions, after.functions):
        name = old['name']
        if name != new['name']:
            raise ValueError('Shared HKS function identities changed')
        if old['code'] == new['code'] and old['constants'] == new['constants']:
            continue
        if name not in named or named[name]['count'] != 1:
            raise ValueError(f'MMV action missing or ambiguous: {name}')
        argc = struct.unpack('>I', new['header'][4:8])[0]
        if new['code'].endswith(old['code']) and new['constants'][:len(old['constants'])] == old['constants']:
            if len(named[name]['args']) != argc:
                raise ValueError(f'MMV argument signature changed: {name}')
            end = new['count'] - old['count']
            add(named[name]['body'], named[name]['body'], render(new, 0, end, dict(enumerate(named[name]['args']))), name + ':prefix')
        else:
            spec = plan['functions'].get(name)
            if not spec or spec['originalHash'] != hash_function(old) or spec['nrhexHash'] != hash_function(new):
                raise ValueError(f'Interior adapter needs review against the current shared HKS: {name}')
            mmv_body = source[named[name]['start']:named[name]['end']]
            if hashlib.sha256(mmv_body.encode()).hexdigest() != spec['mmvFunctionHash']:
                raise ValueError(f'Interior source anchor context changed: {name}')
            for action in spec['actions']:
                kind = action['kind']
                if kind == 'prefix':
                    add(named[name]['body'], named[name]['body'], render(new, *action['range'], dict(enumerate(named[name]['args']))), name + ':prefix')
                elif kind == 'replace':
                    for start, end in find(name, action['old'], action.get('count', 1)):
                        add(start, end, action['new'], name + ':query')
                elif kind in ('before', 'after'):
                    matches = find(name, action['anchor'], action.get('count', 1))
                    selected = matches[action.get('occurrence', 0)]
                    at = selected[0] if kind == 'before' else selected[1]
                    inputs = {int(k): v for k, v in action.get('inputs', {}).items()}
                    block = render(new, *action['range'], inputs)
                    add(at, at, '\n' + block, name + ':interior')
                elif kind == 'sprint-cost':
                    (start, end), = find(name, 'AddStamina(STAMINA_REDUCE_SPRINTSTART)')
                    block = 'do\n    local nrhex_cost = STAMINA_REDUCE_SPRINTSTART\n'
                    block += render(new, *action['range'], {1: 'nrhex_cost'}, {1: 'nrhex_cost'})
                    block += '    AddStamina(nrhex_cost)\n    end'
                    add(start, end, block, name + ':cost')
                elif kind == 'flask-helper':
                    block = 'local function NRHexMMV_Flask(nrhex_value)\n'
                    block += render(new, *action['range'], {0: 'nrhex_value'}, {0: 'nrhex_value'})
                    block += '    return nrhex_value\nend\n\n'
                    add(0, 0, block, name + ':helper')
                else:
                    raise ValueError(f'Unknown HKS adapter action: {kind}')
        adapted.append(name)
    ordered = sorted(edits, key=lambda e: (e['start'], e['end']))
    for left, right in zip(ordered, ordered[1:]):
        if left['end'] > right['start'] or left['start'] == right['start']:
            raise ValueError(f'Overlapping source adapters: {left["reason"]}, {right["reason"]}')
    output = source
    for edit in reversed(ordered):
        output = output[:edit['start']] + edit['after'] + output[edit['end']:]
    # Independently undo every inserted/replaced range: all MMV bytes must return.
    shift = 0; inverse = []
    for edit in ordered:
        at = edit['start'] + shift
        inverse.append((at, len(edit['after']), edit['before']))
        shift += len(edit['after']) - (edit['end'] - edit['start'])
    restored = output
    for at, length, original in reversed(inverse):
        restored = restored[:at] + original + restored[at + length:]
    if restored != source:
        raise ValueError('HKS inverse did not restore MMV source')
    lua = runtime()
    ok, error = lua.eval('function(s) local f,e=loadstring(s); return f~=nil,e end')(output)
    if not ok:
        raise ValueError(f'Merged HKS syntax failure: {error}')
    return output, dict(functions=adapted, fragments=fragments, edits=ordered,
                        inverseRestoresMmvSource=True, lua51SyntaxVerified=True, gameplayVerified=False)


def stage(mmv, directory):
    raw = (mmv / 'mod/action/script/c0000.hks').read_bytes()
    source = raw.decode('utf-8-sig')
    readers = []
    for path in ('assets/original/action/script/c0000.hks', 'dist/mod/action/script/c0000.hks'):
        r = Reader((ROOT / path).read_bytes()); r.function(); readers.append(r)
    output, report = compose(source, *readers, json.loads(PLAN.read_text(encoding='utf8')))
    encoded = (b'\xef\xbb\xbf' if raw.startswith(b'\xef\xbb\xbf') else b'') + output.encode('utf8')
    path = directory / 'mod/action/script/c0000.hks'
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(encoded)
    if path.read_bytes() != encoded:
        raise ValueError('HKS output readback failed')
    report.update(mmvSha256=hashlib.sha256(raw).hexdigest(), outputSha256=hashlib.sha256(encoded).hexdigest())
    (directory / 'hks-validation.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
    return report


if __name__ == '__main__':
    import argparse
    p = argparse.ArgumentParser(); p.add_argument('--mmv', type=Path, required=True)
    p.add_argument('--output', type=Path, default=ROOT / 'work/compat/mmv/staged')
    args = p.parse_args(); report = stage(args.mmv, args.output)
    print(f'HKS: {len(report["functions"])} functions adapted; exact inverse and Lua 5.1 syntax passed')
