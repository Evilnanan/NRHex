"""Reuse the original 8440200 Flask branch for Death Dance without new healing."""
import argparse
import hashlib
import itertools
import json
import os
import struct
import subprocess
from pathlib import Path
from hks_bytecode import Reader, decode

ROOT = Path(__file__).resolve().parents[1]
OWNER = 95003000
NATIVE = 8440200

def abx(opcode, register, constant):
    return opcode << 25 | constant << 8 | register

def abc(opcode, register, first, second):
    return opcode << 25 | first << 17 | second << 8 | register

def execute(words, constants, effects):
    registers = {}
    calls = []
    def query(code, effect):
        assert code == 1116
        return int(effect in effects)
    globals_ = {'env': query, 'act': lambda *arguments: calls.append(arguments), 'TRUE': 1, 'FALSE': 0}
    position = 0
    for _ in range(512):
        instruction = decode(words[position])
        position += 1
        opcode, register = instruction['op'], instruction['a']
        first, second, constant = instruction['b'], instruction['c'], instruction['bx']
        if opcode == 91:
            registers[register] = globals_[constants[constant]]
        elif opcode == 76:
            pass
        elif opcode == 25:
            registers[register] = constants[constant]
        elif opcode == 2:
            result = registers[register](*(registers[index] for index in range(register + 1, register + first)))
            if second == 2:
                registers[register] = result
        elif opcode == 4:
            if (registers[first] == registers[second]) != bool(register):
                position += 1
        elif opcode == 28:
            position += constant - 0x10000 + 1
        elif opcode == 9:
            return calls, tuple(registers[index] for index in range(register, register + first - 1))
        else:
            raise AssertionError(opcode)
    raise AssertionError('Unexpected loop in Flask function')

def build(source, output):
    reader = Reader(source)
    reader.function()
    function = next(function for function in reader.functions if function['name'] == 'ExecChangeFlask')
    assert function['id'] == 101 and function['slots'] == 3
    constants = function['constants']
    assert OWNER not in constants and constants.count(NATIVE) == 1
    words = [word[0] for word in struct.iter_unpack('>I', function['code'])]
    assert len(words) == 130
    anchors = [index for index, word in enumerate(words) if decode(word) == dict(op=25, a=2, b=0, c=constants.index(NATIVE), bx=constants.index(NATIVE))]
    assert anchors == [21, 62, 110]
    insertions = [index + 2 for index in anchors]
    for index in anchors:
        assert words[index - 3:index + 2] == [abx(91, 0, constants.index('env')), abx(76, 20, constants.index('env') + 1), abx(25, 1, constants.index(1116)), abx(25, 2, constants.index(NATIVE)), abc(2, 0, 3, 2)]
    owner_index = len(constants)
    extra = struct.pack('>Bf', 3, OWNER) + b'\0' * 3
    assert struct.unpack('>f', struct.pack('>f', OWNER))[0] == OWNER
    expanded_constants = constants + [float(OWNER), None, None, None]
    prefix = [abx(91, 1, constants.index('TRUE')), abx(76, 20, constants.index('TRUE') + 1), abc(4, 1, 0, 1), abx(28, 0, 0x10004), abx(91, 0, constants.index('env')), abx(76, 20, constants.index('env') + 1), abx(25, 1, constants.index(1116)), abx(25, 2, owner_index), abc(2, 0, 3, 2)]
    def moved(position):
        return position + len(prefix) * sum(anchor <= position for anchor in insertions)
    def destination(position):
        return position + len(prefix) * sum(anchor < position for anchor in insertions)
    patched_words = []
    for index, word in enumerate(words):
        if index in insertions:
            patched_words.extend(prefix)
        instruction = decode(word)
        if instruction['op'] == 28:
            target = index + instruction['bx'] - 0x10000 + 2
            word = abx(28, instruction['a'], 0x10000 + destination(target) - moved(index) - 2)
        patched_words.append(word)
    for index, word in enumerate(words):
        actual = decode(patched_words[moved(index)])
        original = decode(word)
        if original['op'] == 28:
            target = index + original['bx'] - 0x10000 + 2
            assert moved(index) + actual['bx'] - 0x10000 + 2 == destination(target)
        else:
            assert patched_words[moved(index)] == word
    cases = 0
    for values in itertools.product([False, True], repeat=5):
        effects = {effect for effect, present in zip([7010299, 7010200, NATIVE, 8970020, OWNER], values) if present}
        expected = effects | {NATIVE} if OWNER in effects else effects
        assert execute(patched_words, expanded_constants, effects) == execute(words, constants, expected)
        cases += 1
    assert execute(words, constants, {7010299, OWNER}) != execute(words, constants, {7010299, NATIVE})
    code = b''.join(struct.pack('>I', word) for word in patched_words)
    lines = b''.join((function['lines'][index * 4:index * 4 + 4] * len(prefix) if index in insertions else b'') + function['lines'][index * 4:index * 4 + 4] for index in range(len(words)))
    edits = [(function['code_count_offset'], 4, struct.pack('>I', len(patched_words))), (function['code_offset'], len(function['code']), code), (function['constants_count_offset'], 4, struct.pack('>I', len(expanded_constants))), (function['constants_end'], 0, extra), (function['line_count_offset'], 4, struct.pack('>I', len(patched_words))), (function['lines_offset'], len(function['lines']), lines)]
    for _, start_offset, start, end_offset, end in function['locals']:
        edits.extend([(start_offset, 4, struct.pack('>I', moved(start))), (end_offset, 4, struct.pack('>I', moved(end)))])
    patched = source
    for offset, count, value in sorted(edits, reverse=True):
        patched = patched[:offset] + value + patched[offset + count:]
    inverse = []
    shift = 0
    for offset, count, value in sorted(edits):
        inverse.append((offset + shift, len(value), source[offset:offset + count]))
        shift += len(value) - count
    restored = patched
    for offset, count, value in sorted(inverse, reverse=True):
        restored = restored[:offset] + value + restored[offset + count:]
    assert restored == source
    reopened = Reader(patched)
    reopened.function()
    assert len(reopened.functions) == len(reader.functions) and patched[reopened.p:] == source[reader.p:]
    for before, after in zip(reader.functions, reopened.functions):
        if before['id'] == function['id']:
            assert after['code'] == code and after['constants'] == expanded_constants
        elif before['id'] == 0:
            assert before['code'] == after['code'] and before['constants'] == after['constants']
        else:
            assert source[before['start']:before['end']] == patched[after['start']:after['end']]
    output.mkdir(parents=True, exist_ok=True)
    baseline = output / 'before.hks'
    baseline.write_bytes(source)
    target = output / 'c0000.hks'
    target.write_bytes(patched)
    executable = Path(os.environ.get('NRHEX_DSLUA', str(Path(ROOT.anchor) / 'Program Files/DSLuaDecompiler/DSLuaDecompiler.exe')))
    decompiled = []
    for path, name in [(baseline, 'before.lua'), (target, 'current.lua')]:
        result = output / name
        subprocess.run([str(executable), str(path), '--included-function-ids', str(function['id']), '-o', str(result)], check=True, capture_output=True)
        decompiled.append(result.read_text(encoding='utf-8-sig'))
    assert str(OWNER) not in decompiled[0] and decompiled[1].count(str(OWNER)) == 3
    report = dict(inputSha256=hashlib.sha256(source).hexdigest(), outputSha256=hashlib.sha256(patched).hexdigest(), function='ExecChangeFlask', nativeEffect=NATIVE, ownerEffect=OWNER, predicateSites=3, semanticCases=cases, inverseRestoresInput=True, unrelatedFunctionsUnchanged=True, originalGuardsActionsAndReturnsPreserved=True, nativeEffectAndOwnerDoNotDoubleHeal=True, independentDecompilerPassed=True, gameplayVerified=False, multiplayerVerified=False)
    (output / 'validation.json').write_text(json.dumps(report, indent=2), encoding='utf8')
    print(f'PASS: {cases} Flask branch cases; native recovery actions/guards retained, no double heal; only ExecChangeFlask changed')
    return patched

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--source', type=Path, default=ROOT / 'dist/mod/action/script/c0000.hks')
    parser.add_argument('--output', type=Path, default=ROOT / 'work/death-dance/flask')
    parser.add_argument('--inplace', action='store_true')
    arguments = parser.parse_args()
    patched = build(arguments.source.read_bytes(), arguments.output)
    if arguments.inplace:
        arguments.source.write_bytes(patched)
