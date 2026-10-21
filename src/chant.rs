pub const OWNER:i32=95002000;
pub const ENABLED:i32=6999920;
pub const PULSE:i32=6999921;
pub const VISUAL:i32=95002001;
pub const RESTORE:i32=30;
#[derive(Default)]
pub struct Chant { identity:(usize,usize), active:bool, seen:bool, charging:bool }
impl Chant {
 pub fn frame(&mut self,player:usize,stats:usize,active:bool){
  if self.identity!=(player,stats)||!active||!self.seen{self.charging=false;}
  self.identity=(player,stats);self.active=active;self.seen=false;
 }
 pub fn pulse(&mut self,player:usize,stats:usize,active:bool,charging:bool)->bool{
  if !active||!self.active||self.identity!=(player,stats){self.charging=false;return false;}
  self.seen=true;
  let completed=self.charging&&!charging;
  self.charging=charging;completed
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn complete_charge_once_without_periodic_regeneration(){
  let mut state=Chant::default();
  for _ in 0..240{state.frame(1,2,true);assert!(!state.pulse(1,2,true,true));assert!(!state.pulse(1,2,true,true));}
  state.frame(1,2,true);assert!(state.pulse(1,2,true,false));
  for _ in 0..240{state.frame(1,2,true);assert!(!state.pulse(1,2,true,false));}
  assert_eq!(RESTORE,30);
 }
 #[test]fn uncharged_cancelled_or_interrupted_casts_never_complete(){
  let mut state=Chant::default();state.frame(1,2,true);assert!(!state.pulse(1,2,true,false));
  assert!(!state.pulse(1,2,true,true));state.frame(1,2,true);state.frame(1,2,true);
  assert!(!state.pulse(1,2,true,false));
 }
 #[test]fn death_ownership_and_identity_clear_charge(){
  for (player,stats,active)in [(1,2,false),(1,3,true),(3,2,true)]{
   let mut state=Chant::default();state.frame(1,2,true);state.pulse(1,2,true,true);
   state.frame(player,stats,active);assert!(!state.pulse(player,stats,active,false));
  }
  let mut state=Chant::default();state.frame(1,2,true);state.pulse(1,2,true,true);
  assert!(!state.pulse(1,2,false,false));assert!(!state.pulse(1,2,true,false));
 }
 #[test]fn consecutive_casts_rearm_only_after_a_new_charge_window(){
  let mut state=Chant::default();
  for _ in 0..3{
   state.frame(1,2,true);assert!(!state.pulse(1,2,true,true));
   state.frame(1,2,true);assert!(state.pulse(1,2,true,false));assert!(!state.pulse(1,2,true,false));
  }
 }
}
