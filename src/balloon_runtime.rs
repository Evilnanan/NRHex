//! Balloon ownership, death markers and native shockwave emission.
use crate::native_actor::stats;
use crate::{
    balloon, has_effect, is_local, read_i32, read_ptr, ArtsBulletFn, APPLY_SELF_RVA,
    ARTS_BULLET_RVA, BALLOON, BALLOON_DAMAGE, BALLOON_SOURCE, REMOVE_EFFECT_RVA,
};
use std::sync::atomic::Ordering;

pub(crate) unsafe fn flush_balloon(player: usize, base: usize) {
    // Emit after the native death/effect callback has completed, with no mutex
    // held. This does not depend on another character update after death.
    if !is_local(player, base) {
        if let Some(state) = BALLOON.get() {
            if let Ok(mut b) = state.lock() {
                b.synchronize(0, 0, false);
            }
        }
        return;
    }
    let stats = stats(player);
    if stats == 0 {
        return;
    }
    let damage = BALLOON
        .get()
        .and_then(|s| s.lock().ok().and_then(|mut b| b.take_for(player, stats)));
    if let Some(damage) = damage {
        emit_balloon(player, damage, base);
    }
}

unsafe fn emit_balloon(player: usize, damage: i32, base: usize) {
    BALLOON_DAMAGE.store(damage, Ordering::Release);
    BALLOON_SOURCE.store(player, Ordering::Release);
    // Native 3E0E30 skips state-275 emitters once HP is zero. Applying
    // 95001701 therefore creates an effect node but never creates a bullet.
    // Use the same native behavior emitter and descriptor as 50AA20 directly.
    // Do not also apply the emitter effect: near-death updates could double-fire.
    let emit: ArtsBulletFn = std::mem::transmute(base + ARTS_BULLET_RVA);
    emit_balloon_behavior(emit, player);
}

unsafe fn emit_balloon_behavior(emit: ArtsBulletFn, player: usize) -> bool {
    // 396AE0 defaults, with the golden wave's dummy 85 and mode 2.
    let descriptor = [85u32, 0x0000ff02, 0, 0xbf800000, 0x00000101];
    let mut error = -1i32;
    emit(
        player + 0x100,
        balloon::BEHAVIOR,
        -1,
        descriptor.as_ptr() as usize,
        0x10,
        0x10,
        1,
        0,
        0xff,
        &mut error as *mut i32 as usize,
    ) != 0
}
pub(crate) unsafe fn notify(player: usize, effect: i32, base: usize, stats: usize) {
    if effect != balloon::DOWN && effect != balloon::REVIVED {
        return;
    }
    let owned = has_effect(player, balloon::OWNER, base);
    // A native death callback arrives before its effect cleanup. HKS can
    // arrive later; retain the last living snapshot when ownership was removed.
    let buffs=owned.then(||crate::ability_damage::balloon_buffs(player,base));
    let hp = read_i32(stats + 0x144);
    if let Some(state) = BALLOON.get() {
        if let Ok(mut b) = state.lock() {
            let retained = b.owned_by(player, stats);
            let hp = if !owned && retained && b.maximum() > 0 {
                b.maximum()
            } else {
                hp
            };
            b.synchronize(player, stats, owned || retained);
            if let Some(buffs)=buffs {b.remember_buffs(buffs);}
            b.remember_max(hp);
            b.marker(effect, hp);
        }
    }
}
pub(crate) unsafe fn update(player: usize, base: usize, stats: usize, incapacitated: bool) {
    let balloon = has_effect(player, balloon::OWNER, base);
    let buffs=(balloon&&!incapacitated).then(||crate::ability_damage::balloon_buffs(player,base));
    let apply: unsafe extern "system" fn(usize, i32, u8) -> u8 =
        std::mem::transmute(base + APPLY_SELF_RVA);
    let remove: unsafe extern "system" fn(usize, i32) -> u8 =
        std::mem::transmute(base + REMOVE_EFFECT_RVA);
    for (id, owned) in [(balloon::MARKER, balloon && !incapacitated)] {
        let present = has_effect(player, id, base);
        if owned && !present {
            apply(player, id, 0);
        } else if !owned && present {
            remove(read_ptr(player + 0x190), id);
        }
    }
    let emit = BALLOON.get().and_then(|s| {
        s.lock().ok().and_then(|mut b| {
            let pending = b.take_for(player, stats);
            b.synchronize_life(player, stats, balloon, incapacitated);
            if balloon && !incapacitated {
                b.remember_max(read_i32(stats + 0x144));
                if let Some(buffs)=buffs {b.remember_buffs(buffs);}
            }
            pending
        })
    });
    if let Some(damage) = emit {
        emit_balloon(player, damage, base);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;
    unsafe extern "system" fn balloon_emit(
        module: usize,
        behavior: i32,
        event: i32,
        descriptor: usize,
        source: i32,
        original_source: i32,
        flag0: u8,
        flag1: u8,
        kind: u8,
        error: usize,
    ) -> u8 {
        assert_eq!((module, behavior, event), (0x1100, 950017000, -1));
        assert_eq!(
            ptr::read_unaligned(descriptor as *const [u32; 5]),
            [85, 0xff02, 0, 0xbf800000, 0x101]
        );
        assert_eq!(
            (source, original_source, flag0, flag1, kind),
            (16, 16, 1, 0, 255)
        );
        ptr::write(error as *mut i32, 0);
        1
    }
    #[test]
    fn death_explosion_calls_the_native_emitter_without_a_living_effect_update() {
        unsafe {
            assert!(emit_balloon_behavior(balloon_emit, 0x1000));
        }
    }
}
