"""Execute generated Lua against an independent interpreter of NRHex additions."""
import hashlib
import itertools
import json
import os
from pathlib import Path
import re
import struct
import sys
import unittest

from hks_bytecode import Reader, decode
from mmv_hks_fragments import Fragment, functions

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / '.work/mmv-python'))
from lupa.lua51 import LuaRuntime


def interpreted(code, constants, globals_, arguments, outputs=()):
    raw = [w[0] for w in struct.iter_unpack('>I', code)]
    regs = arguments.copy() if isinstance(arguments, dict) else dict(enumerate(arguments)); position = 0
    while position < len(raw):
        ins = decode(raw[position]); position += 1
        op, a, b, c, bx = [ins[k] for k in ('op', 'a', 'b', 'c', 'bx')]
        rk = lambda i: constants[i - 256] if i >= 256 else regs[i]
        if op == 91: regs[a] = globals_[constants[bx]]
        elif op == 76: pass
        elif op == 25: regs[a] = constants[bx]
        elif op == 13:
            regs[a] = bool(b)
            if c: position += 1
        elif op == 27: globals_[constants[bx]] = regs[a]
        elif op in (2, 69):
            value = regs[a](*(regs[i] for i in range(a + 1, a + b)))
            if c == 2: regs[a] = value
        elif op == 4:
            if (rk(b) == rk(c)) != bool(a): position += 1
        elif op == 57:
            if (constants[b] < rk(c)) != bool(a): position += 1
        elif op == 28: position += bx - 0x10000 + 1
        elif op == 9: return tuple(regs[i] for i in range(a, a + b - 1))
        else: raise AssertionError(op)
    return ('__fallthrough__', *(regs[i] for i in outputs))


class FragmentTests(unittest.TestCase):
    def test_parser_ignores_comments_and_strings(self):
        source = '--[[\nfunction fake()\nend\n]]\nfunction real(a)\nif a then local s="end" end\nend\n'
        self.assertEqual(list(functions(source)), ['real'])

    def test_rejects_nonlocal_branch_and_undefined_input(self):
        with self.assertRaisesRegex(ValueError, 'Nonlocal'):
            Fragment(struct.pack('>I', 28 << 25 | (0x10000 + 5) << 8), [])
        with self.assertRaisesRegex(ValueError, 'Undefined'):
            Fragment(struct.pack('>I', 2 << 25 | 1 << 17 | 1 << 8), [])

    def test_current_shared_prefixes(self):
        readers = []
        for path in ('assets/original/action/script/c0000.hks', 'dist/mod/action/script/c0000.hks'):
            reader = Reader((ROOT / path).read_bytes()); reader.function(); readers.append(reader)
        lua = LuaRuntime(unpack_returned_tuples=True)
        pack = lua.eval('function(f, ...) local function p(...) return {n=select("#",...), ...} end return p(f(...)) end')
        cases = 0; translated = 0
        for before, after in zip(readers[0].functions, readers[1].functions):
            if before['code'] == after['code'] or not after['code'].endswith(before['code']): continue
            code = after['code'][:-len(before['code'])]
            argc = struct.unpack('>I', after['header'][4:8])[0]
            args = [f'arg{i}' for i in range(argc)]
            fragment = Fragment(code, after['constants'], dict(enumerate(args)))
            source = 'function(' + ','.join(args) + ')\n' + fragment.lua() + 'return "__fallthrough__"\nend'
            compiled = lua.eval(source)
            global_names = {after['constants'][d['bx']] for d in fragment.code if d['op'] in (91, 27)}
            event_strings = [s for s in after['constants'] if isinstance(s, str) and s.startswith('W_')]
            argument_cases = [tuple(0 for _ in args), tuple(1 for _ in args)]
            if argc: argument_cases += [(event,) + tuple(1 for _ in args[1:]) for event in event_strings]
            for seed, arguments in itertools.product(range(80), argument_cases):
                def world():
                    calls = []
                    def function(name):
                        def call(*values):
                            calls.append((name, *values))
                            # Include all-false / all-true worlds and independent query values.
                            if seed < 4: return seed
                            normalized = tuple(float(v) if isinstance(v, (int, float)) else v for v in values)
                            key = repr((seed, name, normalized)).encode()
                            return hashlib.sha256(key).digest()[0] % 4
                        return call
                    mapping = {}
                    for name in global_names:
                        if name in ('env', 'act', 'ExecWallJump', 'ExecEventHalfBlend', 'GetHalfBlendInfo', 'GetVariable'):
                            mapping[name] = function(name)
                        elif name == 'TRUE': mapping[name] = 1
                        elif name.startswith('Event_'): mapping[name] = name
                        else: mapping[name] = seed % 4
                    return mapping, calls
                expected_globals, expected_calls = world()
                expected = interpreted(code, after['constants'], expected_globals, arguments)
                actual_globals, actual_calls = world()
                for name, value in actual_globals.items(): lua.globals()[name] = value
                result = pack(compiled, *arguments)
                actual = tuple(result[i] for i in range(1, result['n'] + 1))
                self.assertEqual((actual, actual_calls), (expected, expected_calls), (after['name'], seed, arguments))
                for ins in fragment.code:
                    if ins['op'] == 27:
                        name = after['constants'][ins['bx']]
                        self.assertEqual(lua.globals()[name], expected_globals[name])
                cases += 1
            translated += 1
        self.assertGreater(translated, 100)
        print(f'PASS: {translated} translated prefixes, {cases} Lua/bytecode differential cases')

    def test_complete_adapter_and_interior_fragments(self):
        from mmv_hks_stage import compose, PLAN
        mmv = Path(os.environ.get('NRHEX_MMV', 'D:/Users/OwO/Documents/me3-profiles/MMV'))
        if not mmv.exists(): self.skipTest('Set NRHEX_MMV to run the full source integration test')
        source = (mmv / 'mod/action/script/c0000.hks').read_bytes().decode('utf-8-sig')
        readers = []
        for path in ('assets/original/action/script/c0000.hks', 'dist/mod/action/script/c0000.hks'):
            reader = Reader((ROOT / path).read_bytes()); reader.function(); readers.append(reader)
        merged, report = compose(source, *readers, json.loads(PLAN.read_text(encoding='utf8')))
        self.assertEqual(len(report['functions']), 158)
        self.assertTrue(report['inverseRestoresMmvSource'])
        by_name = {f['name']: f for f in readers[1].functions}
        lua = LuaRuntime(unpack_returned_tuples=True)
        pack = lua.eval('function(f, ...) local function p(...) return {n=select("#",...), ...} end return p(f(...)) end')
        cases = 0
        for spec in report['fragments']:
            if spec['function'] not in json.loads(PLAN.read_text())['functions']: continue
            f = by_name[spec['function']]
            code = f['code'][spec['start'] * 4:spec['end'] * 4]
            inputs = {int(reg): f'arg{i}' for i, reg in enumerate(sorted(spec['inputs']))}
            outputs = {int(reg): inputs[int(reg)] for reg in spec['outputs']}
            fragment = Fragment(code, f['constants'], inputs, outputs)
            tail = ', '.join(['"__fallthrough__"', *outputs.values()])
            compiled = lua.eval('function(' + ','.join(inputs.values()) + ')\n' + fragment.lua() + 'return ' + tail + '\nend')
            names = {f['constants'][d['bx']] for d in fragment.code if d['op'] in (91, 27)}
            callables = {'env', 'act', 'ExecWallJump', 'GetVariable', 'SetVariable', 'IsChargeMagic', 'CheckIfHoldMagic'}
            for seed in range(256):
                def world():
                    calls = []
                    def function(name):
                        def call(*values):
                            calls.append((name, *values))
                            if seed < 4: return seed
                            normalized = tuple(float(v) if isinstance(v, (int, float)) else v for v in values)
                            return hashlib.sha256(repr((seed, name, normalized)).encode()).digest()[0] % 4
                        return call
                    mapping = {name: function(name) if name in callables or not name.isupper() and name not in {'c_Style','c_SwordArtsHand','c_SwordArtsID'}
                               else 1 if name == 'TRUE' else 17 if name == 'ATTACK_REQUEST_BACKSTEP' else seed % 4 for name in names}
                    return mapping, calls
                values = [17 if seed % 5 == 0 else seed % 4 for _ in inputs]
                expected_globals, expected_calls = world()
                expected = interpreted(code, f['constants'], expected_globals, dict(zip(inputs, values)), sorted(outputs))
                globals_, calls = world()
                for name, value in globals_.items(): lua.globals()[name] = value
                result = pack(compiled, *values)
                actual = tuple(result[i] for i in range(1, result['n'] + 1))
                self.assertEqual((actual, calls), (expected, expected_calls), (spec['function'], spec['start'], seed))
                cases += 1

        # Exercise the actual merged MMV Flask function, including its original
        # ally/gradual/FP combinations, instead of testing only the injected block.
        before_span = functions(source)['ExecChangeFlask']
        after_span = functions(merged)['ExecChangeFlask']
        original = source[before_span['start']:before_span['end']]
        adapted = merged[after_span['start']:after_span['end']]
        helper = next(e['after'] for e in report['edits'] if e['reason'] == 'ExecChangeFlask:helper')
        effects = [7010299, 7010200, 8440200, 8970020, 95003000]
        for flags in itertools.product([False, True], repeat=len(effects)):
            active = {effect for effect, flag in zip(effects, flags) if flag}
            results = []
            for text, owned in [(original, active | ({8440200} if 95003000 in active else set())), (helper + adapted, active)]:
                state = LuaRuntime(); calls = []
                state.globals().TRUE = 1; state.globals().FALSE = 0
                state.globals().GetSpEffectID = 1116; state.globals().AddSpEffect = 2002
                state.globals().env = lambda code, effect: int(effect in owned)
                state.globals().act = lambda *values: calls.append(values)
                state.execute(text)
                results.append((state.globals().ExecChangeFlask(), calls))
            self.assertEqual(*results)
        print(f'PASS: {len(report["functions"])} MMV functions, {cases} interior-fragment cases, 32 full Flask scenarios')


if __name__ == '__main__':
    unittest.main()
