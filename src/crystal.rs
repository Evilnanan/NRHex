//! One shell per owner, consumed inside the native hit processing callback.
//! The saved native cooldown and local timer both gate absorption. Visuals are
//! managed on character Update, outside the hit callback and the state lock.
pub const OWNER: i32 = 95000700;
pub const READY: i32 = 95000701;
pub const COOLDOWN: i32 = 95000702;
// Original type-69 greatshields (e.g. 32000000) have guardBaseRepel=70.
pub const REBOUND_DEFENCE: i32 = 70;
pub const HIT_SIZE: usize = 0x2c8;

/// Build a private input for the attacker's native recoil dispatcher. Its HP
/// consumer is never called. Keep every original attack/animation qualifier;
/// only supply the greatshield comparison result and defence for this shell.
pub fn rebound_hit(hit: &[u8; HIT_SIZE]) -> Option<[u8; HIT_SIZE]> {
    let power = i32::from_le_bytes(hit[0x2a8..0x2ac].try_into().unwrap());
    // These are the native dispatcher's cancellation/guard-break gates. An
    // existing recoil goes through its normal notification only once.
    if hit[0x2be] != 0 || hit[0x2b5] != 0 || hit[0x2c1] & 2 != 0 ||
        !(0..=REBOUND_DEFENCE).contains(&power) { return None; }
    if !(0..5).any(|i| f32::from_le_bytes(hit[i*4..i*4+4].try_into().unwrap()) > 0.0) { return None; }
    let mut recoil = *hit;
    recoil[0x2be] = 1;
    recoil[0x2ac..0x2b0].copy_from_slice(&REBOUND_DEFENCE.to_le_bytes());
    Some(recoil)
}

#[derive(Clone, Copy)]
pub struct Snapshot { pub owned: bool, pub hp: i32, pub cooldown: bool, pub visual: bool, pub beast: bool }
#[derive(Clone, Copy)]
pub struct Hit { pub ignores_nullify: bool, pub hp_damage: i32, pub already_nullified: bool }
#[derive(Debug, PartialEq, Eq)]
pub enum Action { RemoveVisual, StartCooldown, PrepareVisual }
#[derive(Default)]
pub struct Shell { player: usize, stats: usize, owned: bool, remaining: f32, broken: bool }
impl Shell {
    pub fn visual_ready(&self, s: Snapshot) -> bool {
        s.owned && s.hp > 0 && !s.beast && !s.cooldown && !self.broken && self.remaining <= 0.0
    }
    fn synchronize(&mut self, player: usize, stats: usize, s: Snapshot) {
        if self.player != player || self.stats != stats {
            *self = Self { player, stats, ..Self::default() };
        }
        self.owned = s.owned;
        if !s.owned { self.remaining = 0.0; self.broken = false; }
    }
    pub fn absorb(&mut self, player: usize, stats: usize, s: Snapshot, hit: Hit) -> bool {
        self.synchronize(player, stats, s);
        if !self.owned || s.beast || s.hp <= 0 || s.cooldown || self.remaining > 0.0 ||
            hit.ignores_nullify || hit.hp_damage <= 0 || hit.already_nullified { return false; }
        // Caller holds the mutex across this decision and native hit-bit write.
        self.remaining = 40.0;
        self.broken = true;
        true
    }
    pub fn tick(&mut self, player: usize, stats: usize, s: Snapshot, dt: f32) -> Vec<Action> {
        self.synchronize(player, stats, s);
        if !dt.is_finite() || dt <= 0.0 || dt > 1.0 { return vec![]; }
        self.remaining = (self.remaining - dt).max(0.0);
        if !self.owned {
            self.broken = false;
            return if s.visual { vec![Action::RemoveVisual] } else { vec![] };
        }
        if self.broken {
            self.broken = false;
            return vec![Action::RemoveVisual, Action::StartCooldown];
        }
        if s.hp <= 0 || s.beast { return if s.visual { vec![Action::RemoveVisual] } else { vec![] }; }
        if self.remaining > 0.0 || s.cooldown {
            return if s.visual { vec![Action::RemoveVisual] } else { vec![] };
        }
        if !s.visual {vec![Action::PrepareVisual]} else {vec![]}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn s() -> Snapshot { Snapshot { owned: true, hp: 500, cooldown: false, visual: false, beast: false } }
    fn hit() -> Hit { Hit { ignores_nullify: false, hp_damage: 100, already_nullified: false } }
    #[test] fn greatshield_rebound_preserves_native_attack_and_only_changes_private_result() {
        let mut original=[0u8;HIT_SIZE];original[0..4].copy_from_slice(&100f32.to_le_bytes());
        original[0x2c3]=2;original[0x122]=7;
        for power in [0i32,30,50,69,70,71,80,99,-1] {
            original[0x2a8..0x2ac].copy_from_slice(&power.to_le_bytes());
            let result=rebound_hit(&original);assert_eq!(result.is_some(),(0..=70).contains(&power));
            if let Some(copy)=result {
                assert_eq!(copy[0x2be],1);assert_eq!(&copy[0x2ac..0x2b0],&70i32.to_le_bytes());
                assert!(copy.iter().zip(original).enumerate().all(|(i,(a,b))|i==0x2be||(0x2ac..0x2b0).contains(&i)||*a==b));
            }
            assert_eq!(original[0x2be],0);
        }
    }
    #[test] fn cancelled_guard_break_and_existing_rebound_never_dispatch_again() {
        let mut h=[0u8;HIT_SIZE];h[..4].copy_from_slice(&1f32.to_le_bytes());
        for (offset,value) in [(0x2be,1),(0x2b5,1),(0x2c1,2)] {
            h[offset]=value;assert!(rebound_hit(&h).is_none());h[offset]=0;
        }
        h[..4].copy_from_slice(&0f32.to_le_bytes());assert!(rebound_hit(&h).is_none());
        h[16..20].copy_from_slice(&1f32.to_le_bytes());assert!(rebound_hit(&h).is_some());
    }
    #[test] fn display_lease_does_not_rearm_absorption_during_cooldown() {
        let mut shell=Shell::default();let mut sample=s();
        assert!(shell.visual_ready(sample));
        assert!(shell.absorb(1,2,sample,hit()));
        assert!(!shell.visual_ready(sample));
        shell.tick(1,2,sample,0.1);
        // Losing a short display lease never represents a new shell.
        sample.visual=false;
        for _ in 0..38 {
            shell.tick(1,2,sample,1.0);
            assert!(!shell.visual_ready(sample));
            assert!(!shell.absorb(1,2,sample,hit()));
        }
        sample.cooldown=true;shell.tick(1,2,sample,1.0);shell.tick(1,2,sample,1.0);
        assert!(!shell.visual_ready(sample));
        sample.cooldown=false;assert!(shell.visual_ready(sample));
        sample.hp=0;assert!(!shell.visual_ready(sample));
        sample.hp=500;sample.beast=true;assert!(!shell.visual_ready(sample));
    }
    #[test] fn beast_has_no_shell_or_aura_and_cooldown_continues() {
        let mut shell=Shell::default();let beast=Snapshot{beast:true,visual:true,..s()};
        assert!(!shell.absorb(1,2,beast,hit()));
        assert_eq!(shell.tick(1,2,beast,0.1),[Action::RemoveVisual]);
        assert_eq!(shell.tick(1,2,s(),0.1),[Action::PrepareVisual]);
        assert!(shell.absorb(1,2,s(),hit()));
        assert_eq!(shell.tick(1,2,beast,0.1),[Action::RemoveVisual,Action::StartCooldown]);
        assert!(!shell.absorb(1,2,s(),hit()));
    }
    #[test] fn ready_aura_is_never_renewed_or_periodically_broken() {
        let mut shell=Shell::default();let visual=Snapshot{visual:true,..s()};
        for _ in 0..1000 {assert!(shell.tick(1,2,visual,0.1).is_empty());}
        assert_eq!(shell.tick(1,2,Snapshot{cooldown:true,..visual},0.1),[Action::RemoveVisual]);
    }
    #[test] fn first_hit_consumes_immediately_and_next_hit_in_same_frame_is_not_absorbed() {
        let mut shell=Shell::default(); let mut s=s();
        assert_eq!(shell.tick(1,2,s,0.1),[Action::PrepareVisual]); s.visual=true;
        assert!(shell.absorb(1,2,s,hit()));
        assert!(!shell.absorb(1,2,s,hit()));
        assert_eq!(shell.tick(1,2,s,0.1),[Action::RemoveVisual,Action::StartCooldown]);
        assert!(!shell.absorb(1,2,s,hit()));
    }
    #[test] fn both_local_40_seconds_and_saved_cooldown_must_expire() {
        let mut shell=Shell::default(); let mut s=s();
        assert!(shell.absorb(1,2,s,hit())); shell.tick(1,2,s,0.125);
        for _ in 0..318 { shell.tick(1,2,s,0.125); }
        assert!(!shell.absorb(1,2,s,hit()));
        s.cooldown=true; assert!(shell.tick(1,2,s,0.125).is_empty());
        assert!(!shell.absorb(1,2,s,hit()));
        s.cooldown=false;
        assert_eq!(shell.tick(1,2,s,0.125),[Action::PrepareVisual]);
        assert!(shell.absorb(1,2,s,hit()));
    }
    #[test] fn ignored_hits_do_not_consume_shell() {
        let mut shell=Shell::default();
        for h in [Hit {ignores_nullify:true,..hit()},Hit {hp_damage:0,..hit()},Hit {already_nullified:true,..hit()}] {
            assert!(!shell.absorb(1,2,s(),h));
        }
        assert!(!shell.absorb(1,2,Snapshot {hp:0,..s()},hit()));
        assert!(!shell.absorb(1,2,Snapshot {owned:false,..s()},hit()));
        assert!(shell.absorb(1,2,s(),hit()));
    }
    #[test] fn object_transition_respects_saved_cooldown_and_owner_loss_clears_visual() {
        let mut shell=Shell::default(); let mut s=s();
        assert!(shell.absorb(1,2,s,hit()));
        s.cooldown=true; assert!(!shell.absorb(3,4,s,hit()));
        assert!(shell.tick(3,4,s,0.1).is_empty());
        s.owned=false; s.visual=true;
        assert_eq!(shell.tick(3,4,s,0.1),[Action::RemoveVisual]);
        s.owned=true; s.cooldown=false; s.visual=false;
        assert_eq!(shell.tick(3,4,s,0.1),[Action::PrepareVisual]);
    }
    #[test] fn invalid_frame_delta_does_not_rearm_and_downing_does_not_reset_local_cooldown() {
        let mut shell=Shell::default(); let s=s();
        assert!(shell.absorb(1,2,s,hit()));
        for dt in [0.0,60.0,f32::NAN] { assert!(shell.tick(1,2,s,dt).is_empty()); }
        let down=Snapshot {hp:0,visual:true,..s};
        // If a second hit downs the player in the same frame, still persist
        // the first hit's cooldown instead of losing the queued marker.
        assert_eq!(shell.tick(1,2,down,0.1),[Action::RemoveVisual,Action::StartCooldown]);
        assert!(!shell.absorb(1,2,s,hit()));
    }
}
