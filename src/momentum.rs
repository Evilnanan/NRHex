//! Native AttackRate is already active while waiting for the first hit, so
//! that hit (including a projectile) receives the bonus before its notification.
//! Only accepted native state2103 notifications start the non-refreshing cycle.
use crate::{has_effect,is_local,native_actor,read_ptr,read_i32,ptr};
use std::sync::Mutex;
pub const OWNER:i32=95003700;
pub const PULSE:i32=OWNER+1;
pub const WEAK:i32=OWNER+2;
pub const VISUAL:i32=OWNER+3;
#[derive(Default)]
struct Cycle{elapsed:Option<f64>}
impl Cycle{
 const fn new()->Self{Self{elapsed:None}}
 fn hit(&mut self)->bool{if self.elapsed.is_some(){return false;}self.elapsed=Some(0.);true}
 fn advance(&mut self,dt:f32){
  if !dt.is_finite()||dt<=0.||dt>1.{return;}
  if let Some(time)=&mut self.elapsed{*time+=f64::from(dt);if *time>=10.{self.elapsed=None;}}
 }
 fn weak(&self)->bool{self.elapsed.is_some_and(|t|t>=5.)}
}
static CYCLE:Mutex<Cycle>=Mutex::new(Cycle::new());
pub unsafe fn pulse(player:usize,effect:i32,base:usize)->bool{
 if effect!=PULSE{return false;}
 // This is a private notification, not an effect to install on a peer.
 if player==0||!is_local(player,base)||!has_effect(player,OWNER,base)||has_effect(player,crate::reward_cycle_runtime::RESET,base){return true;}
 let stats=native_actor::stats(player);if stats==0||native_actor::incapacitated(player,stats){return true;}
 let started=CYCLE.lock().map(|mut cycle|cycle.hit()).unwrap_or(false);
 if started{crate::maintain_status_effect(player,stats,VISUAL,true,base);}
 true
}
pub unsafe fn reset(player:usize,base:usize){
 if let Ok(mut cycle)=CYCLE.lock(){*cycle=Cycle::new();}
 clear(player,base);
}
unsafe fn clear(player:usize,base:usize){
 let stats=native_actor::stats(player);
 for effect in [WEAK,VISUAL]{crate::maintain_status_effect(player,stats,effect,false,base);}
}
pub unsafe fn update(player:usize,base:usize,dt:f32){
 if has_effect(player,crate::reward_cycle_runtime::RESET,base){reset(player,base);return;}
 // Neither object replacement nor transient missing ownership restarts a
 // cycle. Ordinary game Update time continues while its records are restored.
 let phase=if let Ok(mut cycle)=CYCLE.lock(){cycle.advance(dt);cycle.elapsed.map(|_|cycle.weak())}else{return;};
 let owned=has_effect(player,OWNER,base);let stats=native_actor::stats(player);
 for(effect,wanted)in [(WEAK,owned&&phase==Some(true)),(VISUAL,owned&&phase==Some(false))]{
  crate::maintain_status_effect(player,stats,effect,wanted,base);
 }
}
// Locomotion/roll body collisions are not attacks. Restrict this existing
// native applicability filter to our own owner, never other on-hit effects.
pub unsafe fn body_contact(node:usize,descriptor:usize)->bool{
 if node==0||descriptor==0{return false;}
 let row=read_ptr(node);if row==0{return false;}
 ptr::read_unaligned((row+0x156)as*const u16)==2103&&read_i32(row+0x12c)==PULSE&&read_i32(descriptor+0x38)==1
  &&matches!(read_i32(descriptor+0x34),300..=307|310..=313|322|550|551|560|561)
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn first_notification_starts_one_fixed_five_plus_five_second_cycle(){
  let mut c=Cycle::new();assert!(!c.weak());assert!(c.hit());
  for _ in 0..4{c.advance(1.);assert!(!c.weak());assert!(!c.hit());}
  c.advance(1.);assert!(c.weak());assert!(!c.hit());
  for _ in 0..4{c.advance(1.);assert!(c.weak());assert!(!c.hit());}
  c.advance(1.);assert!(!c.weak());assert!(c.hit());assert_eq!(c.elapsed,Some(0.));
 }
 #[test]fn empty_swings_and_idle_time_do_not_start_or_automatically_repeat_cycles(){
  let mut c=Cycle::new();for _ in 0..200{c.advance(1.);assert_eq!(c.elapsed,None);}
  c.hit();for _ in 0..10{c.advance(1.);}for _ in 0..100{c.advance(1.);assert_eq!(c.elapsed,None);}
  assert!(c.hit());
 }
 #[test]fn multiple_targets_and_invalid_update_time_cannot_extend_or_skip_phases(){
  let mut c=Cycle::new();c.hit();
  for dt in [0.,-1.,f32::NAN,f32::INFINITY,1.1]{c.advance(dt);assert_eq!(c.elapsed,Some(0.));}
  for step in 1..=80{c.advance(0.125);for hit in 0..30{assert_eq!(c.hit(),step==80&&hit==0);}assert_eq!(c.elapsed,Some(if step==80{0.}else{f64::from(step)*0.125}));}
  // The final contact above starts the next cycle exactly at ten seconds.
  assert_eq!(c.elapsed,Some(0.));
 }
}
#[cfg(test)]#[path="momentum_native_tests.rs"]mod native_tests;
