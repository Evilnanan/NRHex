"""Read-only checks for the native hero-bank insertion hook and Restage audio."""
from pathlib import Path
import hashlib,json,re,struct,pefile

root=Path(__file__).resolve().parents[1]
game=Path(root.anchor)/'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game'
exe=game/'nightreign.exe'
pe=pefile.PE(str(exe),fast_load=True)
callers=[]
for section in pe.sections:
    if not section.Characteristics&0x20000000:continue
    data=section.get_data()
    for match in re.finditer(b'\xe8',data):
        offset=match.start()
        if offset+5<=len(data) and section.VirtualAddress+offset+5+struct.unpack_from('<i',data,offset+1)[0]==0xeed260:
            callers.append(section.VirtualAddress+offset)
assert callers==[0xef8bd0,0xef8c5d,0xef8c68,0xef8d5d],callers
# All four direct calls belong to EF8B20. The training branch adds Duchess (4)
# and Revenant (6); ordinary branches insert native local/team hero IDs.
assert pe.get_data(0xef8c57,6)==bytes.fromhex('b204488d4dd7')
assert pe.get_data(0xef8c62,6)==bytes.fromhex('b206488d4dd7')

def fnv(name):
    result=2166136261
    for byte in name.lower().encode('ascii'):result=((result*16777619)^byte)&0xffffffff
    return result

def events(path):
    data=path.read_bytes();offset=0;result=set()
    while offset<len(data):
        kind=data[offset:offset+4];size=struct.unpack_from('<I',data,offset+4)[0]
        body=data[offset+8:offset+8+size];offset+=8+size
        assert len(body)==size
        if kind==b'HIRC':
            pos=4
            for _ in range(struct.unpack_from('<I',body)[0]):
                typ=body[pos];length=struct.unpack_from('<I',body,pos+1)[0]
                ident=struct.unpack_from('<I',body,pos+5)[0]
                if typ==4:result.add(ident)
                pos+=5+length
            assert pos==len(body)
    assert offset==len(data)
    return result

event=fnv('Play_s000407000')
banks=list((game/'sd').glob('hero*.bnk'))+[game/'sd/cs_main.bnk',game/'sd/cs_smain.bnk']
owners=[p.name for p in banks if event in events(p)]
assert owners==['hero04.bnk'],owners
out=root/'.work/escape-audio';out.mkdir(parents=True,exist_ok=True)
report=dict(exeSha256=hashlib.sha256(exe.read_bytes()).hexdigest(),hookRva='EED260',nativeSelectorRva='EF8B20',
            nativeDirectCallers=[hex(c) for c in callers],nativeEvent='Play_s000407000',eventId=event,
            nativeSoundBank=owners[0],nativeSoundBankBytes=(game/'sd/hero04.bnk').stat().st_size,
            nativeSoundBankSha256=hashlib.sha256((game/'sd/hero04.bnk').read_bytes()).hexdigest(),
            staticChecksPassed=True,gameplayVerified=False)
(out/'validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print('PASS: four native hero selector call sites verified; Restage sound exists in hero04, absent from shared banks.')

parry_name='Play_c000809000';parry_event=fnv(parry_name)
parry_banks=[p.name for p in banks if parry_event in events(p)]
assert parry_banks==['hero08.bnk'],parry_banks
parry_out=root/'.work/parry-fix';parry_out.mkdir(parents=True,exist_ok=True)
(parry_out/'audio-validation.json').write_text(json.dumps(dict(
    exeSha256=report['exeSha256'],nativeEvent=parry_name,eventId=parry_event,
    nativeSoundBank=parry_banks[0],nativeSoundBankSha256=hashlib.sha256((game/'sd/hero08.bnk').read_bytes()).hexdigest(),
    sceneOwnsBankLifetime=True,gameplayVerified=False),indent=2),encoding='utf8')
print('PASS: Executor deflect event 809000 belongs to hero08; scene-owned loading can share the existing selector hook.')
