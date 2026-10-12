"""Execute bounded original native branches in private memory, never the game."""
import ctypes as c,hashlib,json,struct
import pefile
from local_paths import GAME,ROOT

raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
kernel=c.WinDLL('kernel32');kernel.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];kernel.VirtualAlloc.restype=c.c_void_p
kernel.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
blocks=[]
def function(rva,size,result,args):
    p=kernel.VirtualAlloc(None,size,0x3000,0x40);assert p;blocks.append(p)
    c.memmove(p,pe.get_data(rva,size),size)
    return c.WINFUNCTYPE(result,*args)(p)
def ptr(b):return c.addressof(b)
def put(b,off,fmt,value):struct.pack_into(fmt,b,off,value)
cases=[]
try:
    # 508E20 is entirely position independent and has no external calls.
    has_state=function(0x508e20,0x74,c.c_ubyte,[c.c_void_p,c.c_uint16])
    for state in [0,433,434,435]:
        for aggregate in [False,True]:
            for flags in [0,1,0x40000,0x80000000]:
                manager=c.create_string_buffer(0x40);node=c.create_string_buffer(0x80);param=c.create_string_buffer(0x160)
                put(manager,8,'<Q',ptr(node));put(manager,0x28,'<Q',(1<<60) if aggregate else 0)
                put(node,0,'<Q',ptr(param));put(node,0x68,'<I',flags);put(param,0x156,'<H',state)
                expected=state==434 and aggregate and flags==0
                actual=has_state(ptr(manager),434)!=0;assert actual==expected,(state,aggregate,flags,actual)
                cases.append(dict(state=state,aggregate=aggregate,flags=flags,stanceBroken=actual))
    # An inactive first node must not mask a later active stance-break state.
    manager=c.create_string_buffer(0x40);nodes=[c.create_string_buffer(0x80) for _ in range(2)];param=c.create_string_buffer(0x160)
    put(manager,8,'<Q',ptr(nodes[0]));put(manager,0x28,'<Q',1<<60);put(param,0x156,'<H',434)
    for n in nodes:put(n,0,'<Q',ptr(param))
    put(nodes[0],0x38,'<Q',ptr(nodes[1]));put(nodes[0],0x68,'<I',1)
    assert has_state(ptr(manager),434)==1;cases.append(dict(twoNodes=True,inactiveFirst=True,stanceBroken=True))
    # Only the early unequal-base/unequal-art branches are executed. The
    # comparator's obfuscated later metadata branches are outside this proof.
    different=function(0x6c0c70,0x22,c.c_ubyte,[c.c_void_p,c.c_void_p])
    comparisons=[]
    for left,right,skill0,skill1 in [(2000000,2000000,-1,1173),(2000000,2000000,1173,1047),(41000000,41000000,-1,400),(2000000,3000000,1173,1173)]:
        a=c.create_string_buffer(0x48);b=c.create_string_buffer(0x48)
        put(a,0,'<i',left);put(b,0,'<i',right);put(a,4,'<i',skill0);put(b,4,'<i',skill1)
        assert different(ptr(a),ptr(b))==1
        comparisons.append(dict(base0=left,base1=right,skill0=skill0,skill1=skill1,nativeDetectsChange=True))
    getter=function(0x6a1df0,0x13,c.c_int,[c.c_void_p]);instance=c.create_string_buffer(0x30);reference=c.create_string_buffer(0x18)
    put(reference,8,'<Q',ptr(instance))
    for skill in [-1,1173,1047,400]:
        put(instance,0x1c,'<i',skill);assert getter(ptr(reference))==skill
finally:
    for p in blocks:assert kernel.VirtualFree(p,0,0x8000)
report=dict(version='0.17.31',nativeState434Cases=cases,nativeEquipmentDifferenceBranches=comparisons,
            nativeInstanceSkillGetterCases=4,stanceParameterSource='stance-effects.json',
            comparisonProofLimitedToEarlyDifferenceBranches=True,nativePacketDeliveryAndPeerAnimationsNotExecuted=True,
            originalExeSha256=hashlib.sha256(raw).hexdigest(),gameMemoryWritten=False,gameplayVerified=False,multiplayerVerified=False)
(ROOT/'.work/fixes-01731/native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f'PASS: {len(cases)} original native stance-state cases; 4 equipment difference branches; 4 instance skill getter cases.')
