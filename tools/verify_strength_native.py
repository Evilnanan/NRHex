"""Read-only evidence for the scoped outgoing impact hook on the guarded EXE."""
from pathlib import Path
import hashlib,json,struct,xml.etree.ElementTree as ET,re
from local_paths import GAME
import pefile
root=Path(__file__).resolve().parents[1]
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
def read(rva,n):
 off=pe.get_offset_from_rva(rva);return raw[off:off+n]
def check(rva,expected):
 expected=bytes.fromhex(expected);assert read(rva,len(expected))==expected,hex(rva)
def call(rva):
 assert read(rva,1)==b'\xe8';return rva+5+struct.unpack('<i',read(rva+1,4))[0]
offset=0
for f in ET.parse(root/'.work/defs/AtkParam.xml').findall('./Fields/Field'):
 t,n=f.attrib['Def'].split(' ',1);n=n.split(' = ')[0]
 if n=='dmgLevel':break
 a=re.search(r'\[(\d+)\]',n)
 offset+={'f32':4,'s32':4,'u32':4,'s16':2,'u16':2,'u8':1,'s8':1,'dummy8':1}[t]*(int(a[1]) if a else 1)
assert offset==0x72
check(0xe709d4,'0F B6 41 72')
check(0xe709e8,'88 46 24') # AtkParam dmgLevel -> hit's outgoing input.
check(0x43d293,'44 88 4C 24 41 49 8B F0 48 8B DA')
check(0x43d31e,'44 0F B6 6D 7F') # Entry RSP+28 is the fifth flag byte.
check(0x4366c5,'88 5C 24 20 45 0F B6 CF 4C 8B C7 49 8B D6 48 8B CE')
assert call(0x4366d6)==0x43d260
check(0x43d2bf,'83 F8 01 4C 8B E3 74 03 4D 8B E5') # RDX attacker becomes R12 if ChrIns.
check(0x43d4b8,'4C 8B C6 49 8B D4');assert call(0x43d4be)==0x439e80
check(0x43a170,'41 0F B6 56 24')
check(0x43a178,'FF 50 30') # Native recipient damage-level remapping remains.
check(0x43a17b,'41 88 86 B6 02 00 00')
assert call(0x43da15)==0x48d980 # Native impact/poise handling within whole dispatch.
assert call(0x43e12f)==0x4391e0 and call(0x43e14c)==0x438dd0
check(0x438eb2,'0F B6 53 24') # Later reaction still reads the outgoing input.
check(0x4366e4,'83 BF 80 02 00 00 00') # Caller retains hit lifetime after return.
# A constructor emits every hit's original input before any target or replay.
check(0xe70350,'48 8B C4 48 89 58 10 55 56 57 41 54 41 55 41 56 41 57 48 8D A8 88 FE FF FF 48 81 EC 40 02 00 00')
check(0xe7039b,'49 8B 00 48 89 45 E0') # context[0] is copied source pointer.
check(0xe70a72,'48 8B 4D E0 48 85 C9')
check(0xe70a7f,'48 8B 01 FF 50 10 83 F8 01') # native source kind check.
check(0xe7070a,'8B 85 A0 01 00 00') # fifth int at entry RSP+28.
check(0xe70713,'48 8B 85 A8 01 00 00') # sixth pointer at entry RSP+30.
check(0xe7067b,'F3 0F 10 85 B0 01 00 00') # seventh float at entry RSP+38.
check(0x439efc,'41 F6 80 C1 02 00 00 08') # cached hits skip recalculation.
check(0x43e110,'F6 86 C1 02 00 00 08') # cached reaction path differs too.
source=(root/'src/lib.rs').read_text(encoding='utf8')
scope=source[source.index('unsafe extern "system" fn on_strength_constructed'):source.index('unsafe fn spatial_ready')]
assert 'read_ptr(context)!=local' in scope and 'has_effect(local,strength::OWNER,base)' in scope
assert 'strengthen_constructed_hit(hit)' in scope and '0x2c1' not in scope
assert 'on_strength_end' not in source and 'HIT_DISPATCH_ORIG' not in source
assert 'nlog_' not in scope
(root/'dist/reports/strength-native-validation.json').write_text(json.dumps(dict(version='0.9.1',ownerEffect=95001100,attackParamDmgLevelOffset='0x72',hitInputOffset='0x24',mapping={0:1,1:2,2:3,3:7,8:2},hookRva='0xe70350',abi='RCX hit; RDX detail; R8 source context; R9B flag; stack int, pointer, float',sourceContextOwnerOffset='0x0',nativeDefensesAndReactionsRetained=True,boostedInputRetainedForReplayAndCopies=True,sevenArgumentAbiUnitTested=True,gameplayVerified=False,multiplayerVerified=False),ensure_ascii=False,indent=2),encoding='utf8')
print('PASS: source-owned hit construction before cached-hit replay, all seven constructor arguments, native defense/reaction consumers verified.')
