"""Decode and exercise the serialized native-key and personal-quota branches."""
from pathlib import Path
import json, struct

ROOT = Path(__file__).resolve().parents[1]
OPS = {(o['bank'], o['id']): (o['name'], [t for t, _ in o['args']])
       for o in json.loads((ROOT/'data/reference/world/cage-opcodes.json').read_text(encoding='utf8'))['opcodes']}
OPS.update({(3,12): ('IfEventValue',['sbyte','uint','byte','byte','uint']),
 (2003,127): ('IncrementEventValue',['uint','uint','uint']),
 (1000,101): ('GotoIfConditionGroupStateUncompiled',['byte','byte','sbyte']),
 (2009,4): ('ActivateMultiplayerdependantBuffs',['uint']),
 (2004,28): ('SetNetworkUpdateAuthority',['uint','int'])})
for label in range(10): OPS[1014,label] = (f'Label{label}', [])
FORMATS = dict(sbyte='b',byte='B',bool='B',uint='I',int='i',float='f',ushort='H',short='h')

def decode(ins):
    name, types = OPS[ins['Bank'],ins['ID']]
    raw = bytes.fromhex(ins['hex']); at = 0; values = []
    for t in types:
        size = struct.calcsize(FORMATS[t]); at = (at+size-1)//size*size
        values.append(struct.unpack_from('<'+FORMATS[t],raw,at)[0]); at += size
    assert len(raw)==(at+3)//4*4 or ins['Bank']==2000 and ins['ID'] in (0,6)
    return name, values

def bind(event, parameters):
    instructions = []
    for index, ins in enumerate(event['instructions']):
        raw = bytearray.fromhex(ins['hex'])
        for p in event['Parameters']:
            if p['InstructionIndex']==index:
                t,s,n = p['TargetStartByte'],p['SourceStartByte'],p['ByteCount']
                assert s+n<=len(parameters)
                raw[t:t+n] = parameters[s:s+n]
        instructions.append(decode(dict(ins,hex=raw.hex())))
    return instructions

def opening(event, parameters, owner=True, keys=0, count=0, busy=False,
            request=False, normal_open=False, night=False, player=1):
    opened,stone,variation,log,used,busy_flag,request_flag,counter = struct.unpack('<8I',parameters[:32])
    code = bind(event,parameters)
    labels = {int(n[5:]):i for i,(n,_) in enumerate(code) if n.startswith('Label')}
    flags = {busy_flag:busy,request_flag:request,opened:normal_open,7509:night}
    wide = len(parameters)>32
    if wide:
        for bit in range(32): flags[counter+bit]=bool(count & (1<<bit))
    writes=[]; groups = {}; pc=0; removed=0; messages=[]; increments=0
    def group(g,v):groups[g]=(groups.get(g,False) or v) if g<0 else (groups.get(g,True) and v)
    def finish():return dict(request=bool(flags.get(request_flag)),opened=bool(flags.get(opened)),removed=removed,count=count,messages=messages,increments=increments,writes=writes)
    for _ in range(1000):
        n,a=code[pc];pc+=1
        if n=='IfIsMapVariation':group(a[0],a[1]==1 and a[2]==variation)
        elif n=='IfActionButtonInArea':group(a[0],True)
        elif n=='IfEventFlag':group(a[0],bool(flags.get(a[3],False))==bool(a[1]))
        elif n=='IfConditionGroup':
            v=groups.get(a[2],False)==bool(a[1])
            if a[0]==0:
                if not v:return finish()
                groups.clear()
            else:group(a[0],v)
        elif n=='EndIfConditionGroupStateUncompiled':
            if groups.get(a[2],False)==bool(a[1]):return finish()
        elif n=='EndIfEventFlag':
            if bool(flags.get(a[3],False))==bool(a[1]):return finish()
        elif n=='IfEventValue':
            assert a[1:]==[counter,3,4,5];group(a[0],count>=5)
        elif n=='GotoIfConditionGroupStateUncompiled':
            if groups.get(a[2],False)==bool(a[1]):pc=labels[a[0]]+1
        elif n=='SkipIfCharacterHasSpEffect':
            if owner==bool(a[3]):pc+=a[0]
        elif n=='SkipIfEventFlag':
            if bool(flags.get(a[3],False))==bool(a[1]):pc+=a[0]
        elif n=='SkipUnconditionally':pc+=a[0]
        elif n=='GotoUnconditionally':pc=labels[a[0]]+1
        elif n=='IfPlayerHasdoesntHaveItem':group(a[0],keys>0)
        elif n=='SkipIfConditionGroupStateUncompiled':
            if groups.get(a[2],False)==bool(a[1]):pc+=a[0]
        elif n=='SkipIfIsPlayerNo':
            if (player==a[1])==bool(a[2]):pc+=a[0]
        elif n=='SetNetworkconnectedEventFlag':flags[a[1]]=bool(a[2]);writes.append((a[1],bool(a[2])))
        elif n=='IncrementEventValue':
            assert a==([counter,32,0xffffffff] if wide else [counter,3,5]);count=min(count+1,a[2]);increments+=1
        elif n=='RemoveItemFromPlayer':removed+=a[2]
        elif n=='RecordUserDispLog':messages.append(a)
        elif n=='EndUnconditionally':return finish()
        elif n in ['MmvOriginalMapGuard','SetNetworkSyncState','IfElapsedSeconds','WaitFixedTimeSeconds','SetWorldMapPointFlag','ForceAnimationPlayback','WaitFixedTimeFrames'] or n.startswith('Label'):pass
        else:raise AssertionError((n,a))
    raise AssertionError('Opener failed to yield')

def verify_open(event, parameters):
    cases=[]
    for player in (1,2,3):
        for owner in (False,True):
            for keys in (0,1):
                for count in range(6):
                    r=opening(event,parameters,owner,keys,count,player=player)
                    custom=owner and count<5
                    assert r['request']==custom and r['opened']==(not custom and keys>0)
                    assert r['removed']==int(not custom and keys>0)
                    assert r['count']==count+int(custom) and r['increments']==int(custom)
                    assert len(r['messages'])==1
                    assert r['messages'][0][0]==(11240 if custom or keys else 10108)
                    cases.append(r)
    for cause in ['busy','request','normal_open','night']:
        r=opening(event,parameters,keys=1,**{cause:True})
        assert r['increments']==r['removed']==0 and not r['messages']
    assert event['RestBehavior']==1
    return cases
