"""Read original Havok wall-direction mapping and execute native jump impulses.

Read-only TAG0 parsing uses the file's own type and item tables; it neither
deserializes Nightreign with Elden Ring's custom-node schema nor rewrites HKX.
The isolated native runner stubs row lookup, position, homing and the security
cookie only. Original launch math and the original speed getter execute.
"""
from pathlib import Path
import ctypes as c,hashlib,json,math,struct,pefile
from local_paths import ROOT,GAME

work=ROOT/'.work/cat-wall-slope';reference=json.loads((work/'reference.json').read_text())
data=(work/'native-behavior.hkx').read_bytes();assert hashlib.sha256(data).hexdigest().upper()==reference['behaviorSha256']
chunks={}
def sections(begin,end):
 while begin<end:
  size=int.from_bytes(data[begin:begin+4],'big')&0x3fffffff;tag=data[begin+4:begin+8]
  assert size>=8 and begin+size<=end
  chunks[tag]=(begin+8,begin+size)
  if tag in [b'TYPE',b'INDX']:sections(begin+8,begin+size)
  begin+=size
 assert begin==end
sections(8,len(data))
def packed(pos):
 first=data[pos];pos+=1
 if first<0x80:return first,pos
 if first<0xc0:return ((first&0x3f)<<8)|data[pos],pos+1
 if first<0xe0:return ((first&0x1f)<<16)|int.from_bytes(data[pos:pos+2],'big'),pos+2
 if first<0xe8:return ((first&7)<<24)|int.from_bytes(data[pos:pos+3],'big'),pos+3
 raise AssertionError('Unexpected packed type index')
strings=data[slice(*chunks[b'TSTR'])].split(b'\0');count,pos=packed(chunks[b'TNA1'][0]);types=[None]
for _ in range(count-1):
 name,pos=packed(pos);parameters,pos=packed(pos);types.append(strings[name].decode())
 for _ in range(parameters):_,pos=packed(pos);_,pos=packed(pos)
assert set(data[pos:chunks[b'TNA1'][1]])<={0}
items=[struct.unpack_from('<III',data,pos)for pos in range(*chunks[b'ITEM'],12)];origin=chunks[b'DATA'][0]
def name(pointer):
 if pointer==0:return ''
 assert pointer<len(items);_,offset,count=items[pointer]
 return data[origin+offset:origin+offset+count].split(b'\0')[0].decode()
nodes={}
for kind,offset,count in items:
 if not kind or types[kind&0xffffff]!='CustomManualSelectorGenerator':continue
 assert count==1
 label=name(struct.unpack_from('<Q',data,origin+offset+72)[0])
 if label.startswith('WallJumpStart_'):
  # Existing parent name and CMSG animId offsets; actual file contents checked
  # against all four symmetric into-wall and away-from-wall selector branches.
  offset_kind,clip=struct.unpack_from('<ii',data,origin+offset+168);assert offset_kind==11
  nodes[label]=clip
into=['WallJumpStart_F_F_CMSG','WallJumpStart_B_B_CMSG00','WallJumpStart_L_L_CMSG01','WallJumpStart_R_R_CMSG01']
away=['WallJumpStart_F_B_CMSG','WallJumpStart_B_F_CMSG','WallJumpStart_L_R_CMSG01','WallJumpStart_R_L_CMSG01']
assert all(nodes[n]==202420 for n in into)
assert all(nodes[n]in[202410,202411]for n in away)
assert {v['id']:v['impulses'][0]['row']for v in reference['clips']}=={202410:500,202411:501,202420:510}
rows={r['ID']:r['fields']for r in reference['rows']}
assert [rows[i]['horizontalVelocityChange']for i in [500,501,510]]==[4.8,4.8,-1.3]
assert reference['installedParameterRowsUnchanged']

raw=(GAME/'nightreign.exe').read_bytes();assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw);k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p;k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
image=k.VirtualAlloc(None,pe.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert image
callbacks=[];cases=[];owned=False;row_id=510
def ptr(buf):return c.addressof(buf)
def put(buf,off,fmt,value):struct.pack_into(fmt,buf,off,value)
def stub(rva,restype,args,fn):
 f=c.WINFUNCTYPE(restype,*args)(fn);callbacks.append(f);p=c.cast(f,c.c_void_p).value
 c.memmove(image+rva,b'\x48\xb8'+struct.pack('<Q',p)+b'\xff\xe0',12)
try:
 c.memmove(image,raw[:pe.OPTIONAL_HEADER.SizeOfHeaders],pe.OPTIONAL_HEADER.SizeOfHeaders)
 for s in pe.sections:c.memmove(image+s.VirtualAddress,s.get_data(),s.SizeOfRawData)
 for b in pe.DIRECTORY_ENTRY_BASERELOC:
  for rel in b.entries:
   if rel.type==10:
    p=c.c_uint64.from_address(image+rel.rva);p.value=(p.value+image-pe.OPTIONAL_HEADER.ImageBase)&0xffffffffffffffff
   else:assert rel.type==0
 velocity=c.create_string_buffer(0xe0);actor=c.create_string_buffer(0x200);modules=c.create_string_buffer(0x100);physics=c.create_string_buffer(0x100);row=c.create_string_buffer(0x20)
 position=(c.c_float*4)(0,0,0,1);rotation=(c.c_float*4)();put(velocity,0x20,'<Q',ptr(actor));put(actor,0x1b8,'<Q',ptr(modules));put(modules,0x68,'<Q',ptr(physics))
 def lookup(out,id):
  assert id==row_id;c.c_int.from_address(out).value=id;c.c_void_p.from_address(out+8).value=ptr(row)
 stub(0xe73840,None,[c.c_void_p,c.c_int],lookup)
 stub(0x4760a0,c.c_ubyte,[c.c_void_p,c.c_ubyte,c.c_void_p,c.c_void_p],lambda *_:0)
 stub(0x3ceb20,c.c_void_p,[c.c_void_p,c.c_void_p],lambda actor,out:(c.memmove(out,ptr(rotation),16),out)[1])
 stub(0x3d0e60,c.c_void_p,[c.c_void_p,c.c_void_p],lambda actor,out:(c.memmove(out,ptr(position),16),out)[1])
 stub(0x26909a0,None,[c.c_void_p],lambda _:None)
 getter_code=pe.get_data(0xe737b0,19);c.memmove(image+0x1000,getter_code,19)
 getter=c.WINFUNCTYPE(c.c_float,c.c_void_p)(image+0x1000)
 def horizontal(ref):
  value=getter(ref)
  return 4.8 if owned and row_id==510 else value
 stub(0xe737b0,c.c_float,[c.c_void_p],horizontal)
 launch=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p,c.c_void_p,c.c_void_p)(image+0x4739f0)
 initial=(c.c_float*4)(1,2,3,0);output=(c.c_float*4)();dt=c.create_string_buffer(16);put(dt,8,'<f',1/60)
 for yaw in [0.,math.pi/2,math.pi,-math.pi/2]:
  rotation[1]=yaw;results={}
  for row_id in [131,500,501,510]:
   fields=rows.get(row_id,dict(horizontalVelocityScale=0,verticalVelocityScale=0,horizontalVelocityChange=8,verticalVelocityChange=9,horizontalVelocityAngle=0))
   for owned in [False,True]:
    struct.pack_into('<fffff',row,0,*[fields[n]for n in ['horizontalVelocityScale','verticalVelocityScale','horizontalVelocityChange','verticalVelocityChange','horizontalVelocityAngle']])
    put(velocity,0x10,'<i',-1);put(velocity,0x14,'<i',row_id)
    launch(ptr(velocity),ptr(output),ptr(initial),ptr(dt));values=list(output);results[row_id,owned]=values
    assert abs(values[1]-fields['verticalVelocityChange'])<1e-4
    assert struct.unpack_from('<i',velocity,0x14)[0]==-1
    expected=4.8 if owned and row_id==510 else abs(fields['horizontalVelocityChange'])
    assert abs(math.hypot(values[0],values[2])-expected)<1e-4
    cases.append(dict(yaw=yaw,row=row_id,catOwner=owned,velocity=values))
  for row_id in [131,500,501]:assert results[row_id,False]==results[row_id,True]
  for axis in [0,2]:assert abs(results[510,True][axis]-results[500,False][axis])<1e-4
finally:k.VirtualFree(image,0,0x8000)
report=dict(nativeCases=len(cases),nativeBehaviorMapping=nodes,intoWallNodes=into,awayFromWallNodes=away,originalIntoWallSpeed=-1.3,catIntoWallSpeed=4.8,verticalVelocityUnchanged=True,otherJumpRowsAndNonownersUnchanged=True,oneShotConsumedByNative=True,parameterAnimationAndBehaviorAssetsUnchanged=True,gameProcessAccessed=False,gameplayVerified=False,multiplayerVerified=False)
(work/'native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f'PASS: four native into-wall selectors use the backward impulse; {len(cases)} isolated native launches verify Cat forward speed and unchanged height/other jumps')
