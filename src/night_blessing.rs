//! Night's Tide gesture blessing. Timers advance only with character game time.
use crate::{has_effect, native_actor, read_ptr};
use std::sync::{Mutex, OnceLock};
pub const OWNER: i32 = 95002500;
pub const HEAL_FIRST: i32 = 95002501;
pub const HEAL_SECOND: i32 = 95002502;
pub const ATTACK: i32 = 95002503;
pub const VISUAL: i32 = 95002504;
pub const COOLDOWN: i32 = 95002505;
pub const GRACE: i32 = 95002506;
pub const GESTURE: i32 = 95002507;
static STATE: OnceLock<Mutex<State>> = OnceLock::new();
#[derive(Default)]
struct State {
    object: (usize, usize),
    pose: bool,
    session: bool,
    charged: bool,
    charge: f32,
    after: f32,
    heal_tier: Option<i32>,
    attack: bool,
    pending_cooldown: bool,
}
#[derive(Default, Debug, PartialEq)]
struct Output {
    heal: bool,
    visual: bool,
    attack: bool,
    cooldown: bool,
}
impl State {
    fn tick(
        &mut self,
        object: (usize, usize),
        owned: bool,
        alive: bool,
        rain_tier: Option<i32>,
        pose: bool,
        cooldown: bool,
        grace: bool,
        dt: f32,
    ) -> Output {
        if self.object != object || !owned {
            *self = Self {
                object,
                ..Self::default()
            };
        }
        if !owned {
            return Output::default();
        }
        if grace {
            self.pending_cooldown = false;
        }
        let cooldown = cooldown && !grace;
        let dt = if dt.is_finite() && dt > 0. {
            dt.min(1.)
        } else {
            0.
        };
        if cooldown {
            self.pending_cooldown = false;
        }
        self.after = (self.after - dt).max(0.);
        let rain = rain_tier.is_some();
        if !rain {
            self.attack = false;
            self.charge = 0.;
        }
        if !alive {
            self.attack = false;
            self.after = 0.;
            self.charge = 0.;
            self.charged = false;
        }
        // A used pose starts cooldown on EXIT, even if interrupted before charging.
        if (!pose || !alive) && self.session {
            if self.charged && alive {
                self.after = 15.;
            }
            self.session = false;
            self.charge = 0.;
            self.charged = false;
            self.pending_cooldown = !grace;
        }
        // A pose entered while cooling down stays blocked until it is exited.
        // Entering the rain while already posing may start a fresh eligible session.
        if alive
            && rain
            && pose
            && !self.session
            && !cooldown
            && !self.pending_cooldown
            && !self.pose
        {
            self.session = true;
        }
        if self.session && alive && rain && pose {
            self.charge += dt;
            if self.charge >= 5. {
                self.charged = true;
                self.attack = true;
            }
        }
        self.pose = pose && rain;
        let heal = alive && ((rain && self.session && pose) || self.after > 0.);
        if heal && rain {
            self.heal_tier = rain_tier;
        }
        Output {
            heal,
            visual: heal && (self.charged || self.after > 0.),
            attack: alive && rain && self.attack,
            cooldown: self.pending_cooldown,
        }
    }
}
// Only damaging native rows identify actual exposure. The 99873/99878 five-
// second helper rows can outlive exposure and must never prolong our bonuses.
fn rain_tier(mut has: impl FnMut(i32) -> bool) -> Option<i32> {
    if [99811, 99876, 99877].into_iter().any(&mut has) {
        Some(HEAL_SECOND)
    } else if [99810, 99871, 99872].into_iter().any(has) {
        Some(HEAL_FIRST)
    } else {
        None
    }
}
pub unsafe fn update(player: usize, base: usize, dt: f32) {
    let stats = native_actor::stats(player);
    if stats == 0 {
        return;
    }
    let owned = has_effect(player, OWNER, base);
    if !owned && STATE.get().is_none() {
        return;
    }
    let grace = has_effect(player, GRACE, base);
    let rain = rain_tier(|id| has_effect(player, id, base));
    let (output, heal_tier) = {
        let Ok(mut s) = STATE.get_or_init(|| Mutex::new(State::default())).lock() else {
            return;
        };
        let output = s.tick(
            (player, stats),
            owned,
            !native_actor::incapacitated(player, stats),
            rain,
            has_effect(player, GESTURE, base),
            has_effect(player, COOLDOWN, base),
            grace,
            dt,
        );
        (output, s.heal_tier)
    };
    let remove: unsafe extern "system" fn(usize, i32) -> u8 =
        std::mem::transmute(base + crate::REMOVE_EFFECT_RVA);
    let apply: unsafe extern "system" fn(usize, i32, u8) -> u8 =
        std::mem::transmute(base + crate::APPLY_SELF_RVA);
    let manager = read_ptr(player + 0x190);
    if grace && has_effect(player, COOLDOWN, base) {
        remove(manager, COOLDOWN);
    }
    if output.cooldown && !has_effect(player, COOLDOWN, base) {
        apply(player, COOLDOWN, 0);
    }
    for (id, wanted) in [
        (HEAL_FIRST, output.heal && heal_tier == Some(HEAL_FIRST)),
        (HEAL_SECOND, output.heal && heal_tier == Some(HEAL_SECOND)),
        (ATTACK, output.attack),
        (VISUAL, output.visual),
    ] {
        let present = has_effect(player, id, base);
        if wanted && !present {
            apply(player, id, 0);
        } else if !wanted && present {
            remove(manager, id);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn step(s: &mut State, rain: bool, pose: bool, cd: bool, dt: f32) -> Output {
        s.tick(
            (1, 2),
            true,
            true,
            rain.then_some(HEAL_FIRST),
            pose,
            cd,
            false,
            dt,
        )
    }
    #[test]
    fn early_exit_cools_down_all_effects_and_reentry_cannot_heal() {
        let mut s = State::default();
        assert!(step(&mut s, true, true, false, 1.).heal);
        let o = step(&mut s, true, false, false, 0.1);
        assert!(o.cooldown && !o.heal && !o.attack);
        assert!(!step(&mut s, true, true, true, 1.).heal);
        assert!(!step(&mut s, true, true, false, 1.).heal);
        step(&mut s, true, false, false, 0.);
        assert!(step(&mut s, true, true, false, 1.).heal);
    }
    #[test]
    fn five_seconds_unlocks_visual_and_attack_then_fifteen_second_tail() {
        let mut s = State::default();
        for _ in 0..4 {
            let o = step(&mut s, true, true, false, 1.);
            assert!(o.heal && !o.visual && !o.attack);
        }
        assert_eq!(
            step(&mut s, true, true, false, 1.),
            Output {
                heal: true,
                visual: true,
                attack: true,
                cooldown: false
            }
        );
        let o = step(&mut s, true, false, false, 0.1);
        assert!(o.cooldown && o.visual && o.attack);
        for _ in 0..14 {
            assert!(step(&mut s, true, false, true, 1.).heal);
        }
        let o = step(&mut s, true, false, true, 1.);
        assert!(!o.heal && !o.visual && o.attack);
        assert!(!step(&mut s, false, false, true, 0.1).attack);
        assert!(!step(&mut s, true, false, true, 0.1).attack);
    }
    #[test]
    fn leaving_rain_only_cancels_attack_and_keeps_earned_heal_tail() {
        let mut s = State::default();
        for _ in 0..5 {
            step(&mut s, true, true, false, 1.);
        }
        let o = step(&mut s, false, true, false, 1.);
        assert_eq!(o, Output::default());
        assert!(step(&mut s, false, false, false, 1.).cooldown);
        for _ in 0..14 {
            let o = step(&mut s, false, false, true, 1.);
            assert!(o.heal && o.visual && !o.attack && !o.cooldown);
        }
        let o = step(&mut s, true, false, true, 1.);
        assert!(!o.heal && !o.attack && !o.cooldown);
    }
    #[test]
    fn leaving_rain_on_exit_retains_fifteen_seconds_and_last_healing_tier() {
        let mut s = State::default();
        for _ in 0..5 {
            s.tick(
                (1, 2),
                true,
                true,
                Some(HEAL_SECOND),
                true,
                false,
                false,
                1.,
            );
        }
        let o = step(&mut s, false, false, false, 0.1);
        assert!(o.heal && o.visual && !o.attack && o.cooldown);
        assert_eq!(s.after, 15.);
        assert_eq!(s.heal_tier, Some(HEAL_SECOND));
        for _ in 0..7 {
            assert!(step(&mut s, false, false, true, 1.).heal);
        }
        assert_eq!(s.heal_tier, Some(HEAL_SECOND));
        let o = step(&mut s, true, false, true, 1.);
        assert!(o.heal && !o.attack);
        assert_eq!(s.after, 7.);
        for _ in 0..7 {
            step(&mut s, false, false, true, 1.);
        }
        assert!(!step(&mut s, false, false, true, 0.).heal);
    }
    #[test]
    fn interruption_death_loss_and_new_player_clear_charge_and_attack() {
        for mode in 0..3 {
            let mut s = State::default();
            for _ in 0..5 {
                step(&mut s, true, true, false, 1.);
            }
            let o = s.tick(
                if mode == 2 { (3, 4) } else { (1, 2) },
                mode != 1,
                mode != 0,
                None,
                false,
                false,
                false,
                0.1,
            );
            assert!(!o.heal && !o.attack && !o.visual);
        }
    }
    #[test]
    fn grace_clears_pending_and_native_cooldown_without_waiting() {
        let mut s = State::default();
        step(&mut s, true, true, false, 1.);
        step(&mut s, true, false, false, 0.);
        assert!(s.pending_cooldown);
        assert_eq!(
            s.tick((1, 2), true, true, None, false, true, true, 0.1),
            Output::default()
        );
        assert!(!s.pending_cooldown);
        assert!(step(&mut s, true, true, false, 1.).heal);
    }
    #[test]
    fn grace_only_resets_cooldown_and_preserves_remaining_healing() {
        let mut s = State::default();
        for _ in 0..5 {
            step(&mut s, true, true, false, 1.);
        }
        step(&mut s, true, false, false, 0.);
        let o = s.tick((1, 2), true, true, None, false, true, true, 1.);
        assert!(o.heal && o.visual && !o.attack && !o.cooldown);
        assert_eq!(s.after, 14.);
        assert!(step(&mut s, true, true, false, 1.).heal);
        assert!(s.session);
        assert!(step(&mut s, true, false, false, 1.).heal);
        assert_eq!(s.after, 12.);
    }
    #[test]
    fn cooldown_blocks_charge_and_paused_time_does_not_advance() {
        let mut s = State::default();
        for _ in 0..8 {
            assert_eq!(step(&mut s, true, true, true, 1.), Output::default());
        }
        step(&mut s, true, false, false, 0.);
        step(&mut s, true, true, false, 0.);
        for dt in [0., -1., f32::NAN, f32::INFINITY] {
            assert!(!step(&mut s, true, true, false, dt).attack);
        }
        assert_eq!(s.charge, 0.);
    }
    #[test]
    fn native_damage_upgrade_never_increases_healing_and_helpers_do_not_count() {
        for id in [99810, 99871, 99872] {
            assert_eq!(rain_tier(|v| v == id), Some(HEAL_FIRST));
        }
        for id in [99811, 99876, 99877] {
            assert_eq!(rain_tier(|v| v == id), Some(HEAL_SECOND));
        }
        for id in [99873, 99874, 99878, 99879] {
            assert_eq!(rain_tier(|v| v == id), None);
        }
    }
    #[test]
    fn repeated_cycles_never_stack_attack_and_new_visual_requires_charge() {
        let mut s = State::default();
        for _ in 0..5 {
            step(&mut s, true, true, false, 1.);
        }
        step(&mut s, true, false, false, 0.);
        for _ in 0..60 {
            step(&mut s, true, false, true, 1.);
        }
        assert!(step(&mut s, true, true, false, 1.).attack);
        assert!(!step(&mut s, true, true, false, 1.).visual);
        for _ in 0..3 {
            step(&mut s, true, true, false, 1.);
        }
        assert!(s.attack);
    }
}
