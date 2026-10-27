"""Exercise serialized opening/buff events and exact native HUD/stack predicates.

No process attachment or game writes. This does not certify the in-game renderer
or multiplayer timing; it checks generated control flow and native aggregation.
"""
import ctypes as c
import hashlib
import json
import math
from pathlib import Path
import struct
import sys
import pefile
from cage_usage_events import OPS, bind, decode, opening
from local_paths import GAME

OPS.update({(3,20): ('CompareValues',['sbyte','uint','byte','byte','uint','byte']),
            (1012,3): ('MmvOriginalMapGuard',['uint']),
            (2003,32): ('ClearValue',['uint','uint'])})


class BuffVM:
    def __init__(self, event, layout, encounter, stacks, shown=True):
        self.code=bind(event,b''); self.pc=0; self.groups={}; self.flags={}
        self.labels={int(n[5:]):i for i,(n,_) in enumerate(self.code) if n.startswith('Label')}
        self.layout=layout; self.encounter=encounter
        self.flags.update({layout['prefix']+7403:True,encounter['choice']:True})
        self.flags.update({f:shown for f in encounter['shown']})
        self.values={layout['shared']:stacks,layout['scratch']:999}
        self.effects={a:17 for a in encounter['members']}  # stale actor state
    def group(self,g,v):
        self.groups[g]=(self.groups.get(g,False) or v) if g<0 else (self.groups.get(g,True) and v)
    def run(self, budget=20000):
        for _ in range(budget):
            n,a=self.code[self.pc];self.pc+=1
            if n in ('SetNetworkSyncState','WaitFixedTimeFrames') or n.startswith('Label'):pass
            elif n=='IfEventFlag':
                v=bool(self.flags.get(a[3],False))==bool(a[1])
                if a[0]==0:
                    if not v:self.pc-=1;return 'waiting'
                    self.groups.clear()
                else:self.group(a[0],v)
            elif n=='IfConditionGroup':
                v=self.groups.get(a[2],False)==bool(a[1])
                if a[0]==0:
                    if not v:self.pc=0;self.groups.clear();return 'waiting'
                    self.groups.clear()
                else:self.group(a[0],v)
            elif n=='GotoIfEventFlag':
                if bool(self.flags.get(a[3],False))==bool(a[1]):self.pc=self.labels[a[0]]+1
            elif n=='GotoIfConditionGroupStateUncompiled':
                if self.groups.get(a[2],False)==bool(a[1]):self.pc=self.labels[a[0]]+1
            elif n=='CompareValues':
                assert a[2]==a[5]==32 and a[3]==5
                self.group(a[0],self.values[a[1]]<=self.values[a[4]])
            elif n=='ClearValue':self.values[a[0]]=0
            elif n=='IfCharacterHasSpEffect':self.group(a[0],(self.effects[a[1]]>0)==bool(a[3]))
            elif n=='ClearSpEffect':assert a[1]==95001604;self.effects[a[0]]=max(0,self.effects[a[0]]-1)
            elif n=='SetSpEffect':assert a[1]==95001604;self.effects[a[0]]+=1
            elif n=='SetEventFlag':self.flags[a[1]]=bool(a[2])
            elif n=='IncrementEventValue':self.values[a[0]]=min(self.values[a[0]]+1,a[2])
            elif n=='EndUnconditionally':
                assert a==[1];self.pc=0;self.groups.clear()
            else:raise AssertionError((n,a))
        raise AssertionError('Buff event did not yield')


def event_checks(report):
    opener=report['reports'][0]['events'][0]
    cases=0
    workers=set()
    for r in report['reports'][1:]:
        layout=r['layout']; p=layout['prefix']; events={e['ID']:e for e in r['events']}
        ctor=events[0]
        args=[bytes.fromhex(i['hex'])[8:] for i in ctor['instructions'] if i['Bank']==2000 and i['ID']==6 and struct.unpack_from('<I',bytes.fromhex(i['hex']),4)[0]==9100961]
        assert args and all(len(a)==288 for a in args)
        for a in args:
            for owner in (False,True):
                for keys in (0,1):
                    for count in (0,1,2,4,5,6,31,32,255,1024,65536):
                        for player in (1,2,3):
                            result=opening(opener,a,owner,keys,count,player=player)
                            assert result['request']==owner
                            assert result['count']==count+int(owner)
                            assert result['removed']==int(not owner and keys>0)
                            assert result['opened']==(not owner and keys>0)
                            if owner:
                                writes=result['writes']; flags={}
                                assert len(writes)==33 and len({f for f,v in writes})==33
                                # Actual network encoding sends all ON before OFF.
                                for flag,value in sorted(writes,key=lambda x:not x[1]):flags[flag]=value
                                assert sum(int(flags[layout['shared']+bit])<<bit for bit in range(32))==count
                            cases+=1
            for guard in ('busy','request','normal_open','night'):
                result=opening(opener,a,keys=1,count=9,**{guard:True})
                assert result['increments']==0 and not result['messages']
        for enc in layout['encounters']:
            assert p+8500<=enc['id']<p+9000 and enc['id'] not in workers
            workers.add(enc['id'])
            event=events[enc['id']]
            for stacks in (0,1,2,5,7,32,100):
                vm=BuffVM(event,layout,enc,stacks,shown=False)
                assert vm.run()=='waiting' and set(vm.effects.values())=={17}
                vm.flags.update({f:True for f in enc['shown']})
                assert vm.run()=='waiting' and set(vm.effects.values())=={stacks}
                assert vm.flags[layout['done']]
                vm.run();assert set(vm.effects.values())=={stacks} # no duplicate frame applications
                # Reload: every local birth clears phase flags before shown.
                vm.pc=0;vm.groups.clear();vm.flags[layout['initialized']]=False;vm.flags[layout['done']]=False
                vm.run();assert set(vm.effects.values())=={stacks}
                vm.flags[enc['choice']]=False;vm.run();assert not vm.flags[layout['done']]
                cases+=1
            # Battle starts AI only after this peer's buff loop finishes.
            code=events[enc['battle']]['instructions']
            gate=next(i for i,ins in enumerate(code) if (ins['Bank'],ins['ID'])==(3,0) and decode(ins)[1]==[0,1,0,layout['done']])
            ai=next(i for i,ins in enumerate(code) if (ins['Bank'],ins['ID'])==(2004,1) and bytes.fromhex(ins['hex'])[4]==1)
            assert gate<ai
        # Separate map instances and peers cannot share local storage buckets.
        assert layout['counter']//1000%10==layout['scratch']//1000%10==5
        assert 5800<=layout['scratch']%10000<=5966
        assert layout['shared']//1000%10==7
    return cases


def native_checks(fixture_path):
    raw=(GAME/'nightreign.exe').read_bytes()
    assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
    pe=pefile.PE(data=raw,fast_load=True)
    k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p
    k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
    memory=k.VirtualAlloc(None,8192,0x3000,0x40);assert memory
    try:
        code=pe.get_data(0x510b20,0x2b8);assert code[-1]==0xc3;c.memmove(memory,code,len(code))
        row=c.create_string_buffer(0x400);node=c.create_string_buffer(0x88)
        row[0x353]=b'\x08'  # displayIconForPermamentEffect, required for -1 duration
        struct.pack_into('<Q',node,0,c.addressof(row));struct.pack_into('<I',node,8,95001604)
        struct.pack_into('<H',row,0x13e,10)
        same_id=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p,c.c_uint)(memory)
        same_state=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p,c.c_uint)(memory+0x50)
        same_category=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p,c.c_uint,c.c_uint)(memory+0xb0)
        assert same_id(c.addressof(node),95001604)==0
        assert same_state(c.addressof(node),0)==0
        assert same_category(c.addressof(node),10,0)==0
        struct.pack_into('<H',row,0x13e,20);assert same_id(c.addressof(node),95001604)==1
        struct.pack_into('<H',row,0x13e,10)
        code=pe.get_data(0x7deb30,0x2d4);assert code[:10].hex()=='40534883ec40f3410f10'
        c.memmove(memory+4096,code,len(code))
        aggregate=c.WINFUNCTYPE(c.c_void_p,c.c_void_p,c.c_void_p,c.c_void_p)(memory+4096)
        entries=c.create_string_buffer(128*20);vector=c.create_string_buffer(0x30);first=c.addressof(entries)
        counts=[0,1,2,5,6,10,50,100,255]
        for count in counts:
            struct.pack_into('<QQQ',vector,0x10,first,first,first+len(entries))
            for _ in range(count):
                for index,icon in enumerate((20406,20408)):
                    item=c.create_string_buffer(struct.pack('<iiifi',95001604,icon,index,-1,1))
                    aggregate(c.addressof(vector),c.addressof(node),c.addressof(item))
            end=struct.unpack_from('<Q',vector,0x18)[0]
            assert end-first==(40 if count else 0)
            if count:
                assert struct.unpack_from('<iiifi',entries,0)==(95001604,20406,0,-1,count)
                assert struct.unpack_from('<iiifi',entries,20)==(95001604,20408,1,-1,count)
        # Execute the original offensive effect calculator with the generated
        # PARAM bytes, not a restatement of the intended power formula.
        image=k.VirtualAlloc(None,pe.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert image
        attack_cases=0
        try:
            for section in pe.sections:
                if section.SizeOfRawData:c.memmove(image+section.VirtualAddress,raw[section.PointerToRawData:section.PointerToRawData+section.SizeOfRawData],section.SizeOfRawData)
            c.memmove(image+0xe6d630,bytes.fromhex('48 8B C1 C3'),4) # absent PARAM manager lookup
            calc=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p,c.c_int,c.c_int)(image+0x502830)
            defense=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p,c.c_void_p,c.c_void_p,c.c_void_p)(image+0x503800)
            fixture=fixture_path.read_bytes();offset=struct.unpack_from('<Q',fixture,0x48)[0]
            assert struct.unpack_from('<i',fixture,0x40)[0]==95001604
            assert struct.unpack_from('<i',fixture,offset)[0]==20406
            assert struct.unpack_from('<i',fixture,offset+0x118)[0]==20408
            for count in (0,1,2,5,10,32):
                for kind in range(5):
                    manager=c.create_string_buffer(0x80);context=c.create_string_buffer(0x180)
                    struct.pack_into('<ii',context,8,40000,1);context[0x10]=kind
                    params=[];nodes=[]
                    for _ in range(count):
                        param=c.create_string_buffer(fixture[offset:offset+0x400],0x400);node=c.create_string_buffer(0x88)
                        struct.pack_into('<Qi',node,0,c.addressof(param),95001604);params.append(param);nodes.append(node)
                    for a,b in zip(nodes,nodes[1:]):struct.pack_into('<Q',a,0x38,c.addressof(b))
                    if nodes:struct.pack_into('<Q',manager,8,c.addressof(nodes[0]))
                    calc(c.addressof(manager),c.addressof(context),-1,-1)
                    for off in (0x90,0xa4,0xa8,0xac,0xb0):
                        actual=struct.unpack_from('<f',context,off)[0]
                        assert math.isclose(actual,1.1**count,rel_tol=2e-6),(count,kind,hex(off),actual)
                    attack=c.create_string_buffer(0x400);ordinary=c.create_string_buffer(20);conditional=c.create_string_buffer(20);additional=c.create_string_buffer(20)
                    defense(c.addressof(manager),c.addressof(ordinary),c.addressof(conditional),c.addressof(additional),c.addressof(attack))
                    assert all(math.isclose(value,.9**count,rel_tol=2e-6) for value in struct.unpack('<5f',ordinary.raw)),(count,ordinary.raw.hex())
                    assert struct.unpack('<5f',conditional.raw)==struct.unpack('<5f',additional.raw)==(1.,)*5
                    attack_cases+=1
        finally:assert k.VirtualFree(image,0,0x8000)
        return dict(sameIdCanStack=True,exclusiveStateDoesNotMatch=True,categoryDoesNotReplace=True,hudCounts=counts,nativeAttackCases=attack_cases,nativeDefenseCases=attack_cases)
    finally:assert k.VirtualFree(memory,0,0x8000)


if __name__=='__main__':
    path=Path(sys.argv[1]);report=json.loads(path.read_text(encoding='utf8'))
    cases=event_checks(report);native=native_checks(path.with_name('effect.param'))
    path.with_name('tests.json').write_text(json.dumps(dict(eventCases=cases,native=native,gameplayVerified=False,multiplayerVerified=False),indent=2),encoding='utf8')
    print(f'PASS: {cases} serialized event cases; native independent-stack predicates and HUD counts through 255')
