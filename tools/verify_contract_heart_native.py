"""Isolated execution of the native on-hit selector and self-damage entry.

Runs original code in a private image. World ownership, effect filtering and
the HP setter are controlled fixtures; this is not a gameplay/network test.
"""
import ctypes as c
import hashlib, json, struct
import pefile
from local_paths import ROOT, GAME

raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True);pe.parse_data_directories(directories=[5])
# Confirm the player's actual derived type on disk, not a mocked NPC vtable.
player_vtable=0x2c11d00
col=struct.unpack('<Q',pe.get_data(player_vtable-8,8))[0]-pe.OPTIONAL_HEADER.ImageBase
type_rva=struct.unpack('<I',pe.get_data(col+12,4))[0]
assert pe.get_data(type_rva+16,100).split(b'\0')[0]==b'.?AVCSPlayerDamageModule@CS@@'
assert struct.unpack('<Q',pe.get_data(player_vtable,8))[0]-pe.OPTIONAL_HEADER.ImageBase==0x441410
assert pe.get_data(0x42c120,5)==bytes.fromhex('488b4108c3')
runtime=(ROOT/'src/ominous_pact.rs').read_text(encoding='utf8')
assert 'PLAYER_DAMAGE_VTABLE_RVA: usize = 0x2c11d00' in runtime
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
m=k.VirtualAlloc(None,pe.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert m
callbacks=[];cases=[]
def stub(rva,ret,args,fn):
    f=c.WINFUNCTYPE(ret,*args)(fn);callbacks.append(f)
    code=b'\x48\xb8'+struct.pack('<Q',c.cast(f,c.c_void_p).value)+b'\xff\xe0'
    c.memmove(m+rva,code,len(code))
def put(buf,off,fmt,value):struct.pack_into(fmt,buf,off,value)
try:
    for s in pe.sections:c.memmove(m+s.VirtualAddress,s.get_data(),s.SizeOfRawData)
    for block in pe.DIRECTORY_ENTRY_BASERELOC:
        for r in block.entries:
            if r.type==10:
                v=c.c_uint64.from_address(m+r.rva);v.value=(v.value+m-pe.OPTIONAL_HEADER.ImageBase)&0xffffffffffffffff
            else:assert r.type==0
    # Real selector: stateInfo=2103 selects atkOccurrenceSpEffectId at +12C.
    row=c.create_string_buffer(0x400);node=c.create_string_buffer(0x80)
    effects=c.create_string_buffer(0x20);out=c.create_string_buffer(0x20)
    hit=c.create_string_buffer(0x2d0);ids=(c.c_int*8)()
    put(node,0,'<Q',c.addressof(row));put(effects,8,'<Q',c.addressof(node))
    accepted=[True];eligible=[True]
    stub(0x511010,c.c_ubyte,[c.c_void_p,c.c_void_p,c.c_void_p,c.c_int,c.c_int],lambda *_:accepted[0])
    stub(0xea0790,c.c_ubyte,[c.c_void_p,c.c_void_p],lambda *_:eligible[0])
    # Cookie verification is unrelated to gameplay; the private mapping keeps
    # the original frame but has no Windows security-cookie initialization.
    stub(0x26909a0,None,[c.c_void_p],lambda *_:None)
    select=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p,c.c_void_p)(m+0x505e90)
    for state in [0,152,153,2102,2103,2104]:
        for active in [True,False]:
            for filter_ok in [True,False]:
                accepted[0]=filter_ok;eligible[0]=True
                put(row,0x156,'<H',state);put(row,0x12c,'<i',95002601)
                put(node,0x68,'<I',0 if active else 1)
                begin=c.addressof(ids);put(out,8,'<Q',begin);put(out,0x10,'<Q',begin);put(out,0x18,'<Q',begin+c.sizeof(ids))
                select(c.addressof(effects),c.addressof(out),c.addressof(hit))
                count=(c.c_uint64.from_buffer(out,0x10).value-begin)//4
                expected=int(state==2103 and active and filter_ok)
                assert count==expected,(state,active,filter_ok,count)
                if count:assert ids[0]==95002601
                cases.append(dict(kind='on-hit',state=state,active=active,filter=filter_ok,selected=count))
    # Execute the actual native applicability switch. Its later category and
    # weapon restrictions are neutral fixtures; magic/incantation/shaman bits
    # are interpreted by the original game instructions, not by a Python model.
    c.memmove(m+0x511010,pe.get_data(0x511010,12),12)
    stub(0xea1b00,c.c_ubyte,[c.c_void_p,c.c_void_p],lambda *_:1)
    stub(0xea1d40,c.c_ubyte,[c.c_void_p,c.c_void_p,c.c_int],lambda *_:1)
    descriptor=c.create_string_buffer(0x40)
    filtering=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p,c.c_void_p,c.c_void_p,c.c_int,c.c_int)(m+0x511010)
    for spell_bits in [False,True]:
        put(row,0x160,'<B',0x80 if spell_bits else 0)
        put(row,0x161,'<B',1 if spell_bits else 0)
        put(row,0x259,'<B',0x20 if spell_bits else 0)
        for kind_id in [0,1,2,3,4,9,10,11]:
            put(descriptor,0,'<B',kind_id)
            ok=filtering(c.addressof(node),c.addressof(descriptor),0,-1,-1)
            assert bool(ok)==(spell_bits or kind_id not in [3,4,10]),(spell_bits,kind_id,ok)
            cases.append(dict(kind='native-applicability',sourceKind=kind_id,spellBits=spell_bits,accepted=bool(ok)))
    # Real self-damage routine and damage notification. Only external game
    # managers and final HP storage are fixtures; native flag writes execute.
    actor=c.create_string_buffer(0x200);vt=c.create_string_buffer(0x220)
    modules=c.create_string_buffer(0x200);stats=c.create_string_buffer(0x170)
    damage=c.create_string_buffer(0x90);world=c.create_string_buffer(0x17500)
    placeholder=c.create_string_buffer(0x100);kind=c.create_string_buffer(8)
    put(actor,0,'<Q',c.addressof(vt));put(actor,0x1b8,'<Q',c.addressof(modules))
    put(modules,0,'<Q',c.addressof(stats));put(damage,8,'<Q',c.addressof(actor));put(stats,8,'<Q',c.addressof(actor))
    put(damage,0,'<Q',m+player_vtable);put(modules,0x98,'<Q',c.addressof(damage))
    c.c_void_p.from_address(m+0x3c0f0a8).value=c.addressof(world)
    c.c_void_p.from_address(m+0x3c14a70).value=c.addressof(placeholder)
    stub(0x282970,c.c_void_p,[c.c_void_p,c.c_void_p],lambda _,out:out)
    stub(0x519ef0,c.c_ubyte,[c.c_void_p,c.c_void_p],lambda *_:1)
    stub(0x3d4180,c.c_ubyte,[c.c_void_p],lambda *_:0)
    stub(0x3d1b30,c.c_void_p,[c.c_void_p,c.c_void_p],lambda *_:c.addressof(kind))
    stub(0x427260,c.c_ubyte,[c.c_void_p],lambda *_:0)
    stub(0x7c4410,c.c_int,[c.c_void_p,c.c_void_p,c.c_ubyte],lambda *_:-1)
    stub(0x3d3230,c.c_ubyte,[c.c_void_p],lambda *_:0)
    stub(0x52c990,c.c_int,[c.c_void_p,c.c_void_p],lambda *_:0)
    false=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p)(lambda *_:0);callbacks.append(false)
    put(vt,0x120,'<Q',c.cast(false,c.c_void_p).value);put(vt,0x210,'<Q',c.cast(false,c.c_void_p).value)
    def set_hp(ptr,value,*_):put(stats,0x140,'<i',max(0,min(value,c.c_int.from_buffer(stats,0x144).value)))
    stub(0x427bc0,None,[c.c_void_p,c.c_int,c.c_ubyte,c.c_float,c.c_float,c.c_ubyte],set_hp)
    hurt=c.WINFUNCTYPE(None,c.c_void_p,c.c_int)(m+0x431090)
    for maximum in [100,1000,2340]:
        for current in [maximum,10]:
            put(stats,0x140,'<i',current);put(stats,0x144,'<i',maximum);put(damage,0x78,'<B',0)
            amount=maximum//20;hurt(c.addressof(damage),amount)
            after=c.c_int.from_buffer(stats,0x140).value;flags=c.c_ubyte.from_buffer(damage,0x78).value
            assert after==max(0,current-amount),(current,maximum,after)
            assert flags&9==9,hex(flags)
            cases.append(dict(kind='self-damage',maxHP=maximum,before=current,damage=amount,after=after,nativeDamageFlags=flags))
    # Damage flags alone do not expire the two user's relic nodes. Execute the
    # exact native hit-effect notification against their actual criterion 12.
    native_params=json.loads((ROOT/'.work/contract-heart/native-parameters.json').read_text())
    relics={r['ID']:r['fields'] for r in native_params['SpEffectParam'] if r['ID'] in [6820400,6820500]}
    assert set(relics)=={6820400,6820500}
    relic_rows=[c.create_string_buffer(0x400) for _ in relics]
    relic_nodes=[c.create_string_buffer(0x70) for _ in relics]
    neutral_row=c.create_string_buffer(0x400);neutral_node=c.create_string_buffer(0x70)
    for ix,ident in enumerate(relics):
        assert relics[ident]['deleteCriteriaDamage']==12 and relics[ident]['replaceSpEffectId']==ident+1
        put(relic_rows[ix],0x327,'<B',relics[ident]['deleteCriteriaDamage'])
        put(relic_nodes[ix],0,'<Q',c.addressof(relic_rows[ix]));put(relic_nodes[ix],8,'<i',ident)
        put(relic_nodes[ix],0x38,'<Q',c.addressof(relic_nodes[ix+1]) if ix+1<len(relic_nodes) else c.addressof(neutral_node))
        put(relic_nodes[ix],0x68,'<I',4)
    put(neutral_node,0,'<Q',c.addressof(neutral_row));put(neutral_node,0x68,'<I',4)
    put(effects,8,'<Q',c.addressof(relic_nodes[0]))
    notify=c.WINFUNCTYPE(None,c.c_void_p,c.c_float,c.c_ubyte,c.c_ubyte,c.c_ubyte)(m+0x504d30)
    for amount,periodic,send_notification in [(50,0,False),(50,0,True),(50,1,True),(0,0,True)]:
        for n in relic_nodes+[neutral_node]:put(n,0x4c,'<f',-1.)
        put(stats,0x140,'<i',1000);put(stats,0x144,'<i',1000)
        hurt(c.addressof(damage),amount)
        loss=1000-c.c_int.from_buffer(stats,0x140).value
        # The old HP-only path leaves both nodes untouched despite flags=9.
        assert all(c.c_float.from_buffer(n,0x4c).value==-1. for n in relic_nodes)
        if send_notification:notify(c.addressof(effects),float(loss),254,0,periodic)
        expected=0. if loss>0 and send_notification and not periodic else -1.
        assert [c.c_float.from_buffer(n,0x4c).value for n in relic_nodes]==[expected,expected]
        assert c.c_float.from_buffer(neutral_node,0x4c).value==-1.
        cases.append(dict(kind='actual-madness-sleep-relics',hpLoss=loss,notify=send_notification,periodic=periodic,relicTimers=[expected,expected],unrelatedTimerUnchanged=True))
finally:k.VirtualFree(m,0,0x8000)
report=dict(nativeCases=len(cases),cases=cases,originalOnHitSelector=True,originalSelfDamageAndNotification=True,originalHitEffectReplacementDispatch=True,verifiedRelics=[6820400,6820500],playerDamageModuleRttiVerified=True,playerDamageVtableRva=hex(player_vtable),nativeOwnerAccessorVerified=True,externalManagersAndHPSetterStubbed=True,gameplayVerified=False,multiplayerVerified=False)
(ROOT/'.work/contract-heart/native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f'PASS: {len(cases)} isolated native cases; attacker on-hit selection and self-damage notifications verified')
