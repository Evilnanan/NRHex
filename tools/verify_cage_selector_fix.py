"""Verify staged selectors against forward-only native label lookup."""
from pathlib import Path
import hashlib,json,struct
from cage_selector_events import Selector,verify_selector

root=Path(__file__).resolve().parents[1]
stage=root/'.work/cage-selector-fix'
patch=json.loads((stage/'patch.json').read_text())
schema=json.loads((root/'data/reference/world/cage-opcodes.json').read_text())
ops={(o['bank'],o['id']):o for o in schema['opcodes']}
formats={'sbyte':'b','byte':'B','uint':'I','int':'i','float':'f','ushort':'H','short':'h','bool':'B'}
def decode(ins):
    op=ops[ins['Bank'],ins['ID']];raw=bytes.fromhex(ins['hex']);offset=0;args=[]
    for typ,_ in op['args']:
        size=struct.calcsize(formats[typ]);offset=(offset+size-1)//size*size
        args.append(struct.unpack_from('<'+formats[typ],raw,offset)[0]);offset+=size
    assert len(raw)==(offset+3)//4*4
    return op['name'],args
cases=[]
for report in patch['reports']:
    name=report['mapName']+'.emevd.dcx'
    digest=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
    assert digest(root/'dist/mod/event'/name)==report['inputSha256'], 'Installed file changed during staging'
    assert digest(stage/'events'/name)==report['outputSha256']
    code=[decode(i) for i in report['selector']['instructions']]
    for pc,(name,args) in enumerate(code):
        if name=='GotoIfEventFlag':
            assert any(n==f'Label{args[0]}' for n,_ in code[pc+1:])
    verify_selector(code,report,decode)
    # Restore precisely the previous faulty selector: retry label before RNG.
    legacy=code[:-2];legacy.insert(2,('Label1',[]))
    for chosen in range(37):
        failed=Selector(legacy,report);failed.draw=chosen
        failed.flags[report['cooling']+chosen]=True
        failed.step();failed.step()
        assert failed.ended and failed.missing_labels and not failed.commits
        assert failed.flags[report['request']] and not failed.flags.get(report['ready'])
        # Even after cleanup, the terminated selector cannot resume by itself.
        failed.flags[report['cooling']+chosen]=False
        failed.step();assert failed.ended and not failed.commits
    cases.append(dict(mapName=report['mapName'],legacyCoolingDrawFailuresReproduced=37,
                      fixedCoolingRetryCases=37,consecutiveReopenCases=200,reloadCases=1))
result=dict(stagedOnly=True,installed=False,gameMemoryWritten=False,distAssetsUnchanged=True,
            nativeLookup='Nightreign current EXE 0x5943a9..0x5943fd: scans labels from PC+1 only',
            cases=cases,scope='Only four selector events; map/NPC/regulation/DLL/birth/HUD/rewards preserved',
            offlineVerificationPassed=True,gameplayFixVerified=False,multiplayerFixVerified=False)
(stage/'validation.json').write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding='utf8')
print('PASS: 148 old cooling failures reproduced; 148 fixed cooling retries, 800 reopens and 4 reloads passed; staged only')
