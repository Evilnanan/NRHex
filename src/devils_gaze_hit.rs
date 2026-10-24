//! Restore the character attack-immunity gate at environmental bullet contact.
//! 33C120's Chr-owner branch calls 436370 (target vtable+200); the asset-owner
//! branch skips that gate and submits both a damage hit and bullet effects.
use crate::{devils_gaze, ptr, read_i32, read_ptr, ACTIVE, BASE};
use std::sync::atomic::{AtomicPtr, Ordering};

pub const RVA: usize = 0x33c120;
pub const PATTERN: &str = "48 89 5C 24 20 55 56 57 41 54 41 55 41 56 41 57 48 8D AC 24 60 FD FF FF 48 81 EC A0 03 00 00 48 8B 05 ? ? ? ? 48 33 C4 48 89 85 90 02 00 00 4D 8B F8 48 8B F2 48 8B D9 45 32 F6 F6 82 50 05 00 00 08";
pub const LOOKUP_RVA: usize = 0xe6d630;
pub const LOOKUP_PATTERN: &str = "48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 57 48 83 EC 30 41 8B F0 8B EA 48 8B F9 89 51 08 33 DB";
pub static ORIGINAL: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
// 52D039 consumes AL; the decompiler incorrectly labels 33C120 as void.
type Contact = unsafe extern "system" fn(usize, usize, usize) -> u8;
type Lookup = unsafe extern "system" fn(*mut AttackRef, i32, i32);
type Immune = unsafe extern "system" fn(usize, *const AttackRef) -> u8;

// The borrowed native AtkParam reference used by 436370 / E6D630. No ownership
// or refcount is transferred; nothing survives this synchronous contact.
#[repr(C)]
struct AttackRef {
    id: i32,
    cache: u32,
    kind: i32,
    padding: u32,
    row: usize,
}

// PlayerIns includes hostile human NPCs. Use the native expedition roster,
// not the actor class or a local-player-only predicate, for full damage.
type PartyLookup = unsafe extern "system" fn(usize, i32) -> usize;
unsafe fn party_member(target: usize, world: usize, get: PartyLookup) -> bool {
    if target == 0 || world == 0 { return false; }
    if read_ptr(world + 0x174e8) == target { return true; }
    (1..=3).any(|slot| get(world, slot) == target)
}
unsafe fn recipient_multiplier(target: usize, hit: usize, is_member: impl FnOnce(usize) -> bool) -> f32 {
    if attack_depth(target, hit).is_none() { return 1.; }
    if is_member(target) { 4. } else { 1. }
}
unsafe fn attack_depth(target: usize, hit: usize) -> Option<usize> {
    if target == 0 || hit == 0 || read_i32(hit+0x44) != 0 { return None; }
    let id = read_i32(hit+0x40);
    (devils_gaze::OWNER..=devils_gaze::OWNER+5).contains(&id)
        .then(|| (id-devils_gaze::OWNER) as usize)
}
pub unsafe fn damage_multiplier(target: usize, hit: usize, base: usize) -> f32 {
    let Some(depth) = attack_depth(target, hit) else { return 1.; };
    let recipient = recipient_multiplier(target, hit, |actor| {
        if base == 0 { return false; }
        // 5197E0 queries WorldChrMan's player registry by PlayerGameData slot.
        // Its executable signature is already guarded by NEW_5197E0_PATTERN.
        party_member(actor, read_ptr(base + crate::WORLD_RVA), std::mem::transmute(base + 0x5197e0))
    });
    recipient * crate::devils_gaze_scaling::multiplier(depth, base)
}

unsafe fn blocked(active: bool, hit: usize, target: usize, bullet: usize, lookup: Lookup) -> bool {
    if !active || hit == 0 || target == 0 || bullet == 0 {
        return false;
    }
    // Hit +40 is attack ID, +44 is table kind (0 NPC / 1 PC).
    let id = read_i32(hit + 0x40);
    if read_i32(hit + 0x44) != 0 || !(devils_gaze::OWNER..=devils_gaze::OWNER + 5).contains(&id) {
        return false;
    }
    if (ptr::read_unaligned((bullet + 0xb0) as *const u64) >> 28) & 15 != 6 {
        return false;
    }
    let vtable = read_ptr(target);
    if vtable == 0 { return false; }
    let predicate = read_ptr(vtable + 0x200);
    if predicate == 0 { return false; }
    let mut attack = AttackRef { id: -1, cache: 0, kind: 0, padding: 0, row: 0 };
    lookup(&mut attack, 0, id);
    if attack.row == 0 { return false; }
    let immune: Immune = std::mem::transmute(predicate);
    // Same virtual query as the native Chr-owner contact filter, including
    // attack-specific bypass flags and the target's current TAE immunity.
    // Do not infer immunity from HP loss, animation IDs, elapsed time or buffs.
    immune(target, &attack) != 0
}

unsafe fn contact(active: bool, original: Contact, lookup: Lookup, hit: usize, target: usize, bullet: usize) -> u8 {
    if blocked(active, hit, target, bullet, lookup) { return 0; }
    original(hit, target, bullet)
}

pub unsafe extern "system" fn detour(hit: usize, target: usize, bullet: usize) -> u8 {
    contact(ACTIVE.load(Ordering::Acquire),
        std::mem::transmute(ORIGINAL.load(Ordering::Acquire)),
        std::mem::transmute(BASE.load(Ordering::Acquire) + LOOKUP_RVA), hit, target, bullet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    thread_local! { static EVENTS: RefCell<Vec<(usize, usize, usize)>> = const { RefCell::new(Vec::new()) }; }
    unsafe extern "system" fn original(h: usize, t: usize, b: usize) -> u8 { EVENTS.with(|e| e.borrow_mut().push((h,t,b))); 0x5a }
    unsafe extern "system" fn lookup(out: *mut AttackRef, kind: i32, id: i32) {
        assert_eq!(kind, 0);
        assert!((devils_gaze::OWNER..=devils_gaze::OWNER+5).contains(&id));
        (*out).id=id; (*out).row=0x1234;
    }
    unsafe extern "system" fn missing(_: *mut AttackRef, _: i32, _: i32) {}
    unsafe extern "system" fn immune(target: usize, attack: *const AttackRef) -> u8 {
        assert_eq!((*attack).row, 0x1234);
        assert_eq!((*attack).kind, 0);
        ptr::read((target+8) as *const u8)
    }
    unsafe fn fixture(active: bool, id: i32, kind: i32, owner_kind: u64, immunity: u8, resolve: Lookup) -> bool {
        EVENTS.with(|e| e.borrow_mut().clear());
        let mut hit=[0xa5u8;0x2d0];
        hit[0x40..0x44].copy_from_slice(&id.to_le_bytes());
        hit[0x44..0x48].copy_from_slice(&kind.to_le_bytes());
        let mut bullet=[0u64;0x18]; bullet[0xb0/8]=owner_kind<<28;
        let mut vt=[0usize;0x41];vt[0x200/8]=immune as *const () as usize;
        let target=[vt.as_ptr() as usize, immunity as usize];
        let h=hit.as_mut_ptr() as usize;let t=target.as_ptr() as usize;let b=bullet.as_ptr() as usize;
        let before=hit;
        let result=contact(active,original,resolve,h,t,b);
        assert_eq!(hit,before,"no post-hit damage/effect clearing or shared payload writes");
        EVENTS.with(|e| { let e=e.borrow();if !e.is_empty(){assert_eq!(*e,vec![(h,t,b)]);assert_eq!(result,0x5a);}else{assert_eq!(result,0);} e.is_empty() })
    }
    #[test] fn all_six_depths_obey_native_immunity_before_any_contact_side_effects() { unsafe {
        for depth in 0..=5 { for immune in [0,1] {
            assert_eq!(fixture(true,devils_gaze::OWNER+depth,0,6,immune,lookup),immune!=0);
        }}
    }}
    #[test] fn unrelated_attacks_tables_and_sources_keep_original_contact() { unsafe {
        for (active,id,kind,source) in [(false,95002200,0,6),(true,95002199,0,6),(true,95002206,0,6),
            (true,7560221,0,6),(true,95002200,1,6),(true,95002200,0,1),(true,95002200,0,2)] {
            assert!(!fixture(active,id,kind,source,1,lookup));
        }
    }}
    #[test] fn missing_reference_and_null_inputs_preserve_original_path() { unsafe {
        assert!(!fixture(true,95002200,0,6,1,missing));
        for (h,t,b) in [(0,1,2),(1,0,2),(1,2,0)] { assert!(!blocked(true,h,t,b,lookup)); }
    }}
    unsafe extern "system" fn party_lookup(world: usize, slot: i32) -> usize {
        assert!((1..=3).contains(&slot));
        read_ptr(world + slot as usize * 8)
    }
    #[test] fn full_damage_requires_current_party_membership_at_every_depth() { unsafe {
        let mut world=vec![0usize;0x17500/8];
        world[0x174e8/8]=11;
        world[1]=11;world[2]=22;world[3]=33;
        let w=world.as_ptr() as usize;
        // 44 represents a hostile PlayerIns; 55 a friendly summon. Neither is
        // a party player, regardless of player-like model/class/faction.
        for target in [11,22,33,44,55,0] {
            for kind in [0i32,1] { for id in 95002199i32..=95002206 {
                let mut hit=[0u8;0x48];hit[0x40..0x44].copy_from_slice(&id.to_le_bytes());hit[0x44..].copy_from_slice(&kind.to_le_bytes());
                let before=hit;
                let expected=if [11,22,33].contains(&target)&&kind==0&&(95002200..=95002205).contains(&id){4.}else{1.};
                assert_eq!(recipient_multiplier(target,hit.as_ptr() as usize,|p|party_member(p,w,party_lookup)),expected);
                if target==0||kind!=0||!(95002200..=95002205).contains(&id){
                    // A deliberately unusable base detects accidental queries
                    // for unrelated attacks before the private-ID/table gate.
                    assert_eq!(damage_multiplier(target,hit.as_ptr() as usize,1),1.);
                }
                assert_eq!(hit,before);
            }}
        }
        assert_eq!(damage_multiplier(0,0,0),1.);
    }}
    #[test] fn party_lookup_handles_local_remote_departure_and_missing_world() { unsafe {
        let mut world=vec![0usize;0x17500/8];world[0x174e8/8]=33;
        world[1]=11;world[2]=22;world[3]=33;
        assert!(party_member(11,world.as_ptr() as usize,party_lookup)); // host from a client's perspective
        assert!(party_member(33,world.as_ptr() as usize,party_lookup));
        world[2]=0;
        assert!(!party_member(22,world.as_ptr() as usize,party_lookup));
        world[2]=66;
        assert!(party_member(66,world.as_ptr() as usize,party_lookup));
        assert!(!party_member(0,world.as_ptr() as usize,party_lookup));
        assert!(!party_member(11,0,party_lookup));
    }}
}
