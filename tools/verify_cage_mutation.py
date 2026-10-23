"""Verify reopened cage choices and native mutation getters without the game."""
import os
from pathlib import Path
from collections import defaultdict
import ctypes as c, hashlib, json, math, struct, tomllib
import pefile
from local_paths import GAME
from verify_cages import data, decode, root

report=json.loads((root/'.work/cage/mutation.json').read_text())
assert report['marker']==95001602
maps=report['maps']
assert [(m['modifier'],m['flag'],m['slots'],m['counts']) for m in maps]==[
    (10+i,7600+i,slots,[0,count,count,count,count])
    for i,(slots,count) in enumerate(zip([55,64,66,55,55,67],[5,6,7,7,5,9]))]
for m in maps:
    assert all(math.isclose(p,n/m['slots'],rel_tol=1e-6,abs_tol=1e-8) for p,n in zip(m['probabilities'],m['counts']))
marker=report['markerFields']['fields']
assert (marker['maxHpRate'],marker['saveCategory'],marker['iconId'],marker['spEffectTextId_1'],marker['effectEndurance'])==(1,-1,-1,-1,-1)
assert marker['displayIconForPermamentEffect']==0
for r in report['mutationEffects']:
    assert r['fields']['maxHpRate']>1 and r['fields']['physicsAttackPowerRate']>1 and r['fields']['haveSoulRate']>1
assert len(report['npcs'])==37 and len(report['towers'])==7
lots={r['ID']:r['fields'] for r in report['loot']}
for t in report['towers']:
    assert t['firstFlag']==t['entity']-500 and t['secondFlag']==t['firstFlag']+1
    assert t['firstLot'] in (6850000,6851000) and t['secondLot']==t['firstLot']+1000
    for i in (0,1):
        row=lots[t['firstLot']+i]
        assert row['getItemFlagId']==0
        selected=[row[f'lotItemId{slot:02}'] for slot in range(1,9) if row[f'lotItemBasePoint{slot:02}']>0]
        assert selected and set(selected)<={2000003,2000004,3000300,3000400},selected
    n=next(n for n in report['npcs'] if n['npc']==t['cloneNpc'])
    assert n['rewardLot']==t['firstLot']
for n in report['npcs']:
    assert n['effects'][0] in range(4480,4484) and n['effects'][1] in (7220,7230,7240)
    assert len(n['deepEffects'])==5 and all(x>0 for x in n['deepEffects'])
    assert n['rewardLot'] in lots and lots[n['rewardLot']]['getItemFlagId']==0

class Decision:
    def __init__(self,code,flags,depth_effect,client,draw):
        self.code=code; self.flags=dict(flags); self.effect=depth_effect; self.client=client; self.draw=draw
        self.pc=0; self.groups=defaultdict(list); self.random_flags=[]; self.outputs=[]; self.frames=0
        self.labels={int(n[5:]):i for i,(n,_) in enumerate(code) if n.startswith('Label')}
    def group(self,g):return bool(self.groups[g]) and (any(self.groups[g]) if g<0 else all(self.groups[g]))
    def run(self):
        for _ in range(100):
            if self.pc==len(self.code):return True
            n,a=self.code[self.pc]; self.pc+=1
            if n=='GotoIfEventFlag':
                if bool(self.flags.get(a[3],False))==bool(a[1]):self.pc=self.labels[a[0]]
            elif n=='GotoIfPlayerIsInWorldType':
                assert a[1]==1
                if self.client:self.pc=self.labels[a[0]]
            elif n=='SetNetworkconnectedEventFlag':
                assert not self.client,'A replica must never overwrite the host decision'
                self.flags[a[1]]=bool(a[2]); self.outputs.append((n,a))
            elif n=='WaitFixedTimeFrames':self.frames+=a[0]
            elif n=='IfCharacterHasSpEffect':self.groups[a[0]].append(a[2]==self.effect)
            elif n=='IfEventFlagAndRandomCondition':
                active=bool(self.flags.get(a[1],False))
                if active:self.random_flags.append(a[1])
                self.groups[a[0]].append(active and self.draw<a[2])
            elif n=='GotoIfConditionGroupStateUncompiled':
                if self.group(a[2])==bool(a[1]):self.pc=self.labels[a[0]]
            elif n=='IfEventFlag':
                assert a[0]==0
                if bool(self.flags.get(a[3],False))!=bool(a[1]):self.pc-=1;return False
                self.groups.clear()
            elif n=='SetSpEffect':self.outputs.append((n,a))
            elif n.startswith('Label'):pass
            else:raise AssertionError((n,a))
        raise AssertionError('Mutation decision never yields')

cases=0
for r in data['reports']:
    for index,npc in enumerate(report['npcs']):
        encounter=next(e for e in r['events'] if e['ID']%10000==9710+index)
        decoded=[decode(i) for i in encounter['instructions']]
        red,decided=r['red']+index,r['decided']+index
        start=decoded.index(('GotoIfEventFlag',[8,1,0,decided]))
        end=decoded.index(('Label9',[]))+1
        code=decoded[start:end]
        entity=encounter['ID']//10000*10000+9000+index
        for m in maps:
            for depth in range(6):
                for draw in (0,.5,1):
                    effect=npc['deepEffects'][depth-1] if depth else None
                    host=Decision(code,{m['flag']:True},effect,False,draw)
                    assert host.run() and host.flags[decided] and host.frames==1
                    expected=depth>=2 and draw<m['probabilities'][depth-1]
                    assert host.flags[red]==expected
                    assert host.random_flags==([m['flag']] if depth>=2 else [])
                    assert [a for n,a in host.outputs if n=='SetSpEffect']==([[entity,95001602]] if expected else [])
                    for client in (False,True):
                        restored=Decision(code,host.flags,effect,client,1-draw)
                        assert restored.run() and not restored.random_flags and restored.frames==0
                        assert not any(n=='SetNetworkconnectedEventFlag' for n,a in restored.outputs)
                        assert [a for n,a in restored.outputs if n=='SetSpEffect']==([[entity,95001602]] if expected else [])
                    peer=Decision(code,{m['flag']:True},effect,True,draw)
                    assert not peer.run() and not peer.random_flags and not peer.outputs
                    peer.flags.update(host.flags)
                    assert peer.run() and not peer.random_flags
                    cases+=1
        dead=decoded.index(('IfCharacterHPValue',[0,entity,0,0,0,1]))
        cleared=decoded[dead+1:decoded.index(('WaitFixedTimeSeconds',[15]))]
        assert ('SetNetworkconnectedEventFlag',[0,red,0]) in cleared
        assert ('SetNetworkconnectedEventFlag',[0,decided,0]) in cleared

# Execute native code only in this Python process with fake actor/param data.
# The two external table lookups are relocated to bounded local callbacks;
# the native mutation-bit and null-reference branches are unchanged.
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
native=lambda rva,size:raw[pe.get_offset_from_rva(rva):][:size]
code=bytearray(native(0x4fff30,0x72))
assert code[:8]==bytes.fromhex('40 53 48 83 EC 30 4C 8B') and code[-1]==0xc3
loot_code=native(0xe95360,0x15)
assert loot_code==bytes.fromhex('48 8B 01 48 85 C0 74 07 8B 80 38 03 00 00 C3 B8 FF FF FF FF C3')
k=c.WinDLL('kernel32',use_last_error=True)
k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
k.FlushInstructionCache.argtypes=[c.c_void_p,c.c_void_p,c.c_size_t]
address=k.VirtualAlloc(None,4096,0x3000,0x40);assert address
try:
    actor=c.create_string_buffer(0x600);row=c.create_string_buffer(0x350);ref=c.create_string_buffer(16);source=c.create_string_buffer(0x30)
    effectset=(c.c_int*4)(4481,7230,-1,-1);lookups=[]
    @c.WINFUNCTYPE(c.c_void_p,c.c_void_p,c.c_int)
    def lookup(out,id):
        lookups.append(id);c.c_void_p.from_address(out+8).value=c.addressof(effectset) if id==113130 else None
        return out
    @c.WINFUNCTYPE(c.c_int,c.c_void_p,c.c_uint)
    def effect_at(ref,index):return effectset[index] if index<4 else -1
    # Call sites relative to the bounded native function; redirect only callees.
    for offset,stub,callback in [(0x48,0x100,lookup),(0x5c,0x120,effect_at)]:
        assert code[offset]==0xe8
        struct.pack_into('<i',code,offset+1,stub-offset-5)
        c.memmove(address+stub,b'\x48\xb8'+struct.pack('<Q',c.cast(callback,c.c_void_p).value)+b'\xff\xe0',12)
    c.memmove(address,bytes(code),len(code));c.memmove(address+0x200,loot_code,len(loot_code))
    assert k.FlushInstructionCache(c.c_void_p(-1),address,4096)
    getter=c.WINFUNCTYPE(c.c_int,c.c_void_p,c.c_int)(address)
    reward=c.WINFUNCTYPE(c.c_int,c.c_void_p)(address+0x200)
    struct.pack_into('<Q',ref,0,c.addressof(row));struct.pack_into('<Q',source,8,c.addressof(actor));struct.pack_into('<Q',source,0x28,c.addressof(ref))
    struct.pack_into('<i',row,0x334,113130);struct.pack_into('<i',row,0x338,6850000)
    for flags in range(256):
        actor[0x1fb]=bytes([flags]);lookups.clear()
        assert [getter(c.addressof(source),i) for i in range(4)]==([4481,7230,-1,-1] if flags&1 else [-1]*4)
        assert lookups==([113130]*4 if flags&1 else [])
    assert reward(c.addressof(ref))==6850000
    struct.pack_into('<Q',ref,0,0);assert reward(c.addressof(ref))==-1
    assert getter(c.addressof(source),0)==-1
    struct.pack_into('<Q',source,8,0);assert getter(c.addressof(source),0)==-1
finally:k.VirtualFree(address,0,0x8000)

output=dict(version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version'],maps=maps,decisionCases=cases,
            hostOnlyRoll=True,clientWaitAndReloadPassed=True,deathResetPassed=True,nativeGetterBitCases=256,
            nativeMutationAndRewardGettersExecutedInIsolation=True,towerSources=report['towers'],towerRewardPolicy=report['towerRewardPolicy'],
            markerNeutral=True,combatScalingAndRuneBonusUseNativeEffects=True,gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(Path(os.environ.get('NRHEX_VALIDATION_ROOT',root/'dist/reports'))/'cage-mutation-validation.json').write_text(json.dumps(output,ensure_ascii=False,indent=2),encoding='utf8')
print(f'PASS: {cases} depth/map/host/replica decisions, native mutation/reward getters, seven tower weapon rewards; gameplay remains unverified.')
