"""Read-only native function/xref evidence for the peer special-effect display."""
from pathlib import Path
import bisect, hashlib, json, re, struct, sys
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from local_paths import GAME, ROOT

raw = (GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest() == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe = pefile.PE(data=raw, fast_load=True)
base = pe.OPTIONAL_HEADER.ImageBase
md = Cs(CS_ARCH_X86, CS_MODE_64)
md.detail = True
directory = pe.OPTIONAL_HEADER.DATA_DIRECTORY[3]
pdata = pe.get_data(directory.VirtualAddress, directory.Size)
functions = [struct.unpack_from('<III', pdata, i) for i in range(0, len(pdata), 12)]
starts = [f[0] for f in functions]
output = ROOT/'.work/effect-sync'
output.mkdir(parents=True, exist_ok=True)


def function(rva):
    index = bisect.bisect_right(starts, rva)-1
    begin, end, _ = functions[index]
    return (begin, end) if begin <= rva < end else (rva, rva+0x100)


def disassembly(rva, size=None):
    begin, end = function(rva)
    if size is not None:
        begin, end = rva, rva+size
    return '\n'.join(f'{i.address-base:08x} {i.mnemonic:8} {i.op_str}' for i in md.disasm(pe.get_data(begin, end-begin), base+begin))


def callers(target):
    found = []
    for section in pe.sections:
        if not section.Characteristics & 0x20000000:
            continue
        code = raw[section.PointerToRawData:section.PointerToRawData+section.SizeOfRawData]
        for match in re.finditer(b'\xe8', code):
            offset = match.start()
            rva = section.VirtualAddress+offset
            if offset+5 <= len(code) and rva+5+struct.unpack_from('<i', code, offset+1)[0] == target:
                # Check the opcode is on a decoded instruction boundary.
                begin, end = function(rva)
                if any(i.address == base+rva and i.mnemonic == 'call' for i in md.disasm(pe.get_data(begin, end-begin), base+begin)):
                    found.append(dict(call=f'{rva:x}', function=f'{begin:x}', end=f'{end:x}'))
    return found


if sys.argv[1] == 'calls':
    target = int(sys.argv[2], 16)
    found = callers(target)
    (output/f'calls-{target:x}.json').write_text(json.dumps(found, indent=2), encoding='utf8')
    print(json.dumps(found))
elif sys.argv[1] == 'dis':
    for value in sys.argv[2].split(','):
        rva = int(value, 16)
        text = disassembly(rva)
        (output/f'{rva:x}.asm').write_text(text, encoding='utf8')
        print(f'{rva:x}: {function(rva)[1]-function(rva)[0]} bytes; .work/effect-sync/{rva:x}.asm')
elif sys.argv[1] == 'refs':
    target = int(sys.argv[2], 16)
    found = []
    for section in pe.sections:
        if not section.Characteristics & 0x20000000: continue
        code = raw[section.PointerToRawData:section.PointerToRawData+section.SizeOfRawData]
        for match in re.finditer(rb'[\x48-\x4f][\x8b\x8d][\x05\x0d\x15\x1d\x25\x2d\x35\x3d]', code):
            offset = match.start(); rva = section.VirtualAddress+offset
            if offset+7 <= len(code) and rva+7+struct.unpack_from('<i',code,offset+3)[0] == target:
                found.append(dict(reference=f'{rva:x}',function=f'{function(rva)[0]:x}'))
    (output/f'refs-{target:x}.json').write_text(json.dumps(found,indent=2),encoding='utf8')
    print(json.dumps(found))
elif sys.argv[1] == 'field':
    target = int(sys.argv[2],16); found = {}
    for section in pe.sections:
        if not section.Characteristics & 0x20000000: continue
        code = raw[section.PointerToRawData:section.PointerToRawData+section.SizeOfRawData]
        for match in re.finditer(re.escape(struct.pack('<I',target)),code):
            offset=match.start()
            for start in range(max(0,offset-9),offset):
                instruction=next(md.disasm(code[start:offset+4],base+section.VirtualAddress+start),None)
                if instruction and instruction.address+instruction.size==base+section.VirtualAddress+offset+4 and any(o.type==3 and o.mem.disp==target for o in instruction.operands):
                    rva=instruction.address-base
                    found[section.VirtualAddress+offset]=dict(instruction=f'{rva:x}',function=f'{function(rva)[0]:x}',assembly=f'{instruction.mnemonic} {instruction.op_str}')
                    break
    records=list(found.values());(output/f'field-{target:x}.json').write_text(json.dumps(records,indent=2),encoding='utf8')
    print(json.dumps(records[:90]));print('Total candidates',len(records))
elif sys.argv[1] == 'strings':
    strings = []
    for match in re.finditer(rb'[ -~]{8,}', raw):
        value = match.group().decode('ascii')
        if re.search(r'SpEffect|SpecialEffect|PermanentBuff|StatusMenu', value, re.I):
            strings.append(dict(offset=f'{match.start():x}', value=value[:300]))
    (output/'strings.json').write_text(json.dumps(strings, indent=2), encoding='utf8')
    print(json.dumps(strings))
else:
    raise ValueError('Use calls RVA, dis RVAS_CSV or strings')
