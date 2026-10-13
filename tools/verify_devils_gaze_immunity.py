"""Reproduce the native asset/Chr contact difference in an isolated EXE copy."""
import ctypes as c
import hashlib, json, struct
import pefile
from local_paths import GAME, ROOT

raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw)
base=pe.OPTIONAL_HEADER.ImageBase
# Native character immunity vtables, not a guessed HP/no-damage flag.
for vt,fn in [(0x2c307f8,0x66d3e0),(0x2c18120,0x3d3db0)]:
    assert struct.unpack('<Q',pe.get_data(vt+0x200,8))[0]==base+fn
assert pe.get_data(0x52d039,5)==b'\xe8'+struct.pack('<i',0x33c120-0x52d03e)
assert pe.get_data(0x33c4c6,4)==bytes.fromhex('41 0f b6 c6') # AL result
k=c.WinDLL('kernel32')
k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
memory=k.VirtualAlloc(None,pe.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert memory
callbacks=[];events=[];owner_kind=6
def ptr(b):return c.addressof(b)
def put(b,off,fmt,value):struct.pack_into(fmt,b,off,value)
def function(result,args,fn):
    f=c.WINFUNCTYPE(result,*args)(fn);callbacks.append(f);return c.cast(f,c.c_void_p).value
def stub(rva,result,args,fn):
    p=function(result,args,fn)
    c.memmove(memory+rva,b'\x48\xb8'+struct.pack('<Q',p)+b'\xff\xe0',12)
try:
    c.memmove(memory,raw[:pe.OPTIONAL_HEADER.SizeOfHeaders],pe.OPTIONAL_HEADER.SizeOfHeaders)
    for s in pe.sections:c.memmove(memory+s.VirtualAddress,s.get_data(),s.SizeOfRawData)
    for block in pe.DIRECTORY_ENTRY_BASERELOC:
        for rel in block.entries:
            if rel.type==10:
                p=c.c_uint64.from_address(memory+rel.rva);p.value=(p.value+memory-base)&0xffffffffffffffff
            else:assert rel.type==0
    actor=c.create_string_buffer(0x800);state=c.create_string_buffer(0x300)
    modules=c.create_string_buffer(0x200);data=c.create_string_buffer(0x20)
    controller=c.create_string_buffer(0xd0);controller_state=c.create_string_buffer(0x30)
    vt=c.create_string_buffer(0x300);damage=c.create_string_buffer(0x100);damagevt=c.create_string_buffer(0x50)
    source=c.create_string_buffer(0x200);sourcevt=c.create_string_buffer(0x30)
    source_modules=c.create_string_buffer(0x200);source_damage=c.create_string_buffer(0x20)
    bullet=c.create_string_buffer(0xc0);row=c.create_string_buffer(0x1c0)
    bullet_row=c.create_string_buffer(0xc8);bullet_ref=c.c_void_p(ptr(bullet_row))
    put(actor,0,'<Q',ptr(vt));put(actor,0x18,'<Q',ptr(data));data[8]=4
    put(actor,0x60,'<Q',ptr(controller));put(controller,0xc8,'<Q',ptr(controller_state))
    put(actor,0x1b8,'<Q',ptr(modules));put(modules,8,'<Q',ptr(state));put(modules,0x98,'<Q',ptr(damage))
    put(damage,0,'<Q',ptr(damagevt));put(damage,8,'<Q',ptr(actor))
    put(vt,0x200,'<Q',memory+0x66d3e0)
    put(source,0,'<Q',ptr(sourcevt));put(source,0x1b8,'<Q',ptr(source_modules))
    put(source_modules,0x98,'<Q',ptr(source_damage));put(source_damage,8,'<Q',ptr(source))
    put(sourcevt,0x10,'<Q',function(c.c_int,[c.c_void_p],lambda _:owner_kind))
    put(damagevt,0x38,'<Q',function(None,[c.c_void_p]*3,lambda *_:events.append('damage/reaction')))
    def lookup(out,kind,attack):
        assert kind==0 and attack==95002200
        c.c_int.from_address(out).value=attack;c.c_int.from_address(out+8).value=kind
        c.c_uint64.from_address(out+16).value=ptr(row)
    stub(0xe6d630,None,[c.c_void_p,c.c_int,c.c_int],lookup)
    stub(0x26909a0,None,[c.c_void_p],lambda _:None)
    stub(0x34dad0,c.c_ubyte,[c.c_void_p],lambda _:1)
    stub(0x34caf0,c.c_void_p,[c.c_void_p]*3,lambda _,out,__:(c.memset(out,0,16),out)[1])
    stub(0x354aa0,c.c_void_p,[c.c_void_p],lambda _:ptr(bullet_ref))
    stub(0x63f840,c.c_void_p,[c.c_uint64],lambda _:ptr(source))
    stub(0x3d31b0,c.c_ubyte,[c.c_void_p],lambda _:0)
    stub(0x52cb90,c.c_int,[c.c_void_p]*3,lambda *_:1)
    stub(0x52cd30,c.c_int,[c.c_void_p]*3,lambda *_:1)
    stub(0x3d1b30,c.c_void_p,[c.c_void_p]*2,lambda _,out:out)
    for rva in [0x4316b0,0x431770,0x431710]:stub(rva,c.c_ubyte,[c.c_void_p]*3,lambda *_:0)
    stub(0x4364b0,c.c_ubyte,[c.c_void_p,c.c_int],lambda *_:0)
    stub(0x33ba30,c.c_void_p,[c.c_void_p]*2,lambda out,hit:(c.memmove(out,hit,0x2d0),out)[1])
    stub(0x33f3c0,c.c_void_p,[c.c_void_p]*2,lambda out,_:out)
    stub(0x34dd20,c.c_ubyte,[c.c_void_p]*3,lambda *_:(events.append('bullet effects/Madness'),1)[1])
    stub(0x6ada30,c.c_ubyte,[],lambda:1)
    c.c_uint64.from_address(memory+0x3c0f338).value=1
    c.c_ubyte.from_address(memory+0x3c0f2fd).value=0
    native=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p,c.c_void_p,c.c_void_p)(memory+0x33c120)
    immune=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p,c.c_void_p)(memory+0x66d3e0)
    attack=c.create_string_buffer(24);lookup(ptr(attack),0,95002200)
    cases=[]
    for name,flags,char_flags,timer in [
        ('ordinary',0,0,0.),('TAE full immunity',1,0,0.),('TAE standard immunity',2,0,0.),
        ('character immunity',0,0x10,0.),('player invulnerability timer',0,0,1.),
        ('unrelated guard flag',0x20,0,0.)]:
        put(state,0x58,'<Q',flags);actor[0x1f3]=char_flags;put(actor,0x6e4,'<f',timer)
        blocked=bool(immune(ptr(actor),ptr(attack)));assert blocked==(name not in ['ordinary','unrelated guard flag'])
        results={}
        for owner_kind in [1,6]:
            hit=c.create_string_buffer(0x2d0);put(hit,0x40,'<i',95002200);put(hit,0x44,'<i',0)
            events.clear();native(ptr(hit),ptr(actor),ptr(bullet))
            expected=[] if owner_kind==1 and blocked else ['damage/reaction','bullet effects/Madness']
            assert events==expected,(name,owner_kind,events,expected)
            results[str(owner_kind)]=events.copy()
        cases.append(dict(state=name,nativeImmune=blocked,contactBySourceKind=results))
finally:
    assert k.VirtualFree(memory,0,0x8000)
report=dict(exeSha256=hashlib.sha256(raw).hexdigest(),nativeContactRva='0x33c120',
    nativeCharacterFilterRva='0x436370',nativePlayerImmunityRva='0x66d3e0',
    nativePredicateAndCharacterFilterExecuted=True,environmentSkipsImmunityReproduced=True,cases=cases,
    stubbedDependencies=['source/parameter lookup','team eligibility','damage/reaction sink','bullet-effect sink','contact bookkeeping'],
    gameMemoryWritten=False,gameplayVerified=False,multiplayerVerified=False)
out=ROOT/'.work/devils-gaze-iframes';out.mkdir(exist_ok=True)
(out/'native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print('PASS: native contact reproduces missing asset-source immunity gate in 12 cases; original player immunity predicate and Chr filter executed.')
