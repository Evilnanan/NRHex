//! Observe native damaging hits rather than HP changes. Attack immunity still
//! triggers this perk, matching the requested native on-damage semantics.
pub const OWNER: i32 = 95000800;
pub const INVISIBLE: i32 = 95000801;
#[derive(Default)]
pub struct Cat { player: usize, stats: usize, queued: bool }
impl Cat {
    fn synchronize(&mut self, player: usize, stats: usize) {
        if self.player != player || self.stats != stats { *self = Self { player, stats, queued: false }; }
    }
    pub fn hit(&mut self, player: usize, stats: usize, owned: bool, hp_damage: i32, hp: i32) -> bool {
        self.synchronize(player,stats);
        if !owned || hp_damage <= 0 || hp <= 0 { return false; }
        self.queued = true; true
    }
    pub fn tick(&mut self, player: usize, stats: usize, owned: bool, hp: i32, other_invisibility: bool) -> bool {
        self.synchronize(player,stats);
        let requested = std::mem::take(&mut self.queued);
        requested && owned && hp > 0 && !other_invisibility
    }
}
#[cfg(test)]
mod tests {
 use super::*;
 #[test] fn requires_damaging_hit_survival_and_ownership_but_not_hp_loss() {
  let mut c=Cat::default();
  for (owned,damage,hp) in [(false,100,50),(true,0,100),(true,-1,100),(true,100,0)] {
   assert!(!c.hit(1,2,owned,damage,hp));assert!(!c.tick(1,2,true,100,false));
  }
  // A shell-immune hit has positive native damage even though HP stays 100.
  assert!(c.hit(1,2,true,100,100));assert!(c.tick(1,2,true,100,false));assert!(!c.tick(1,2,true,100,false));
 }
 #[test] fn coalesces_hits_and_rearms_for_refresh() {
  let mut c=Cat::default();c.hit(1,2,true,100,90);c.hit(1,2,true,90,80);
  assert!(c.tick(1,2,true,80,false));assert!(!c.tick(1,2,true,80,false));
  c.hit(1,2,true,80,70);assert!(c.tick(1,2,true,70,false));
 }
 #[test] fn discards_request_after_death_transition_owner_loss_or_other_invisibility() {
  for (player,stats,owned,hp,other) in [(3,2,true,50,false),(1,3,true,50,false),(1,2,false,50,false),(1,2,true,0,false),(1,2,true,50,true)] {
   let mut c=Cat::default();c.hit(1,2,true,100,50);
   assert!(!c.tick(player,stats,owned,hp,other));assert!(!c.tick(player,stats,true,50,false));
  }
 }
}
