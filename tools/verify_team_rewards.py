"""Execute native team reward dispatch with creation and networking replaced by recording stubs."""
import ctypes as native,hashlib,json,struct
import pefile
from local_paths import ROOT,GAME

raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
image=pefile.PE(data=raw,fast_load=True)
def call(address):
 assert image.get_data(address,1)==b'\xe8'
 return address+5+struct.unpack('<i',image.get_data(address+1,4))[0]
assert call(0x576e6f)==0x5759e0
assert call(0x575a47)==0x57fde0 and call(0x575a4e)==0x57d0a0 and call(0x575aa3)==0x57fa30
assert image.get_data(0x576a7e,10)==bytes.fromhex('B9 03 00 00 00 BA 01 00 00 00')
assert call(0x576c02)==0x6a1bc0
kernel=native.WinDLL('kernel32')
kernel.VirtualAlloc.argtypes=[native.c_void_p,native.c_size_t,native.c_uint,native.c_uint];kernel.VirtualAlloc.restype=native.c_void_p
kernel.VirtualFree.argtypes=[native.c_void_p,native.c_size_t,native.c_uint]
memory=kernel.VirtualAlloc(None,image.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert memory
calls=[];cases=[];node=native.create_string_buffer(0xe0)
@native.WINFUNCTYPE(native.c_uint64,native.c_void_p,native.c_void_p)
def create(manager,descriptor):
 calls.append(('create',manager,descriptor));return native.addressof(node)
@native.WINFUNCTYPE(native.c_uint64,native.c_void_p,native.c_void_p)
def broadcast(manager,descriptor):
 calls.append(('broadcast',manager,descriptor));return 0
@native.WINFUNCTYPE(native.c_uint64,native.c_void_p,native.c_void_p)
def forward(manager,descriptor):
 calls.append(('forward',manager,descriptor));return 0
try:
 for section in image.sections:
  if section.SizeOfRawData:native.memmove(memory+section.VirtualAddress,raw[section.PointerToRawData:section.PointerToRawData+section.SizeOfRawData],section.SizeOfRawData)
 for address,callback in [(0x57d0a0,create),(0x57fa30,broadcast),(0x57fde0,forward)]:
  code=b'\x48\xb8'+struct.pack('<Q',native.cast(callback,native.c_void_p).value)+b'\xff\xe0';native.memmove(memory+address,code,len(code))
 native.memmove(memory+0x575aa8,b'\xe9'+struct.pack('<i',0x575b39-0x575aad),5)
 session=native.create_string_buffer(0x20)
 global_address=0x575a08+struct.unpack('<i',image.get_data(0x575a04,4))[0]
 native.memmove(memory+global_address,struct.pack('<Q',native.addressof(session)),8)
 dispatch=native.WINFUNCTYPE(native.c_uint64,native.c_void_p,native.c_void_p)(memory+0x5759e0)
 for mode in [0,3,6]:
  struct.pack_into('<i',session,0xc,mode)
  for slot in [1,2,3]:
   for network in [False,True]:
    descriptor=native.create_string_buffer(0xc0)
    struct.pack_into('<I',descriptor,0x58,0x12 if network else 2)
    struct.pack_into('<I',descriptor,0x5c,0x40012345|(slot<<27))
    struct.pack_into('<H',descriptor,0xaa,0x40|(slot<<7))
    struct.pack_into('<i',descriptor,0xb8,95410000)
    before=descriptor.raw;calls.clear();dispatch(0x123400,descriptor)
    expected=['forward'] if network and mode==6 else ['create','broadcast'] if network and mode==3 else ['create']
    assert [entry[0] for entry in calls]==expected
    assert all(entry[1]==0x123400 for entry in calls)
    assert all(entry[2]==(native.addressof(node)+0x20 if entry[0]=='broadcast' else native.addressof(descriptor)) for entry in calls)
    assert descriptor.raw==before
    cases.append(dict(sessionMode=mode,slot=slot,network=network,dispatch=expected))
finally:kernel.VirtualFree(memory,0,0x8000)
source=(ROOT/'src/red_envelope.rs').read_text()
assert 'spawn_personal_lot' not in source
assert 'spawn(manager,player,lot,0,1,1,0,position.as_ptr())' in source
assert 'red_envelope::observe_dispatch(manager,descriptor)' in (ROOT/'src/lib.rs').read_text()
report=dict(nativeCases=len(cases),cases=cases,nativePartyLoop=True,descriptorOwnershipUidAndNetworkFlagsPreserved=True,networkReturnValueNotUsedAsCreationReceipt=True,fixtureStubs=['local node creator','host broadcast','guest forwarding','post-dispatch TLS maintenance bypass'],gameplayVerified=False,multiplayerVerified=False)
(ROOT/'dist/team-reward-validation.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print(f'PASS: {len(cases)} native solo/host/guest dispatch cases; original per-slot owner and network flags unchanged; no packets sent')
