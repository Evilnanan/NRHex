use super::*;
use crate::win::*;
use sha2::{Digest, Sha256};
use std::cell::Cell;

thread_local! {
    static REWARDS: Cell<u32> = const { Cell::new(0) };
    static CUES: Cell<u32> = const { Cell::new(0) };
    static SPECIAL: Cell<u8> = const { Cell::new(0) };
    static AMOUNT: Cell<i32> = const { Cell::new(0) };
    static PACKET_AVAILABLE: Cell<bool> = const { Cell::new(true) };
    static OWNED: Cell<bool> = const { Cell::new(true) };
}

unsafe fn pointer(address: usize, value: usize) {
    ptr::write_unaligned(address as *mut usize, value);
}
unsafe fn integer(address: usize, value: i32) {
    ptr::write_unaligned(address as *mut i32, value);
}
unsafe fn jump(base: usize, address: usize, target: usize) {
    let mut bytes = vec![0x48, 0xb8];
    bytes.extend_from_slice(&target.to_le_bytes());
    bytes.extend_from_slice(&[0xff, 0xe0]);
    ptr::copy_nonoverlapping(bytes.as_ptr(), (base + address) as *mut u8, bytes.len());
}
unsafe extern "system" fn npc_row(actor: usize) -> usize { read_ptr(actor + 0x5b8) }
unsafe extern "system" fn think(_: usize) -> i32 { 48010000 }
unsafe extern "system" fn multiplier(_: usize) -> f32 { 1. }
unsafe extern "system" fn no_op() {}
unsafe extern "system" fn false_flag() -> u8 { 0 }
unsafe extern "system" fn effect(_: usize, id: i32) -> u8 {
    u8::from(id == spirit::OWNER && OWNED.with(Cell::get))
}
unsafe extern "system" fn award(_: usize, amount: i32, flag: u8) -> i32 {
    assert_eq!(flag, 1);
    AMOUNT.with(|value| value.set(amount));
    amount
}
unsafe extern "system" fn reward_effect(_: usize, _: usize, special: u8) {
    REWARDS.with(|value| value.set(value.get() + 1));
    SPECIAL.with(|value| value.set(special));
}
unsafe extern "system" fn cue(_: usize, id: i32, flag: u8) -> u8 {
    assert_eq!((id, flag), (spirit::CAPTURE_CUE, 0));
    CUES.with(|value| value.set(value.get() + 1));
    1
}
unsafe extern "system" fn dequeue(_: usize, packet: *mut [u8; 20]) -> u8 {
    if !PACKET_AVAILABLE.with(Cell::get) { return 0; }
    let mut bytes = [0u8; 20];
    bytes[..4].copy_from_slice(&0x3c000000u32.to_le_bytes());
    bytes[12..16].copy_from_slice(&0x10000001u32.to_le_bytes());
    ptr::write_unaligned(packet, bytes);
    1
}

#[test]
#[ignore = "requires NRHEX_NATIVE_EXE; run alone"]
fn native_kill_settlement_preserves_rewards_and_captures_without_last_attacker() { unsafe {
    let raw = std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();
    assert_eq!(format!("{:x}", Sha256::digest(&raw)), crate::EXE_HASH);
    let headers = crate::pe::Headers::parse(&raw, 0).unwrap();
    let memory = VirtualAlloc(ptr::null_mut(), headers.image_size as usize,
        MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
    assert!(!memory.is_null());
    let base = memory as usize;
    for section in &headers.sections {
        ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),
            (base + section.virtual_address as usize) as *mut u8, section.raw_size as usize);
    }
    for (address, target) in [
        (0x4d1ea0, npc_row as *const () as usize),
        (0x506e30, multiplier as *const () as usize),
        (0x3de140, no_op as *const () as usize),
        (0x3ced90, false_flag as *const () as usize),
        (0x4d1ae0, false_flag as *const () as usize),
        (0x4b3b50, no_op as *const () as usize),
        (0x4b39a0, no_op as *const () as usize),
        (0x3c92f0, no_op as *const () as usize),
        (0x50a150, reward_effect as *const () as usize),
        (0x426e60, false_flag as *const () as usize),
        (0xb2d7b0, no_op as *const () as usize),
        (crate::HAS_EFFECT_RVA, effect as *const () as usize),
        (crate::APPLY_SELF_RVA, cue as *const () as usize),
    ] { jump(base, address, target); }
    ptr::write((base + 0x26909a0) as *mut u8, 0xc3);
    let hook = crate::hook::install((base + KILL_RVA) as *mut u8,
        kill_detour as *const u8, false, 64).unwrap();
    let old_original = KILL_ORIGINAL.swap(hook.trampoline, Ordering::AcqRel);
    let old_base = crate::BASE.swap(base, Ordering::AcqRel);
    let old_active = ACTIVE.swap(true, Ordering::AcqRel);
    let old_receive = RECEIVE_ORIGINAL.swap(dequeue as *mut u8, Ordering::AcqRel);
    let state = SPIRITS.get_or_init(Default::default);
    let old_state = std::mem::take(&mut *state.lock().unwrap());
    let mut world = vec![0usize; 0x17500 / 8];
    let mut player = vec![0usize; 0x740 / 8];
    let mut actor = vec![0usize; 0x600 / 8];
    let player = player.as_mut_ptr() as usize;
    let actor = actor.as_mut_ptr() as usize;
    let mut attributes = vec![0usize; 0x200 / 8];
    let attributes = attributes.as_mut_ptr() as usize;
    let mut player_attributes = vec![0usize; 0x200 / 8];
    let player_attributes = player_attributes.as_mut_ptr() as usize;
    let modules = [attributes];
    let player_modules = [player_attributes];
    let mut actor_vtable = vec![0usize; 0x1b0 / 8];
    actor_vtable[0x1a0 / 8] = base + 0x4d1ea0;
    actor_vtable[0x1a8 / 8] = think as *const () as usize;
    let mut player_vtable = vec![0usize; 0x5d8 / 8];
    player_vtable[0x5d0 / 8] = award as *const () as usize;
    let mut row = vec![0u8; 0x270];
    row[0x26a] = 4;
    let row_reference = [row.as_ptr() as usize, 48010010];
    let mut game_data = [0u8; 0xd0];
    game_data[0xc7] = 6;
    let manager = [0usize, game_data.as_ptr() as usize, 0];
    pointer(base + crate::GAME_DATA_RVA, manager.as_ptr() as usize);
    pointer(base + 0x3c17248, manager.as_ptr() as usize);
    world[0x174e8 / 8] = player;
    pointer(base + WORLD_RVA, world.as_ptr() as usize);
    pointer(player, player_vtable.as_ptr() as usize);
    pointer(player + 0x1b8, player_modules.as_ptr() as usize);
    pointer(player + 0x190, player);
    integer(player_attributes + 0x140, 100);
    pointer(actor, actor_vtable.as_ptr() as usize);
    pointer(actor + 8, 123);
    pointer(actor + 0x198, usize::MAX);
    pointer(actor + 0x1a0, usize::MAX);
    pointer(actor + 0x1b8, modules.as_ptr() as usize);
    pointer(actor + 0x190, actor);
    pointer(actor + 0x5b8, row_reference.as_ptr() as usize);
    integer(attributes + 0x144, 10000);
    let context = [0usize, 0x3c000000];
    let reference = [0usize, context.as_ptr() as usize];
    pointer(attributes + 0x78, reference.as_ptr() as usize);
    integer(attributes + 0xc8, 0x10000001);
    assert_eq!(reward_key(attributes), crate::spirit_kills::Key::new(0x3c000000, 0x10000001));
    let native: unsafe extern "system" fn(usize) -> f32 = std::mem::transmute(base + 0x3de710);

    observe_soul(actor, base, false);
    assert_eq!(state.lock().unwrap().request_status().0, None);
    ptr::write((actor + 0x1f4) as *mut u8, 4);
    assert_eq!(native(actor), 0.);
    assert_eq!(REWARDS.with(Cell::get), 0);
    assert_eq!(state.lock().unwrap().request_status().0, None);
    ptr::write((actor + 0x1f4) as *mut u8, 0);
    assert_eq!(native(actor), 0.);
    assert_eq!(state.lock().unwrap().request_status().0, Some(48010010));
    assert_eq!(REWARDS.with(Cell::get), 1);
    assert_eq!(SPECIAL.with(Cell::get), 1);
    assert_eq!(CUES.with(Cell::get), 1);
    state.lock().unwrap().created(555);
    assert_eq!(native(actor), 0.);
    assert_eq!(REWARDS.with(Cell::get), 1);
    assert_eq!(state.lock().unwrap().request_status().0, None);
    kill_detour(player, 0., actor, 1);
    assert_eq!(state.lock().unwrap().request_status().0, None);

    state.lock().unwrap().synchronize(player, false);
    ptr::write((actor + 0x1f4) as *mut u8, 0);
    integer(row.as_mut_ptr() as usize + 0x2c, 1234);
    assert_eq!(native(actor), 1234.);
    assert_eq!(AMOUNT.with(Cell::get), 1234);
    assert_eq!(state.lock().unwrap().request_status().0, Some(48010010));
    for receipt_first in [false, true] {
        state.lock().unwrap().synchronize(player, false);
        let mut packet = [0u8; 20];
        PACKET_AVAILABLE.with(|value| value.set(true));
        if !receipt_first { observe_soul(actor, base, false); }
        assert_eq!(receive_detour(0, &mut packet), 1);
        assert_eq!(kill_detour(player, 0., 0, 1), 0);
        if receipt_first { observe_soul(actor, base, false); }
        assert_eq!(state.lock().unwrap().request_status().0, Some(48010010));
        state.lock().unwrap().created(556);
        receive_detour(0, &mut packet);
        kill_detour(player, 0., 0, 1);
        assert_eq!(state.lock().unwrap().request_status().0, None);
        PACKET_AVAILABLE.with(|value| value.set(false));
        assert_eq!(receive_detour(0, &mut packet), 0);
        assert_eq!(RECEIPT.with(|receipt| receipt.get()), None);
    }
    state.lock().unwrap().synchronize(player, false);
    kill_detour(player, 0., 0, 0);
    assert_eq!(state.lock().unwrap().request_status().0, None);
    kill_detour(actor, 0., actor, 1);
    assert_eq!(state.lock().unwrap().request_status().0, None);
    OWNED.with(|value| value.set(false));
    kill_detour(player, 0., actor, 1);
    assert_eq!(state.lock().unwrap().request_status().0, None);
    OWNED.with(|value| value.set(true));
    ACTIVE.store(false, Ordering::Release);
    ptr::write((actor + 0x1f4) as *mut u8, 0);
    assert_eq!(native(actor), 1234.);
    assert_eq!(state.lock().unwrap().request_status().0, None);
    *state.lock().unwrap() = old_state;
    KILL_ORIGINAL.store(old_original, Ordering::Release);
    RECEIVE_ORIGINAL.store(old_receive, Ordering::Release);
    crate::BASE.store(old_base, Ordering::Release);
    ACTIVE.store(old_active, Ordering::Release);
    VirtualFree(hook.trampoline as *mut _, 0, MEM_RELEASE);
    if !hook.relay.is_null() { VirtualFree(hook.relay as *mut _, 0, MEM_RELEASE); }
    assert_ne!(VirtualFree(memory, 0, MEM_RELEASE), 0);
} }
