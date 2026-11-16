//! Latched Night Tide protection. Only the local holder's rain contribution is
//! replaced; the native periodic pipeline still owns heal-before-damage and death.
use crate::{has_effect,is_local,native_actor,read_i32,read_ptr,ACTIVE,BASE};
use std::{cell::Cell,ptr,sync::{Mutex,OnceLock,atomic::{AtomicPtr,Ordering}}};
pub const OWNER:i32=95002500;
pub const SPENT:i32=OWNER+5;
pub const GRACE:i32=OWNER+6;
const VISUAL:i32=OWNER+4;
const STATUS:i32=OWNER+3;
pub const COLLECT_RVA:usize=0x506ec0;
pub const COLLECT_PATTERN:&str="48 8B C4 48 89 58 08 48 89 70 18 48 89 78 20 55 48 8D 68 B1 48 81 EC D0 00 00 00";
pub const RATE_RVA:usize=0x510100;
pub const RATE_PATTERN:&str="48 8B 01 48 85 C0 74 0A F3 0F 10 88 9C 00 00 00 EB 03 0F 57 C9 F3 0F 10 41 64";
pub const POINT_RVA:usize=0x510130;
pub const POINT_PATTERN:&str="48 8B 01 48 85 C0 74 0D 66 0F 6E 88 A0 00 00 00 0F 5B C9";
pub static COLLECT_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static RATE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static POINT_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static STATE:OnceLock<Mutex<State>>=OnceLock::new();
thread_local!{static FILTER_RAIN:Cell<bool>=const{Cell::new(false)};}
struct Scope(bool);
impl Scope{fn enter(value:bool)->Self{Self(FILTER_RAIN.with(|s|s.replace(value)))}}
impl Drop for Scope{fn drop(&mut self){FILTER_RAIN.with(|s|s.set(self.0));}}
#[derive(Default)]
struct State{active:bool,receipt_seen:bool,elapsed:f64,fraction:f64,due:bool,grace_clean:u8}
impl State{
 fn observe(&mut self,owned:bool,grace:bool,receipt:bool,rain:bool,alive:bool,hp:i32,max_hp:i32,dt:f32)->bool{
  if !owned||grace||(self.receipt_seen&&!receipt){*self=Self::default();return false;}
  self.receipt_seen|=receipt;
  self.active|=receipt||(rain&&alive&&hp>0&&max_hp>0&&i64::from(hp)*5<i64::from(max_hp));
  if self.active&&rain&&alive&&dt.is_finite()&&dt>0.&&dt<=1.{
   self.elapsed+=f64::from(dt);self.fraction+=f64::from(dt);
   if self.fraction>=1.{self.fraction-=1.;self.due=true;}
  }else{self.due=false;}
  self.active
 }
 fn claim(&mut self)->Option<f64>{if self.active&&std::mem::take(&mut self.due){Some(self.elapsed)}else{None}}
}
fn tier(id:i32)->Option<i32>{match id{99810=>Some(15),99811=>Some(30),_=>None}}
fn rain_tier(mut has:impl FnMut(i32)->bool)->Option<i32>{
 // Prolonged-exposure damage always wins, including a transition frame with
 // both old and new rows. It neither activates nor benefits from this hex.
 if [99871,99872,99876,99877].into_iter().any(&mut has){return None;}
 if has(99811){Some(30)}else if has(99810){Some(15)}else{None}
}
fn amounts(hp:i32,max_hp:i32,flat:i32,seconds:f64)->(i32,i32){
 let b=f64::from(max_hp.max(0))*0.02+f64::from(flat);
 let extra=(f64::from(hp)-f64::from(max_hp)*0.2).max(0.)*0.1;
 // Native periodic HP conversion truncates toward zero. Clamp before integer
 // conversion, so very long exposure cannot wrap into healing.
 ((2.*b).floor()as i32,(b*(1.+seconds.max(0.)/30.)+extra).floor()as i32)
}
pub unsafe fn update(player:usize,base:usize,dt:f32){
 let stats=native_actor::stats(player);if stats==0{return;}
 let owned=has_effect(player,OWNER,base);let grace=has_effect(player,GRACE,base);
 if !owned&&!grace&&STATE.get().is_none(){return;}
 let receipt=has_effect(player,SPENT,base);let rain=rain_tier(|id|has_effect(player,id,base)).is_some();
 let alive=!native_actor::incapacitated(player,stats);
 let (active,consume)={let Ok(mut s)=STATE.get_or_init(||Mutex::new(State::default())).lock()else{return;};
  let clean=s.grace_clean;
  let active=s.observe(owned,grace,receipt,rain,alive,read_i32(stats+0x140),read_i32(stats+0x144),dt);
  s.grace_clean=if grace&&!receipt{clean.saturating_add(1)}else{0};(active,s.grace_clean>=2)};
 let manager=read_ptr(player+0x190);if manager==0{return;}
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);
 for offset in [1,2,7,8,9,10,11]{let id=OWNER+offset;if has_effect(player,id,base){remove(manager,id);}}
 if grace&&receipt{remove(manager,SPENT);}
 if consume{remove(manager,GRACE);}
 if active&&!receipt{apply(player,SPENT,0);}
 for (id,wanted) in [(STATUS,active&&alive),(VISUAL,active&&alive&&rain)]{
  let present=has_effect(player,id,base);if wanted&&!present{apply(player,id,0);}else if !wanted&&present{remove(manager,id);}
 }
}
// 431B30 calls 506EC0 with (manager,maxHP,source*,attribute*,deltas*,flag).
// delta[1] is applied by 426130 at 431C47 BEFORE delta[0] at 431E01.
// The getters below are scoped to this collector invocation; PARAM rows and
// native effect lists are never mutated, and recursive/remote calls restore TLS.
pub unsafe extern "system" fn collect(manager:usize,max_hp:i32,source:usize,attribute:usize,out:usize,flag:u8){
 let original:unsafe extern "system" fn(usize,i32,usize,usize,usize,u8)=std::mem::transmute(COLLECT_ORIGINAL.load(Ordering::Acquire));
 let base=BASE.load(Ordering::Acquire);
 let mut filtered=false;let mut payment=None;
 if ACTIVE.load(Ordering::Acquire)&&base!=0&&manager!=0{
  let player=read_ptr(manager+0x10);
  if player!=0&&is_local(player,base)&&has_effect(player,OWNER,base)&&!has_effect(player,GRACE,base){
   let stats=native_actor::stats(player);
   if stats!=0&&!native_actor::incapacitated(player,stats){
    if let Some(flat)=rain_tier(|id|has_effect(player,id,base)){
     if let Some(state)=STATE.get(){if let Ok(mut s)=state.lock(){
      filtered=s.active;
      if let Some(seconds)=s.claim(){payment=Some(amounts(read_i32(stats+0x140),max_hp,flat,seconds));}
     }}
    }
   }
  }
 }
 let _scope=Scope::enter(filtered);
 original(manager,max_hp,source,attribute,out,flag);
 if let Some((heal,hurt))=payment{
  let deltas=out as *mut i32;
  ptr::write_unaligned(deltas.add(1),ptr::read_unaligned(deltas.add(1)).saturating_add(heal));
  ptr::write_unaligned(deltas,ptr::read_unaligned(deltas).saturating_sub(hurt));
 }
}
unsafe fn suppress(node:usize)->bool{node!=0&&FILTER_RAIN.with(|s|s.get())&&tier(read_i32(node+8)).is_some()}
pub unsafe extern "system" fn rate(node:usize)->f32{
 if suppress(node){return 0.;}
 let original:unsafe extern "system" fn(usize)->f32=std::mem::transmute(RATE_ORIGINAL.load(Ordering::Acquire));original(node)
}
pub unsafe extern "system" fn point(node:usize)->i32{
 if suppress(node){return 0;}
 let original:unsafe extern "system" fn(usize)->i32=std::mem::transmute(POINT_ORIGINAL.load(Ordering::Acquire));original(node)
}
#[cfg(test)]mod tests{
 use super::*;
 fn tick(s:&mut State,hp:i32,rain:bool,receipt:bool,grace:bool,dt:f32)->bool{s.observe(true,grace,receipt,rain,true,hp,1000,dt)}
 #[test]fn prolonged_exposure_blocks_activation_and_pauses_an_existing_latch(){
  for id in [99871,99872,99876,99877]{assert_eq!(rain_tier(|n|n==id||n==99810),None);assert_eq!(rain_tier(|n|n==id||n==99811),None);}
  let mut s=State::default();assert!(!tick(&mut s,1,false,false,false,1.));tick(&mut s,199,true,false,false,0.5);assert!(tick(&mut s,199,false,true,false,1.));assert_eq!(s.elapsed,0.5);assert_eq!(s.claim(),None);
 }
 #[test]fn threshold_latches_and_full_hp_entry_is_vanilla(){let mut s=State::default();assert!(!tick(&mut s,1000,true,false,false,1.));assert!(!tick(&mut s,199,false,false,false,1.));assert!(!tick(&mut s,200,true,false,false,1.));assert!(tick(&mut s,199,true,false,false,1.));assert!(tick(&mut s,1000,true,true,false,1.));assert_eq!(s.elapsed,2.);}
 #[test]fn exposure_pauses_outside_rain_and_on_death_without_rearming(){let mut s=State::default();tick(&mut s,199,true,false,false,0.5);tick(&mut s,900,false,true,false,1.);assert_eq!(s.elapsed,0.5);assert_eq!(s.claim(),None);s.observe(true,false,true,true,false,0,1000,1.);assert_eq!(s.elapsed,0.5);tick(&mut s,199,true,true,false,0.5);assert_eq!(s.claim(),Some(1.));assert_eq!(s.claim(),None);}
 #[test]fn grace_and_expedition_receipt_removal_reset_latch_and_clock(){for grace in [false,true]{let mut s=State::default();tick(&mut s,199,true,false,false,1.);tick(&mut s,199,true,true,false,1.);assert!(!tick(&mut s,199,true,false,grace,1.));assert!(!s.active);assert_eq!(s.elapsed,0.);}}
 #[test]fn native_receipt_restores_latch_without_pointer_based_reset(){let mut s=State::default();assert!(tick(&mut s,900,true,true,false,1.));assert_eq!(s.elapsed,1.);}
 #[test]fn equilibrium_snapshot_excess_and_native_tiers(){for flat in [15,30]{let b=20+flat;assert_eq!(amounts(200,1000,flat,30.),(2*b,2*b));assert_eq!(amounts(800,1000,flat,30.),(2*b,2*b+60));assert_eq!(amounts(199,1000,flat,0.),(2*b,b));assert_eq!(amounts(200,1000,flat,60.),(2*b,3*b));}for id in [99810]{assert_eq!(tier(id),Some(15));}for id in [99811]{assert_eq!(tier(id),Some(30));}for id in [99871,99872,99876,99877,99873,99874,99878,99879,100,7000]{assert_eq!(tier(id),None);}}
 #[test]fn invalid_frame_times_do_not_advance_and_long_exposure_saturates(){let mut s=State::default();for dt in [f32::NAN,f32::INFINITY,-1.,2.]{tick(&mut s,199,true,false,false,dt);}assert_eq!(s.elapsed,0.);assert_eq!(amounts(200,1000,15,1e20).1,i32::MAX);}
 #[test]fn filter_is_rain_only_and_nested_context_restores(){unsafe{let mut node=[0usize;16];let p=node.as_mut_ptr()as usize;ptr::write_unaligned((p+8)as*mut i32,99810);assert!(!suppress(p));{let _a=Scope::enter(true);assert!(suppress(p));{let _b=Scope::enter(false);assert!(!suppress(p));}assert!(suppress(p));ptr::write_unaligned((p+8)as*mut i32,100);assert!(!suppress(p));}assert!(!FILTER_RAIN.with(|s|s.get()));}}
}
#[cfg(test)]
#[path="night_blessing_native_tests.rs"]mod native_tests;
