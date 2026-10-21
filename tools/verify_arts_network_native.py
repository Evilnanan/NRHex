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
assert image.get_data(0x20bde0, 12) == bytes.fromhex('48 81 C1 D8 01 00 00 E9 14 66 FE FF')
assert image.get_data(0xde21e4, 16) == bytes.fromhex('41 B9 F8 01 00 00 4C 8D 44 24 30 BA 0C 00 00 00')
assert image.get_data(0x66d7f0, 15) == bytes.fromhex('48 8B 81 A0 05 00 00 0F B6 80 6E 08 00 00 C3')
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
    callbacks=[]
    def stub(address, result, arguments, function):
        callback=ctypes.WINFUNCTYPE(result,*arguments)(function)
        callbacks.append(callback)
        code=b'\x48\xb8'+struct.pack('<Q',ctypes.cast(callback,ctypes.c_void_p).value)+b'\xff\xe0'
        ctypes.memmove(base+address,code,len(code))

    player_data=ctypes.create_string_buffer(0x870)
    player=ctypes.create_string_buffer(0x5a8)
    manager=ctypes.create_string_buffer(16)
    ctypes.c_void_p.from_buffer(player,0x5a0).value=ctypes.addressof(player_data)
    ctypes.c_void_p.from_buffer(manager,8).value=ctypes.addressof(player_data)
    ctypes.c_void_p.from_address(base+0x3c078d0).value=ctypes.addressof(manager)
    ctypes.c_void_p.from_address(base+0x3c21ec8).value=ctypes.addressof(network)
    ctypes.c_void_p.from_buffer(network,0x80).value=ctypes.addressof(peers)+0xf0
    eligibility=ctypes.WINFUNCTYPE(ctypes.c_ubyte,ctypes.c_void_p)(base+0x66d7f0)
    for enabled in [0,1]:
        ctypes.c_ubyte.from_buffer(player_data,0x86e).value=enabled
        assert eligibility(ctypes.addressof(player))==enabled
    body=bytes(index%251 for index in range(0x1f8))
    serialized=[]
    def serialize(data,output):
        serialized.append(data)
        ctypes.memmove(output,body,len(body))
        return 0
    stub(0x20bde0,ctypes.c_ubyte,[ctypes.c_void_p,ctypes.c_void_p],serialize)
    stub(0x1fe4f0,ctypes.c_ubyte,[],lambda:2)
    for address in [0x26909ec,0x2690a5c,0x26909a0]:
        ctypes.memmove(base+address,b'\xc3',1)
    received.clear()
    native_send=ctypes.WINFUNCTYPE(None,ctypes.c_void_p)(base+0xde20e0)
    native_send(None)
    assert serialized==[ctypes.addressof(player_data)]
    assert received==[(ctypes.addressof(network),ctypes.addressof(peers),0x0c,body)]
    ctypes.memmove(base+0x20bde0,image.get_data(0x20bde0,12),12)
    handles=[]
    def weapon_record(output,handle):
        value=ctypes.c_uint.from_address(handle).value
        handles.append(value)
        ctypes.memmove(output,bytes([value])*0x48,0x48)
        return output
    stub(0x6c16f0,ctypes.c_void_p,[ctypes.c_void_p,ctypes.c_void_p],weapon_record)
    for index in range(6):
        ctypes.c_uint.from_buffer(player_data,0x270+0x1c+index*4).value=index+1
    received.clear()
    native_send(None)
    assert handles==[1,2,3,4,5,6]
    actual=received[0][3]
    assert received[0][2]==0x0c and len(actual)==0x1f8
    assert actual[:0x1b0]==b''.join(bytes([index])*0x48 for index in range(1,7))
    assert actual[0x1b0:]==bytes(0x48)
    report = dict(nativeExecutionPassed=True, nativeBroadcasterRva='DF24D0', peerStride=0xf0,
                  peerCountsTested=[0, 1, 2], packetKind=0x0c, packetLength=0x1f8,
                  nativeEquipmentSenderRva='DE20E0', serializerRva='20BDE0',
                  nativeSixSlotSerializerExecuted=True,
                  oldPacket29ReceiverRejectsZero86e=True,
                  receiveState86fWrittenByDeserializer=True, publicSerializeCallerVerified=True,
                  transportReplaced=True, privateImageOnly=True, gameMemoryWritten=False,
                  gameplayVerified=False, multiplayerVerified=False)
    output = ROOT / 'work/arts-network/native-validation.json'
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2) + '\n', encoding='utf8')
    print('PASS: native player-equipment sender, six-slot serializer, peer transport and old receive gate; real multiplayer untested')
finally:
    assert kernel.VirtualFree(base, 0, 0x8000)
