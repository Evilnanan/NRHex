"""Read-only signature and jump-chain audit of a running compatibility test."""
import argparse
import ctypes as c
import json
from pathlib import Path
import re
import struct

from capstone import Cs, CS_ARCH_X86, CS_MODE_64


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('pid', type=int)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    kernel = c.WinDLL('kernel32', use_last_error=True)
    psapi = c.WinDLL('psapi', use_last_error=True)
    kernel.OpenProcess.argtypes = [c.c_uint, c.c_int, c.c_uint]
    kernel.OpenProcess.restype = c.c_void_p
    kernel.ReadProcessMemory.argtypes = [c.c_void_p, c.c_void_p, c.c_void_p, c.c_size_t, c.POINTER(c.c_size_t)]
    kernel.CloseHandle.argtypes = [c.c_void_p]
    psapi.EnumProcessModules.argtypes = [c.c_void_p, c.c_void_p, c.c_uint, c.POINTER(c.c_uint)]
    psapi.GetModuleFileNameExW.argtypes = [c.c_void_p, c.c_void_p, c.c_void_p, c.c_uint]
    class Info(c.Structure):
        _fields_ = [('base', c.c_void_p), ('size', c.c_uint), ('entry', c.c_void_p)]
    psapi.GetModuleInformation.argtypes = [c.c_void_p, c.c_void_p, c.POINTER(Info), c.c_uint]
    handle = kernel.OpenProcess(0x410, 0, args.pid)  # QUERY_INFORMATION | VM_READ only
    if not handle:
        raise OSError(c.get_last_error(), 'OpenProcess')
    def read(address, size):
        buf = c.create_string_buffer(size)
        got = c.c_size_t()
        if not kernel.ReadProcessMemory(handle, address, buf, size, c.byref(got)):
            if size > 16:
                return read(address, 16)
            raise OSError(c.get_last_error(), f'ReadProcessMemory {address:x}')
        return buf.raw[:got.value]
    try:
        array = (c.c_void_p * 2048)()
        size = c.c_uint()
        assert psapi.EnumProcessModules(handle, array, c.sizeof(array), c.byref(size))
        modules = []
        for address in array[:size.value // c.sizeof(c.c_void_p)]:
            name = c.create_unicode_buffer(32768)
            psapi.GetModuleFileNameExW(handle, address, name, len(name))
            info = Info()
            assert psapi.GetModuleInformation(handle, address, c.byref(info), c.sizeof(info))
            modules.append(dict(path=name.value, base=info.base, size=info.size))
        base = next(m['base'] for m in modules if m['path'].lower().endswith('nightreign.exe'))
        source = (root / 'src/lib.rs').read_text()
        block = source.split('fn signatures()', 1)[1].split('\n}', 1)[0]
        entries = re.findall(r'\(\s*([A-Za-z_][\w:]*|"[^"]+")\s*,([^,()]+)\)', block)
        def constant(expr):
            expr = re.sub(r'\s+as\s+\w+', '', expr.strip())
            if expr.startswith('"'):
                return expr[1:-1]
            if re.fullmatch(r'0x[0-9a-fA-F]+|\d+', expr):
                return int(expr, 0)
            names = expr.split('::')
            text = source if len(names) == 1 else (root / 'src' / (names[0] + '.rs')).read_text()
            match = re.search(r'\bconst\s+' + re.escape(names[-1]) + r'\s*:[^=]+=(.*?);', text)
            if not match:
                raise ValueError(f'Cannot resolve {expr}')
            rhs = match[1].strip()
            return rhs[1:-1] if rhs.startswith('"') else int(rhs.replace('_', ''), 0)
        md = Cs(CS_ARCH_X86, CS_MODE_64)
        failures = []
        for pat, addr in entries:
            pattern, rva = constant(pat), constant(addr)
            tokens = pattern.split()
            actual = read(base + rva, len(tokens))
            if len(actual) == len(tokens) and all(t in ('?', '??') or actual[i] == int(t, 16) for i, t in enumerate(tokens)):
                continue
            chain = []
            at = base + rva
            for _ in range(5):
                data = read(at, 64)
                owner = next((m['path'] for m in modules if m['base'] <= at < m['base'] + m['size']), 'private memory')
                chain.append(dict(address=at, owner=owner, bytes=data.hex(), instructions=[f'{x.address:x}: {x.mnemonic} {x.op_str}' for x in md.disasm(data, at)][:10]))
                if data[0] == 0xe9:
                    at += 5 + struct.unpack_from('<i', data, 1)[0]
                elif data[:2] == b'\xff\x25':
                    at = struct.unpack('<Q', read(at + 6 + struct.unpack_from('<i', data, 2)[0], 8))[0]
                else:
                    break
            extra = {}
            if rva == 0x8b7fc0:
                mod = next((m for m in modules if m['path'].endswith('RestoredLegendaryStones.dll')), None)
                if mod:
                    trampoline = struct.unpack('<Q', read(mod['base'] + 0xfb028, 8))[0]
                    extra = dict(originalPointer=trampoline, trampolineBytes=read(trampoline, 32).hex())
                    print('Legendary Stones original trampoline:', hex(trampoline), extra['trampolineBytes'])
            failures.append(dict(pattern=pat, rva=rva, expected=pattern, actual=actual.hex(), chain=chain, extra=extra))
        output = root / 'work/compat/mmv/live/signatures.json'
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(dict(pid=args.pid, modules=modules, inspected=len(entries), failures=failures), indent=2))
        print(f'{len(entries)} signatures inspected; {len(failures)} differ')
        for failure in failures:
            print(hex(failure['rva']), failure['pattern'])
            for step in failure['chain']:
                print(step['owner'], *step['instructions'], sep='\n')
    finally:
        kernel.CloseHandle(handle)


if __name__ == '__main__':
    main()
