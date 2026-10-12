//! Enhanced Necromancy: team soul capture, native creation and lifetime observation.
//! State and descriptors live in spirit.rs; geometry lives in spirit_placement.rs.
use crate::native_actor::{buddy, incapacitated, npc_id, resolve, stats};
use crate::{
    balloon, has_effect, has_state, local_hero, nlog_info, read_ptr, runtime, spatial_ready,
    spirit, spirit_placement, ACTIVE, APPLY_SELF_RVA, MAP_POSITION_RVA, SPIRITS, SPIRIT_DEAD_RVA,
    SPIRIT_MODEL_ORIG, SPIRIT_SLOT_RVA, WORLD_RVA,
};
use std::ptr;
use std::sync::atomic::Ordering;

type ModelFn = unsafe extern "system" fn(*mut i32, i32) -> *mut i32;
pub(crate) unsafe extern "system" fn spirit_model_detour(out: *mut i32, npc: i32) -> *mut i32 {
    let original: ModelFn = std::mem::transmute(SPIRIT_MODEL_ORIG.load(Ordering::Acquire));
    model_call(original, out, npc, ACTIVE.load(Ordering::Acquire))
}

unsafe fn model_call(original: ModelFn, out: *mut i32, npc: i32, active: bool) -> *mut i32 {
    if active {
        if let Some(model) = spirit::private_model(npc) {
            ptr::write(out, model);
            return out;
        }
    }
    original(out, npc)
}

unsafe fn team_kill(actor: usize, local: usize, base: usize) -> bool {
    let killer = ptr::read_unaligned((actor + 0x198) as *const u64);
    let dot = if has_state(actor, 628, base) {
        ptr::read_unaligned((actor + 0x1a0) as *const u64)
    } else {
        u64::MAX
    };
    let world = read_ptr(base + WORLD_RVA);
    let get: unsafe extern "system" fn(usize, i32) -> usize = std::mem::transmute(base + 0x5197e0);
    for player in std::iter::once(local).chain((0..=3).map(|index| get(world, index))) {
        if player == 0 {
            continue;
        }
        let matches = |chr: usize| {
            chr != 0
                && spirit::matches_killer(killer, dot, ptr::read_unaligned((chr + 8) as *const u64))
        };
        if matches(player) {
            return true;
        }
        for i in 0..3 {
            if matches(buddy(player, base, 0x160, i)) {
                return true;
            }
        }
        for i in 0..6 {
            if matches(buddy(player, base, 0x168, i)) {
                return true;
            }
        }
    }
    false
}

pub(crate) unsafe fn record_soul(actor: usize, base: usize) {
    let Some(npc) = npc_id(actor, base) else {
        return;
    };
    if spirit::is_private_npc(npc) {
        return;
    }
    let world = read_ptr(base + WORLD_RVA);
    if world == 0 {
        return;
    }
    let player = read_ptr(world + 0x174e8);
    if player == 0 || local_hero(base) != Some(6) {
        return;
    }
    let st = stats(player);
    let incapacitated = st != 0 && incapacitated(player, st);
    let retained = incapacitated
        && SPIRITS
            .get()
            .is_some_and(|s| s.lock().ok().is_some_and(|s| s.owned_by(player)));
    if !has_effect(player, spirit::OWNER, base) && !retained {
        return;
    }
    // Threat tags are the native world-boss classifications. The event-derived
    // catalogue also covers bosses whose health bar is linked/shared by script.
    if !spirit::boss_npc(npc)
        && ((95300000..95301000).contains(&npc)
            || !(7740..=7769)
                .chain(7930..=7969)
                .any(|id| has_effect(actor, id, base)))
    {
        return;
    }
    if !team_kill(actor, player, base) {
        nlog_info!("Soul capture rejected: npc={} reason=team-attribution killer={} local={}", npc,
            ptr::read_unaligned((actor + 0x198) as *const u64), ptr::read_unaligned((player + 8) as *const u64));
        return;
    }
    // npc_id has established an NPC, not a PC. Follow native 391DC0:
    // +1A8 returns NpcThinkParam; the PC-only CharaInitParam slot stays -1.
    // +190 is PlayerGameData, not AI data, and must not be used for this NPC.
    let Some(soul) = npc_soul(actor, npc) else {
        nlog_info!("Soul capture rejected: npc={npc} reason=missing-ai");
        return;
    };
    if soul.think < 0 || spirit::private_npc(npc).is_none() {
        nlog_info!("Soul capture rejected: npc={npc} think={} reason=unsupported-descriptor", soul.think);
        return;
    }
    if let Some(state) = SPIRITS.get() {
        if let Ok(mut s) = state.lock() {
            s.synchronize(player, true);
            let st = stats(actor);
            let max_hp = if st == 0 { 0 } else { crate::read_i32(st + 0x144) };
            let corpse = ptr::read_unaligned((actor + 8) as *const u64);
            let stored = s.defeated(soul, corpse, max_hp);
            nlog_info!("Soul capture: npc={} think={} corpse={} max_hp={} stored={}",
                soul.npc, soul.think, corpse, max_hp, stored);
        }
    }
}

unsafe fn npc_soul(actor: usize, npc: i32) -> Option<spirit::Soul> {
    let getter = read_ptr(read_ptr(actor) + 0x1a8);
    if getter == 0 {
        return None;
    }
    let get: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(getter);
    Some(spirit::Soul {
        npc,
        think: get(actor),
        chara_init: -1,
    })
}

unsafe fn soul_finished(player: usize, base: usize, handle: u64) -> bool {
    let modules = read_ptr(player + 0x1b8);
    let wrapper = if modules == 0 { 0 } else { read_ptr(modules + 0x168) };
    let implementation = if wrapper == 0 { 0 } else { read_ptr(wrapper + 0x10) };
    if implementation != 0 {
        let slots = std::array::from_fn(|i| {
            let entry = implementation + 0x50 + i * 0x50;
            (ptr::read_unaligned(entry as *const u64), ptr::read((entry + 8) as *const u8))
        });
        if spirit::slot_finished(handle, &slots) { return true; }
    }
    let actor = resolve(base, handle);
    // A temporarily missing actor is not confirmed dead; retain its handle.
    if actor == 0 || !spatial_ready(actor) {
        return false;
    }
    let Some(npc) = npc_id(actor, base) else {
        return false;
    };
    if !spirit::is_private_npc(npc) {
        return true;
    }
    let dead: unsafe extern "system" fn(usize) -> u8 = std::mem::transmute(base + SPIRIT_DEAD_RVA);
    // Native HP drain/death and slot retirement own the entire lifetime.
    dead(actor) != 0
}

unsafe fn summon(player: usize, soul: spirit::Soul, base: usize) -> Option<u64> {
    if !spatial_ready(player) {
        return None;
    }
    let npc = spirit::private_npc(soul.npc)?;
    let modules = read_ptr(player + 0x1b8);
    let wrapper = read_ptr(modules + 0x168);
    if wrapper == 0 {
        return None;
    }
    let implementation = read_ptr(wrapper + 0x10);
    if implementation == 0 {
        return None;
    }
    // Use native 3922C0, exactly as 3912D0 does before 391DC0. Only FF is
    // free: 2 waits for corpse retirement, 3 awaits collision/world removal
    // by 391780. Reusing either early makes 3918D0 destroy a still-registered
    // actor. Death confirmation alone does not release its native slot.
    let select: unsafe extern "system" fn(usize) -> i32 =
        std::mem::transmute(base + SPIRIT_SLOT_RVA);
    let slot = select(implementation);
    if !(0..6).contains(&slot) {
        return None;
    }
    let slot = slot as usize;
    // Match 391DC0: the getter normalizes the owner to index 1 offline. The
    // module's cached index alone does not guarantee the native owner namespace.
    let get_owner: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + 0x66c7d0);
    let owner = get_owner(player);
    if !(0..256).contains(&owner) {
        return None;
    }
    use crate::spirit_placement::Vector;
    let Some((position, yaw)) = spirit_placement::locate(player, npc, base) else {
        return None;
    };
    let world_pos = Vector(position);
    let rotation = Vector([0., yaw, 0., 0.]);
    let get_map: unsafe extern "system" fn(usize, *mut i32) -> *const i32 =
        std::mem::transmute(base + 0x3d0040);
    let mut map = -1;
    let map_ptr = get_map(player, &mut map);
    if map_ptr.is_null() {
        return None;
    }
    map = ptr::read_unaligned(map_ptr);
    if map == -1 {
        return None;
    }
    let mut local_vector = Vector([0.; 4]);
    let convert: unsafe extern "system" fn(*mut f32, *const f32, *mut i32) -> u8 =
        std::mem::transmute(base + MAP_POSITION_RVA);
    if convert(local_vector.0.as_mut_ptr(), world_pos.0.as_ptr(), &mut map) == 0 {
        return None;
    }
    let local = [local_vector.0[0], local_vector.0[1], local_vector.0[2]];
    let parameters = [npc, soul.think, soul.chara_init];
    let mut handle = u64::MAX;
    let create: unsafe extern "system" fn(usize, *mut u64, u32, i32, *const i32, u8) -> *mut u64 =
        std::mem::transmute(base + 0x3918d0);
    create(
        implementation,
        &mut handle,
        owner as u32,
        slot as i32,
        parameters.as_ptr(),
        1,
    );
    if handle == 0 || handle == u64::MAX {
        return None;
    }
    let actor = resolve(base, handle);
    if actor == 0 {
        return None;
    }
    let entry = implementation + 0x50 + slot * 0x50;
    ptr::write_unaligned(entry as *mut u64, handle);
    ptr::write_unaligned((entry + 0x20) as *mut [f32; 4], world_pos.0);
    ptr::write_unaligned((entry + 0x30) as *mut f32, yaw);
    ptr::write_unaligned((entry + 0x34) as *mut i32, map);
    ptr::write_unaligned((entry + 0x38) as *mut [f32; 3], local);
    // Retain native state transitions/flags. 392810 supplies allegiance, white
    // phantom visuals and all five native passive effects once backread is ready.
    ptr::write_unaligned((entry + 0x44) as *mut u8, 1);
    let activate: unsafe extern "system" fn(usize) = std::mem::transmute(base + 0x3d78d0);
    activate(actor);
    let mode: unsafe extern "system" fn(usize, i32) = std::mem::transmute(base + 0x3d7490);
    mode(actor, 5);
    let teleport: unsafe extern "system" fn(usize, *const f32, *const f32) =
        std::mem::transmute(base + 0x3d7130);
    teleport(actor, world_pos.0.as_ptr(), rotation.0.as_ptr());
    let world = read_ptr(base + WORLD_RVA);
    let collision = read_ptr(world + 0x17530);
    if collision != 0 {
        let set: unsafe extern "system" fn(usize, *const u64, i32) =
            std::mem::transmute(base + 0x4eafe0);
        set(collision, &handle, 1);
        let mask: unsafe extern "system" fn(usize, *const u64, i32) =
            std::mem::transmute(base + 0x4ea470);
        mask(collision, &handle, 0xfff);
    }
    ptr::write_unaligned((entry + 8) as *mut u8, 0);
    let packet = spirit::Packet::new(spirit::Soul { npc, ..soul }, handle, map, local, yaw);
    let network = read_ptr(base + 0x3c21ec8);
    if network != 0 {
        let broadcast: unsafe extern "system" fn(usize, u32, *const spirit::Packet, u32) =
            std::mem::transmute(base + 0xdf24d0);
        broadcast(network, 0x1b, &packet, 48);
    }
    Some(handle)
}
pub(crate) unsafe fn notify(player: usize, effect: i32, base: usize) {
    if effect == balloon::DOWN {
        if let Some(state) = SPIRITS.get() {
            if let Ok(mut s) = state.lock() {
                s.cancel_request();
            }
        }
        return;
    }
    if effect != runtime::ULTIMATE_TRIGGER {
        return;
    }
    let owned = local_hero(base) == Some(6) && has_effect(player, spirit::OWNER, base);
    if let Some(state) = SPIRITS.get() {
        if let Ok(mut s) = state.lock() {
            s.synchronize(player, owned);
            s.ultimate();
            let (npc, live, pending) = s.request_status();
            nlog_info!("Soul ultimate: owned={owned} stored={npc:?} live={live} pending={pending}");
        }
    }
}
pub(crate) unsafe fn update(player: usize, base: usize, incapacitated: bool, dt: f32) {
    let spirit = local_hero(base) == Some(6) && has_effect(player, spirit::OWNER, base);
    let Some(state) = SPIRITS.get() else {
        return;
    };
    let revenant = local_hero(base) == Some(6);
    let live = {
        let Ok(mut s) = state.lock() else {
            return;
        };
        s.synchronize_life(player, spirit, revenant && incapacitated);
        s.advance(dt);
        s.live()
    };
    flush_capture(base);
    if live != 0 && soul_finished(player, base, live) {
        if let Ok(mut s) = state.lock() {
            s.finished();
            nlog_info!("Stored soul retired: handle={live}");
        }
    }
    let soul = {
        let Ok(mut s) = state.lock() else {
            return;
        };
        s.take(!incapacitated)
    };
    if let Some(soul) = soul {
        let handle = summon(player, soul, base);
        if let Some(handle) = handle {
            nlog_info!(
                "Stored soul summon: npc={} think={} handle={handle}",
                soul.npc,
                soul.think
            );
            if let Ok(mut s) = state.lock() {
                s.created(handle);
            }
        }
    }
}

// Collection is acknowledged on the current owner, independently of corpse
// visibility or lifetime. The queued corpse handle identifies the capture only.
pub(crate) unsafe fn flush_capture(base: usize) {
    let world = read_ptr(base + WORLD_RVA);
    if world == 0 { return; }
    let player = read_ptr(world + 0x174e8);
    if player == 0 || stats(player) == 0 { return; }
    let cue = SPIRITS
        .get()
        .and_then(|s| s.lock().ok().and_then(|mut s| {
            if s.owned_by(player) { s.take_capture_cue() } else { None }
        }));
    let Some(corpse) = cue else {
        return;
    };
    let apply: unsafe extern "system" fn(usize, i32, u8) -> u8 =
        std::mem::transmute(base + APPLY_SELF_RVA);
    apply(player, spirit::CAPTURE_CUE, 0);
    nlog_info!("Soul collection cue requested: owner={} corpse={} effect={}",
        ptr::read_unaligned((player + 8) as *const u64), corpse, spirit::CAPTURE_CUE);
}

#[cfg(test)]
mod tests;
