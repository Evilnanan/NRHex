//! Trigger logic independent of animation script replacement. Effects are
//! observed only after successful native application, then applied on Update.
pub const ULTIMATE: i32 = 95000400;
pub const DRINKING: i32 = 95000500;
pub const REGEN: i32 = 95000600;
// Internal action token: handled by the native gauge API, never as a SpEffect.
pub const REFILL: i32 = 95000401;
pub const COOLDOWN: i32 = 95000402;
pub const ULTIMATE_READY:i32=95000403;
pub const DRINK_BUFF: i32 = 95000501;
pub const DRINK_GLOW: i32 = 95000509;
// 0.2s marker gate permits at most 76 living 15s stacks. Eighty native rows
// leave room for frame rounding; each different ID expires independently.
pub const DRINK_STACKS: usize = 80;
pub const DRINK_STACK_BASE: i32 = 95005000;
pub fn drinking_effect(slot: usize) -> i32 { if slot==0 {DRINK_BUFF} else {DRINK_STACK_BASE+slot as i32} }
pub fn is_drinking_effect(effect: i32) -> bool { effect==DRINK_BUFF || (DRINK_STACK_BASE+1..DRINK_STACK_BASE+DRINK_STACKS as i32).contains(&effect) }
pub const REGEN_PULSE: i32 = 95000601;
pub const REGEN_FLASK: i32 = 95000602;
pub const REGEN_GLOW: i32 = 95000603;
pub const REGEN_INTERVAL: f32 = 0.25;
pub fn can_regenerate(s: Snapshot) -> bool { s.owners & 4 != 0 && s.hp>0 && s.hp<s.max_hp && s.fp>=4 }
pub const ULTIMATE_TRIGGER: i32 = 6999615;
pub const FLASK_TRIGGER: i32 = 7010299;

#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    pub hp: i32, pub max_hp: i32, pub fp: i32, pub gauge: f32,
    pub owners: u8, pub cooldown: bool, pub beast: bool,
}
#[derive(Debug)]
struct Pending {
    initial: f32, age: f32, spent_for: Option<f32>, retry_in: f32,
    awaiting_confirmation: bool,
}
#[derive(Default)]
pub struct Runtime {
    player: usize, stats: usize, last_gauge: Option<f32>,
    pending: Option<Pending>, flask_queued: bool, regen_elapsed: f32,
    cooldown_remaining: f32, drinking_marker_gap: f32,
    drinking_active: Vec<(usize,f32)>, drinking_reset: bool,
    flask_full_before: Option<(bool,f32)>, fp_flask_queued: bool,
    regen_flask_marker_gap: f32,
}
impl Runtime {
    pub fn ultimate_ready(&self,s:Snapshot)->bool{s.owners&1!=0&&s.hp>0&&self.pending.is_none()&&!s.cooldown&&self.cooldown_remaining<=0.0}
    /// An object transition clears all pending requests, even if owners persist.
    pub fn synchronize(&mut self, player: usize, stats: usize, s: Snapshot) {
        let object_changed = self.player != player || self.stats != stats;
        if object_changed {
            *self = Self { player, stats, drinking_reset:true, ..Self::default() };
        }
        if s.owners & 1 == 0 { self.pending = None; self.cooldown_remaining = 0.0; }
        if s.hp <= 0 { self.pending = None; }
        if s.owners & 2 == 0 || s.hp <= 0 {
            self.flask_queued = false; self.drinking_marker_gap = 0.0;
            if !self.drinking_active.is_empty() {self.drinking_reset=true; self.drinking_active.clear();}
        }
        if s.owners & 4 == 0 || s.hp <= 0 { self.regen_elapsed = 0.0; self.fp_flask_queued=false; self.flask_full_before=None; self.regen_flask_marker_gap=0.0; }
    }
    pub fn applied(&mut self, effect: i32, s: Snapshot) {
        if s.hp <= 0 { return; }
        match effect {
            FLASK_TRIGGER => {
                // The native flask marker is emitted on three adjacent frames.
                // A 0.2s gap coalesces that burst, below the observed 0.6s
                // interval of rapid repeated drinks.
                if s.owners & 2 != 0 && self.drinking_marker_gap <= 0.0 {
                    self.flask_queued = true; self.drinking_marker_gap = 0.2;
                }
                if s.owners & 4 != 0 && self.regen_flask_marker_gap<=0.0 {
                    let full=self.flask_full_before.map(|v|v.0).unwrap_or(s.hp>=s.max_hp && s.max_hp>0);
                    self.fp_flask_queued |= full;
                    self.regen_flask_marker_gap=0.2;
                }
            },
            ULTIMATE_TRIGGER if s.owners & 1 != 0 && !s.cooldown && self.cooldown_remaining <= 0.0 && self.pending.is_none() => {
                // Consumption may precede the marker in the same animation
                // frame. Keep the most recent Update reading as well.
                let initial = self.last_gauge.unwrap_or(s.gauge).max(s.gauge);
                self.pending = Some(Pending { initial, age: 0.0, spent_for: None, retry_in: 0.0, awaiting_confirmation: false });
            }
            _ => {}
        }
    }
    pub fn before_flask_heal(&mut self, s: Snapshot) {
        if s.hp>0 && s.owners & 4 != 0 && self.flask_full_before.is_none() {
            self.flask_full_before=Some((s.max_hp>0 && s.hp>=s.max_hp,0.5));
        }
    }
    pub fn take_drinking_reset(&mut self) -> bool { std::mem::take(&mut self.drinking_reset) }
    pub fn drinking_active(&self) -> bool { !self.drinking_active.is_empty() }
    /// An immediate native readback is only provisional. Confirm on the next
    /// Update, after the original game has run and could have reset the gauge.
    pub fn note_refill_attempt(&mut self, reached_full: bool) {
        if let Some(p) = &mut self.pending { p.awaiting_confirmation = reached_full; }
    }
    pub fn tick(&mut self, s: Snapshot, dt: f32) -> Vec<i32> {
        let mut effects = Vec::new();
        if !dt.is_finite() || dt <= 0.0 || dt > 1.0 || !s.gauge.is_finite() { return effects; }
        self.cooldown_remaining = (self.cooldown_remaining - dt).max(0.0);
        self.drinking_marker_gap = (self.drinking_marker_gap - dt).max(0.0);
        self.regen_flask_marker_gap = (self.regen_flask_marker_gap - dt).max(0.0);
        for (_,remaining) in &mut self.drinking_active {*remaining-=dt;}
        self.drinking_active.retain(|(_,remaining)|*remaining>0.00001);
        if s.hp <= 0 {
            self.pending = None; self.flask_queued = false; self.drinking_marker_gap = 0.0; self.regen_elapsed = 0.0; self.fp_flask_queued=false; self.flask_full_before=None; self.regen_flask_marker_gap=0.0;
            self.last_gauge = Some(s.gauge); return effects;
        }
        if s.owners & 2 != 0 && self.flask_queued {
            if let Some(slot)=(0..DRINK_STACKS).find(|slot|!self.drinking_active.iter().any(|v|v.0==*slot)) {
                self.drinking_active.push((slot,15.0)); effects.push(drinking_effect(slot));
            }
        }
        self.flask_queued = false;
        if s.owners & 4 != 0 && self.fp_flask_queued { effects.push(REGEN_FLASK); }
        self.fp_flask_queued=false;
        if let Some((_,age))=&mut self.flask_full_before { *age-=dt; if *age<=0.0 {self.flask_full_before=None;} }
        if s.owners & 1 == 0 || s.cooldown { self.pending = None; }
        let mut confirmed = false;
        if let Some(p) = &mut self.pending {
            p.age += dt;
            p.retry_in = (p.retry_in - dt).max(0.0);
            if p.awaiting_confirmation {
                confirmed = s.gauge >= 99.9 && !s.beast;
                p.awaiting_confirmation = false;
            }
            if p.spent_for.is_none() && s.gauge < p.initial - 0.01 { p.spent_for = Some(0.0); }
            if let Some(elapsed) = &mut p.spent_for {
                *elapsed += dt;
                if !confirmed && *elapsed >= 0.5 && !s.beast && p.retry_in <= 0.0 {
                    effects.push(REFILL);
                    p.retry_in = 0.1;
                }
            }
            if p.age >= 60.0 { self.pending = None; }
        }
        if confirmed {
            self.pending = None;
            self.cooldown_remaining = 45.0;
            effects.push(COOLDOWN);
        }
        if can_regenerate(s) {
            self.regen_elapsed += dt;
            if self.regen_elapsed + 0.00001 >= REGEN_INTERVAL {
                // Retain fractional frame time; one pulse per Update permits
                // a fresh HP/FP check before any delayed catch-up pulse.
                self.regen_elapsed = (self.regen_elapsed-REGEN_INTERVAL).max(0.0); effects.push(REGEN_PULSE);
            }
        } else { self.regen_elapsed = 0.0; }
        self.last_gauge = Some(s.gauge);
        effects
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn ultimate_ready_icon_follows_native_and_local_cooldown() {
        let mut r=Runtime::default();let s=s();r.synchronize(1,2,s);
        assert!(r.ultimate_ready(s));
        assert!(!r.ultimate_ready(Snapshot{cooldown:true,..s}));
        assert!(!r.ultimate_ready(Snapshot{hp:0,..s}));
        assert!(!r.ultimate_ready(Snapshot{owners:0,..s}));
        r.applied(ULTIMATE_TRIGGER,s);assert!(!r.ultimate_ready(s));
        r.cooldown_remaining=45.0;r.pending=None;assert!(!r.ultimate_ready(s));
        for _ in 0..360{r.tick(s,0.125);}assert!(r.ultimate_ready(s));
    }
    fn s() -> Snapshot { Snapshot { hp: 500, max_hp: 1000, fp: 100, gauge: 100.0, owners: 7, cooldown: false, beast: false } }
    fn step(r: &mut Runtime, s: Snapshot, n: usize) -> Vec<i32> {
        (0..n).flat_map(|_| r.tick(s, 0.125)).collect()
    }
    #[test] fn ultimate_requires_success_marker_and_spent_gauge_then_obeys_cooldown() {
        let mut r = Runtime::default(); let mut s = s(); s.owners = 1;
        r.synchronize(1,2,s); r.tick(s,0.1);
        s.gauge = 0.0; assert!(step(&mut r,s,10).is_empty());
        s.gauge = 100.0; r.tick(s,0.1); r.applied(ULTIMATE_TRIGGER,s);
        assert!(step(&mut r,s,10).is_empty());
        s.gauge = 0.0; assert!(step(&mut r,s,3).is_empty());
        assert_eq!(step(&mut r,s,1),[REFILL]);
        r.note_refill_attempt(true); s.gauge=100.0;
        assert_eq!(step(&mut r,s,1),[COOLDOWN]);
        assert!(step(&mut r,s,20).is_empty());
        s.gauge=100.0; s.cooldown=true; r.tick(s,0.1); r.applied(ULTIMATE_TRIGGER,s);
        s.gauge=0.0; assert!(step(&mut r,s,10).is_empty());
        s.cooldown=false; assert!(step(&mut r,s,10).is_empty());
    }
    #[test] fn ultimate_handles_consumption_before_marker_and_delays_beast_refund() {
        let mut r=Runtime::default(); let mut s=s(); s.owners=1;
        r.synchronize(1,2,s); r.tick(s,0.1); s.gauge=0.0; s.beast=true;
        r.applied(ULTIMATE_TRIGGER,s); assert!(step(&mut r,s,80).is_empty());
        s.beast=false; assert_eq!(step(&mut r,s,1),[REFILL]);
        r.note_refill_attempt(true); s.gauge=100.0;
        assert_eq!(step(&mut r,s,1),[COOLDOWN]);
    }
    #[test] fn flask_coalesces_same_frame_marker_and_rearms_for_next_drink() {
        let mut r=Runtime::default(); let mut s=s(); s.owners=2;
        r.synchronize(1,2,s); r.applied(FLASK_TRIGGER,s); r.applied(FLASK_TRIGGER,s);
        assert_eq!(step(&mut r,s,1),[DRINK_BUFF]); assert!(step(&mut r,s,30).is_empty());
        r.applied(FLASK_TRIGGER,s); assert_eq!(step(&mut r,s,1),[drinking_effect(1)]);
        s.owners=0; r.applied(FLASK_TRIGGER,s); assert!(step(&mut r,s,10).is_empty());
    }
    #[test] fn drinking_coalesces_adjacent_frames_but_accepts_rapid_next_flask() {
        let mut r=Runtime::default(); let mut s=s(); s.owners=2;
        r.synchronize(1,2,s);
        for frame in 0..3 {
            r.applied(FLASK_TRIGGER,s);
            assert_eq!(r.tick(s,1.0/60.0),if frame==0 {vec![DRINK_BUFF]} else {vec![]});
        }
        for _ in 0..33 {assert!(r.tick(s,1.0/60.0).is_empty());}
        r.applied(FLASK_TRIGGER,s); assert_eq!(r.tick(s,1.0/60.0),[drinking_effect(1)]);
    }
    #[test] fn drinking_marker_gap_clears_on_object_death_and_owner_loss() {
        let mut r=Runtime::default(); let mut s=s(); s.owners=2;
        r.synchronize(1,2,s); r.applied(FLASK_TRIGGER,s);
        r.synchronize(3,4,s); r.applied(FLASK_TRIGGER,s);
        assert_eq!(r.tick(s,1.0/60.0),[DRINK_BUFF]);
        s.hp=0; r.synchronize(3,4,s); s.hp=500;
        r.synchronize(3,4,s); r.applied(FLASK_TRIGGER,s);
        assert_eq!(r.tick(s,1.0/60.0),[DRINK_BUFF]);
        s.owners=0; r.synchronize(3,4,s); s.owners=2;
        r.synchronize(3,4,s); r.applied(FLASK_TRIGGER,s);
        assert_eq!(r.tick(s,1.0/60.0),[DRINK_BUFF]);
    }
    #[test] fn drinking_each_layer_expires_fifteen_seconds_after_its_own_drink() {
        let mut r=Runtime::default(); let mut s=s(); s.owners=2;
        r.synchronize(1,2,s); r.take_drinking_reset();
        r.applied(FLASK_TRIGGER,s); assert_eq!(r.tick(s,0.125),[DRINK_BUFF]);
        step(&mut r,s,40); // Five seconds after the first stack.
        r.applied(FLASK_TRIGGER,s); assert_eq!(r.tick(s,0.125),[drinking_effect(1)]);
        assert_eq!(r.drinking_active.len(),2);
        step(&mut r,s,79); // The first expires, while the second has 5s left.
        assert_eq!(r.drinking_active.len(),1); assert_eq!(r.drinking_active[0],(1,5.125));
        r.applied(FLASK_TRIGGER,s); assert_eq!(r.tick(s,0.125),[DRINK_BUFF]);
        step(&mut r,s,40); assert_eq!(r.drinking_active.len(),1);
        assert_eq!(r.drinking_active[0].0,0);
        step(&mut r,s,80); assert!(r.drinking_active.is_empty());
    }
    #[test] fn drinking_extreme_marker_rate_stays_within_native_slots_and_resets() {
        let mut r=Runtime::default(); let mut s=s(); s.owners=2;
        r.synchronize(1,2,s); r.take_drinking_reset();
        for _ in 0..10000 {
            r.applied(FLASK_TRIGGER,s); r.tick(s,0.001);
            assert!(r.drinking_active.len()<DRINK_STACKS);
        }
        s.owners=0; r.synchronize(1,2,s);
        assert!(r.take_drinking_reset()); assert!(r.drinking_active.is_empty());
    }
    #[test] fn regeneration_checks_full_hp_fp_death_and_game_time() {
        let mut r=Runtime::default(); let mut s=s(); s.owners=4;
        r.synchronize(1,2,s); assert!(step(&mut r,s,1).is_empty());
        assert_eq!(step(&mut r,s,1),[REGEN_PULSE]);
        s.hp=s.max_hp; assert!(step(&mut r,s,80).is_empty());
        s.hp=500; s.fp=3; assert!(step(&mut r,s,80).is_empty());
        s.fp=4; assert_eq!(step(&mut r,s,2),[REGEN_PULSE]);
        s.hp=0; assert!(step(&mut r,s,80).is_empty());
        s.hp=500; assert!(r.tick(s,60.0).is_empty()); assert!(r.tick(s,0.0).is_empty());
        assert!(step(&mut r,s,1).is_empty());
    }
    #[test] fn regeneration_keeps_cadence_with_uneven_frames_and_rechecks_resources() {
        let mut r=Runtime::default();let mut s=s();s.owners=4;r.synchronize(1,2,s);
        assert!(r.tick(s,0.15).is_empty());assert_eq!(r.tick(s,0.15),[REGEN_PULSE]);
        assert_eq!(r.tick(s,0.2),[REGEN_PULSE]); // Fractional time was retained.
        assert_eq!(r.tick(s,0.5),[REGEN_PULSE]); // Never spend twice on a stale snapshot.
        s.fp=3;assert!(r.tick(s,0.01).is_empty());assert!(!can_regenerate(s));
        s.fp=4;assert!(r.tick(s,0.24).is_empty());assert_eq!(r.tick(s,0.01),[REGEN_PULSE]);
        s.hp=s.max_hp;assert!(!can_regenerate(s));assert!(r.tick(s,0.5).is_empty());
        s.hp=0;assert!(!can_regenerate(s));s.hp=500;s.owners=0;assert!(!can_regenerate(s));
    }
    #[test] fn regen_flask_uses_hp_before_heal_and_coalesces_duplicate_marker() {
        let mut r=Runtime::default(); let mut s=s(); s.owners=4;
        r.synchronize(1,2,s); r.before_flask_heal(s);
        s.hp=s.max_hp; r.applied(FLASK_TRIGGER,s);
        assert!(step(&mut r,s,1).is_empty()); // Healing to full is ineligible.
        step(&mut r,s,5); r.before_flask_heal(s);
        r.applied(FLASK_TRIGGER,s); r.applied(FLASK_TRIGGER,s);
        assert_eq!(step(&mut r,s,1),[REGEN_FLASK]);
        assert!(step(&mut r,s,5).is_empty());
        s.hp=500; r.before_flask_heal(s); s.hp=1000;
        r.applied(FLASK_TRIGGER,s); assert!(step(&mut r,s,1).is_empty());
    }
    #[test] fn regen_full_flask_and_drinking_can_trigger_together_and_clear_on_transfer() {
        let mut r=Runtime::default(); let mut s=s(); s.owners=6; s.hp=s.max_hp;
        r.synchronize(1,2,s); r.before_flask_heal(s); r.applied(FLASK_TRIGGER,s);
        assert_eq!(step(&mut r,s,1),[DRINK_BUFF,REGEN_FLASK]);
        r.applied(FLASK_TRIGGER,s); r.synchronize(3,4,s);
        assert!(step(&mut r,s,1).is_empty());
        r.applied(FLASK_TRIGGER,s); s.owners=2; r.synchronize(3,4,s);
        assert_eq!(step(&mut r,s,1),[DRINK_BUFF]);
    }
    #[test] fn regen_full_flask_visual_and_restore_only_once_across_native_marker_frames() {
        let mut r=Runtime::default();let mut s=s();s.owners=4;s.hp=s.max_hp;
        r.synchronize(1,2,s);r.before_flask_heal(s);
        for frame in 0..3 {
            r.applied(FLASK_TRIGGER,s);
            assert_eq!(r.tick(s,1.0/60.0),if frame==0 {vec![REGEN_FLASK]} else {vec![]});
        }
        for _ in 0..33 {assert!(r.tick(s,1.0/60.0).is_empty());}
        r.before_flask_heal(s);r.applied(FLASK_TRIGGER,s);assert_eq!(r.tick(s,1.0/60.0),[REGEN_FLASK]);
    }
    #[test] fn object_transition_and_owner_removal_clear_requests() {
        let mut r=Runtime::default(); let mut s=s(); s.owners=3;
        r.synchronize(1,2,s); r.tick(s,0.1); r.applied(FLASK_TRIGGER,s); r.applied(ULTIMATE_TRIGGER,s);
        r.synchronize(3,4,s); s.gauge=0.0; assert!(step(&mut r,s,10).is_empty());
        r.applied(FLASK_TRIGGER,s); s.owners=0; r.synchronize(3,4,s);
        assert!(step(&mut r,s,10).is_empty());
    }
    #[test] fn blocked_or_overwritten_refill_retries_without_starting_cooldown() {
        let mut r=Runtime::default(); let mut s=s(); s.owners=1;
        r.synchronize(1,2,s); r.tick(s,0.1); r.applied(ULTIMATE_TRIGGER,s);
        s.gauge=0.0; assert_eq!(step(&mut r,s,4),[REFILL]);
        r.note_refill_attempt(false); assert_eq!(step(&mut r,s,1),[REFILL]);
        r.note_refill_attempt(true); // Game resets the provisional refill.
        assert_eq!(step(&mut r,s,1),[REFILL]);
        r.note_refill_attempt(true); s.gauge=100.0;
        assert_eq!(step(&mut r,s,1),[COOLDOWN]);
        assert!(step(&mut r,s,10).is_empty());
    }
    #[test] fn confirmed_refill_has_45_second_local_gate_even_before_effect_is_visible() {
        let mut r=Runtime::default(); let mut s=s(); s.owners=1;
        r.synchronize(1,2,s); r.tick(s,0.1); r.applied(ULTIMATE_TRIGGER,s);
        s.gauge=0.0; assert_eq!(step(&mut r,s,4),[REFILL]);
        r.note_refill_attempt(true); s.gauge=100.0;
        assert_eq!(step(&mut r,s,1),[COOLDOWN]);
        // The native saved marker is absent in this simulated async window.
        s.cooldown=false; r.applied(ULTIMATE_TRIGGER,s); s.gauge=0.0;
        assert!(step(&mut r,s,359).is_empty());
        s.gauge=100.0; r.tick(s,0.125); r.applied(ULTIMATE_TRIGGER,s);
        s.gauge=0.0; assert_eq!(step(&mut r,s,4),[REFILL]);
    }
}
