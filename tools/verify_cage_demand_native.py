import ctypes as native
import hashlib
import json
import re
import struct
from pathlib import Path

import pefile
from local_paths import ROOT, GAME

raw = (GAME / 'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest() == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
binary = pefile.PE(data=raw, fast_load=True)
binary.parse_data_directories(directories=[5])
source = (ROOT / 'src/cage_demand.rs').read_text(encoding='utf-8')
signatures = re.findall(r'\(0x([0-9a-f]+),[^\n]*?&\[([0-9a-fx,]+)\]\)', source)
assert len(signatures) == 33
for address, values in signatures:
    expected = bytes(int(value, 16) for value in values.split(','))
    assert binary.get_data(int(address, 16), len(expected)) == expected, address

kernel = native.WinDLL('kernel32')
kernel.VirtualAlloc.argtypes = [native.c_void_p, native.c_size_t, native.c_uint, native.c_uint]
kernel.VirtualAlloc.restype = native.c_void_p
kernel.VirtualFree.argtypes = [native.c_void_p, native.c_size_t, native.c_uint]
base = kernel.VirtualAlloc(None, binary.OPTIONAL_HEADER.SizeOfImage, 0x3000, 0x40)
assert base
cases = []

def patch(address, code):
    native.memmove(base + address, code, len(code))

def pointer(buffer, offset, value):
    native.c_void_p.from_buffer(buffer, offset).value = value

try:
    native.memmove(base, raw[:binary.OPTIONAL_HEADER.SizeOfHeaders], binary.OPTIONAL_HEADER.SizeOfHeaders)
    for section in binary.sections:
        native.memmove(base + section.VirtualAddress, section.get_data(), section.SizeOfRawData)
    for block in binary.DIRECTORY_ENTRY_BASERELOC:
        for entry in block.entries:
            if entry.type == 10:
                value = native.c_uint64.from_address(base + entry.rva)
                value.value = (value.value + base - binary.OPTIONAL_HEADER.ImageBase) & 0xffffffffffffffff
            else:
                assert entry.type == 0

    patch(0xe3d7b0, bytes.fromhex('31 c0 c3'))
    patch(0x75bb30, bytes.fromhex('89 11 48 8b c1 c3'))
    patch(0x75b990, bytes.fromhex('c7 01 ff ff ff ff 48 8b c1 c3'))
    data = native.create_string_buffer(0x100)
    table = native.create_string_buffer(0x20)
    part = native.create_string_buffer(0x80)
    enemy = native.create_string_buffer(0x20)
    pointer(data, 0x98, native.addressof(table))
    native.c_int.from_buffer(table, 4).value = 2
    pointer(table, 0x10, native.addressof(part))
    native.c_int.from_buffer(part, 0xc).value = 2
    native.c_int.from_buffer(part, 0x10).value = 67
    native.c_int.from_buffer(part, 0x14).value = -1
    pointer(part, 0x68, native.addressof(enemy))
    native.c_int.from_buffer(enemy, 0xc).value = 95300012
    constructor = native.WINFUNCTYPE(native.c_void_p, native.c_void_p, native.c_void_p, native.c_void_p, native.c_int)(base + 0xe338a0)
    is_enemy = native.WINFUNCTYPE(native.c_ubyte, native.c_void_p)(base + 0xe351b0)
    get_index = native.WINFUNCTYPE(native.c_int, native.c_void_p)(base + 0xe34b70)
    get_npc = native.WINFUNCTYPE(native.c_int, native.c_void_p)(base + 0xe35940)
    release = native.WINFUNCTYPE(None, native.c_void_p)(base + 0xe33bd0)
    for index in [0, -1, 1, 999]:
        storage = native.create_string_buffer(b'\xa5' * 0x70, 0x70)
        address = native.addressof(storage) + 0x10
        assert constructor(address, native.addressof(data), None, index) == address
        assert storage.raw[:0x10] == b'\xa5' * 0x10 and storage.raw[0x60:] == b'\xa5' * 0x10
        assert bool(is_enemy(address)) == (index == 0)
        assert get_index(address) == (67 if index == 0 else -1)
        assert get_npc(address) == (95300012 if index == 0 else -1)
        release(address)
        cases.append({'partIndex': index, 'descriptorSize': 0x50, 'canariesIntact': True})

    collection = native.create_string_buffer(0x28)
    slots = native.create_string_buffer(3 * 0x10)
    pointer(collection, 0x18, native.addressof(slots))
    native.c_int.from_buffer(collection, 0x10).value = 3
    free_slot = native.WINFUNCTYPE(native.c_void_p, native.c_void_p, native.c_void_p)(base + 0x4af970)
    for occupied in [0, 1, 2, 3]:
        for index in range(3):
            pointer(slots, index * 0x10, 123 if index < occupied else None)
        native.c_int.from_buffer(collection, 0x20).value = 0
        output = native.create_string_buffer(16)
        before = slots.raw
        assert free_slot(native.addressof(collection), native.addressof(output)) == native.addressof(output)
        selected = native.c_void_p.from_buffer(output).value
        expected_index = next((index for index in [1, 2, 0] if index >= occupied), -1)
        assert selected == (native.addressof(slots) + expected_index * 16 if expected_index >= 0 else None)
        assert native.c_int.from_buffer(output, 8).value == expected_index
        assert slots.raw == before
        cases.append({'occupied': occupied, 'capacity': 3, 'nativeSlotWrites': False})

    patch(0x26909a0, bytes.fromhex('c3'))
    member = native.create_string_buffer(0x320)
    marker = native.WINFUNCTYPE(None, native.c_void_p, native.c_ubyte)(base + 0x3d5ee0)
    individual_drop = native.WINFUNCTYPE(native.c_ubyte, native.c_void_p, native.c_int, native.c_ubyte, native.c_int)(base + 0x3df550)
    for flags in [0, 0x02, 0xbf, 0xff]:
        native.c_ubyte.from_buffer(member, 0x1f3).value = flags
        native.c_ubyte.from_buffer(member, 0x1f4).value = 0x80
        marker(native.addressof(member), 1)
        assert native.c_ubyte.from_buffer(member, 0x1f3).value == flags | 0x40
        assert individual_drop(native.addressof(member), 6200000, 1, 6200000) == 0
        assert native.c_ubyte.from_buffer(member, 0x1f4).value == 0x82
        marker(native.addressof(member), 0)
        assert native.c_ubyte.from_buffer(member, 0x1f3).value == flags & ~0x40
        cases.append({'groupMemberSuppressesIndividualReward': True, 'originalFlags': flags})

    rider = native.create_string_buffer(0x320)
    mount = native.create_string_buffer(0x320)
    rider_entry = native.create_string_buffer(0x100)
    mount_entry = native.create_string_buffer(0x100)
    pointer(rider, 0x20, native.addressof(rider_entry))
    pointer(mount, 0x20, native.addressof(mount_entry))
    native.c_uint64.from_buffer(rider, 8).value = 0x2e461a0711000050
    native.c_uint64.from_buffer(mount, 8).value = 0x2e461a0711000052
    riding = native.WINFUNCTYPE(None, native.c_void_p, native.c_void_p, native.c_ubyte)(base + 0x3d7660)
    riding(native.addressof(rider), native.addressof(mount), 1)
    assert native.c_uint32.from_buffer(rider_entry, 0x80).value == 5
    assert native.c_uint32.from_buffer(mount_entry, 0x80).value == 6
    assert native.c_uint64.from_buffer(rider_entry, 0x88).value == native.c_uint64.from_buffer(mount, 8).value
    assert native.c_uint64.from_buffer(mount_entry, 0x88).value == native.c_uint64.from_buffer(rider, 8).value
    before = rider_entry.raw, mount_entry.raw
    riding(native.addressof(rider), native.addressof(mount), 1)
    assert before == (rider_entry.raw, mount_entry.raw)
    cases.append({'nativeRidingUsesEntry20': True, 'reciprocalHandles': True, 'idempotent': True})

    world = native.create_string_buffer(0x17540)
    primary_pool = native.create_string_buffer(0x128)
    pointer(world, 0x9910, native.addressof(primary_pool))
    native.c_uint32.from_buffer(primary_pool, 0x120).value = 12
    map_manager = native.create_string_buffer(8)
    map_vtable = native.create_string_buffer(0x30)
    pointer(map_manager, 0, native.addressof(map_vtable))
    map_slot = native.create_string_buffer(8)
    pointer(map_slot, 0, native.addressof(rider))
    lookups = []
    map_lookup = native.WINFUNCTYPE(native.c_void_p, native.c_void_p, native.c_void_p)(
        lambda manager, map_pointer: lookups.append(('map', native.c_uint32.from_address(map_pointer).value)) or native.addressof(data))
    actor_lookup = native.WINFUNCTYPE(native.c_void_p, native.c_void_p, native.c_void_p)(
        lambda context, handle_pointer: lookups.append(('actor', native.c_uint64.from_address(handle_pointer).value)) or native.addressof(map_slot))
    pointer(map_vtable, 0x28, native.cast(map_lookup, native.c_void_p).value)
    patch(0x5199b0, b'\x48\xb8' + struct.pack('<Q', native.addressof(map_manager)) + b'\xc3')
    patch(0x528590, b'\x48\xb8' + struct.pack('<Q', native.cast(actor_lookup, native.c_void_p).value) + b'\xff\xe0')
    patch(0x4e78c0, b'\xc3')
    map_resolve = native.WINFUNCTYPE(native.c_void_p, native.c_void_p, native.c_void_p)(base + 0x518f30)
    runtime_resolve = native.WINFUNCTYPE(native.c_void_p, native.c_void_p, native.c_void_p)(base + 0x519140)
    for map_id in [0x2e321a03, 0x2e3c1a06, 0x2e461a07, 0x2e501a08]:
        identifier = native.c_uint64((map_id << 32) | 0x11000050)
        assert not runtime_resolve(native.addressof(world), native.addressof(identifier))
        lookups.clear()
        assert map_resolve(native.addressof(world), native.addressof(identifier)) == native.addressof(rider)
        assert lookups == [('map', map_id), ('actor', identifier.value)]
        cases.append({'map': hex(map_id), 'mapActorResolverFindsPair': True, 'runtimeResolverRejectsMapHandle': True})

    patch(0x3d45a0, bytes.fromhex('31 c0 c3'))
    patch(0x26909a0, bytes.fromhex('c3'))
    generator = native.create_string_buffer(0x260)
    actor = native.create_string_buffer(0x20)
    entry = native.create_string_buffer(16)
    vector = (native.c_void_p * 32)()
    vtable = native.create_string_buffer(0x60)
    patch(0x1000, bytes.fromhex('c6 41 48 01 c3'))
    pointer(vtable, 0x30, base + 0x1000)
    pointer(generator, 0, native.addressof(vtable))
    pointer(generator, 0xd0, native.addressof(vector))
    pointer(generator, 0xd8, native.addressof(vector))
    native.c_ubyte.from_buffer(generator, 0xf0).value = 1
    native.c_ushort.from_buffer(generator, 0xf4).value = 1
    native.c_ushort.from_buffer(generator, 0xf6).value = 1
    native.c_int.from_buffer(generator, 0x40).value = 2
    native.c_int.from_buffer(generator, 0x44).value = 2
    pointer(actor, 0x18, native.addressof(entry))
    native.c_ubyte.from_buffer(entry, 0xa).value = 2
    select = native.WINFUNCTYPE(None, native.c_void_p, native.c_void_p)(base + 0x4ded70)
    select(native.addressof(generator), None)
    assert native.c_int.from_buffer(generator, 0x44).value == 2
    vector[0] = native.addressof(actor)
    pointer(generator, 0xd8, native.addressof(vector) + 8)
    select(native.addressof(generator), None)
    assert native.c_int.from_buffer(generator, 0x44).value == 3
    assert native.c_int.from_buffer(generator, 0x224).value == 1
    assert native.c_ubyte.from_buffer(generator, 0x48).value == 1
    cases.append({'emptyWaits': True, 'boundDormantActorAdvancesToSpawn': True})
finally:
    assert kernel.VirtualFree(base, 0, 0x8000)

output = ROOT / '.work/cage-demand/native-validation.json'
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps({'signatures': len(signatures), 'cases': cases, 'gameplayVerified': False}, indent=2), encoding='utf-8')
print(f'PASS: {len(signatures)} native signatures; {len(cases)} isolated descriptor/slot/generator/reward/riding cases; no game process modified')
