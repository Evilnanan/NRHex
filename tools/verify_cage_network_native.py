"""Run NR's actual flag packet encoder/decoder against decoded cage selectors.

Transport and buffer allocation are fixtures; flag ordering is original code.
This proves the packet-level regression, not real cooperative gameplay.
"""
import ctypes as c
import hashlib, json, struct, sys, tomllib
from pathlib import Path
import pefile
from local_paths import GAME, ROOT
from cage_selector_events import Selector, verify_selector

work=ROOT/'.work/cage-network-01791'
work.mkdir(parents=True,exist_ok=True)
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw)
k=c.WinDLL('kernel32');k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint,c.c_uint];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint]
mem=k.VirtualAlloc(None,pe.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40);assert mem
callbacks=[]
def stub(rva,result,args,fn):
 f=c.WINFUNCTYPE(result,*args)(fn);callbacks.append(f)
 c.memmove(mem+rva,b'\x48\xb8'+struct.pack('<Q',c.cast(f,c.c_void_p).value)+b'\xff\xe0',12)
def put(b,offset,fmt,value):struct.pack_into(fmt,b,offset,value)
schema={(o['bank'],o['id']):(o['name'],[t for t,_ in o['args']]) for o in json.loads((ROOT/'data/reference/world/cage-opcodes.json').read_text())['opcodes']}
schema.update({(2003,17):('RandomlySetEventFlagInRange',['uint','uint','byte']), (2003,22):('BatchSetEventFlags',['uint','uint','byte']), (1003,14):('EndIfPlayerIsInWorldType',['byte','byte'])})
formats=dict(sbyte='b',byte='B',bool='B',int='i',uint='I',short='h',ushort='H',float='f')
def decode(i):
 name,types=schema[i['Bank'],i['ID']];raw=bytes.fromhex(i['hex']);at=0;args=[]
 for t in types:
  size=struct.calcsize(formats[t]);at=(at+size-1)//size*size
  args.append(struct.unpack_from('<'+formats[t],raw,at)[0]);at+=size
 assert len(raw)==(at+3)//4*4
 return name,args
try:
 c.memmove(mem,raw[:pe.OPTIONAL_HEADER.SizeOfHeaders],pe.OPTIONAL_HEADER.SizeOfHeaders)
 for sec in pe.sections:c.memmove(mem+sec.VirtualAddress,sec.get_data(),sec.SizeOfRawData)
 for block in pe.DIRECTORY_ENTRY_BASERELOC:
  for rel in block.entries:
   if rel.type==10:
    v=c.c_uint64.from_address(mem+rel.rva);v.value=(v.value+mem-pe.OPTIONAL_HEADER.ImageBase)&0xffffffffffffffff
   else:assert rel.type==0
 buffer=c.create_string_buffer(8192);manager=c.create_string_buffer(0x200)
 c.c_uint64.from_address(mem+0x3c21ec8).value=c.addressof(manager)
 wire=[]
 stub(0xdd99c0,c.c_void_p,[],lambda:c.addressof(buffer))
 stub(0xdd99d0,c.c_uint,[],lambda:len(buffer))
 def send(manager,kind,data,size):wire.append((kind,c.string_at(data,size)))
 def receive(manager,kind,data,capacity):
  assert wire and wire[0][0]==kind
  packet=wire.pop(0)[1];assert len(packet)<=capacity
  c.memmove(data,packet,len(packet));return len(packet)
 stub(0xdf24d0,None,[c.c_void_p,c.c_uint,c.c_void_p,c.c_uint],send)
 stub(0xdf1d40,c.c_uint,[c.c_void_p,c.c_uint,c.c_void_p,c.c_uint],receive)
 encode=c.WINFUNCTYPE(None,c.c_void_p,c.c_uint,c.c_void_p)(mem+0xe0d9b0)
 unpack=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p,c.c_uint,c.c_void_p)(mem+0xe0d7e0)
 def roundtrip(writes,kind=0x1a):
  assert writes
  records=c.create_string_buffer(8*len(writes));vector=c.create_string_buffer(32)
  for i,(flag,on) in enumerate(writes):struct.pack_into('<IB3x',records,8*i,flag,on)
  for off,value in [(8,c.addressof(records)),(16,c.addressof(records)+len(records)),(24,c.addressof(records)+len(records))]:put(vector,off,'<Q',value)
  encode(0,kind,c.addressof(vector));assert len(wire)==1
  result=c.create_string_buffer(8*len(writes));out=c.create_string_buffer(32)
  for off,value in [(8,c.addressof(result)),(16,c.addressof(result)),(24,c.addressof(result)+len(result))]:put(out,off,'<Q',value)
  assert unpack(0,kind,c.addressof(out)) and not wire
  size=struct.unpack_from('<Q',out,16)[0]-c.addressof(result);assert size==len(result)
  return [(flag,bool(on)) for flag,on in struct.iter_unpack('<IB3x',result.raw)]
 # Direct protocol regression: duplicate writes are preserved, but reordered.
 for kind in [0x1a,0x22]:
  assert roundtrip([(46507500,False),(46507500,True)],kind)==[(46507500,True),(46507500,False)]
 if '--full-build' in sys.argv:
  patch=json.loads((ROOT/'data/reference/world/cage-network-regression.json').read_text())
  built=json.loads((ROOT/'.work/cage/built.json').read_text())
  for r in patch['reports']:
   current=next(m for m in built['reports'] if m['mapName']==r['mapName'])
   r['after']=next(e for e in current['events'] if e['ID']==r['prefix']+9700)
   assert hashlib.sha256((ROOT/'dist/mod/event'/f"{r['mapName']}.emevd.dcx").read_bytes()).hexdigest()==current['outputSha256'].lower()
 else:
  patch=json.loads((work/'selectors.json').read_text())
 cases=0;legacy_failures=0
 for r in patch['reports']:
  p=r['prefix'];count=r['encounterCount']
  report=dict(mapName=r['mapName'],request=p+7402,ready=p+7403,choices=p+7500,cooling=p+7550,encounterCount=count)
  before=list(map(decode,r['before']['instructions']));after=list(map(decode,r['after']['instructions']))
  verify_selector(after,report,decode)
  assert not any(n in ('RandomlySetNetworkconnectedEventFlagInRange','BatchSetNetworkconnectedEventFlags') for n,a in after)
  for chosen in range(count):
   old=Selector(before,report);old.draw=chosen;old.step();old.step()
   peer=dict(roundtrip(old.network_writes))
   assert old.flags[p+7500+chosen] and peer[p+7403] and not peer[p+7500+chosen]
   legacy_failures+=1
   for retries in [0,3]:
    host=Selector(after,report);host.draw=chosen
    host.flags[p+7550+chosen]=True
    host.step()
    for _ in range(retries):host.step()
    assert not any(p+7500<=f<p+7550 for f,on in host.network_writes)
    host.flags[p+7550+chosen]=False;host.step()
    assert host.commits==[chosen]
    assert [(f,on) for f,on in host.network_writes if p+7500<=f<p+7550]==[(p+7500+chosen,True)]
    assert not any(p+7100<=f<p+7150 for f,on in host.network_writes)
    for kind in [0x1a,0x22]:
     decoded=roundtrip(host.network_writes,kind);peer=dict(decoded)
     assert peer[p+7403] and peer[p+7500+chosen]
     assert sum(peer.get(p+7500+i,False) for i in range(count))==1
     # Independent peers receive the same packet, even if delivery is delayed.
     assert dict(roundtrip(host.network_writes,kind))==peer
     cases+=1
    # A reload never redraws or writes another choice.
    reload=Selector(after,report);reload.flags.update(host.flags);reload.step()
    assert reload.random_calls==0 and not reload.network_writes
   remote=Selector(after,report,client=True);remote.step()
   assert remote.ended and not remote.network_writes
  host=Selector(after,report);peer={};host.step()
  for turn in range(200):
   host.draw=turn%count;host.network_writes.clear();host.step()
   peer.update(roundtrip(host.network_writes));assert peer[p+7403] and peer[p+7500+host.draw]
   assert sum(peer.get(p+7500+i,False) for i in range(count))==1
   # Encounter completion is a separate packet; retry selection never clears
   # shared choices. No timing assumption about sleeping between clear/draw.
   peer.update(roundtrip([(p+7500+host.draw,False),(p+7403,False)]))
   host.victory_and_reopen();host.step()
 result=dict(version=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version'],exeSha256=hashlib.sha256(raw).hexdigest(),nativeEncoderRva='0xe0d9b0',nativeDecoderRva='0xe0d7e0',oldHostPeerMismatchCases=legacy_failures,newPacketCases=cases,repeatedRounds=800,selectorLifecycleRegressionPassed=True,privateRejectedDraws=True,hostOnlySelection=True,reloadPreservesCommittedSelection=True,onlyFourSelectorsChanged=patch['onlyFourSelectorsChanged'],fixtures=['packet buffer','transport send/receive','decoded event interpreter'],gameMemoryAccessed=False,multiplayerVerified=False)
 (work/'native-validation.json').write_text(json.dumps(result,indent=2))
 print(f'PASS: {legacy_failures} old host/peer mismatches reproduced with native packet code; {cases} new packet cases pass')
finally:k.VirtualFree(mem,0,0x8000)
