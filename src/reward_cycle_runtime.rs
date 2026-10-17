//! Local saved exposure records. Native reset notices distinguish a new run
//! from revival, pointer replacement, or a temporarily missing expedition gate.
use crate::{has_effect,read_ptr,reward_cycle};
use std::sync::Mutex;
pub const FIRST:i32=95002800;
pub const RESET:i32=95002825;
pub const fn marker(index:usize)->i32{if index<25{FIRST+index as i32}else{95002829+(index-25)as i32}}
static HISTORY:Mutex<History>=Mutex::new(History::new());

struct History { seen:Option<u32>,dirty:bool,wait:f32,resetting:bool }
impl History{
 const fn new()->Self{Self{seen:None,dirty:false,wait:0.,resetting:false}}
 fn due(&mut self,reset:bool,dt:f32)->bool{
  if dt.is_finite()&&dt>0.{self.wait=(self.wait-dt).max(0.);}
  reset||self.seen.is_none()||self.resetting||(self.dirty&&self.wait<=0.)
 }
 fn observe(&mut self,native:u32,reset:bool)->Option<u32>{
  if reset{self.seen=Some(0);self.dirty=true;self.resetting=true;}
  else{self.resetting=false;}
  let wanted=*self.seen.get_or_insert(native&reward_cycle::ALL);
  self.dirty=native!=wanted;self.wait=0.25;
  self.dirty.then_some(wanted)
 }
 fn seen(&self)->Option<u32>{if self.resetting{None}else{self.seen}}
 fn commit(&mut self,seen:u32){self.seen=Some(seen&reward_cycle::ALL);self.dirty=true;self.wait=0.;}
}

pub unsafe fn update(player:usize,base:usize,dt:f32){
 let manager=read_ptr(player+0x190);if manager==0{return;}
 let reset=has_effect(player,RESET,base);
 if reset{crate::reward_reroll::reset();}
 let due=HISTORY.lock().map(|mut s|s.due(reset,dt)).unwrap_or(false);if !due{return;}
 let native=(0..crate::hex_pool::HEX_COUNT).fold(0,|m,i|m|if has_effect(player,marker(i),base){1<<i}else{0});
 let wanted=HISTORY.lock().ok().and_then(|mut s|s.observe(native,reset));
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);
 if let Some(wanted)=wanted{
  for i in 0..crate::hex_pool::HEX_COUNT{
   if (native^wanted)&(1<<i)==0{continue;}
   if wanted&(1<<i)!=0{apply(player,marker(i),0);}else{remove(manager,marker(i));}
  }
 }
 if reset{remove(manager,RESET);}
}
pub unsafe fn seen(player:usize,base:usize)->Option<u32>{
 update(player,base,0.);HISTORY.lock().ok().and_then(|s|s.seen())
}
pub unsafe fn commit(player:usize,base:usize,seen:u32){
 if let Ok(mut s)=HISTORY.lock(){s.commit(seen);}
 update(player,base,0.);
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn appended_history_markers_do_not_alias_reset_or_reroll_records(){
  let ids:Vec<_>=(0..crate::hex_pool::HEX_COUNT).map(marker).collect();
  assert_eq!(&ids[..25],&(FIRST..FIRST+25).collect::<Vec<_>>());
  assert_eq!(ids[25],95002829);assert_eq!(ids[26],95002830);assert!(!ids.contains(&RESET));
  assert!(ids.iter().all(|id|!(crate::reward_reroll::FIRST..crate::reward_reroll::FIRST+3).contains(id)));
 }
 #[test]fn native_records_seed_after_runtime_restart_and_survive_revival(){
  let mut h=History::new();assert_eq!(h.observe(7,false),None);assert_eq!(h.seen(),Some(7));
  h.commit(63);assert_eq!(h.observe(7,false),Some(63));assert_eq!(h.seen(),Some(63));
  assert_eq!(h.observe(63,false),None);assert!(!h.dirty);
  let mut restarted=History::new();restarted.observe(63,false);assert_eq!(restarted.seen(),Some(63));
 }
 #[test]fn deferred_partial_writes_never_erase_committed_options(){
  let mut h=History::new();h.observe(0,false);h.commit(7);
  for native in [0,1,3,7]{h.observe(native,false);assert_eq!(h.seen(),Some(7));}
  assert!(!h.dirty);
 }
 #[test]fn rollover_clears_old_records_and_preserves_new_round_until_native_ack(){
  let mut h=History::new();h.observe(reward_cycle::ALL,false);h.commit(3);
  for native in [reward_cycle::ALL,255,7]{assert_eq!(h.observe(native,false),Some(3));assert_eq!(h.seen(),Some(3));}
  assert_eq!(h.observe(3,false),None);
 }
 #[test]fn only_explicit_native_reset_starts_a_new_expedition(){
  let mut h=History::new();h.observe(7,false);assert_eq!(h.observe(7,true),Some(0));assert_eq!(h.seen(),None);
  assert_eq!(h.observe(0,false),None);assert_eq!(h.seen(),Some(0));h.commit(3);assert_eq!(h.seen(),Some(3));
 }
}
