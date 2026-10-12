"""Read original group encounters and resolve their actual event argument bindings.

Run HexBuild --cage-group-research first. This writes research evidence only.
"""
from pathlib import Path
import hashlib,json,struct
from local_paths import SMITHBOX

root=Path(__file__).resolve().parents[1]
work=root/'.work/cage-groups'
load=lambda p:json.loads(p.read_text(encoding='utf8'))
events={p.stem:{e['ID']:e for e in load(p)} for p in (work/'events').glob('*.json')}
maps={p.stem:load(p) for p in (work/'maps').glob('*.json')}
names={e['ID']:e['Name'] for e in load(SMITHBOX/'Assets/Aliases/NR/Characters.json')}
manifest=load(work/'manifest.json')
assert len({(r['kind'],r['name']) for r in manifest})==len(manifest)
for r in manifest:
    suffix='.emevd.dcx' if r['kind']=='event' else '.msb.dcx'
    folder='event' if r['kind']=='event' else 'map/mapstudio'
    candidates=[root/'assets/original'/folder/(r['name']+suffix)]
    candidates += [work/d/folder/(r['name']+suffix) for d in ['extracted','field-extracted']]
    p=next(p for p in candidates if p.exists())
    assert hashlib.sha256(p.read_bytes()).hexdigest()==r['sha256']

schema={(o['bank'],o['id']):o for o in load(root/'data/reference/world/cage-opcodes.json')['opcodes']}
def opcode(bank,op,name,types):
    schema[bank,op]=dict(name=name,args=[[t,str(n)] for n,t in enumerate(types)])
opcode(2003,102,'LinkToBossHealthBar',['sbyte','int','uint'])
opcode(4,23,'IfCharacterRatioHPValue',['sbyte','uint','sbyte','int','sbyte','float'])
opcode(4,15,'IfCharacterRatioDeadalive',['sbyte','uint','byte','sbyte','float'])
opcode(4,2,'IfCharacterHPRatio',['sbyte','uint','sbyte','float','sbyte','float'])
opcode(4,16,'IfCharacterRatioHPRatio',['sbyte','uint','sbyte','float','sbyte','float'])
fmts=dict(sbyte='b',byte='B',bool='B',uint='I',int='i',short='h',ushort='H',float='f')
def decode(i):
    raw=bytes.fromhex(i['hex']);s=schema.get((i['Bank'],i['ID']))
    if s is None:return dict(bank=i['Bank'],id=i['ID'],hex=i['hex'])
    values=[];at=0
    for t,_ in s['args']:
        f=fmts[t];size=struct.calcsize(f);at=(at+size-1)//size*size
        values.append(struct.unpack_from('<'+f,raw,at)[0]);at+=size
    return dict(name=s['name'],args=values)
def bind(event,args):
    ins=[dict(i) for i in event['instructions']]
    for p in event['Parameters']:
        ix=p['InstructionIndex'];target=p['TargetStartByte'];source=p['SourceStartByte'];n=p['ByteCount']
        assert source+n<=len(args)
        raw=bytearray.fromhex(ins[ix]['hex']);assert target+n<=len(raw)
        raw[target:target+n]=args[source:source+n];ins[ix]['hex']=raw.hex().upper()
    return [dict(pc=pc,**decode(i)) for pc,i in enumerate(ins)]
def calls(map_name):
    for e in events[map_name].values():
        for pc,i in enumerate(e['instructions']):
            raw=bytes.fromhex(i['hex'])
            if i['Bank']==2000 and i['ID'] in [0,6] and len(raw)>=8:
                slot,eid=struct.unpack_from('<II',raw)
                yield dict(caller=e['ID'],pc=pc,slot=slot,event=eid,args=raw[8:])
def members(m,target):
    assert target!=0
    return [dict(e,modelName=names.get(e['ModelName'])) for e in maps[m]['enemies'] if target==e['entity'] or target in e['groups']]
def bound(m,c):
    e=events[m].get(c['event']) or events['common_func'].get(c['event'])
    assert e is not None
    return dict(caller=c['caller'],pc=c['pc'],slot=c['slot'],event=c['event'],argumentHex=c['args'].hex(),instructions=bind(e,c['args']))

encounters=[]
for m in sorted(maps):
    if m not in events:continue
    by_target={}
    for c in calls(m):
        if c['caller']!=0 or c['event'] not in [90015000,90015021,90015023]:continue
        args=c['args'];target=struct.unpack_from('<I',args,12 if c['event']==90015023 else 4)[0]
        group=members(m,target)
        if len(group)<=1:continue
        hud=bound(m,c)
        key=(c['event'],args)
        if key in by_target:
            by_target[key]['initializerOccurrences']+=1;continue
        if c['event']==90015023:
            bars=[dict(entity=struct.unpack_from('<I',args,off)[0],name=struct.unpack_from('<I',args,off+4)[0]) for off in [16,24,32] if struct.unpack_from('<I',args,off)[0]]
        else:bars=[dict(entity=target,name=struct.unpack_from('<I',args,8)[0])]
        row=dict(map=m,target=target,members=group,hudEvent=c['event'],hudBars=bars,
                 mode='separate' if len(bars)>1 else 'group',initializerOccurrences=1,hud=hud)
        by_target[key]=row;encounters.append(row)

for m in ['m46_50_00_00','m46_60_00_00','m46_80_00_00']:
    for c in calls(m):
        if c['caller']!=0 or c['event'] not in [90045001,90045002]:continue
        target=struct.unpack_from('<I',c['args'],12 if c['event']==90045001 else 8)[0]
        group=members(m,target)
        if len(group)<=1:continue
        existing=next((r for r in encounters if r['map']==m and r['target']==target),None)
        if existing is None:
            assert m=='m46_80_00_00' and target==46805800
            bars=[];hud=[]
            for h in calls(m):
                if h['caller']==0 and h['event']==90015000:
                    entity,name=struct.unpack_from('<II',h['args'],4)
                    if entity in [e['entity'] for e in group]:bars.append(dict(entity=entity,name=name));hud.append(bound(m,h))
            assert len(bars)==2
            existing=dict(map=m,target=target,members=group,hudEvent=90015000,hudBars=bars,mode='separate',hud=hud)
            encounters.append(existing)
        existing.setdefault('lifecycle',[]).append(bound(m,c))

for r in encounters:
    if r['map'].startswith(('m48','m49','m52')):
        for c in calls(r['map']):
            if c['caller']==0 and c['event']==90065911 and struct.unpack_from('<I',c['args'],8)[0]==r['target']:
                r.setdefault('lifecycle',[]).append(bound(r['map'],c))
    r['treasures']=maps[r['map']]['treasures']
if (work/'npcs.json').exists():
    params=load(work/'npcs.json')
    assert params['sourceRegulationSha256'].lower()=='876a3ca279a4561d0c69f81fe5e510c75c59ab9a8a201694f8cf4a42c91e0268'
    rows={r['ID']:r['fields'] for r in params['rows']}
    for r in encounters:
        for e in r['members']:e['npcFields']=rows[e['NpcParamId']]

# Concrete evidence checks, including actual argument substitution rather than
# assuming every occurrence of the same model shares a native encounter.
pick=lambda m,target:next(r for r in encounters if r['map']==m and r['target']==target)
crystal=pick('m46_60_00_00',46605820)
assert len(crystal['members'])==3 and crystal['hudBars']==[dict(entity=46605820,name=903350323)]
assert any(i.get('name')=='LinkToBossHealthBar' and i['args']==[1,903350323,46605820] for i in crystal['hud']['instructions'])
death=next(e for e in crystal['lifecycle'] if e['event']==90045002)
assert death['instructions'][3]==dict(pc=3,name='IfCharacterRatioHPValue',args=[0,46605820,5,0,0,1.0])
godskin=pick('m46_80_00_00',46805800)
assert godskin['hudBars']==[dict(entity=46800820,name=903560000),dict(entity=46800821,name=903570000)]
assert len(pick('m48_50_00_00',48505800)['hudBars'])==3
assert len(pick('m49_28_00_00',49285800)['hudBars'])==2
assert len(pick('m60_43_38_10',1038105800)['members'])==2
assert len(pick('m60_42_38_10',1028105820)['members'])==4
payload=dict(sourceManifestSha256=hashlib.sha256((work/'manifest.json').read_bytes()).hexdigest(),
             opcodeReference='https://soulsmods.github.io/emedf/nr-emedf.html',
             scope='Original assets inspected; group encounter runtime not implemented or gameplay-tested',encounters=encounters)
(work/'encounters.json').write_text(json.dumps(payload,ensure_ascii=False,indent=2),encoding='utf8')
for r in encounters:
    print(r['map'],r['target'],r['mode'],len(r['members']),'members',len(r['hudBars']),'bars')
print('PASS: source hashes, native bindings, shared gaol HUD/all-member death gate, separate gaol/tower HUD and field groups verified; package untouched')
