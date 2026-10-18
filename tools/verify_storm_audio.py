"""Check original Storm Ruler audio events and the native named-bank request ABI."""
import hashlib
import json
import struct

import pefile
from local_paths import ROOT, GAME

directory = ROOT / '.work/storm-audio/native'
inputs = json.loads((directory / 'inputs.json').read_text(encoding='utf8'))
assert hashlib.sha256((GAME / 'sd/sd.bhd').read_bytes()).hexdigest().upper() == inputs['sourceBhdSha256']
assert inputs['installedSoundsUnchanged']

def events(path):
    data = path.read_bytes()
    offset = 0
    result = set()
    while offset + 8 <= len(data):
        kind = data[offset:offset + 4]
        size = struct.unpack_from('<I', data, offset + 4)[0]
        if kind == b'\0' * 4 and size == 0:
            break
        body = data[offset + 8:offset + 8 + size]
        assert len(body) == size
        offset += 8 + size
        if kind == b'HIRC':
            position = 4
            for index in range(struct.unpack_from('<I', body)[0]):
                category = body[position]
                length, identity = struct.unpack_from('<II', body, position + 1)
                if category == 4:
                    result.add(identity)
                position += 5 + length
            assert position == len(body)
    assert not any(data[offset:])
    return result

def fnv(name):
    result = 2166136261
    for value in name.lower().encode('ascii'):
        result = ((result * 16777619) ^ value) & 0xffffffff
    return result

banks = {}
for name, digest in inputs['banks'].items():
    path = directory / name
    assert hashlib.sha256(path.read_bytes()).hexdigest().upper() == digest
    banks[name] = events(path)
tae_ids = sorted({struct.unpack_from('<ii', bytes.fromhex(row['hex']))[1]
                  for row in inputs['sounds'] if struct.unpack_from('<i', bytes.fromhex(row['hex']))[0] == 5})
assert tae_ids == [450956, 450957, 450958, 450959]
storm_ids = [450940, 450941, 450946, 450951, 450954, 450955, 450956, 450957, 450958, 450959, 450960, 450965]
for identity in storm_ids:
    event = fnv(f'Play_s{identity:09}')
    assert event in banks['cs_m19.bnk']
    assert all(event not in banks[name] for name in ['cs_main.bnk', 'cs_smain.bnk'])
raw = (GAME / 'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest() == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
image = pefile.PE(data=raw, fast_load=True)
assert image.get_data(0xf14360, 5) == bytes.fromhex('E9 A0 53 A2 FF')
assert image.get_data(0xeee03d, 14) == bytes.fromhex('45 33 C9 4C 8B C3 48 8B D7 E8 15 63 02 00')
assert image.get_data(0x21f599, 5) == b'\xe8' + struct.pack('<i', 0xf14360 - 0x21f59e)
assert image.get_data(0x21f59e, 7) == bytes.fromhex('49 89 87 30 02 00 00')
report = dict(staticChecksPassed=True, nativeBank='cs_m19', bankHashes=inputs['banks'],
              sourceBhdSha256=inputs['sourceBhdSha256'], sfxEvents=storm_ids, taeSoundIds=tae_ids,
              installedSoundEventsUnchanged=True, missingFromSharedBanks=True,
              nativeRequestRva='F14360', nativeFourArgumentCallVerified=True, nativeReferenceReturnVerified=True,
              audioFilesAddedToMod=0, gameMemoryWritten=False, gameplayVerified=False, multiplayerVerified=False)
output = ROOT / 'work/storm-audio/validation.json'
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps(report, indent=2) + '\n', encoding='utf8')
print('PASS: 12 Storm Ruler audio events in cs_m19, four native action sound IDs, named-bank request ABI; audible playback untested')
