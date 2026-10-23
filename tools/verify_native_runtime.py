"""Read-only evidence for the current game's native NRHex runtime bindings."""
from pathlib import Path
import hashlib
import json
import struct
import subprocess
import sys
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_64

root = Path(__file__).resolve().parents[1]
exe = Path(r"D:\Program Files (x86)\Steam\steamapps\common\ELDEN RING NIGHTREIGN\Game\nightreign.exe")
raw = exe.read_bytes()
digest = hashlib.sha256(raw).hexdigest()
assert digest == "9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a"
pe = pefile.PE(data=raw, fast_load=True)
base = pe.OPTIONAL_HEADER.ImageBase
dis = Cs(CS_ARCH_X86, CS_MODE_64)

def read(rva, length):
    off = pe.get_offset_from_rva(rva)
    return raw[off:off + length]

def checked(rva, expected):
    b = bytes.fromhex(expected)
    assert read(rva, len(b)) == b, f"Native binding mismatch at {rva:#x}"

def lines(rva, length):
    return [f"{i.address-base:08X} {i.mnemonic} {i.op_str}" for i in dis.disasm(read(rva, length), base+rva)]

def call_target(rva):
    assert read(rva, 1) == b"\xe8"
    return rva + 5 + struct.unpack("<i", read(rva+1, 4))[0]

# Current HKS env dispatcher 3F7B50: 375 follows this actual jump table.
idx = read(0x3fb0d4 + 375 - 223, 1)[0]
target = struct.unpack("<I", read(0x3fadf0 + idx*4, 4))[0]
assert target == 0x3f96c4
assert call_target(0x3f96e3) == 0x48b870
checked(0x3f96c9, "48 8B 06 48 8B 88 B8 01 00 00 48 8B 89 48 01 00 00")
# env(2016) / env(2013): current FP / max HP, via player+1B8, module+0.
checked(0x3fab53, "48 8B 01 48 8B 88 B8 01 00 00 48 8B 01 8B B8 50 01 00 00")
checked(0x3fab6b, "48 8B 01 48 8B 88 B8 01 00 00 48 8B 01 8B B8 44 01 00 00")
checked(0x3fa5bc, "48 8B 01 48 8B 88 B8 01 00 00 48 8B 01 8B B8 40 01 00 00")
checked(0x3e0ad5, "0F 28 F1")  # Update's second parameter is XMM1, saved as dt.
checked(0x3e0d05, "0F 57 C0 F3 0F 5C CE")  # Subtract dt from a seconds countdown.
assert call_target(0x3c8bbd) == 0x3c80a0
assert call_target(0x3c8885) == 0x3db410
checked(0x3db7c7, "40 0F B6 C7")  # Success byte forwarded in EAX.
checked(0x3db7e3, "32 C0")  # Failure byte = 0.
checked(0x508df0, "48 8B 41 08 48 85 C0 74 17 F7 40 68 03 00 0C 80 75 05 39 50 08 74 06 48 8B 40 38 EB E7 B0 01 C3 C3")
# The current character-effect update uses this percentage accumulator.
assert call_target(0x67150a) == 0x48bc60
checked(0x6714f6, "48 8B 8F B8 01 00 00 F3 0F 10 4C 24 28 48 8B 89 48 01 00 00")
checked(0x48bc60, "40 53 48 83 EC 40 F6 41 31 04 48 8B D9 0F 29 74 24 30 0F 28 F1")
checked(0x48bcc5, "F3 0F 59 F0 F3 0F 58 F1")  # .01 * percentage * max + current.
checked(0x48bced, "F3 0F 11 7B 14")  # Native clamped gauge write.
percent_scale_rva = 0x48bcb7 + 8 + struct.unpack("<i", read(0x48bcb7+4, 4))[0]
assert abs(struct.unpack("<f", read(percent_scale_rva, 4))[0] - .01) < 1e-8
assert call_target(0x48bd1d) == 0x48c2d0  # Notify charge readiness when full.

# Crystal shell: modify the exact per-hit bit written by the native proc. This
# site is after its chance test and before its VFX and HP/impact consumers.
assert call_target(0x43d579) == 0x504520
checked(0x50453f, "BA 52 08 00 00")  # stateInfo 2130 owner predicate.
checked(0x43d5c6, "8B B9 DC 02 00 00")  # PlayerCommon attack-nullify chance.
assert call_target(0x43d5de) == 0xf563c0  # Native random range 0..99.
checked(0x43d5e3, "3B C7 73 07 80 8E C3 02 00 00 02")
checked(0x43d5ee, "F6 86 C3 02 00 00 02 74 53")
assert call_target(0x43d4be)==0x439e80  # Generic calculator precedes NPC bypass.
assert call_target(0x43d523)==0x3d4180  # Recipient player predicate.
checked(0x43d534,"F6 80 F9 01 00 00 01 0F 84 09 01 00 00")  # NPC -> 43D64A.
# Reaction remapping explicitly filters category 1001, then lowest priority.
checked(0x507c6f,"41 BE E9 03 00 00")
checked(0x507c7d,"0F B6 88 40 01 00 00 66 44 39 B0 3E 01 00 00")
checked(0x507c8e,"40 3A F9 76 22")
assert call_target(0x507cac)==0xea0cf0
checked(0xea0cfd,"48 8D 88 48 01 00 00")  # Damage-level remap table.
checked(0x43d298, "49 8B F0")  # hit R8 -> RSI.
checked(0x43d2a3, "4C 8B F1")  # damage module RCX -> R14.
checked(0x43d488, "44 0F B6 6D 7F")  # native special-hit flag -> R13B.
checked(0x42c120, "48 8B 41 08 C3")  # module -> recipient player getter.
checked(0x43d582, "45 84 ED 75 67 F6 86 60 01 00 00 20 75 5E 44 39 BE 80 02 00 00 7E 55")
checked(0x43d5ab, "44 39 B8 40 01 00 00 7E 3A")  # recipient HP > 0.
checked(0x43d609, "8B B9 E0 02 00 00")  # native nullify VFX ID.
checked(0x43d643, "81 48 30 00 80 00 00")  # native icon/request bit.
checked(0x43bf4f, "41 F6 86 C3 02 00 00 02 0F 85 86 00 00 00")  # skip HP subtraction.
assert call_target(0x43bfde) == 0x426130
checked(0x43c55d, "41 F6 86 C3 02 00 00 02 75 0D")  # skip next HP subtraction route.
checked(0x48d9a5, "41 F6 80 C3 02 00 00 02 0F 85 8B 04 00 00")  # skip impact processing.
checked(0x3cdd40, "48 8B 89 90 01 00 00 E9 34 71 13 00")  # remove wrapper.
checked(0x504e80, "48 83 EC 28 8B C2 48 8B 51 08 48 85 D2")
checked(0x504ea5, "41 B0 01")
assert call_target(0x504ea8) == 0x50b5b0
checked(0x50b618, "48 89 70 38 48 89 46 40")  # remove node from linked list.

# Cat: native damaging-hit dispatch; saved pre-hit HP also gates Sword lifesteal.
checked(0x43d49f, "8B 80 40 01 00 00 89 45 A7")
checked(0x43e27e, "8B 45 A7 89 44 24 20 44 0F B6 CB 44 0F B6 C7 8B 96 80 02 00 00 49 8B CE")
checked(0x43cf4d, "85 ED 0F 8E 98 02 00 00")  # Native dispatch requires positive hit damage.
assert call_target(0x43e296) == 0x43ced0
checked(0xefabf9, "66 BA 58 02")  # Rendering uses state 600, independent of effect ID.
assert call_target(0xefabfd) == 0x508e20
checked(0xefac02, "84 C0 B9 03 00 00 00 0F 45 F9")
checked(0x508e77, "0F B7 91 56 01 00 00")  # Reads stateInfo from active effect params.

# Rear bonus follows the native calculator, which overwrites old hit+280.
checked(0x439e80, "48 8B C4 48 89 58 20 55 56 57 41 54 41 55 41 56 41 57 48 8D A8 58 FB FF FF 48 81 EC 70 05 00 00")
checked(0x439ef3, "4D 8B F0 4C 8B E2 4C 8B F9")
checked(0x439efc, "41 F6 80 C1 02 00 00 08 0F 85 33 0F 00 00")
checked(0x43ab95, "F3 0F 2C C0 41 89 86 80 02 00 00")
assert call_target(0x43d4be)==0x439e80
checked(0x43d4b8,'4C 8B C6 49 8B D4') # Calculator sees resolved attacker R12.
checked(0x43d2bf,'83 F8 01 4C 8B E3 74 03 4D 8B E5') # R12 is ChrIns or zero.
checked(0x439f65,'41 8B 56 44') # AtkParam table kind at hit+44.
checked(0x439f7c,'45 8B 46 40') # AtkParam row ID at hit+40.
assert call_target(0x439f84)==0xe6d630
checked(0x43e39c,'F3 0F 10 89 2C 01 00 00') # Soulblood HP percent /100 * damage.
checked(0x43e3b5,'66 0F 6E 86 80 02 00 00 0F 5B C0 F3 0F 59 C8 F3 0F 2C F9')
# Position is read directly from the physics transform, without calling a
# coordinate proxy virtual whose optional third parameter was not verified.
checked(0x466620, "48 83 EC 68 0F 10 59 70 0F 10 61 60")
checked(0x46416e, "0F 10 76 70")
checked(0x3fab83, "48 8B 01 48 8B 88 B8 01 00 00 48 8B 41 68 0F B6 B8 C0 02 00 00")
# Retain old damage function evidence for crystal/invisibility consumers.
checked(0x43d260, "48 8B C4 48 89 58 20 55 56 57 41 54 41 55 41 56 41 57 48 8D 68 A9 48 81 EC 00 01 00 00")
checked(0x43d293, "44 88 4C 24 41 49 8B F0 48 8B DA 48 89 54 24 48 4C 8B F1")
checked(0x43d31e, "44 0F B6 6D 7F")  # RBP = entry RSP-57; +7F = entry RSP+28.
checked(0x4366c5, "88 5C 24 20 45 0F B6 CF 4C 8B C7 49 8B D6 48 8B CE")
assert call_target(0x4366d6)==0x43d260
checked(0x4366e4, "83 BF 80 02 00 00 00")  # Caller still owns/reads the hit after native return.
checked(0x3cfc20, "40 53 48 83 EC 20 48 8B 49 60 48 8B DA")
assert call_target(0x3cfc2d) == 0x39ba60
checked(0x39ba66, "48 8B 41 10")
checked(0x39ba7d, "48 8B 88 B8 01 00 00 48 8B 49 68")
assert call_target(0x39ba88) == 0x466820
checked(0x466820, "48 8D 41 60 C3")
checked(0x39bfa0, "40 53 48 83 EC 20 48 8B C1 48 8B DA 48 8B 89 C0 03 00 00 48 85 C9 75 04 48 8B 48 18 48 8B 01 FF 90 00 01 00 00")
checked(0x3db872, "48 8B 4E 60 48 8D 55 C0")
assert call_target(0x3db8ce) == 0x39bfa0  # Native spatial helper consumes its returned xyz.
assert call_target(0x3db82e) == 0x39ba60  # Same helper obtains body orientation.
# env(1106, action): input module, mode=1, action in R8D; AL returned.
idx=read(0x3fb2cc+1106-1001,1)[0]
assert struct.unpack('<I',read(0x3fb258+idx*4,4))[0]==0x3fa6d5
checked(0x3fa6e4,'48 8B 0E 48 8B 91 B8 01 00 00 48 8B 8A 80 00 00 00 44 8B C0 BA 01 00 00 00')
assert call_target(0x3fa6fd)==0x3e91a0
checked(0x3fa702,'0F B6 F8')
checked(0x3e91a0,'48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 30 41 8B F8 8B F2 48 8B D9')
assert call_target(0x3e91b7)==0x42c120
checked(0x42c120,'48 8B 41 08 C3')
checked(0x3e91cd,'48 8B 43 20') # unfiltered action requests
checked(0x3e9234,'48 8B 43 30') # animation-filtered requests
checked(0x3e923d,'48 0F A3 F8') # BT requested action; following SETB AL
checked(0x3f7ef1,'40 38 B9 93 00 00 00') # grounded gate
subprocess.run([sys.executable, str(root/'tools/build_walljump.py'), '--check'],check=True,cwd=root)

# Current HKS env(1011) calls the HP flask reader. Its virtual call +190
# obtains the inventory root, with slot list at root+1D8+1C8.
assert call_target(0x3faa33)==0x66bd60
checked(0x66bd6c,'FF 90 90 01 00 00')
checked(0x66bd77,'48 8D 88 D8 01 00 00 4C 8B C3 B2 01')
checked(0x1f2634,'48 8D A9 C8 01 00 00')
assert call_target(0x753e47)==0x1f6d80
checked(0x1f6d86,'8B 41 70 33 DB FF C0 4C 8B C1 3B D0 73 3D')
checked(0x1f6da0,'8B C2 48 8D 14 80 48 8B 41 40 48 8D 3C 90')
checked(0x754030,'8B 41 04 89 02 48 8B C2 C3')
checked(0x754040,'8B 41 08 C3')
# 1F5CE0 / 753F10 is NOT a setter: both branches read 754040. Do not call it
# as a mutation API. The verified entry+8 write is explicitly recorded below.
checked(0x753f16,'0F 85 24 01 00 00')

evidence = {
    "version": "0.9.0",
    "flaskBindings": {"countRva":"0x66bd60", "countAbi":"RCX=actor, RDX=out i32, AL=success", "capacityProbe":"Read remaining count after native event refills and waits one frame; capacity mutations use AddEstusCharge only", "quantityWrite":"none; direct inventory write removed", "gameplayVerified":False},
    "exeSha256": digest,
    "nativeBindingsVerifiedAgainstCurrentExecutable": True,
    "hksOverridePresent": (root / "dist/mod/action/script/c0000.hks").exists(),
    "effectsRva": "0x3db410",
    "effectObserverAbi": "Win64: target, int effect, source, pointer, pointer, five bytes, int; returns AL success; all 11 arguments forwarded",
    "applySelfRva": "0x3c8b10",
    "ownershipRva": "0x508df0",
    "gaugeRva": "0x48b870",
    "gaugeAddPercentRva": "0x48bc60",
    "gaugeAddPercentAbi": "Win64: gauge module in RCX, f32 percentage in XMM1, return ignored",
    "crystalNullifySiteRva": "0x43d5ee",
    "crystalHitBindings": {"recipient": "[R14+8]", "hit": "RSI", "special": "R13B", "nullifyBit": "hit+0x2c3 & 2", "hpDamage": "hit+0x280", "ignoreFlag": "hit+0x160 & 0x20", "stackAlignment": "RSP mod 16 = 0 at mid-function site"},
    "catHitCompleteSiteRva": "0x43e27e",
    "catHitBindings": {"recipient": "[R14+8]", "hit": "RSI", "hpBefore": "[RBP-0x59]", "hpAfter": "recipient stats+0x140", "nativeHpDamage": "hit+0x280 > 0", "immuneHits": "included; no HP-loss condition"},
    "nativeInvisibilityState": 600,
    "rearDamageRva": "0x439e80",
    "rearDamageAbi": "3 native arguments: recipient module, character source, hit context; callback after native recalculation; preserves GP/XMM returns; native hit+2c1 bit8 skip excluded",
    "bodyForwardRva": "0x3cfc20",
    "position": "ChrIns+1b8 -> modules+68 -> physics+70, four floats; verified against native transform and wall-physics update",
    "wallContactEnv2012": "ChrPhysics+2c0; preserved",
    "wallJumpBytecodePatchVerified": True,
    "actionRequestRva": "0x3e91a0",
    "actionRequestAbi": "Win64: RCX=input module, EDX=mode, R8D=action ID, AL result; owner=module+8; raw mask=module+20; animation-filtered mask=module+30",
    "rawWallJumpScope": "Only a false native result for mode 1/action 6, local Cat owner, alive, grounded byte==0, real wall contact, real raw bit6 and original TAE 5031 takeoff marker",
    "hasStateRva": "0x508e20",
    "removeEffectRva": "0x504e80",
    "removeEffectAbi": "Win64: effect manager in RCX, i32 effect ID in EDX; returns AL; unlinks native node and updates flags",
    "stats": {"playerModules": "0x1b8", "statsModule": "0x0", "hp": "0x140", "maxHp": "0x144", "fp": "0x150", "ultimateModule": "0x148"},
    "evidence": {
        "ultimate375Dispatch": lines(0x3f96c4, 0x2d),
        "fp2016Dispatch": lines(0x3fab38, 0x33),
        "maxHp2013Dispatch": lines(0x3fab6b, 0x18),
        "hp1000Dispatch": lines(0x3fa5bc, 0x18),
        "successfulEffectSignature": lines(0x3db410, 0x8a),
        "successfulEffectReturn": lines(0x3db7c7, 0x26),
        "selfApplicationCallArguments": lines(0x3c8b63, 0x5f),
        "ownershipActiveNodeWalk": lines(0x508df0, 0x21),
        "frameDeltaArgument": lines(0x3e0ac7, 0x14),
        "frameDeltaCountdown": lines(0x3e0cfd, 0x1c),
        "percentageAccumulatorCaller": lines(0x6714cc, 0x43),
        "percentageRestoreAndReadyNotification": lines(0x48bc60, 0xd8),
        "nativeNullifyChanceAndHitBit": lines(0x43d56a, 0xe0),
        "genericCalculatorBeforeNpcBypass": lines(0x43d4b8,0x90),
        "poiseReactionCategory1001": lines(0x507c40,0x9c),
        "poiseReactionCaller": lines(0x6d1be0,0xd7),
        "nullifyOwnerPredicate": lines(0x504520, 0x80),
        "nullifySkipsHpWrite": lines(0x43bf4f, 0x99),
        "nullifySkipsImpact": lines(0x48d980, 0x33),
        "visualRemovalById": lines(0x504e80, 0x34),
        "nativeNodeUnlink": lines(0x50b606, 0x5d),
        "catSavedPreHitHp": lines(0x43d48d, 0x1b),
        "catPostHitHpSite": lines(0x43e274, 0x27),
        "nativeInvisibilityRenderer": lines(0xefabea, 0x22),
        "nativeStateLookup": lines(0x508e20, 0x74),
        "damageArguments": lines(0x43d260, 0xd0),
        "damageCalculatorArguments": lines(0x439ef3, 0x27),
        "nativeDamageOverwrite": lines(0x43ab95, 0x24),
        "physicsWorldTransformPosition": lines(0x466620, 0xc),
        "wallContactDispatch": lines(0x3fab83, 0x1a),
        "damageCallerAndContextLifetime": lines(0x4366c5, 0x47),
        "bodyForwardGetter": lines(0x3cfc20, 0x1b),
        "bodyQuaternionLookup": lines(0x39ba60, 0x40),
        "coordinatePositionGetter": lines(0x39bfa0, 0x2e),
        "nativePositionAndForwardCaller": lines(0x3db820, 0xd6),
        "actionRequestDispatch": lines(0x3fa6d5,0x30),
        "nativeActionRequestMasks": lines(0x3e91a0,0xaf),
    },
    "inGameAbilityEffectsVerified": False,
}
assert evidence["hksOverridePresent"], "Exact validated original-bytecode wall jump patch required"
(root / "dist/reports/native-validation.json").write_text(json.dumps(evidence, ensure_ascii=False, indent=2), encoding="utf-8")
print("PASS: current EXE runtime bindings, per-hit rear bonus ABI, spatial getters and surgical original-bytecode HKS patch.")

