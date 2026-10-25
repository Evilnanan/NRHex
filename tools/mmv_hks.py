"""Inspect shared HKS changes against MMV's source-form action script.

Classify prefix-only and interior changes. This is an audit, not a transpiler:
even prefixes need Lua scope, return-value and branch-target verification.
"""
from pathlib import Path
import argparse, hashlib, json, re
from hks_bytecode import Reader

ROOT = Path(__file__).resolve().parents[1]
def sha(data): return hashlib.sha256(data).hexdigest()
def read_functions(data):
    r = Reader(data); r.function()
    return r
def audit(mmv, output):
    original = (ROOT/'assets/original/action/script/c0000.hks').read_bytes()
    current = (ROOT/'dist/mod/action/script/c0000.hks').read_bytes()
    source = (mmv/'mod/action/script/c0000.hks').read_text(encoding='utf-8-sig')
    before = read_functions(original); after = read_functions(current)
    if len(before.functions) != len(after.functions):
        raise ValueError('Shared HKS prototype topology changed')
    names = re.findall(r'^function\s+([\w.]+)\s*\(', source, re.M)
    functions = []; conflicts = []
    for a, b in zip(before.functions, after.functions):
        if a['id'] != b['id'] or a['name'] != b['name']:
            raise ValueError('Shared HKS prototype identities changed')
        if a['code'] == b['code'] and a['constants'] == b['constants']: continue
        prefix = b['code'].endswith(a['code']) and b['constants'][:len(a['constants'])] == a['constants']
        record = dict(id=a['id'], name=a['name'], kind='prefix' if prefix else 'interior',
                      mmvDefinitions=names.count(a['name']), originalHash=sha(a['code']), nrhexHash=sha(b['code']))
        functions.append(record)
        if names.count(a['name']) != 1:
            conflicts.append(dict(function=a['name'], reason='missing-or-ambiguous-source-function'))
        else:
            conflicts.append(dict(function=a['name'], reason=('prefix-needs-source-adapter' if prefix else 'interior-edit-needs-source-adapter')))
    result = dict(originalHash=sha(original), nrhexHash=sha(current), mmvHash=sha((mmv/'mod/action/script/c0000.hks').read_bytes()),
                  functions=functions, conflicts=conflicts, prefixFunctions=sum(f['kind']=='prefix' for f in functions),
                  interiorFunctions=sum(f['kind']=='interior' for f in functions), gameplayVerified=False)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2)+'\n', encoding='utf8')
    return result

if __name__ == '__main__':
    p=argparse.ArgumentParser();p.add_argument('--mmv',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    args=p.parse_args();r=audit(args.mmv,args.output)
    print(f"HKS: {r['prefixFunctions']} prefix changes, {r['interiorFunctions']} interior changes; {len(r['conflicts'])} source-adapter requirements")
