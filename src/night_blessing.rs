//! Native periodic regeneration, followed by backlash until leaving the Tide.
//! Native effect lifetime owns the 15-second timer; grace rearms one use.
use crate::{has_effect,native_actor,read_i32,read_ptr};
use std::sync::{Mutex,OnceLock};
pub const OWNER:i32=95002500;
pub const HEAL_FIRST:i32=OWNER+1;
pub const HEAL_SECOND:i32=OWNER+2;
pub const ATTACK:i32=OWNER+3;
pub const VISUAL:i32=OWNER+4;
pub const SPENT:i32=OWNER+5;
pub const GRACE:i32=OWNER+6;
const LEGACY_GESTURE:i32=OWNER+7;
pub const BACKLASH_FIRST:i32=OWNER+8;
pub const BACKLASH_SECOND:i32=OWNER+9;
pub const DRAIN_FIRST:i32=OWNER+10;
pub const DRAIN_SECOND:i32=OWNER+11;
static STATE:OnceLock<Mutex<State>>=OnceLock::new();
#[derive(Default)]
struct State{object:(usize,usize),pending:bool,grace_clean_frames:u8}
#[derive(Clone,Copy,Default)]
struct Input{object:(usize,usize),owned:bool,alive:bool,rain:Option<i32>,hp:i32,max_hp:i32,spent:bool,grace:bool,heal:bool,backlash:Option<i32>,phase_extra:bool}
#[derive(Default,Debug,PartialEq)]
struct Output{start:Option<i32>,attack:bool,visual:bool,drain:Option<i32>,clear_phase:bool,clear_spent:bool,clear_backlash:bool,consume_grace:bool}
impl State{
 fn tick(&mut self,i:Input)->Output{
  if self.object!=i.object{*self=Self{object:i.object,..Self::default()};}
  if !i.grace{self.grace_clean_frames=0;}
  if !i.owned||i.grace{
   self.pending=false;
   let clean=!i.spent&&!i.heal&&i.backlash.is_none()&&!i.phase_extra;
   self.grace_clean_frames=if i.grace&&clean{self.grace_clean_frames.saturating_add(1)}else{0};
   return Output{attack:i.owned&&i.alive&&i.rain.is_some(),clear_phase:true,clear_spent:i.grace,clear_backlash:true,consume_grace:i.grace&&self.grace_clean_frames>=2,..Output::default()};
  }
  if i.spent{self.pending=false;}
  if !i.alive{return Output{clear_phase:true,clear_backlash:true,..Output::default()};}
  let start=if !i.spent&&!self.pending&&!i.heal&&i.backlash.is_none()&&i.max_hp>0&&i.hp>0&&(i.hp as i64)*5<(i.max_hp as i64){i.rain}else{None};
  if start.is_some(){self.pending=true;}
  let drain=if i.rain.is_some(){match i.backlash{Some(BACKLASH_FIRST)=>Some(DRAIN_FIRST),Some(BACKLASH_SECOND)=>Some(DRAIN_SECOND),_=>None}}else{None};
  Output{start,attack:i.rain.is_some(),visual:i.heal||start.is_some(),drain,clear_backlash:i.rain.is_none(),..Output::default()}
 }
}
// Only damaging rows establish actual exposure. Helper rows may linger after
// leaving rain; the prolonged-exposure upgrades never raise our base rate.
fn rain_tier(mut has:impl FnMut(i32)->bool)->Option<i32>{
 if [99811,99876,99877].into_iter().any(&mut has){Some(HEAL_SECOND)}else if [99810,99871,99872].into_iter().any(has){Some(HEAL_FIRST)}else{None}
}
pub unsafe fn update(player:usize,base:usize,_dt:f32){
 let stats=native_actor::stats(player);if stats==0{return;}
 let owned=has_effect(player,OWNER,base);let grace=has_effect(player,GRACE,base);if !owned&&!grace&&STATE.get().is_none(){return;}
 let i=Input{object:(player,stats),owned,alive:!native_actor::incapacitated(player,stats),rain:rain_tier(|id|has_effect(player,id,base)),hp:read_i32(stats+0x140),max_hp:read_i32(stats+0x144),spent:has_effect(player,SPENT,base),grace,heal:has_effect(player,HEAL_FIRST,base)||has_effect(player,HEAL_SECOND,base),backlash:if has_effect(player,BACKLASH_SECOND,base){Some(BACKLASH_SECOND)}else if has_effect(player,BACKLASH_FIRST,base){Some(BACKLASH_FIRST)}else{None},phase_extra:has_effect(player,VISUAL,base)||has_effect(player,DRAIN_FIRST,base)||has_effect(player,DRAIN_SECOND,base)};
 let o={let Ok(mut s)=STATE.get_or_init(||Mutex::new(State::default())).lock()else{return;};s.tick(i)};
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);
 let manager=read_ptr(player+0x190);if manager==0{return;}
 for id in [HEAL_FIRST,HEAL_SECOND,LEGACY_GESTURE,SPENT,BACKLASH_FIRST,BACKLASH_SECOND]{
  let clear=match id{HEAL_FIRST|HEAL_SECOND=>o.clear_phase,SPENT=>o.clear_spent,BACKLASH_FIRST|BACKLASH_SECOND=>o.clear_backlash,_=>true};
  if clear&&has_effect(player,id,base){remove(manager,id);}
 }
 if o.consume_grace{remove(manager,GRACE);}
 if let Some(heal)=o.start{
  // Reserve once while native application is queued. The saved receipt
  // survives revival; healing or re-entering rain cannot rearm it.
  apply(player,SPENT,0);apply(player,heal,0);
 }
 for (id,wanted) in [(ATTACK,o.attack),(VISUAL,o.visual),(DRAIN_FIRST,o.drain==Some(DRAIN_FIRST)),(DRAIN_SECOND,o.drain==Some(DRAIN_SECOND))]{
  let present=has_effect(player,id,base);
  if wanted&&!present{apply(player,id,0);}else if !wanted&&present{remove(manager,id);}
 }
}
#[cfg(test)]mod tests{
 use super::*;
 fn ready()->Input{Input{object:(1,2),owned:true,alive:true,rain:Some(HEAL_FIRST),hp:199,max_hp:1000,..Input::default()}}
 #[test]fn attack_is_immediate_in_rain_at_any_hp_and_stops_on_exit(){
  let mut s=State::default();let mut i=ready();i.hp=1000;let o=s.tick(i);assert!(o.attack&&o.start.is_none());i.rain=None;assert!(!s.tick(i).attack);i.rain=Some(HEAL_FIRST);assert!(s.tick(i).attack);
 }
 #[test]fn strict_twenty_percent_threshold_requires_rain_and_living_hp(){
  for (hp,max_hp,rain,wanted) in [(200,1000,true,false),(199,1000,true,true),(1,5,true,false),(1,6,true,true),(0,1000,true,false),(1,0,true,false),(199,1000,false,false),(i32::MAX,i32::MAX,true,false)]{
   let mut s=State::default();let i=Input{hp,max_hp,rain:rain.then_some(HEAL_FIRST),..ready()};assert_eq!(s.tick(i).start.is_some(),wanted);
  }
 }
 #[test]fn queued_application_cannot_retrigger_then_receipt_blocks_healed_and_low_again(){
  let mut s=State::default();let mut i=ready();assert_eq!(s.tick(i).start,Some(HEAL_FIRST));for _ in 0..10{assert_eq!(s.tick(i).start,None);}
  i.spent=true;i.heal=true;assert!(s.tick(i).visual);i.heal=false;i.hp=1000;assert_eq!(s.tick(i).start,None);i.hp=1;assert_eq!(s.tick(i).start,None);
 }
 #[test]fn leaving_rain_keeps_healing_but_ends_backlash_and_never_restarts_it(){
  let mut s=State::default();let mut i=ready();i.spent=true;i.heal=true;i.rain=None;let o=s.tick(i);assert!(o.visual&&!o.clear_phase&&o.drain.is_none());
  i.heal=false;i.backlash=Some(BACKLASH_FIRST);let o=s.tick(i);assert!(o.clear_backlash&&o.drain.is_none());i.backlash=None;i.rain=Some(HEAL_FIRST);let o=s.tick(i);assert!(o.attack&&o.start.is_none()&&o.drain.is_none());
 }
 #[test]fn backlash_has_no_timeout_and_uses_the_triggering_base_tier(){
  let mut s=State::default();let mut i=ready();i.spent=true;i.backlash=Some(BACKLASH_SECOND);assert_eq!(s.tick(i).drain,Some(DRAIN_SECOND));
  for _ in 0..10000{assert_eq!(s.tick(i).drain,Some(DRAIN_SECOND));}i.rain=None;assert_eq!(s.tick(i).drain,None);
 }
 #[test]fn grace_clears_receipt_healing_backlash_and_pending_trigger(){
  let mut s=State::default();let mut i=ready();s.tick(i);i.grace=true;i.spent=true;i.heal=true;i.backlash=Some(BACKLASH_FIRST);let o=s.tick(i);assert!(o.clear_phase&&o.clear_spent&&o.clear_backlash&&!o.visual&&o.drain.is_none()&&o.start.is_none());
  i.grace=false;i.spent=false;i.heal=false;i.backlash=None;assert!(s.tick(i).start.is_some());
 }
 #[test]fn death_and_owner_loss_cancel_phases_without_rearming(){
  let mut s=State::default();let mut i=ready();s.tick(i);i.spent=true;i.alive=false;let o=s.tick(i);assert!(o.clear_phase&&!o.clear_spent&&!o.attack);i.alive=true;assert!(s.tick(i).start.is_none());
  i.object=(3,4);assert!(s.tick(i).start.is_none());i.owned=false;let o=s.tick(i);assert!(o.clear_phase&&!o.clear_spent&&o.clear_backlash);
 }
 #[test]fn persistent_grace_waits_for_deferred_chain_cleanup_before_consumption(){
  let mut s=State::default();let mut i=ready();i.grace=true;i.spent=true;i.heal=true;
  for _ in 0..10{assert!(!s.tick(i).consume_grace);}
  i.spent=false;i.heal=false;assert!(!s.tick(i).consume_grace);
  i.backlash=Some(BACKLASH_FIRST);assert!(!s.tick(i).consume_grace);
  i.backlash=None;i.phase_extra=true;assert!(!s.tick(i).consume_grace);
  i.phase_extra=false;assert!(!s.tick(i).consume_grace);assert!(s.tick(i).consume_grace);
 }
 #[test]fn native_prolonged_rain_and_helpers_do_not_raise_base_rate(){
  for id in [99810,99871,99872]{assert_eq!(rain_tier(|v|v==id),Some(HEAL_FIRST));}
  for id in [99811,99876,99877]{assert_eq!(rain_tier(|v|v==id),Some(HEAL_SECOND));}
  for id in [99873,99874,99878,99879]{assert_eq!(rain_tier(|v|v==id),None);}
 }
}
