//! Dispatch shared game callbacks to ability-specific runtimes.
use crate::native_actor::{incapacitated, stats};
use crate::{balloon_runtime, is_local, spirit_runtime, ACTIVE, BASE, DEATH_ORIG};
use std::sync::atomic::Ordering;

pub(crate) unsafe extern "system" fn death_detour(actor: usize) {
    let original: unsafe extern "system" fn(usize) =
        std::mem::transmute(DEATH_ORIG.load(Ordering::Acquire));
    let base = BASE.load(Ordering::Acquire);
    let local = ACTIVE.load(Ordering::Acquire) && is_local(actor, base);
    if ACTIVE.load(Ordering::Acquire) {
        crate::demons_runtime::died(actor);
        // Covers direct deaths (including falling/riding), before native cleanup.
        if is_local(actor, base) {
            balloon_runtime::prepare(actor, base);
            spirit_runtime::notify(actor, crate::balloon::DOWN, base);
            crate::ultimate_challenge::clear(actor,base);
        }
        spirit_runtime::observe_soul(actor, base, false);
    }
    original(actor);
    if ACTIVE.load(Ordering::Acquire) {
        spirit_runtime::flush_capture(base);
    }
    if local {
        balloon_runtime::transition(actor, base);
    }
}

pub(crate) unsafe fn on_effect(player: usize, effect: i32, base: usize) {
    let stats = stats(player);
    if stats == 0 {
        return;
    }
    spirit_runtime::notify(player, effect, base);
}
pub(crate) unsafe fn update(player: usize, base: usize, dt: f32) {
    crate::essence_reaver::update(player, base);
    let stats = stats(player);
    if stats == 0 {
        return;
    }
    let down = incapacitated(player, stats);
    balloon_runtime::update(player, base, stats);
    spirit_runtime::update(player, base, down, dt);
}
