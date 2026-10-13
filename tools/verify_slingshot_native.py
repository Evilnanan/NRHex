"""Run the original jump impulse and its horizontal getter in an isolated PE copy."""
import ctypes as c
import hashlib, json, struct
import pefile
from local_paths import GAME, ROOT

raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw)
base=pe.OPTIONAL_HEADER.ImageBase
k=c.WinDLL('kernel32')
k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
memory=k.VirtualAlloc(None,pe.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert memory
callbacks=[];rate=1.;row_id=131
def ptr(b):return c.addressof(b)
def put(b,off,fmt,value):struct.pack_into(fmt,b,off,value)
def function(result,args,fn):
    f=c.WINFUNCTYPE(result,*args)(fn);callbacks.append(f);return c.cast(f,c.c_void_p).value
def stub(rva,result,args,fn):
    p=function(result,args,fn)
    c.memmove(memory+rva,b'\x48\xb8'+struct.pack('<Q',p)+b'\xff\xe0',12)
try:
    c.memmove(memory,raw[:pe.OPTIONAL_HEADER.SizeOfHeaders],pe.OPTIONAL_HEADER.SizeOfHeaders)
    for sec in pe.sections:c.memmove(memory+sec.VirtualAddress,sec.get_data(),sec.SizeOfRawData)
    for block in pe.DIRECTORY_ENTRY_BASERELOC:
        for rel in block.entries:
            if rel.type==10:
                p=c.c_uint64.from_address(memory+rel.rva);p.value=(p.value+memory-base)&0xffffffffffffffff
            else:assert rel.type==0
    velocity=c.create_string_buffer(0xe0);actor=c.create_string_buffer(0x200);modules=c.create_string_buffer(0x100);physics=c.create_string_buffer(0x100);row=c.create_string_buffer(0x20)
    position=(c.c_float*4)(0,0,0,1)
    put(velocity,0x20,'<Q',ptr(actor));put(actor,0x1b8,'<Q',ptr(modules));put(modules,0x68,'<Q',ptr(physics))
    def lookup(out,ident):
        assert ident==row_id
        c.c_int.from_address(out).value=ident;c.c_uint64.from_address(out+8).value=ptr(row)
    stub(0xe73840,None,[c.c_void_p,c.c_int],lookup)
    stub(0x4760a0,c.c_ubyte,[c.c_void_p,c.c_ubyte,c.c_void_p,c.c_void_p],lambda *_:0)
    stub(0x3ceb20,c.c_void_p,[c.c_void_p,c.c_void_p],lambda actor,out:(c.memmove(out,ptr(position),16),out)[1])
    stub(0x3d0e60,c.c_void_p,[c.c_void_p,c.c_void_p],lambda actor,out:(c.memmove(out,ptr(position),16),out)[1])
    stub(0x26909a0,None,[c.c_void_p],lambda _:None)
    # Original getter bytes, including null-row behavior, at a separate address.
    original_code=pe.get_data(0xe737b0,19);c.memmove(memory+0x1000,original_code,19)
    native_get=c.WINFUNCTYPE(c.c_float,c.c_void_p)(memory+0x1000)
    def horizontal(ref):
        assert c.c_int.from_address(ref).value==row_id
        return native_get(ref)*rate
    stub(0xe737b0,c.c_float,[c.c_void_p],horizontal)
    launch=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p,c.c_void_p,c.c_void_p)(memory+0x4739f0)
    initial=(c.c_float*4)(1,2,3,0);output=(c.c_float*4)();dt=c.create_string_buffer(16);put(dt,8,'<f',1/60)
    cases=[]
    for row_id,horizontal_speed in [(100,0.),(101,1.8),(111,4.3),(121,7.),(131,8.)]:
        baseline=None
        for stacks in range(4):
            rate=1.+.25*stacks if row_id==131 else 1.
            struct.pack_into('<fffff',row,0,0.,0.,horizontal_speed,9.,0.)
            put(velocity,0x10,'<i',-1);put(velocity,0x14,'<i',row_id)
            launch(ptr(velocity),ptr(output),ptr(initial),ptr(dt))
            values=list(output)
            if stacks==0:baseline=values
            assert abs(values[1]-9.)<1e-5,(row_id,stacks,values)
            for axis in [0,2]:assert abs(values[axis]-baseline[axis]*rate)<1e-4,(row_id,stacks,values,baseline)
            assert struct.unpack_from('<i',velocity,0x14)[0]==-1
            assert abs((values[0]**2+values[2]**2)**.5-horizontal_speed*rate)<1e-4
            cases.append(dict(row=row_id,stacks=stacks,velocity=values))
    report=dict(exeSha256=hashlib.sha256(raw).hexdigest(),nativeLaunch='0x4739f0',horizontalGetter='0xe737b0',nativeCases=cases,sprintJumpRow=131,ordinaryJumpRowsUnchanged=True,verticalVelocityUnchanged=True,oneShotRowConsumedByNative=True,parameterRowsUnchanged=True,gameplayVerified=False,multiplayerVerified=False)
    (ROOT/'.work/slingshot/native-validation.json').write_text(json.dumps(report,indent=2))
    print('PASS: 20 original native jump impulses (131 sprint only); horizontal scale, original vertical speed and one-shot consumption verified')
finally:k.VirtualFree(memory,0,0x8000)
