"""Execute the untouched native damage combiner in an isolated PE image."""
import ctypes as c
import hashlib
import json
import struct
from pathlib import Path
import pefile
from local_paths import GAME, ROOT

raw = (GAME / 'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest() == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe = pefile.PE(data=raw, fast_load=True)
pe.parse_data_directories(directories=[5])
k = c.WinDLL('kernel32')
k.VirtualAlloc.argtypes = [c.c_void_p, c.c_size_t, c.c_uint, c.c_uint]
k.VirtualAlloc.restype = c.c_void_p
k.VirtualFree.argtypes = [c.c_void_p, c.c_size_t, c.c_uint]
memory = k.VirtualAlloc(None, pe.OPTIONAL_HEADER.SizeOfImage, 0x3000, 0x40)
assert memory
reports = []
try:
 c.memmove(memory, raw[:pe.OPTIONAL_HEADER.SizeOfHeaders], pe.OPTIONAL_HEADER.SizeOfHeaders)
 for section in pe.sections:
  c.memmove(memory + section.VirtualAddress, section.get_data(), section.SizeOfRawData)
 delta = memory - pe.OPTIONAL_HEADER.ImageBase
 for block in pe.DIRECTORY_ENTRY_BASERELOC:
  for rel in block.entries:
   if rel.type == 10:
    value = c.c_uint64.from_address(memory + rel.rva)
    value.value = (value.value + delta) & 0xffffffffffffffff
   else:
    assert rel.type == 0
 # No dependency stubs. Keep disabled-case total >= 1 to avoid the protected
 # subunit rounding helper, which is unrelated to defensive penetration.
 calculate = c.WINFUNCTYPE(c.c_float, *([c.c_void_p] * 8 + [c.c_float] * 7))(memory + 0x6c5fc0)
 array = c.c_float * 5
 attack = array(100, 200, 300, 400, 500)
 rates = array(1.2, 1.5, 2, .5, 1)
 neutral = array(*([1.] * 5))
 scalar = [.25, .5, .25, 2., 1.5, .5, .75]
 for bypass in [False, True]:
  for layer in range(5):
   for cut in [.2, .5, 2.]:
    coefficients = [array(*([1.] * 5)) for _ in range(5)]
    coefficients[layer] = array(*([cut] * 5))
    before = [bytes(a) for a in coefficients]
    output = array()
    vulnerability = array(*(max(coefficients[0][i] * coefficients[3][i] * coefficients[4][i], 1.) for i in range(5)))
    args = [vulnerability, neutral, neutral, neutral, neutral] if bypass else coefficients
    scalars = [1., scalar[1], 1., scalar[3], scalar[4], 1., scalar[6]] if bypass else scalar
    total = calculate(c.addressof(output), c.addressof(attack), c.addressof(rates),
                      *map(c.addressof, args), *scalars)
    factor = 1.
    for v in scalars:
     factor *= v
    applied = (max(cut, 1.) if layer in [0, 3, 4] else 1.) if bypass else cut
    expected = [float(attack[i]) * float(rates[i]) * applied * factor for i in range(5)]
    for got, wanted in zip(output, expected):
     assert abs(got - wanted) < .002, (layer, cut, bypass, got, wanted)
    assert abs(total - sum(expected)) < .005, (total, expected)
    assert before == [bytes(a) for a in coefficients]
    reports.append(dict(bypass=bypass, layer=layer, cut=cut, output=list(output), total=total))
 # Mixed elemental reduction/vulnerability, including cancellation between
 # native DamageCutRate categories. Armor and guard must never contribute.
 mixed = [array(.5,.75,1.5,0,1), array(*([3.]*5)), array(*([3.]*5)),
          array(1.5,2,1,4,1), array(1,1,.5,2,2)]
 before = [bytes(a) for a in mixed]
 retained = array(*(max(mixed[0][i]*mixed[3][i]*mixed[4][i],1.) for i in range(5)))
 for repeat in range(2):
  output = array()
  total = calculate(c.addressof(output),c.addressof(attack),c.addressof(rates),
                    *map(c.addressof,[retained,neutral,neutral,neutral,neutral]),*([1.]*7))
  expected = [attack[i]*rates[i]*[1.,1.5,1.,1.,2.][i] for i in range(5)]
  assert all(abs(a-b)<.002 for a,b in zip(output,expected))
  assert abs(total-sum(expected))<.005 and before==[bytes(a) for a in mixed]
  reports.append(dict(mixedDamageCutGroups=True,repeat=repeat,output=list(output),total=total))
 # Immunity expressed as a numerical defensive coefficient, including a 100%
 # shield. Collision invulnerability/attack-nullification flags remain separate.
 out = array()
 total = calculate(c.addressof(out), c.addressof(attack), c.addressof(rates),
                   *([c.addressof(neutral)] * 5), 1., .5, 1., 2., 1.5, 1., .75)
 assert total > 0 and all(v > 0 for v in out)
 # Native pre-combiner object-defense aggregation reads +318 only from
 # valid effect nodes. The Rust manager guard is tested separately.
 manager = c.create_string_buffer(0x10)
 node = c.create_string_buffer(0x70)
 effect = c.create_string_buffer(0x320)
 c.c_void_p.from_buffer(manager, 8).value = c.addressof(node)
 c.c_void_p.from_buffer(node, 0).value = c.addressof(effect)
 object_defense = c.WINFUNCTYPE(c.c_float, c.c_void_p)(memory + 0x506700)
 object_cases = []
 for flags in [0, 1, 0x40000, 0x80000000]:
  for value in [0., .2, .5, 2.]:
   c.c_uint32.from_buffer(node, 0x68).value = flags
   c.c_float.from_buffer(effect, 0x318).value = value
   actual = object_defense(c.addressof(manager))
   expected = value if flags == 0 else 1.
   assert abs(actual - expected) < .000001
   object_cases.append(dict(flags=flags, factor=value, nativeFactor=actual))
 # Verify the resolved-owner argument and placement before generic NPC dispatch.
 def call(site):
  data = pe.get_data(site, 5)
  assert data[0] == 0xe8
  return site + 5 + struct.unpack('<i', data[1:])[0]
 assert call(0x43d4be) == 0x439e80
 assert pe.get_data(0x43d4b8, 6) == bytes.fromhex('4c 8b c6 49 8b d4')
 assert pe.get_data(0x43d2bf, 11) == bytes.fromhex('83 f8 01 4c 8b e3 74 03 4d 8b e5')
 assert pe.get_data(0x2c16f10 + 0x3d8, 8) == struct.pack('<Q', 0x1404da660)
 assert pe.get_data(0x2c307f8 + 0x3d8, 8) == struct.pack('<Q', 0x1406695a0)
 # Gaze uses the same combiner with a per-recipient copied holy component.
 # The native PlayerIns predicate is identity-based, not the local-PC getter.
 assert pe.get_data(0x2c307f8 + 0x118,8)==struct.pack('<Q',0x1404d2f00)
 assert pe.get_data(0x2c18120 + 0x118,8)==struct.pack('<Q',0x1403d45c0)
 player_query=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p)(memory+0x4d2f00)
 enemy_query=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p)(memory+0x3d45c0)
 assert player_query(0x111)==1 and player_query(0x222)==1 and enemy_query(0x333)==0
 gaze_cases=[]
 for damage in [252,328,390,464,640,743]:
  for scale in [1.,4.]:
   for guard_rate in [1.,.25]:
    incoming=array(0,0,0,0,damage*scale);defense=array(*([.5]*5));guard=array(*([guard_rate]*5));output=array()
    total=calculate(c.addressof(output),c.addressof(incoming),c.addressof(neutral),c.addressof(neutral),
      c.addressof(defense),c.addressof(guard),c.addressof(neutral),c.addressof(neutral),*([1.]*7))
    expected=damage*scale*.5*guard_rate
    assert abs(output[4]-expected)<.001 and list(output)[:4]==[0.]*4
    assert list(incoming)==[0.,0.,0.,0.,damage*scale]
    gaze_cases.append(dict(quarterBase=damage,playerScale=scale,defense=.5,guard=guard_rate,holyOutput=output[4],nativeTotal=total))
finally:
 assert k.VirtualFree(memory, 0, 0x8000)
work = ROOT / '.work/glass-cannon-01798'
work.mkdir(exist_ok=True)
(work / 'native-validation.json').write_text(json.dumps(dict(
 cases=reports, objectDefenseCases=object_cases, nativeCasesPassed=len(reports)+1+len(object_cases), dependencyStubs=[],
 fiveElementsAndFiveDefenseLayersVerified=True, originalInputsPreserved=True,
 netDamageCutVulnerabilityPreserved=True, armorAndGuardVulnerabilityBypassed=True,
 sourceResolvedByNative=True, originalFlagsPreserved=True,
 gameMemoryWritten=False, gameplayVerified=False, multiplayerVerified=False), indent=2), encoding='utf-8')
print(f'PASS: {len(reports)+1+len(object_cases)} native cases; five damage elements, five defensive layers, shield and source resolution; no game writes')
gaze_work=ROOT/'.work/devils-gaze-player-damage';gaze_work.mkdir(exist_ok=True)
(gaze_work/'native-validation.json').write_text(json.dumps(dict(cases=gaze_cases,playerPredicateCoversLocalAndRemoteInstances=True,
 nativeDefensiveCoefficientsPreserved=True,gameMemoryWritten=False,gameplayVerified=False,multiplayerVerified=False),indent=2),encoding='utf8')
print('PASS: 24 gaze damage cases; player versus enemy identity and native defense/guard coefficients preserved.')
