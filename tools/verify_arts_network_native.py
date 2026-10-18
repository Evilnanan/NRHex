"""Execute the native equipment broadcaster in private memory, with transport replaced."""
import ctypes
import hashlib
import json
import struct
from pathlib import Path

import pefile
from local_paths import ROOT, GAME

raw = (GAME / 'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest() == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
image = pefile.PE(data=raw, fast_load=True)
image.parse_data_directories(directories=[5])
assert image.get_data(0x20aa4a, 8) == bytes.fromhex('41 C6 85 6F 08 00 00 01')
assert image.get_data(0x202886, 18) == bytes.fromhex('0F B6 80 D3 00 00 00 C0 E8 04 24 01 88 87 6E 08 00 00')
assert image.get_data(0x20cb0c, 5) == b'\xe8' + struct.pack('<i', 0x6c16f0 - 0x20cb11)
assert image.get_data(0x4f2815, 17) == bytes.fromhex('44 8D 4B 09 4C 8B C6 BA 29 00 00 00 E8 AA FC 8F 00')
kernel = ctypes.WinDLL('kernel32', use_last_error=True)
kernel.VirtualAlloc.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_uint, ctypes.c_uint]
kernel.VirtualAlloc.restype = ctypes.c_void_p
kernel.VirtualFree.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_uint]
base = kernel.VirtualAlloc(None, image.OPTIONAL_HEADER.SizeOfImage, 0x3000, 0x40)
assert base
try:
    ctypes.memmove(base, raw[:image.OPTIONAL_HEADER.SizeOfHeaders], image.OPTIONAL_HEADER.SizeOfHeaders)
    for section in image.sections:
        ctypes.memmove(base + section.VirtualAddress, section.get_data(), section.SizeOfRawData)
    for block in image.DIRECTORY_ENTRY_BASERELOC:
        for relocation in block.entries:
            if relocation.type == 10:
                address = ctypes.c_uint64.from_address(base + relocation.rva)
                address.value = (address.value + base - image.OPTIONAL_HEADER.ImageBase) & 0xffffffffffffffff
            else:
                assert relocation.type == 0
    received = []

    @ctypes.WINFUNCTYPE(None, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_uint, ctypes.c_void_p, ctypes.c_uint)
    def transport(network, peer, kind, packet, length):
        received.append((network, peer, kind, ctypes.string_at(packet, length)))

    jump = b'\x48\xb8' + struct.pack('<Q', ctypes.cast(transport, ctypes.c_void_p).value) + b'\xff\xe0'
    ctypes.memmove(base + 0xdf2270, jump, len(jump))
    broadcast = ctypes.WINFUNCTYPE(None, ctypes.c_void_p, ctypes.c_uint, ctypes.c_void_p, ctypes.c_uint)(base + 0xdf24d0)
    network = ctypes.create_string_buffer(0x90)
    peers = ctypes.create_string_buffer(2 * 0xf0)
    packet = bytes([4]) + struct.pack('<Q', 0x123456789abcdef0) + bytes(range(72))
    payload = ctypes.create_string_buffer(packet)
    for count in range(3):
        received.clear()
        ctypes.c_void_p.from_buffer(network, 0x78).value = ctypes.addressof(peers)
        ctypes.c_void_p.from_buffer(network, 0x80).value = ctypes.addressof(peers) + count * 0xf0
        broadcast(ctypes.addressof(network), 0x29, payload, len(packet))
        assert received == [(ctypes.addressof(network), ctypes.addressof(peers) + index * 0xf0, 0x29, packet) for index in range(count)]
    report = dict(nativeExecutionPassed=True, nativeBroadcasterRva='DF24D0', peerStride=0xf0,
                  peerCountsTested=[0, 1, 2], packetKind=0x29, packetSubtype=4,
                  receiveState86fWrittenByDeserializer=True, publicSerializeCallerVerified=True,
                  transportReplaced=True, privateImageOnly=True, gameMemoryWritten=False,
                  gameplayVerified=False, multiplayerVerified=False)
    output = ROOT / 'work/arts-network/native-validation.json'
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2) + '\n', encoding='utf8')
    print('PASS: native public-equipment broadcaster, peer-vector layout and receive-state provenance; real multiplayer untested')
finally:
    assert kernel.VirtualFree(base, 0, 0x8000)
