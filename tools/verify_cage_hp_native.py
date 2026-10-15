"""Execute original correction selection/refresh against serialized parameters.

Private image only. Lookup, player count, effect application and stat updates
are fixtures; the original cache gate and forced native refresh execute. This
does not establish real multiplayer or native HP-packet transport behavior.
"""
import ctypes as c,hashlib,json,struct,pefile
from local_paths import ROOT,GAME

work=ROOT/'.work/cage-hp-122';work.mkdir(parents=True,exist_ok=True)
fixture=json.loads((work/'native-fixtures.json').read_text())
assert fixture['nativeFieldsAndMultipliersPreserved'] and fixture['localCorrectionEffects']
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw);k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p;k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
mem=k.VirtualAlloc(None,pe.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert mem
callbacks=[];rows={};count=[2];applied=[];cleared=[];recalculations=[];cases=[]
def put(buf,off,fmt,v):struct.pack_into(fmt,buf,off,v)
def stub(rva,result,args,fn):
    f=c.WINFUNCTYPE(result,*args)(fn);callbacks.append(f)
    c.memmove(mem+rva,b'\x48\xb8'+struct.pack('<Q',c.cast(f,c.c_void_p).value)+b'\xff\xe0',12)
try:
    for s in pe.sections:c.memmove(mem+s.VirtualAddress,s.get_data(),s.SizeOfRawData)
    for block in pe.DIRECTORY_ENTRY_BASERELOC:
        for rel in block.entries:
            if rel.type==10:
                v=c.c_uint64.from_address(mem+rel.rva);v.value=(v.value+mem-pe.OPTIONAL_HEADER.ImageBase)&0xffffffffffffffff
            else:assert rel.type==0
    for r in fixture['corrections']:
        fields=r['fields'];b=c.create_string_buffer(20)
        for off,name in [(4,'client1SpEffectId'),(8,'client2SpEffectId'),(12,'client3SpEffectId')]:put(b,off,'<i',fields[name])
        put(b,16,'<B',fields['bOverrideSpEffect']);rows[r['ID']]=b
    actor=c.create_string_buffer(0x250);vt=c.create_string_buffer(0x1a8);npc=c.create_string_buffer(0x200);ref=c.create_string_buffer(8);modules=c.create_string_buffer(8);stats=c.create_string_buffer(0x170);manager=c.create_string_buffer(0x30)
    addr=c.addressof(actor);put(actor,0,'<Q',c.addressof(vt));put(actor,0x1b8,'<Q',c.addressof(modules));put(actor,0x190,'<Q',c.addressof(manager));put(ref,0,'<Q',c.addressof(npc));put(modules,0,'<Q',c.addressof(stats))
    get=c.WINFUNCTYPE(c.c_void_p,c.c_void_p)(lambda _:c.addressof(ref));callbacks.append(get);put(vt,0x1a0,'<Q',c.cast(get,c.c_void_p).value)
    c.c_uint64.from_address(mem+0x3c0f0a8).value=c.addressof(actor)
    def lookup(out,id):
        assert id in rows;c.c_int.from_address(out).value=id;c.c_uint64.from_address(out+8).value=c.addressof(rows[id])
    stub(0xe93dd0,None,[c.c_void_p,c.c_int],lookup)
    stub(0x519660,c.c_int,[],lambda:count[0])
    stub(0x50ee20,c.c_void_p,[c.c_void_p],lambda out:(c.memset(out,0,16),out)[1])
    def apply(target,id,source,force,selector,options,b1,b2,b3,b4,b5,b6,flags):
        assert target==source==addr and force==1 and flags==0x10
        applied.append(id)
    stub(0x3c80a0,None,[c.c_void_p,c.c_int,c.c_void_p,c.c_ubyte,c.c_void_p,c.c_void_p,c.c_ubyte,c.c_ubyte,c.c_ubyte,c.c_ubyte,c.c_ubyte,c.c_ubyte,c.c_int],apply)
    stub(0x3e0ab0,None,[c.c_void_p,c.c_float],lambda actor,_:recalculations.append(('update',actor)))
    stub(0x428bf0,None,[c.c_void_p,c.c_ubyte],lambda stats,heal:recalculations.append(('stats',stats,heal)))
    stub(0x504c40,None,[c.c_void_p,c.c_int],lambda manager,state:cleared.append((manager,state)))
    refresh=c.WINFUNCTYPE(None,c.c_void_p)(mem+0x3c7df0)
    cached=c.WINFUNCTYPE(None,c.c_void_p)(mem+0x3c7e50)
    for id,b in rows.items():
        fields=next(r['fields'] for r in fixture['corrections'] if r['ID']==id)
        put(npc,0x1d8,'<i',id)
        for party in [1,2,3,4]:
            for remembered in [0,party,4]:
                count[0]=party;put(actor,0x238,'<i',remembered);applied.clear();cleared.clear();recalculations.clear()
                refresh(addr)
                effect=fields[f'client{party-1}SpEffectId'] if party>1 else -1
                if effect>=0:
                    assert applied==[effect] and not cleared
                    assert recalculations==[('update',addr),('stats',c.addressof(stats),1)]
                    assert c.c_int.from_buffer(actor,0x238).value==(party if fields['bOverrideSpEffect']==2 else remembered)
                else:
                    assert not applied and cleared==[(c.addressof(manager),282)] and c.c_int.from_buffer(actor,0x238).value==0
                cases.append(dict(correction=id,party=party,cachedParty=remembered,selected=effect))
        if fields['bOverrideSpEffect']==2:
            count[0]=2;put(actor,0x238,'<i',2);applied.clear();recalculations.clear();cached(addr)
            assert not applied and not recalculations,'Native cache gate must reproduce a lost effect remaining lost'
            refresh(addr);assert applied==[fields['client1SpEffectId']]
            cases.append(dict(correction=id,oldEqualPartyCacheSkipsMissingEffect=True,forcedRefreshRestoresNativeSelection=True))
    fx={r['ID']:r['fields'] for r in fixture['effects']}
    assert all(v['stateInfo']==282 and v['isDisableNetSync']==1 for v in fx.values())
finally:k.VirtualFree(mem,0,0x8000)
report=dict(nativeCases=len(cases),cases=cases,originalForcedRefreshRva='0x3c7df0',originalPartyCacheGateRva='0x3c7e50',originalCorrectionSelectorRva='0x3db250',nativeHPRecalculationCallsPreserved=True,externalLookupAndUpdatesStubbed=True,nativePacketTransportVerified=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print(f'PASS: {len(cases)} actual native correction cases; cache-loss reproduced, forced refresh selects native effects independent of remembered count')
