"""Translate guarded, acyclic NRHex bytecode additions to ordinary Lua 5.1.

This is deliberately not a general HKS decompiler. Unknown instructions, open
calls, external branch destinations and reads of undefined registers fail closed.
The original MMV body is never decompiled or replaced by this translator.
"""
import json
import math
import re
import struct

from hks_bytecode import decode


def literal(value):
    if value is None:
        return 'nil'
    if isinstance(value, bool):
        return 'true' if value else 'false'
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False)
    if not math.isfinite(value):
        raise ValueError('Non-finite HKS literal')
    return str(int(value)) if value == int(value) else repr(value)


def words(code):
    return [decode(word[0]) for word in struct.iter_unpack('>I', code)]


class Fragment:
    def __init__(self, code, constants, inputs=None, outputs=None):
        self.code = words(code)
        self.constants = constants
        self.inputs = inputs or {}
        self.outputs = outputs or {}
        self.end = len(self.code)
        self.exit = self.end + 1
        self.edges = {self.end: [self.exit], self.exit: []}
        self.reads = {}; self.writes = {}; self.statements = {}; self.conditions = {}
        self.registers = set(self.inputs)
        for pc, ins in enumerate(self.code):
            a, b, c, bx, op = [ins[k] for k in ('a', 'b', 'c', 'bx', 'op')]
            read = set(); write = set(); statement = None
            def reg(index):
                if not 0 <= index < 256:
                    raise ValueError(f'Invalid register {index}')
                read.add(index)
                return f'nrhex_r{index}'
            def rk(index):
                return literal(constants[index - 256]) if index >= 256 else reg(index)
            def dest(index):
                write.add(index)
                return f'nrhex_r{index}'
            def global_name(index):
                name = constants[index]
                if not isinstance(name, str) or not re.fullmatch(r'[A-Za-z_]\w*', name):
                    raise ValueError(f'Unsupported global name: {name!r}')
                return name
            edges = [pc + 1]
            if op == 25:
                statement = f'{dest(a)} = {literal(constants[bx])}'
            elif op == 13:
                statement = f'{dest(a)} = {literal(bool(b))}'
                if c:
                    edges = [pc + 2]
            elif op == 91:
                statement = f'{dest(a)} = {global_name(bx)}'
            elif op == 27:
                statement = f'{global_name(bx)} = {reg(a)}'
            elif op == 76:
                if pc == 0 or self.code[pc - 1]['op'] != 91:
                    raise ValueError('DATA is not part of a cached global lookup')
            elif op in (2, 69):
                if b == 0 or c not in (1, 2):
                    raise ValueError('Open or multi-result call needs an explicit adapter')
                call = f'{reg(a)}(' + ', '.join(reg(i) for i in range(a + 1, a + b)) + ')'
                statement = f'{dest(a)} = {call}' if c == 2 else call
            elif op in (4, 57):
                expression = f'{rk(b)} == {rk(c)}' if op == 4 else f'{literal(constants[b])} < {rk(c)}'
                # When the predicate differs from A, HKS skips the next instruction.
                self.conditions[pc] = f'not ({expression})' if a else expression
                edges = [pc + 2, pc + 1]
            elif op == 28:
                edges = [pc + 2 + bx - 0x10000]
            elif op == 9:
                if b == 0:
                    raise ValueError('Open return needs an explicit adapter')
                statement = 'return ' + ', '.join(reg(i) for i in range(a, a + b - 1))
                edges = [self.exit]
            else:
                raise ValueError(f'Unsupported HKS fragment opcode {op} at {pc}')
            if any(target <= pc or target > self.end and target != self.exit for target in edges):
                raise ValueError(f'Nonlocal or backward fragment branch at {pc}: {edges}')
            self.edges[pc] = edges
            self.reads[pc] = read; self.writes[pc] = write
            self.registers.update(read | write)
            self.statements[pc] = statement
        self._validate_defined_registers()
        self.postdom = {self.exit: {self.exit}}
        for pc in reversed(range(self.end + 1)):
            self.postdom[pc] = {pc} | set.intersection(*(self.postdom[p] for p in self.edges[pc]))

    def _validate_defined_registers(self):
        incoming = {0: set(self.inputs)}
        for pc in range(self.end):
            if pc not in incoming:
                continue
            undefined = self.reads[pc] - incoming[pc]
            if undefined:
                raise ValueError(f'Undefined register(s) at {pc}: {sorted(undefined)}')
            defined = incoming[pc] | self.writes[pc]
            for target in self.edges[pc]:
                incoming[target] = incoming[target] & defined if target in incoming else defined.copy()

    def lua(self, indent='    '):
        def region(pc, stop, level):
            result = []
            while pc != stop and pc < self.end:
                statement = self.statements[pc]
                if statement:
                    result.append(level + statement)
                if pc in self.conditions:
                    yes, no = self.edges[pc]
                    common = self.postdom[yes] & self.postdom[no]
                    join = min(common)
                    result.append(level + 'if ' + self.conditions[pc] + ' then')
                    result += region(yes, join, level + '    ')
                    result.append(level + 'else')
                    result += region(no, join, level + '    ')
                    result.append(level + 'end')
                    pc = join
                else:
                    pc = self.edges[pc][0]
            return result
        result = [indent + 'do']
        names = ', '.join(f'nrhex_r{i}' for i in sorted(self.registers))
        if names:
            result.append(indent + '    local ' + names)
        for index, expression in sorted(self.inputs.items()):
            result.append(indent + f'    nrhex_r{index} = {expression}')
        result += region(0, self.exit, indent + '    ')
        # Entry arguments are required to survive unchanged; fragments that write
        # them need an explicit source adapter rather than implicit propagation.
        overwritten = set(self.inputs) & set.union(set(), *self.writes.values()) - set(self.outputs)
        if overwritten:
            raise ValueError(f'Fragment overwrites live inputs: {sorted(overwritten)}')
        for index, expression in sorted(self.outputs.items()):
            if index not in self.inputs:
                raise ValueError('Outputs require an initialized input register')
            result.append(indent + f'    {expression} = nrhex_r{index}')
        result.append(indent + 'end')
        return '\n'.join(result) + '\n'


TOKEN = re.compile(r'--\[(=*)\[[\s\S]*?\]\1\]|--[^\n]*|\[(=*)\[[\s\S]*?\]\2\]|'
                   r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|[A-Za-z_]\w*|[^\s]', re.M)


def functions(source):
    """Locate named global functions while ignoring comments and string contents."""
    result = {}
    active_headers = {token.start() for token in TOKEN.finditer(source) if token[0] == 'function'}
    for header in re.finditer(r'^function\s+(\w+)\s*\(([^)]*)\)[ \t]*\r?\n', source, re.M):
        if header.start() not in active_headers:
            continue
        name = header[1]
        stack = ['function']
        for token in TOKEN.finditer(source, header.end()):
            word = token[0]
            if word in ('function', 'if', 'do', 'repeat'):
                stack.append(word)
            elif word in ('end', 'until'):
                if not stack or (word == 'until') != (stack[-1] == 'repeat'):
                    raise ValueError(f'Unbalanced Lua block in {name}')
                stack.pop()
                if not stack:
                    args = [s.strip() for s in header[2].split(',') if s.strip()]
                    count = result.get(name, {}).get('count', 0) + 1
                    result[name] = dict(start=header.start(), body=header.end(), end=token.end(), args=args, count=count)
                    break
        else:
            raise ValueError(f'Unclosed Lua function: {name}')
    return result
