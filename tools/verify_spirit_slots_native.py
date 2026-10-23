"""Execute the original empty-slot selector; no live process or native actor.

This covers state selection, not physical teardown or a full game spawn.
"""
from pathlib import Path
import ctypes as c
import hashlib
import itertools
import json
import os
import re
import pefile

root = Path(__file__).resolve().parents[1]
game = Path(os.environ.get('NRHEX_GAME', str(Path(root.anchor) / 'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game')))
raw = (game / 'nightreign.exe').read_bytes()
digest = hashlib.sha256(raw).hexdigest()
assert digest == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe = pefile.PE(data=raw, fast_load=True)
source = (root / 'src/lib.rs').read_text(encoding='utf8')
pattern = re.search(r'SPIRIT_SLOT_PATTERN:&str="([^"]+)"', source)[1]
code = bytes.fromhex(pattern)
assert pe.get_data(0x3922c0, len(code)) == code
k = c.WinDLL('kernel32')
k.VirtualAlloc.argtypes = [c.c_void_p, c.c_size_t, c.c_ulong, c.c_ulong]
k.VirtualAlloc.restype = c.c_void_p
k.VirtualFree.argtypes = [c.c_void_p, c.c_size_t, c.c_ulong]
arena = k.VirtualAlloc(None, 4096, 0x3000, 0x40)
assert arena
cases = 0
try:
    c.memmove(arena, code, len(code))
    select = c.WINFUNCTYPE(c.c_int32, c.c_void_p)(arena)
    slots = c.create_string_buffer(0x240)

    def check(states):
        global cases
        for i, state in enumerate(states):
            slots[0x58 + i * 0x50] = bytes([state])
        before = slots.raw
        expected = next((i for i, value in enumerate(states) if value == 255), -1)
        assert select(c.addressof(slots)) == expected, states
        assert slots.raw == before
        cases += 1

    for states in itertools.product([0, 1, 2, 3, 255], repeat=6):
        check(states)
    for slot in range(6):
        for value in range(256):
            states = [3] * 6
            states[slot] = value
            check(states)
    report = dict(version='0.17.42', exeSha256=digest, selectorRva='3922C0',
                  cases=cases, onlyFFIsFree=True, selectorDoesNotMutateSlots=True,
                  nativeLifecycleRvas=['3922F0', '391780'],
                  lifecycleEvidence='Static: 1 -> 2 on death/unload; 2 -> 3 after timer; 391780 removes collision/world actor then sets FF.',
                  regression='Old >1 predicate selected dying/removing states 2 and 3.',
                  gameplayVerified=False, multiplayerVerified=False,
                  limits='Executes the real selector only. Does not reproduce a full physics crash or prove game-thread safety.')
    (root / 'dist/reports/spirit-slot-validation.json').write_text(json.dumps(report, indent=2), encoding='utf8')
    print(f'PASS: {cases} original native slot-selection cases; no gameplay claim')
finally:
    k.VirtualFree(arena, 0, 0x8000)
