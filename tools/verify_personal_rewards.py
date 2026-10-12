"""Read-only native evidence and isolated execution; never attaches to the game."""
from pathlib import Path
import ctypes, hashlib, json, struct
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_64

root = Path(__file__).resolve().parents[1]
exe = Path(r'D:\Program Files (x86)\Steam\steamapps\common\ELDEN RING NIGHTREIGN\Game\nightreign.exe')
raw = exe.read_bytes()
digest = hashlib.sha256(raw).hexdigest()
assert digest == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe = pefile.PE(data=raw, fast_load=True)
base = pe.OPTIONAL_HEADER.ImageBase
def read(rva, size):
    off = pe.get_offset_from_rva(rva)
    return raw[off:off + size]
def check(rva, hexbytes):
    expected = bytes.fromhex(hexbytes)
    assert read(rva, len(expected)) == expected, hex(rva)
def call(rva):
    assert read(rva, 1) == b'\xe8'
    return rva + 5 + struct.unpack('<i', read(rva + 1, 4))[0]

# Party loop, three slot-specific reward descriptors, slot bits, network type.
check(0x576a7e, 'B9 03 00 00 00 BA 01 00 00 00')
check(0x576eb9, '41 FF C6')
assert call(0x576aa6) == 0x1fe7b0
assert call(0x576e6f) == 0x5759e0
check(0x576b90, '44 0F B6 A5 20 01 00 00')  # fifth ABI argument -> enabled bit6
check(0x576b98, '66 41 C1 E4 06')
check(0x576b9d, '66 41 83 E6 03')
check(0x576ba6, '66 C1 E0 07')             # personal visibility slot bits7..8
check(0x576baf, '66 41 C1 E6 09')          # companion slot bits9..10
assert call(0x576bf7) == 0x6a1bd0
assert call(0x576c02) == 0x6a1bc0          # network bit4 = true
check(0x6a1ba0, '8B 01 C1 E8 04 24 01 C3')
check(0x572c1a, 'F6 81 AA 00 00 00 40')
check(0x572ce1, 'C1 EA 07 83 E2 03 3B D1 75 0D')
assert call(0x5754e3) == 0x572c10          # pickup uses the visibility predicate
assert call(0x5755f8) == 0x6a1ba0
check(0x5755ff, '0F 84 9E 01 00 00')      # no-network -> direct pickup branch
assert call(0x5757c6) == 0x57ca60          # native reward grant remains intact
assert call(0x5757d3) == 0x57d400          # native local cleanup
check(0x5759fb, '0F 84 2D 01 00 00')      # no-network creation -> local creator
assert call(0x575b34) == 0x57d0a0
assert call(0x575aa3) == 0x57fa30          # synchronized creation broadcast
assert call(0x575a47) == 0x57fde0          # other network mode request
check(0x575856, '0F 84 62 01 00 00')      # no-network owner removal -> local branch
assert call(0x5759c4) == 0x57cf30

k32 = ctypes.WinDLL('kernel32', use_last_error=True)
k32.VirtualAlloc.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_ulong, ctypes.c_ulong]
k32.VirtualAlloc.restype = ctypes.c_void_p
k32.VirtualFree.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_ulong]
k32.VirtualFree.restype = ctypes.c_int
k32.FlushInstructionCache.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_size_t]
memory = k32.VirtualAlloc(None, 0x2000, 0x3000, 0x40)
assert memory
def put(offset, data):
    ctypes.memmove(memory + offset, bytes(data), len(data))
def relcall(offset, target):
    put(offset, b'\xe8' + struct.pack('<i', target - offset - 5))
try:
    # Exact original predicate, relocated only to isolated fake session data.
    # Type2 skips the unrelated native NPC/special-drop branch.
    put(0, read(0x572c10, 0xf5))
    put(0x200, read(0x6a1b50, 6))
    relcall(0x572c2e - 0x572c10, 0x200)
    global_insn = 0x572cc4 - 0x572c10
    put(global_insn + 3, struct.pack('<i', 0x300 - global_insn - 7))
    put(0x300, struct.pack('<Q', memory + 0x400))
    put(0x408, struct.pack('<Q', memory + 0x500))
    descriptor = ctypes.create_string_buffer(0xc0)
    struct.pack_into('<I', descriptor, 0x58, 2)
    visibility = ctypes.WINFUNCTYPE(ctypes.c_uint8, ctypes.c_void_p)(memory)
    k32.FlushInstructionCache(ctypes.c_void_p(-1), memory, 0x2000)
    matrix = []
    for viewer in (1, 2, 3):
        put(0x508, struct.pack('<I', viewer))
        for owner in (0, 1, 2, 3):
            for enabled in (False, True):
                struct.pack_into('<H', descriptor, 0xaa, (0x40 if enabled else 0) | owner << 7)
                result = bool(visibility(ctypes.addressof(descriptor)))
                assert result == (enabled and (owner == 0 or owner == viewer))
                matrix.append(dict(viewer=viewer, owner=owner, enabled=enabled, visibleAndPickupEligible=result))

    # Execute the original complete dispatch function with network bit false.
    # Only its flag getter and local creator are relocated. Creator is a stub,
    # so no reward, packet, game memory, or game state is changed.
    dispatch_start = 0x800
    put(dispatch_start, read(0x5759e0, 0x1aa))
    put(0xb00, read(0x6a1ba0, 8))
    relcall(dispatch_start + 0x5759f4 - 0x5759e0, 0xb00)
    # Stub records descriptor and manager, increments a call count, returns7.
    stub = bytearray()
    for opcode, dest in [(b'\x48\x89\x15', 0xc08), (b'\x48\x89\x0d', 0xc10)]:
        next_offset = 0xb40 + len(stub) + 7
        stub += opcode + struct.pack('<i', dest - next_offset)
    next_offset = 0xb40 + len(stub) + 6
    stub += b'\xff\x05' + struct.pack('<i', 0xc00 - next_offset)
    stub += b'\xb8\x07\x00\x00\x00\xc3'
    put(0xb40, stub)
    relcall(dispatch_start + 0x575b34 - 0x5759e0, 0xb40)
    k32.FlushInstructionCache(ctypes.c_void_p(-1), memory, 0x2000)
    dispatch = ctypes.WINFUNCTYPE(ctypes.c_uint64, ctypes.c_void_p, ctypes.c_void_p)(memory + dispatch_start)
    struct.pack_into('<I', descriptor, 0x58, 2)  # network bit4 clear
    result = dispatch(0x123400, ctypes.addressof(descriptor))
    assert result == 7
    assert ctypes.c_uint32.from_address(memory + 0xc00).value == 1
    assert ctypes.c_uint64.from_address(memory + 0xc08).value == ctypes.addressof(descriptor)
    assert ctypes.c_uint64.from_address(memory + 0xc10).value == 0x123400
finally:
    assert k32.VirtualFree(memory, 0, 0x8000)

md = Cs(CS_ARCH_X86, CS_MODE_64)
def lines(rva, size):
    return [f'{i.address-base:08X} {i.mnemonic} {i.op_str}' for i in md.disasm(read(rva, size), base+rva)]
report = dict(exeSha256=digest, gameVersion=pe.VS_FIXEDFILEINFO[0].FileVersionLS if hasattr(pe, 'VS_FIXEDFILEINFO') else None,
    existingSpawnLoopsPartySlots=True, slotCount=3, personalVisibilityAndPickupGuardConfirmed=True,
    descriptorVisibleBit='descriptor+0xAA bit6', descriptorOwner='descriptor+0xAA bits7..8',
    descriptorNetworkFlag='descriptor+0x58 bit4',
    nativeVisibilityMatrix=matrix, nativeNonNetworkDispatchExecutedInIsolation=True,
    networkCallsNotReachedOnLocalBranch=True, localPickupAndRemovalBranchesStaticallyConfirmed=True,
    actualGameStateChanged=False, gameplayOrMultiplayerVerified=False,
    implementationCandidate='Filter only the local slot during our custom spawn; clear only its descriptor network bit; keep native lottery, owner, visuals and pickup.',
    evidence={'partyLoop': lines(0x576a6f, 0x51), 'ownerBits': lines(0x576b90, 0x24),
        'visibility': lines(0x572cb3, 0x51), 'dispatch': lines(0x5759e0, 0x21),
        'localDispatchTail': lines(0x575b2e, 0x16), 'pickupOwnerGuard': lines(0x5754ce, 0x22),
        'pickupNetworkBranch': lines(0x5755f4, 0x17), 'localPickup': lines(0x5757a3, 0x35),
        'localRemoval': lines(0x5759be, 0x1b)})
(root/'dist/personal-reward-validation.json').write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf8')
print('PASS: exact native visibility/pickup guard, all24 slot cases; exact native no-network dispatch reaches only local creator. Static pickup/removal paths match. No game modifications; multiplayer still pending.')
