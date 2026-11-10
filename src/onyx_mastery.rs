//! Onyx mastery uses native weapon-art provenance and the accepted action fee.
//! Motion resources retain the native planted sword pose; no spell slot.
use crate::{has_effect,read_i32,read_ptr,native_actor};
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
pub const START_END_QUERY:i32=990056;
pub const DASH_QUERY:i32=990057;
pub const METEOR_FP:i16=10;
pub const ARTS_FEE_RVA:usize=0x6ce5a0;
pub const ARTS_FEE_PATTERN:&str="40 53 48 83 EC 20 4C 8B 49 08 4D 85 C0";
pub const FP_ADD_RVA:usize=0x4261c0;
pub const FP_ADD_PATTERN:&str="03 91 50 01 00 00 E9 85 1C 00 00";
#[cfg(not(feature="mmv-compat"))]mod data{include!("onyx_dash_data.rs");}
#[cfg(feature="mmv-compat")]mod data{include!("onyx_dash_mmv_data.rs");}
thread_local!{static ACTION:Cell<(usize,i32,bool)>=const{Cell::new((0,-1,false))};}
static NETWORK_OWNED:AtomicBool=AtomicBool::new(false);
static GIVEN:AtomicBool=AtomicBool::new(false);
pub fn weapon(id:i32)->bool{id/10000==801}
pub unsafe fn owned(actor:usize,base:usize)->bool{actor!=0&&has_effect(actor,OWNER,base)}
pub unsafe fn selected(actor:usize,w:crate::arts::Weapon,base:usize)->bool{weapon(w.base)&&owned(actor,base)}
unsafe fn ready(actor:usize)->bool{let stats=native_actor::stats(actor);stats!=0&&!native_actor::incapacitated(actor,stats)}
/// PlayerIns +250 resolves the action's source directly to its equipped ID.
/// Unlike the randomized-art reader this also accepts static weapons without
/// an instance record and reinforced IDs absent from the default-art catalog.
unsafe fn source_weapon(actor:usize,source:i32)->bool{
 if actor==0||!(-6..=15).contains(&source){return false;}
 let vt=read_ptr(actor);if vt==0{return false;}let address=read_ptr(vt+0x250);if address==0{return false;}
 let get:unsafe extern "system" fn(usize,i32)->i32=std::mem::transmute(address);
 weapon(get(actor,source))
}
unsafe fn hand_owned(actor:usize,hand:i32,base:usize)->bool{
 // c_SwordArtsHand is HAND_LEFT=0 / HAND_RIGHT=1, unlike the bartender's
 // RELIC_WEAPONCHANGE_ENCHANT variable, which uses the opposite numbering.
 matches!(hand,0|1)&&owned(actor,base)&&source_weapon(actor,if hand==0{-2}else{-1})
}
/// The HKS caller additionally requires ATTACK_RIGHT_HEAVYDASH and this clip's
/// per-frame TAE flag 28. No wall-time timer or movement state survives a cancel.
pub unsafe fn dash_ready(actor:usize,base:usize)->bool{
 if !crate::is_local(actor,base)||!ready(actor){return false;}
 let equipment=read_ptr(actor+0x670);
 equipment!=0&&read_i32(equipment)==1&&hand_owned(actor,1,base)&&crate::parry::grounded(actor)
}
pub unsafe fn fp_exception(actor:usize,hand:Option<i32>,base:usize)->bool{
 crate::is_local(actor,base)&&owned(actor,base)&&has_effect(actor,crate::arts::OWNER,base)
  &&hand.is_none_or(|hand|hand_owned(actor,hand,base))
}
/// Called before native one-shot/loop updates. Never masks native cancellation.
pub unsafe fn query(actor:usize,phase:i32,art:i32,hand:i32,startup_paid:bool,base:usize)->i32{
 ACTION.with(|a|a.set((actor,if art==241{hand}else{-1},startup_paid)));
 if phase!=1||art!=241||!ready(actor){return 0;}
 // Loss of ownership/weapon during a channel must end, not leave a live loop.
 i32::from(!hand_owned(actor,hand,base)||has_effect(actor,FINISH,base))
}
/// Called only by the patched loop's FP query, never by the startup check.
pub unsafe fn free_channel(actor:usize,base:usize)->bool{
 let (source,hand,_)=ACTION.with(Cell::get);
 source==actor&&hand_owned(actor,hand,base)&&ready(actor)
}
/// A private ten-point art fee passed through the native art multiplier and
/// rounding routine. Never edit the shared SwordArtsParam or its 27-FP startup.
unsafe fn meteor_fee(actor:usize,base:usize)->i32{
 let _scope=crate::onyx_fp::Guard::enter(actor,None,base);
 let mut row=[0u8;0x18];row[0x14..0x16].copy_from_slice(&METEOR_FP.to_le_bytes());
 let reference=[0usize,row.as_ptr()as usize];
 let fee:unsafe extern "system" fn(usize,i32,usize)->i32=std::mem::transmute(base+ARTS_FEE_RVA);
 fee(reference.as_ptr()as usize,0,read_ptr(actor+0x190)).max(0)
}
pub unsafe fn channel_available(actor:usize,base:usize)->Option<bool>{
 if !free_channel(actor,base){return None;}
 Some(read_i32(native_actor::stats(actor)+0x150)>=meteor_fee(actor,base))
}
/// Called once per actual TAE bullet submission, not the repeated behavior
/// lookups or the meteor's child/explosion chain. Native creation failure costs
/// nothing; successful creation uses the normal FP setter and immunity checks.
pub unsafe fn emit(actor:usize,behavior:i32,descriptor:usize,source:i32,error:usize,base:usize,original:impl FnOnce()->u8)->u8{
 if behavior!=BEHAVIOR||descriptor==0||!crate::is_local(actor,base){return original();}
 if crate::ptr::read((descriptor+0x10)as*const u8)==0{return original();}
 let stats=native_actor::stats(actor);
 if stats==0||!ready(actor)||!owned(actor,base)||!source_weapon(actor,source){if error!=0{crate::ptr::write_unaligned(error as*mut i32,2);}return 0;}
 let fee=meteor_fee(actor,base);
 if read_i32(stats+0x150)<fee{if error!=0{crate::ptr::write_unaligned(error as*mut i32,2);}return 0;}
 let result=original();
 if result!=0&&fee>0{
  let add:unsafe extern "system" fn(usize,i32)=std::mem::transmute(base+FP_ADD_RVA);add(stats,-fee);
 }
 result
}
pub unsafe fn may_loop(actor:usize,base:usize)->bool{
 let (source,hand,paid)=ACTION.with(Cell::get);
 source==actor&&hand_owned(actor,hand,base)&&ready(actor)&&paid
}
/// Scoped to startup HKS queries; native cancellation and held-input branches
/// still run. A paid planted-sword wave may enter channel before recovery.
pub unsafe fn planted(actor:usize,base:usize)->bool{
 may_loop(actor,base)&&has_effect(actor,STARTED,base)
}
pub unsafe fn behavior(actor:usize,judge:i32,source:i32,base:usize)->Option<i32>{
 if judge!=JUDGE||!owned(actor,base){return None;}
 source_weapon(actor,source).then_some(BEHAVIOR)
}
fn discounted(cost:i32,dash:bool,heavy:bool)->i32{if cost<=0||dash{0}else if heavy{cost/2+cost%2}else{cost-cost/4}}
fn action_cost(cost:i32,behavior:i32)->i32{
 discounted(cost,data::DASH_BEHAVIORS.binary_search(&behavior).is_ok(),data::HEAVY_BEHAVIORS.binary_search(&behavior).is_ok())
}
/// The native melee descriptor already resolves the weapon that created the
/// hitbox. Do not substitute the currently selected (possibly opposite) hand.
pub unsafe fn melee_cost(actor:usize,weapon_id:i32,behavior:i32,cost:i32,base:usize)->i32{
 if cost<=0||behavior<0||!weapon(weapon_id)||!owned(actor,base){return cost;}
 action_cost(cost,behavior)
}
pub unsafe fn cost(context:usize,actor:usize,cost:i32,base:usize)->i32{
 if cost<=0||read_i32(context+0xc)>=0||read_i32(context+8)<0||!owned(actor,base){return cost;}
 if !source_weapon(actor,read_i32(context+0x38)){return cost;}
 action_cost(cost,read_i32(context+8))
}
pub fn reset(){NETWORK_OWNED.store(false,Ordering::Release);GIVEN.store(false,Ordering::Release);ACTION.with(|a|a.set((0,-1,false)));}
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
 #[test]fn action_discounts_round_up_and_dash_takes_priority(){for cost in 1..10000{assert_eq!(discounted(cost,false,false),(cost*3+3)/4);assert_eq!(discounted(cost,false,true),(cost+1)/2);assert_eq!(discounted(cost,true,false),0);assert_eq!(discounted(cost,true,true),0);}assert_eq!(discounted(i32::MAX,false,true),1073741824);assert_eq!(discounted(i32::MAX,false,false),1610612736);}
 #[test]fn heavy_and_charged_variants_keep_half_cost(){assert!(data::HEAVY_BEHAVIORS.windows(2).all(|a|a[0]<a[1]));for id in [100400100,100400105,100400110,100400115,100101116,100400300,100400305,100400310,100400315,100101316,100400160,100400360]{assert_eq!(action_cost(35,id),18,"heavy {id}");}for id in [100400000,100400200,100400180,100400380,300805901,BEHAVIOR]{assert_eq!(action_cost(35,id),27,"ordinary/art {id}");}for id in data::DASH_BEHAVIORS{assert_eq!(action_cost(35,*id),0);assert!(!data::HEAVY_BEHAVIORS.contains(id));}}
 #[test]fn target_dash_map_is_sorted_and_excludes_stationary_attacks(){assert!(data::DASH_BEHAVIORS.windows(2).all(|a|a[0]<a[1]));for id in [100400120,100400125]{assert!(data::DASH_BEHAVIORS.contains(&id));}for id in [100400000,100400200,300805901,BEHAVIOR]{assert!(!data::DASH_BEHAVIORS.contains(&id));}}
}
#[cfg(test)]#[path="onyx_fp_tests.rs"]mod fp_tests;
#[cfg(test)]#[path="onyx_dash_tests.rs"]mod dash_tests;
