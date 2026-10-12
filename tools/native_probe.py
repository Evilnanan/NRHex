"""Read-only disassembly of current Nightreign reward entry points."""
import sys, re, json, hashlib, struct, bisect
from pathlib import Path
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from local_paths import GAME as GAME_DIR
GAME = GAME_DIR / 'nightreign.exe'
raw = GAME.read_bytes()
pe = pefile.PE(data=raw, fast_load=True)
base = pe.OPTIONAL_HEADER.ImageBase
md = Cs(CS_ARCH_X86, CS_MODE_64); md.detail = True
def read(rva, size):
    off = pe.get_offset_from_rva(rva)
    return raw[off:off+size]
def scan(pat):
    rx = re.compile(b''.join(b'.' if t == '?' else re.escape(bytes([int(t,16)])) for t in pat.split()),re.S)
    return [s.VirtualAddress + m.start() for s in pe.sections if s.Characteristics & 0x20000000 for m in rx.finditer(raw[s.PointerToRawData:s.PointerToRawData+s.SizeOfRawData])]
d = pe.OPTIONAL_HEADER.DATA_DIRECTORY[3]
pdata = read(d.VirtualAddress,d.Size)
funcs = [struct.unpack_from('<III',pdata,i) for i in range(0,len(pdata),12)]
starts = [f[0] for f in funcs]
def func(rva):
    ix=bisect.bisect_right(starts,rva)-1
    return funcs[ix][:2]
def dis(rva,size):
    for i in md.disasm(read(rva,size),base+rva):
        print(f'{i.address-base:08X} {i.mnemonic:8} {i.op_str}')
patterns = {
 'drop': '41 0f b6 e9 41 0f b6 f8 48 8b da 48 8b f1 33 c0 48 89 44 24 30',
 'mapitem': '48 8b c8 e8 ? ? ? ? 0f 28 00 66 0f 7f 44 24 50',
 'world': '48 8B 05 ? ? ? ? 0F 28 F1 48 85 C0',
}
if len(sys.argv)>1 and sys.argv[1]=='dis':
    dis(int(sys.argv[2],16),int(sys.argv[3],16) if len(sys.argv)>3 else 0x400)
elif len(sys.argv)>1 and sys.argv[1]=='calls':
    target = int(sys.argv[2],16)
    for s in pe.sections:
        if not s.Characteristics & 0x20000000: continue
        data=raw[s.PointerToRawData:s.PointerToRawData+s.SizeOfRawData]
        for m in re.finditer(b'\xe8',data):
            off=m.start(); rva=s.VirtualAddress+off
            if off+5 <= len(data) and rva+5+struct.unpack_from('<i',data,off+1)[0] == target:
                print('CALL',hex(rva),'FUNCTION',tuple(hex(x) for x in func(rva)))
                dis(rva-80,100)
else:
    result={'exeSha256':hashlib.sha256(raw).hexdigest(),'imageBase':base,'patterns':{}}
    for name,pat in patterns.items():
        hits=scan(pat);result['patterns'][name]={'pattern':pat,'rvas':hits}
        print(name,[hex(x) for x in hits])
        for hit in hits:
            start,end=func(hit);print('FUNCTION',hex(start),hex(end));dis(start,min(end-start,0x400))
    (Path(__file__).resolve().parents[1]/'.work/native-probe.json').write_text(json.dumps(result,indent=2),encoding='utf8')
