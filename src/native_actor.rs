//! Native actor access shared by the death-triggered abilities.
//! Actor pointers never outlive the synchronous game-thread callback.
use crate::{ptr, read_i32, read_ptr, WORLD_RVA};

pub(crate) unsafe fn npc_id(actor: usize, base: usize) -> Option<i32> {
    if actor == 0 {
        return None;
    }
    let vt = read_ptr(actor);
    if vt == 0 || read_ptr(vt + 0x1a0) != base + 0x4d1ea0 {
        return None;
    }
    let reference = read_ptr(actor + 0x5b8);
    (reference != 0).then(|| read_i32(reference + 8))
}

pub(crate) unsafe fn stats(actor: usize) -> usize {
    if actor == 0 {
        return 0;
    }
    let modules = read_ptr(actor + 0x1b8);
    if modules == 0 {
        0
    } else {
        read_ptr(modules)
    }
}

// Native 66F3B0/674EB0: Player +710 is 0 normal, 1 down, 2 revival.
// The 102100 pulse / conditional 102120 effect do not cover the whole state.
pub(crate) unsafe fn incapacitated(actor: usize, stats: usize) -> bool {
    read_i32(stats + 0x140) <= 0
        || matches!(ptr::read_unaligned((actor + 0x710) as *const u8), 1 | 2)
}

pub(crate) unsafe fn buddy(player: usize, base: usize, offset: usize, index: i32) -> usize {
    let modules = read_ptr(player + 0x1b8);
    if modules == 0 {
        return 0;
    }
    let module = read_ptr(modules + offset);
    if module == 0 {
        return 0;
    }
    let get: unsafe extern "system" fn(usize, i32) -> usize =
        std::mem::transmute(base + if offset == 0x160 { 0x460870 } else { 0x47f890 });
    get(module, index)
}

pub(crate) unsafe fn resolve(base: usize, handle: u64) -> usize {
    let world = read_ptr(base + WORLD_RVA);
    if world == 0 || handle == 0 || handle == u64::MAX {
        return 0;
    }
    let get: unsafe extern "system" fn(usize, *const u64) -> usize =
        std::mem::transmute(base + 0x519140);
    get(world, &handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{balloon, spirit};
    #[test]
    fn positive_rescue_hp_stays_incapacitated_until_native_near_death_state_ends() {
        unsafe {
            let mut actor = vec![0u8; 0x718];
            let actor = actor.as_mut_ptr() as usize;
            let mut stats = vec![0i32; 0x148 / 4];
            stats[0x140 / 4] = 100;
            let st = stats.as_ptr() as usize;
            let mut balloon = balloon::Balloon::default();
            balloon.synchronize_life(actor, st, true, false);
            balloon.marker(balloon::DOWN, 1500);
            assert_eq!(balloon.take(), Some(1500));
            let mut spirits = spirit::Spirits::default();
            spirits.synchronize(actor, true);
            spirits.defeated(
                spirit::Soul {
                    npc: 1,
                    think: 1,
                    chara_init: -1,
                },
                123,
                100,
            );
            for down in [1, 1, 2] {
                ptr::write((actor + 0x710) as *mut u8, down);
                let is_down = incapacitated(actor, st);
                assert!(is_down);
                balloon.synchronize_life(actor, st, false, is_down);
                balloon.marker(balloon::DOWN, 1500);
                assert_eq!(balloon.take(), None);
                spirits.synchronize_life(actor, false, is_down);
                spirits.ultimate();
                assert_eq!(spirits.take(!is_down), None);
                assert!(spirits.owned_by(actor));
            }
            ptr::write((actor + 0x710) as *mut u8, 0);
            assert!(!incapacitated(actor, st));
            balloon.synchronize_life(actor, st, true, false);
            balloon.marker(balloon::DOWN, 1500);
            assert_eq!(balloon.take(), Some(1500));
            stats[0x140 / 4] = 0;
            assert!(incapacitated(actor, st));
        }
    }
}
