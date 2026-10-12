"""Execute the guarded game's category predicate in an isolated memory image."""
from pathlib import Path
import ctypes as c
import hashlib, json, struct
import pefile

root=Path(__file__).resolve().parents[1]
raw=Path(r'D:\Program Files (x86)\Steam\steamapps\common\ELDEN RING NIGHTREIGN\Game\nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
def read(rva,n):
    off=pe.get_offset_from_rva(rva);return raw[off:off+n]
def check(rva,text):
    expected=bytes.fromhex(text);assert read(rva,len(expected))==expected,hex(rva)
def call(rva):
    assert read(rva,1)==b'\xe8';return rva+5+struct.unpack('<i',read(rva+1,4))[0]

# The constructor and modifier descriptor retain the same full category mask.
check(0xe70662,'0F 11 86 24 01 00 00')
check(0xe7066d,'0F 11 8E 34 01 00 00')
check(0x50f239,'0F 10 82 24 01 00 00 0F 11 41 04')
check(0x50f244,'0F 10 8A 34 01 00 00 0F 11 49 14')
assert call(0x511180)==0xea1b00
# Ordinary AttackRate is cached from the SOURCE descriptor, before hit creation.
assert call(0x502ba7)==0x50f2a0
assert call(0x502c6b)==0x511010
assert call(0x50312b)==0x510750
check(0x50f2a6,'0F 10 42 14 0F 11 41 04')
check(0x5110b3,'0F B6 98 60 01 00 00 C0 EB 07')
check(0x5110ce,'0F B6 98 61 01 00 00 80 E3 01')
check(0xea1b34,'0F B6 88 3A 01 00 00')
check(0xea1b66,'0F B6 88 3B 01 00 00')
check(0xea1b80,'0F B6 88 28 03 00 00')
assert call(0xea1b5e)==0xe6d3e0
check(0xe6d3f8,'81 FA 80 00 00 00')
# Native active-magic getter, not the last spell that happened to hit.
check(0x3f8226,'48 8B 88 B8 01 00 00 48 8B 49 60')
assert call(0x3f8231)==0x456870
check(0x456874,'48 8B 01 48 8D 54 24 40 FF 50 18')
check(0x456884,'8B 10')
check(0x3f8255,'48 8B 49 60 48 8B 01 48 8D 54 24 30 FF 50 18')
check(0x4568d0,'C7 02 FF FF FF FF 48 8B C2 C3')
# Native Soulblood Song uses completed damage, not recipient HP loss.
check(0x43e3b5,'66 0F 6E 86 80 02 00 00 0F 5B C0 F3 0F 59 C8 F3 0F 2C F9')

reference=json.loads((root/'data/reference/combat/sword-categories.json').read_text(encoding='utf-8'))
rows={r['ID']:r['fields'] for r in reference['effects']}
assert rows[8320500]['magicSubCategoryChange1']==130
assert {rows[8350000]['magicSubCategoryChange1'],rows[8350000]['magicSubCategoryChange2']}=={111,112}

# Relative calls/data stay at original RVAs. No game process, imports or hooks
# are involved; this predicate uses only its mask helper and GS-cookie check.
k=c.WinDLL('kernel32',use_last_error=True)
k.VirtualAlloc.argtypes=[c.c_void_p,c.c_size_t,c.c_uint32,c.c_uint32];k.VirtualAlloc.restype=c.c_void_p
k.VirtualFree.argtypes=[c.c_void_p,c.c_size_t,c.c_uint32];k.VirtualFree.restype=c.c_int
memory=k.VirtualAlloc(None,pe.OPTIONAL_HEADER.SizeOfImage,0x3000,0x40)
assert memory,c.get_last_error()
tests=[]
try:
    for section in pe.sections:
        if section.SizeOfRawData:
            payload=raw[section.PointerToRawData:section.PointerToRawData+section.SizeOfRawData]
            c.memmove(memory+section.VirtualAddress,payload,len(payload))
    match=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p,c.c_void_p)(memory+0xea1b00)
    add=c.WINFUNCTYPE(None,c.c_void_p,c.c_int)(memory+0xe6d3e0)
    def native_match(categories,filters):
        mask=c.create_string_buffer(32)
        for category in categories:add(c.addressof(mask),category)
        # Independently check native bit layout against Rust's word/bit rule.
        expected=bytearray(32)
        for category in categories:expected[category//8]|=1<<(category%8)
        assert mask.raw==bytes(expected)
        param=c.create_string_buffer(0x360);node=c.create_string_buffer(0x70)
        for off,category in zip([0x13a,0x13b,0x328],filters):param[off]=category
        struct.pack_into('<Q',node,0,c.addressof(param))
        return bool(match(c.addressof(node),c.addressof(mask)))
    for original in [1,6,7,12,30,129,200]:
        added=[original,130]
        for effect,filters in [(8320500,[130]),(8350000,[112,111])]:
            assert not native_match([original],filters)
            expected=effect==8320500
            assert native_match(added,filters)==expected
            tests.append(dict(original=original,effect=effect,before=False,after=expected))
        assert native_match(added,[original])
        assert not native_match(added,[201])
        assert native_match(added,[])
    # Only the absent game PARAM lookup is replaced in this isolated image.
    # The real descriptor, effect conditions, loop, getters, multiplication and
    # cached output stores all execute unchanged from the guarded game EXE.
    c.memmove(memory+0xe6d630,bytes.fromhex('48 8B C1 C3'),4)
    calculate=c.WINFUNCTYPE(None,c.c_void_p,c.c_void_p,c.c_int,c.c_int)(memory+0x502830)
    def attack_rates(effect_ids,categories,apply_type=1,overrides=None):
        manager=c.create_string_buffer(0x80);context=c.create_string_buffer(0x180)
        params=[];nodes=[]
        struct.pack_into('<ii',context,8,40000,1)
        context[0x10]=apply_type
        for category in categories:add(c.addressof(context)+0x14,category)
        for effect in effect_ids:
            fixture=(root/f'data/reference/combat/effect-{effect}.param').read_bytes()
            assert struct.unpack_from('<i',fixture,0x40)[0]==effect
            offset=struct.unpack_from('<Q',fixture,0x48)[0]
            param=c.create_string_buffer(fixture[offset:offset+0x400],0x400)
            for field,value in (overrides or {}).get(effect,{}).items():param[field]=value
            node=c.create_string_buffer(0x80)
            struct.pack_into('<Qi',node,0,c.addressof(param),effect)
            params.append(param);nodes.append(node)
        for i,node in enumerate(nodes[:-1]):struct.pack_into('<Q',node,0x38,c.addressof(nodes[i+1]))
        if nodes:struct.pack_into('<Q',manager,8,c.addressof(nodes[0]))
        calculate(c.addressof(manager),c.addressof(context),-1,-1)
        assert context.raw[0x10]==apply_type,'Spell apply-type must stay unchanged'
        return {hex(off):struct.unpack_from('<f',context,off)[0] for off in range(0x6c,0xb4,4)}
    rate_tests=[]
    damage_offsets=['0x90','0xa4','0xa8','0xac','0xb0']
    # Types 0/1/2 are used by native projectile creation at 8211D3..8211F2.
    # Preserve other native type restrictions; do not forge the apply-type byte.
    for apply_type in [0,1,2]:
        for ids,expected in [([],1.0),([8320500],1.09),([8350000],1.0),([8320500,8350000],1.09)]:
            before=attack_rates(ids,[1],apply_type)
            after=attack_rates(ids,[1,130],apply_type)
            for off in damage_offsets:
                assert abs(before[off]-1.0)<1e-6,(ids,off,before)
                assert abs(after[off]-expected)<1e-6,(ids,off,after)
            rate_tests.append(dict(applyType=apply_type,effects=ids,before=1.0,after=after['0xa4']))
    # Live Pebble samples use type 3, which rejects weapon-only rows before
    # category matching. Type 4 similarly rejects them for prayers.
    for kind in [3,4]:
        baseline=attack_rates([7006600,7006700],[130],kind)
        assert all(abs(baseline[off]-1.0)<1e-6 for off in damage_offsets)
    # Execute the original predicate through a relocated entry. Only its five-
    # argument dispatch is replaced by the same additive retry used by NRHex.
    # All of the game's native restrictions and rate multiplication remain live.
    saved=read(0x511010,15)
    trampoline=k.VirtualAlloc(None,64,0x3000,0x40);assert trampoline
    filter_type=c.WINFUNCTYPE(c.c_ubyte,c.c_void_p,c.c_void_p,c.c_void_p,c.c_int,c.c_int)
    c.memmove(trampoline,saved+b'\x48\xb8'+struct.pack('<Q',memory+0x51101f)+b'\xff\xe0',27)
    original_filter=filter_type(trampoline)
    @filter_type
    def additive_filter(node,descriptor,owner,weapon,extra):
        result=original_filter(node,descriptor,owner,weapon,extra)
        if result:return result
        desc=c.string_at(descriptor,0x3c);param=c.string_at(c.c_void_p.from_address(node).value,0x400)
        mask=desc[4:36];filters=[param[0x13a],param[0x13b],param[0x328]]
        if desc[0] not in [3,4] or 130 not in filters:return result
        if not mask[130//8]&(1<<(130%8)):return result
        copied=c.create_string_buffer(desc,0x3c);copied[0]=1
        return original_filter(node,c.addressof(copied),owner,weapon,extra)
    callback_address=c.cast(additive_filter,c.c_void_p).value
    c.memmove(memory+0x511010,b'\x48\xb8'+struct.pack('<Q',callback_address)+b'\xff\xe0'+b'\x90'*3,15)
    try:
        for kind in [3,4]:
            for ids,expected in [([7006600],1.06),([7006700],1.0),([7006600,7006700],1.06),([8320500,8350000],1.09),([7006600,7006600,7006700,7006700],1.06**2)]:
                values=attack_rates(ids,[7,130],kind)
                assert all(abs(values[off]-expected)<1e-6 for off in damage_offsets),values
                rate_tests.append(dict(applyType=kind,effects=ids,before=1.0,after=values['0xa4']))
        # Original spell eligibility stays valid and must not multiply twice.
        values=attack_rates([7006600],[7,130],3,{7006600:{0x160:0x80}})
        assert abs(values['0xa4']-1.06)<1e-6
        # Do not bypass a native extra weapon restriction (absent fixture weapon).
        values=attack_rates([7006600],[7,130],3,{7006600:{0x3a4:1}})
        assert abs(values['0xa4']-1.0)<1e-6
        # Unrelated spell categories retain their original magic eligibility.
        values=attack_rates([7006700],[7,130],3,{7006700:{0x160:0x80,0x13a:7,0x13b:0}})
        assert abs(values['0xa4']-1.15)<1e-6
        # Existing native skill categories remain; only newly added tags changed.
        for original_skill in [111,112]:
            values=attack_rates([7006700],[original_skill,130],1)
            assert abs(values['0xa4']-1.15)<1e-6
            values=attack_rates([7006700],[original_skill,130],3,{7006700:{0x160:0x80}})
            assert abs(values['0xa4']-1.15)<1e-6
    finally:
        c.memmove(memory+0x511010,saved,len(saved));assert k.VirtualFree(trampoline,0,0x8000)
finally:
    assert k.VirtualFree(memory,0,0x8000)
source=(root/'src/lib.rs').read_text(encoding='utf-8')
assert 'SWORD_CAST_MAGIC.store(active_magic(player)' in source
assert 'ptr::write_unaligned((hit+0x124)' in source
assert 'ptr::write_unaligned((context+0x14)' in source
assert 'callback=sym on_sword_attack_rates,original=sym ATTACK_RATES_ORIG' in source
assert 'manager!=read_ptr(local+0x190)&&read_ptr(manager+0x10)!=local' in source
assert 'bonuses.for_attack(read_i32(context+8),read_i32(context+0xc)).is_none()' in source
assert 'sword::extra_melee_application' in source
assert 'sword_retry_as_melee(original,node,descriptor,owner,weapon,extra)' in source
assert 'let mut melee=ptr::read_unaligned(descriptor as *const [u8;0x3c]);melee[0]=1;' in source
assert source.index('\"call {before}\"',source.index('fn rear_detour'))<source.index('\"call qword ptr [rip+{original}]\"',source.index('fn rear_detour'))
assert 'is_local(source,base)||!has_effect(source,sword::OWNER,base)||spell_percentage(hit).is_none()' in source
assert 'sword::heal_amount(read_i32(hit+0x280),spell_percentage(hit).unwrap_or(0.),ptr::read((hit+0x2c3) as *const u8)&2!=0)' in source
sword_healing=source.split('unsafe extern "system" fn on_cat_hit',1)[1].split('if is_local(player,base)&&has_effect(player,jiahao::OWNER,base)',1)[0]
assert 'recipient_stats' not in sword_healing and 'heal_amount(before' not in sword_healing
assert 'spell_percentage(hit).is_some()&&sword::near(sword_hit_distance(source,hit,base),sword::NEAR_DISTANCE)' in sword_healing
report=dict(version='0.10.3',effectFilterRva='0x511010',attackRateCalculationRva='0x502830',sourceMaskOffset='0x14',cachedRateOffsets=damage_offsets,nativeAttackRateTests=rate_tests,isolatedFixtureLookupStub='E6D630 absent PARAM row; additive melee filter retry at 511010 calls original native predicate; fixture assumes eligible Sword owner',nativeCategoryPredicateRva='0xea1b00',hitMaskOffset='0x124',maskBytes=32,additionalCategories=[130],removedAddedCategories=[111,112],nativeCategoryTests=tests,originalCategoriesRetained=True,originalSkillCategoryChecksPassed=4,unrelatedFiltersExcluded=True,originalSpellApplicationRetained=True,nativeWeaponRestrictionsRetained=True,originalEligibleEffectsNotDoubled=True,activeMagicGetter='ChrIns+1b8 -> module+60 -> virtual+18 -> i32 MagicID',gameplayVerified=False,multiplayerVerified=False)
report.update(healingUsesCompletedDamage=True,healingRequiresRecipientHpLoss=False,nullifiedHitsExcludedFromHealing=True,lockedHpGameplayVerified=False)
(root/'dist/sword-native-validation.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print('PASS: 14 native category cases, 22 native AttackRate cases; added melee works, skill bonuses excluded, four original skill checks preserved; no duplicate multiplication.')
