//! Action markers are owner-only; native hit completion confirms backstep hits.
use std::collections::VecDeque;
pub const OWNER:i32=95001300;
pub const MARKER:i32=6999700;
pub const BACK_START:i32=6999701;
pub const BACKSTEP_CATEGORY:usize=129;
pub const BACKSTEP_RESTORE_PERCENT:f32=25.;
pub const CRITICAL_RESTORE_PERCENT:f32=25.;
pub fn is_backstep_attack(mask:&[u32;8])->bool{
 let tagged=|category:usize|mask[category/32]&(1<<(category%32))!=0;
 tagged(BACKSTEP_CATEGORY)&&tagged(130)&&!tagged(105)
}
/// Native gauge getter returns 0..100; apply to resolved HP damage before hit rewards.
pub fn boosted(damage:i32,gauge_percent:f32)->i32{
 if damage<=0||!gauge_percent.is_finite(){return damage;}
 let factor=1.+2.*gauge_percent.clamp(0.,100.) as f64/100.;
 (damage as f64*factor).round().min(i32::MAX as f64) as i32
}
/// ThrowParam attack animations and wrapping facing-angle window distinguish front crits.
pub fn is_front_critical(animation:i32,min:f32,max:f32)->bool{matches!(animation,31700|31750)||(animation==31720&&min.is_finite()&&max.is_finite()&&min>0.&&max<0.)}
pub const ROLL:i32=6999705;
pub const CRITICAL:i32=7030801;
/// Only apply on the absent -> owned/alive edge. Native submission is deferred.
pub fn marker_action(owned:bool,alive:bool,present:bool)->Option<bool>{
 match (owned&&alive,present){(true,false)=>Some(true),(false,true)=>Some(false),_=>None}
}
#[derive(Default)]
pub struct Stance {player:usize,stats:usize,owned:bool,rewarded:bool,last_critical:Option<u64>,pending:VecDeque<f32>}
impl Stance {
 pub fn synchronize(&mut self,player:usize,stats:usize,owned:bool,alive:bool){
  if self.player!=player||self.stats!=stats||!owned||!alive{*self=Self{player,stats,owned,..Self::default()};}
  self.owned=owned&&alive;
 }
 pub fn marker(&mut self,effect:i32,now:u64){
  if !self.owned{return;}
  match effect {
   BACK_START=>{self.rewarded=false;},
   ROLL=>{self.queue(-30.);},
   CRITICAL=>{if self.last_critical.is_none_or(|last|now.saturating_sub(last)>200){self.last_critical=Some(now);self.queue(CRITICAL_RESTORE_PERCENT);}},
   _=>{}
  }
 }
 pub fn hit(&mut self,backstep_attack:bool,damage:i32,stance_broken:bool){
  if self.owned&&backstep_attack&&damage>0&&!stance_broken&&!self.rewarded{self.rewarded=true;self.queue(BACKSTEP_RESTORE_PERCENT);}
 }
 pub fn damaged(&mut self,damage:i32){if self.owned&&damage>0{self.queue(-30.);}}
 fn queue(&mut self,value:f32){self.pending.push_back(value);}
 pub fn pending(&self)->Option<f32>{self.pending.front().copied()}
 pub fn acknowledge(&mut self){self.pending.pop_front();}
}
#[cfg(test)] mod tests{
 use super::*;
 fn stance()->Stance{let mut s=Stance::default();s.synchronize(1,2,true,true);s}
 #[test] fn hks_owner_marker_is_created_kept_and_removed_for_lifecycle_edges(){
  assert_eq!(marker_action(true,true,false),Some(true));assert_eq!(marker_action(true,true,true),None);
  assert_eq!(marker_action(false,true,true),Some(false));assert_eq!(marker_action(true,false,true),Some(false));
  assert_eq!(marker_action(false,true,false),None);assert_eq!(marker_action(true,false,false),None);
  assert_eq!(marker_action(true,true,false),Some(true)); // revived or native marker lost
 }
 #[test] fn dual_hits_and_multiple_targets_reward_once_per_backstep(){let mut s=stance();s.marker(BACK_START,0);for _ in 1..5{s.hit(true,100,false);}assert_eq!(s.pending.iter().copied().collect::<Vec<_>>(),[25.]);s.marker(BACK_START,1000);s.hit(true,100,false);assert_eq!(s.pending.iter().copied().collect::<Vec<_>>(),[25.,25.]);}
 #[test] fn backstep_requires_melee_and_excludes_native_ranged_category(){
  for back in [false,true]{for melee in [false,true]{for ranged in [false,true]{
   let mut mask=[0;8];for (tag,on) in [(129,back),(130,melee),(105,ranged)]{if on{mask[tag/32]|=1<<(tag%32);}}
   assert_eq!(is_backstep_attack(&mask),back&&melee&&!ranged);
  }}}
 }
 #[test] fn misses_and_nonbackstep_hits_do_not_reward(){let mut s=stance();s.marker(BACK_START,0);s.hit(false,100,false);s.hit(true,0,false);assert_eq!(s.pending(),None);s.hit(true,100,false);assert_eq!(s.pending(),Some(25.));}
 #[test] fn broken_targets_never_restore_or_consume_the_backstep_reward(){
  let mut s=stance();s.marker(BACK_START,0);
  for _ in 0..10{s.hit(true,100,true);}assert_eq!(s.pending(),None);
  s.hit(true,100,false);s.hit(true,100,false);assert_eq!(s.pending.iter().copied().collect::<Vec<_>>(),[25.]);
  s.marker(BACK_START,1000);s.hit(true,100,true);s.marker(CRITICAL,1000);
  assert_eq!(s.pending.iter().copied().collect::<Vec<_>>(),[25.,25.]);
  assert_eq!(boosted(100,100.),300);
 }
 #[test] fn front_crit_includes_riposte_and_excludes_backstabs(){for id in [31700,31750]{assert!(is_front_critical(id,-180.,180.));}assert!(!is_front_critical(31710,-90.,90.));assert!(is_front_critical(31720,90.,-90.));assert!(!is_front_critical(31720,-90.,90.));assert!(!is_front_critical(31720,f32::NAN,-90.));assert!(!is_front_critical(40000,90.,-90.));}
 #[test] fn critical_pulse_is_not_a_kill_and_native_duplicates_are_filtered(){let mut s=stance();s.marker(CRITICAL,0);s.marker(CRITICAL,20);s.marker(CRITICAL,1000);assert_eq!(s.pending.iter().copied().collect::<Vec<_>>(),[25.,25.]);}
 #[test] fn roll_loss_damage_loss_and_rewards_keep_native_clamp_order(){let mut s=stance();s.marker(ROLL,0);s.damaged(10);s.marker(CRITICAL,20);assert_eq!(s.pending(),Some(-30.));s.acknowledge();assert_eq!(s.pending(),Some(-30.));s.acknowledge();assert_eq!(s.pending(),Some(25.));}
 #[test] fn backstep_damage_tracks_current_gauge_before_hit_restores_it(){
  for (gauge,expected) in [(0.,100),(25.,150),(50.,200),(75.,250),(100.,300)]{assert_eq!(boosted(100,gauge),expected);}
  let mut s=stance();s.marker(BACK_START,0);assert_eq!(boosted(100,50.),200);s.hit(true,200,false);assert_eq!(s.pending(),Some(25.));
 }
 #[test] fn damage_bonus_handles_invalid_gauges_and_integer_limits(){
  for gauge in [f32::NAN,f32::INFINITY,f32::NEG_INFINITY,-1.]{assert_eq!(boosted(101,gauge),101);}
  assert_eq!(boosted(100,200.),300);assert_eq!(boosted(0,100.),0);assert_eq!(boosted(-1,100.),-1);
  assert_eq!(boosted(i32::MAX,100.),i32::MAX);assert_eq!(boosted(1,25.),2);
 }
 #[test] fn death_owner_loss_and_actor_replacement_drop_pending_events(){for (p,st,o,a) in [(1,2,true,false),(1,2,false,true),(3,4,true,true)]{let mut s=stance();s.marker(ROLL,0);s.marker(BACK_START,1);s.synchronize(p,st,o,a);assert_eq!(s.pending(),None);}}
}
