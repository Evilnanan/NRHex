"""Execute native menu confirmation construction and sound queuing, without audio output."""
import ctypes as native
import hashlib
import json
import struct
import capstone
import pefile
from local_paths import ROOT, GAME

raw = (GAME / 'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest() == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
binary = pefile.PE(data=raw, fast_load=True)
engine = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
engine.detail = True
kernel = native.WinDLL('kernel32', use_last_error=True)
kernel.VirtualAlloc.argtypes = [native.c_void_p, native.c_size_t, native.c_ulong, native.c_ulong]
kernel.VirtualAlloc.restype = native.c_void_p
kernel.VirtualFree.argtypes = [native.c_void_p, native.c_size_t, native.c_ulong]
kernel.VirtualFree.restype = native.c_int
memory = kernel.VirtualAlloc(None, 8192, 0x3000, 0x40)
assert memory
cases = []
try:
    queue = memory + 4096
    name = memory + 4200
    original_name = binary.get_data(0x2c39024, 6)
    assert original_name == '決定\0'.encode('utf-16-le')
    native.memmove(name, original_name, len(original_name))
    constructor_code = bytearray(binary.get_data(0x79aee0, 0x80))
    for instruction in engine.disasm(bytes(constructor_code), 0x79aee0):
        for operand in instruction.operands:
            if operand.type == capstone.CS_OP_MEM and operand.mem.base == capstone.x86_const.X86_REG_RIP:
                displacement = name - (memory + instruction.address - 0x79aee0 + instruction.size)
                struct.pack_into('<i', constructor_code, instruction.address - 0x79aee0 + instruction.disp_offset, displacement)
    native.memmove(memory, bytes(constructor_code), len(constructor_code))
    constructor = native.WINFUNCTYPE(native.c_void_p, native.c_void_p)(memory)
    options = native.create_string_buffer(0x60)
    assert constructor(native.addressof(options)) == native.addressof(options)
    descriptor = options.raw[0x48:0x60]
    assert struct.unpack('<iiiIQ', descriptor) == (100, 2, 1, 0, name)
    source = native.create_string_buffer(descriptor)
    sound_code = bytearray(binary.get_data(0x870290, 0x8e))
    references = []
    for instruction in engine.disasm(bytes(sound_code), 0x870290):
        for operand in instruction.operands:
            if operand.type == capstone.CS_OP_MEM and operand.mem.base == capstone.x86_const.X86_REG_RIP:
                target = instruction.address + instruction.size + operand.mem.disp
                assert target in (0x39aa560, 0x39aa564, 0x39aa570)
                references.append(target)
                displacement = queue + target - 0x39aa560 - (memory + 512 + instruction.address - 0x870290 + instruction.size)
                struct.pack_into('<i', sound_code, instruction.address - 0x870290 + instruction.disp_offset, displacement)
    assert len(references) == 3
    native.memmove(memory + 512, bytes(sound_code), len(sound_code))
    submit = native.WINFUNCTYPE(None, native.c_void_p)(memory + 512)
    for priority in (-1, 0, 1, 2, 3, 10):
        previous = struct.pack('<iiiIQ', 777, priority, 1, 0, name)
        native.memmove(queue, previous, 24)
        submit(native.addressof(source))
        expected = descriptor if priority <= 2 else previous
        assert native.string_at(queue, 24) == expected
        assert source.raw[:24] == descriptor
        cases.append(dict(pendingPriority=priority,confirmationQueued=priority <= 2))
    for sound_id, category in ((-1, 1), (100, -1)):
        previous = native.string_at(queue, 24)
        rejected = native.create_string_buffer(struct.pack('<iiiIQ', sound_id, 2, category, 0, name))
        submit(native.addressof(rejected))
        assert native.string_at(queue, 24) == previous
        cases.append(dict(soundId=sound_id,category=category,confirmationQueued=False))
finally:
    assert kernel.VirtualFree(memory, 0, 0x8000)

report = dict(nativeCases=len(cases),cases=cases,nativeConfirmationId=100,priority=2,category=1,
              nativeConstructorRva='0x79aee0',nativeSubmitRva='0x870290',nativeQueueRva='0x39aa560',
              nativeConstructorAndQueueExecuted=True,onlyRipDataReferencesRelocated=True,
              gameProcessAccessed=False,audioPlaybackVerified=False,gameplayVerified=False)
output = ROOT / '.work/hex-reroll/sound-validation.json'
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps(report, indent=2), encoding='utf8')
print(f'PASS: native confirmation descriptor and {len(cases)} native queue/priority cases; no live audio playback claimed')
