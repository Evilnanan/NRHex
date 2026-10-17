pub const OWNER:i32=95001700;
pub const MARKER:i32=6999900;
pub const DOWN:i32=6999901;
pub const BEHAVIOR:i32=950017000;
pub const ATTACK:i32=95001760;
pub const BULLET:i32=95001750;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Life{Alive,Uncommitted,Starting,Down,Reviving,Dead}
impl Life{pub fn committed(self)->bool{matches!(self,Self::Starting|Self::Down|Self::Dead)}}
#[derive(Default)]
pub struct Balloon{player:usize,stats:usize,owned:bool,spent:bool,pending:bool,damage:i32,observed_down:bool,maximum:i32,awaiting_native:bool}
impl Balloon{
 pub fn synchronize(&mut self,player:usize,stats:usize,owned:bool){
  if self.player!=player||self.stats!=stats||!owned{*self=Self{player,stats,owned,..Self::default()};}
  self.owned=owned;
 }
 pub fn commit(&mut self,life:Life,max_hp:i32){
  if self.owned&&life.committed()&&!self.spent&&max_hp>0{
   self.spent=true;self.pending=true;self.damage=max_hp;self.observed_down=true;self.awaiting_native=life==Life::Starting;
  }
 }
 pub fn owned_by(&self,player:usize,stats:usize)->bool{self.owned&&self.player==player&&self.stats==stats}
 pub fn remember_max(&mut self,hp:i32){if self.owned&&hp>0{self.maximum=hp;}}
 pub fn maximum(&self)->i32{self.maximum}
 pub fn base_hp_for(&self,player:usize,stats:usize)->Option<i32>{(self.owned_by(player,stats)&&self.spent&&self.damage>0).then_some(self.damage)}
 pub fn synchronize_life(&mut self,player:usize,stats:usize,owned:bool,life:Life){
  let life=if self.player==player&&self.awaiting_native&&life==Life::Alive{Life::Starting}else{life};
  if self.player==player&&self.observed_down&&life!=Life::Alive{self.stats=stats;}
  let retained=self.owned_by(player,stats)&&(life!=Life::Alive||self.observed_down);
  self.synchronize(player,stats,owned||retained);self.life(life);
 }
 pub fn take(&mut self)->Option<i32>{std::mem::take(&mut self.pending).then_some(self.damage)}
 pub fn take_for(&mut self,player:usize,stats:usize)->Option<i32>{if self.player==player&&self.stats==stats{self.take()}else{None}}
 pub fn life(&mut self,life:Life){
  if matches!(life,Life::Down|Life::Reviving|Life::Dead){self.observed_down=true;self.awaiting_native=false;}
  else if life==Life::Alive&&self.observed_down&&!self.awaiting_native{self.spent=false;self.observed_down=false;}
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn action_entry_emits_before_cleanup_but_healing_cannot_rearm(){
  let mut state=Balloon::default();state.synchronize(1,2,true);state.remember_max(1500);
  state.commit(Life::Starting,1500);assert_eq!(state.take(),Some(1500));
  for _ in 0..100{
   state.synchronize_life(1,2,true,Life::Alive);state.commit(Life::Starting,1500);assert_eq!(state.take(),None);
   state.synchronize_life(1,2,true,Life::Uncommitted);state.commit(Life::Starting,1500);assert_eq!(state.take(),None);
  }
  state.synchronize_life(1,3,false,Life::Alive);assert_eq!(state.base_hp_for(1,3),Some(1500));
  state.synchronize_life(1,3,false,Life::Dead);state.commit(Life::Dead,200);assert_eq!(state.take(),None);
  state.synchronize_life(1,3,true,Life::Alive);state.commit(Life::Starting,2000);assert_eq!(state.take(),Some(2000));
 }
 #[test]fn action_entry_down_revival_and_late_duplicate_notifications_share_one_life(){
  let mut state=Balloon::default();state.synchronize(1,2,true);state.commit(Life::Starting,1500);state.take();
  for life in [Life::Down,Life::Starting,Life::Reviving,Life::Starting]{
   state.synchronize_life(1,2,true,life);state.commit(life,1500);assert_eq!(state.take(),None);
  }
  state.synchronize_life(1,2,true,Life::Alive);state.commit(Life::Starting,1600);assert_eq!(state.take(),Some(1600));
 }
 #[test]fn repeated_zero_hp_and_healing_before_commit_cannot_emit(){
  let mut state=Balloon::default();state.synchronize(1,2,true);
  for _ in 0..100{
   state.synchronize_life(1,2,true,Life::Uncommitted);state.commit(Life::Uncommitted,1500);
   assert_eq!(state.take(),None);
   state.synchronize_life(1,2,true,Life::Alive);assert_eq!(state.take(),None);
  }
 }
 #[test]fn committed_down_then_death_only_explodes_once_despite_positive_hp(){
  let mut state=Balloon::default();state.synchronize(1,2,true);state.commit(Life::Down,1500);
  assert_eq!(state.take(),Some(1500));
  for life in [Life::Down,Life::Reviving,Life::Down,Life::Dead,Life::Uncommitted]{
   state.life(life);state.commit(life,2000);assert_eq!(state.take(),None);
  }
  assert_eq!(state.base_hp_for(1,2),Some(1500));
 }
 #[test]fn only_completed_native_revival_rearms(){
  let mut state=Balloon::default();state.synchronize(1,2,true);state.commit(Life::Down,1500);state.take();
  state.life(Life::Reviving);state.commit(Life::Down,1600);assert_eq!(state.take(),None);
  state.life(Life::Alive);state.commit(Life::Down,1600);assert_eq!(state.take(),Some(1600));
 }
 #[test]fn native_death_reset_followed_by_living_spawn_rearms(){
  let mut state=Balloon::default();state.synchronize(1,2,true);state.commit(Life::Dead,1500);state.take();
  state.life(Life::Dead);state.commit(Life::Dead,2000);assert_eq!(state.take(),None);
  state.life(Life::Uncommitted);state.commit(Life::Dead,2000);assert_eq!(state.take(),None);
  state.life(Life::Alive);state.commit(Life::Dead,2000);assert_eq!(state.take(),Some(2000));
 }
 #[test]fn ownership_cleanup_retains_snapshot_and_cannot_rearm(){
  let mut state=Balloon::default();state.synchronize(1,2,true);state.remember_max(1500);
  state.synchronize_life(1,2,false,Life::Uncommitted);assert_eq!(state.maximum(),1500);
  state.commit(Life::Dead,state.maximum());assert_eq!(state.take(),Some(1500));
  state.synchronize_life(1,2,false,Life::Dead);state.commit(Life::Dead,200);assert_eq!(state.take(),None);
  assert_eq!(state.base_hp_for(1,2),Some(1500));
 }
 #[test]fn attribute_replacement_during_one_death_does_not_rearm(){
  let mut state=Balloon::default();state.synchronize(1,2,true);state.commit(Life::Dead,1500);state.take();
  state.synchronize_life(1,3,false,Life::Dead);state.commit(Life::Dead,3000);assert_eq!(state.take(),None);
  assert_eq!(state.base_hp_for(1,3),Some(1500));
  state.synchronize_life(1,3,true,Life::Alive);state.commit(Life::Down,2000);assert_eq!(state.take(),Some(2000));
 }
 #[test]fn identity_loss_and_invalid_max_hp_cannot_transfer_pending_damage(){
  let mut state=Balloon::default();state.synchronize(1,2,true);state.commit(Life::Down,0);assert_eq!(state.take(),None);
  state.commit(Life::Down,1000);assert_eq!(state.take_for(1,3),None);
  state.synchronize(1,2,false);assert_eq!(state.take(),None);assert_eq!(state.base_hp_for(1,2),None);
  state.synchronize(3,4,true);state.commit(Life::Dead,2000);assert_eq!(state.take_for(1,2),None);assert_eq!(state.take_for(3,4),Some(2000));
 }
}
