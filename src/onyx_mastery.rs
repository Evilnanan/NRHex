//! Onyx mastery uses native weapon-art provenance and the accepted action fee.
//! Motion resources import the existing player Meteorite clips; no spell slot.
use crate::{has_effect,read_i32,native_actor};
use std::{cell::Cell,sync::atomic::{AtomicBool,Ordering}};
pub const OWNER:i32=95005300;
pub const RECEIPT:i32=OWNER+1;
pub const FINISH:i32=6955300;
pub const STARTED:i32=6955301;
pub const LOT:i32=95553000;
pub const BEHAVIOR:i32=95005390;
pub const JUDGE:i32=3998;
pub const QUERY:i32=990053;
pub const FP_QUERY:i32=990054;
#[cfg(not(feature="mmv-compat"))]mod data{include!("onyx_dash_data.rs");}
#[cfg(feature="mmv-compat")]mod data{include!("onyx_dash_mmv_data.rs");}
thread_local!{static ACTION:Cell<(usize,i32)>=const{Cell::new((0,-1))};}
static NETWORK_OWNED:AtomicBool=AtomicBool::new(false);
static GIVEN:AtomicBool=AtomicBool::new(false);
pub fn weapon(id:i32)->bool{id/10000==801}
pub unsafe fn owned(actor:usize,base:usize)->bool{actor!=0&&has_effect(actor,OWNER,base)}
pub unsafe fn selected(actor:usize,w:crate::arts::Weapon,base:usize)->bool{weapon(w.base)&&owned(actor,base)}
unsafe fn ready(actor:usize)->bool{let stats=native_actor::stats(actor);stats!=0&&!native_actor::incapacitated(actor,stats)}
unsafe fn hand_owned(actor:usize,hand:i32,base:usize)->bool{
 matches!(hand,0|1)&&owned(actor,base)&&crate::arts_hand_weapon(actor,hand,base).is_some_and(|w|weapon(w.base))
}
/// Called before native one-shot/loop updates. Never masks native cancellation.
pub unsafe fn query(actor:usize,phase:i32,art:i32,hand:i32,base:usize)->i32{
 ACTION.with(|a|a.set((actor,if art==241{hand}else{-1})));
 if phase!=1||art!=241||!ready(actor){return 0;}
 // Loss of ownership/weapon during a channel must end, not leave a live loop.
 i32::from(!hand_owned(actor,hand,base)||has_effect(actor,FINISH,base))
}
/// Called only by the patched loop's FP query, never by the startup check.
pub unsafe fn free_channel(actor:usize,base:usize)->bool{
 let (source,hand)=ACTION.with(Cell::get);
 source==actor&&hand_owned(actor,hand,base)&&ready(actor)
}
pub unsafe fn may_loop(actor:usize,base:usize)->bool{
 let (source,hand)=ACTION.with(Cell::get);
 source==actor&&hand_owned(actor,hand,base)&&ready(actor)&&has_effect(actor,STARTED,base)
}
pub unsafe fn behavior(actor:usize,judge:i32,source:i32,base:usize)->Option<i32>{
 if judge!=JUDGE||!owned(actor,base){return None;}
 crate::arts_source_weapon(actor,source,base).filter(|w|weapon(w.base)).map(|_|BEHAVIOR)
}
fn discounted(cost:i32,dash:bool)->i32{if cost<=0||dash{0}else{cost/2+cost%2}}
pub unsafe fn cost(context:usize,actor:usize,cost:i32,base:usize)->i32{
 if cost<=0||read_i32(context+0xc)>=0||read_i32(context+8)<0||!owned(actor,base){return cost;}
 let Some(w)=crate::arts_source_weapon(actor,read_i32(context+0x38),base)else{return cost;};
 if !weapon(w.base){return cost;}
 discounted(cost,data::DASH_BEHAVIORS.binary_search(&read_i32(context+8)).is_ok())
}
pub fn reset(){NETWORK_OWNED.store(false,Ordering::Release);GIVEN.store(false,Ordering::Release);ACTION.with(|a|a.set((0,-1)));}
pub unsafe fn update(actor:usize,base:usize){
 let owns=owned(actor,base);
 if NETWORK_OWNED.swap(owns,Ordering::AcqRel)!=owns{
  // Existing randomized Onyx instances must immediately publish the effective
  // native art when mastery is acquired/lost, before the next art use.
  if let Some(state)=crate::ARTS.get(){if let Ok(mut s)=state.lock(){s.invalidate_network();}}
 }
 if !owns||GIVEN.load(Ordering::Acquire){return;}
 if has_effect(actor,RECEIPT,base){GIVEN.store(true,Ordering::Release);return;}
 let stats=native_actor::stats(actor);
 if stats==0||native_actor::incapacitated(actor,stats)||!crate::spatial_ready(actor){return;}
 if crate::CALLBACK_LOCK.swap(true,Ordering::AcqRel){return;}
 let created=crate::spawn_personal_lot_kind(actor,base,LOT,None,1);
 crate::CALLBACK_LOCK.store(false,Ordering::Release);
 if created>0{GIVEN.store(true,Ordering::Release);crate::wraith::set(actor,RECEIPT,true,base);}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn only_onyx_variants_receive_mastery(){for id in [8010000,8010001,8010025,8010100]{assert!(weapon(id));}for id in [-1,0,8009999,8020000,80100000]{assert!(!weapon(id));}}
 #[test]fn half_cost_rounds_up_and_dash_is_free(){for cost in 1..10000{assert_eq!(discounted(cost,false),(cost+1)/2);assert_eq!(discounted(cost,true),0);}assert_eq!(discounted(i32::MAX,false),1073741824);}
 #[test]fn target_dash_map_is_sorted_and_excludes_stationary_attacks(){assert!(data::DASH_BEHAVIORS.windows(2).all(|a|a[0]<a[1]));for id in [100400120,100400125]{assert!(data::DASH_BEHAVIORS.contains(&id));}for id in [100400000,100400200,300805901,BEHAVIOR]{assert!(!data::DASH_BEHAVIORS.contains(&id));}}
}
