//! Charged-hit notifications emit native charm mist. Native saved cooldown
//! survives death/reload; a game-time reservation closes deferred/reentrant gaps.
use crate::{has_effect,is_local,native_actor};
use std::sync::Mutex;
pub const OWNER:i32=95002700;
pub const COOLDOWN:i32=OWNER+3;
pub const PULSE:i32=OWNER+6;
pub const MIST:i32=OWNER+7;
pub const READY:i32=OWNER+8;
struct Gate{remaining:f64}
impl Gate{
 const fn new()->Self{Self{remaining:0.}}
 fn claim(&mut self)->bool{if self.remaining>0.{return false;}self.remaining=30.;true}
 fn advance(&mut self,dt:f32){if dt.is_finite()&&dt>0.&&dt<=1.{self.remaining=(self.remaining-f64::from(dt)).max(0.);}}
}
static GATE:Mutex<Gate>=Mutex::new(Gate::new());
pub unsafe fn reset(player:usize,base:usize){
 if let Ok(mut gate)=GATE.lock(){*gate=Gate::new();}
 let manager=crate::read_ptr(player+0x190);
 if manager!=0{
  let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);
  for id in [COOLDOWN,MIST,READY]{if has_effect(player,id,base){remove(manager,id);}}
 }
}
pub unsafe fn ready(player:usize,base:usize)->bool{
 if player==0||!is_local(player,base)||!has_effect(player,OWNER,base)||has_effect(player,crate::reward_cycle_runtime::RESET,base){return false;}
 let stats=native_actor::stats(player);
 stats!=0&&!native_actor::incapacitated(player,stats)&&!has_effect(player,COOLDOWN,base)&&GATE.lock().map(|gate|gate.remaining<=0.).unwrap_or(false)
}
pub unsafe fn update(player:usize,base:usize,dt:f32){
 if let Ok(mut gate)=GATE.lock(){gate.advance(dt);}
 if player==0||!is_local(player,base){return;}
 let stats=native_actor::stats(player);if stats==0{return;}
 crate::maintain_status_effect(player,stats,READY,ready(player,base),base);
}
pub unsafe fn pulse(player:usize,effect:i32,base:usize)->bool{
 if effect!=PULSE{return false;}
 // Never install/replay this private notification on a remote replica.
 if player==0||!is_local(player,base)||!has_effect(player,OWNER,base)||has_effect(player,crate::reward_cycle_runtime::RESET,base){return true;}
 let stats=native_actor::stats(player);
 if stats==0||native_actor::incapacitated(player,stats)||has_effect(player,COOLDOWN,base){return true;}
 if !GATE.lock().map(|mut gate|gate.claim()).unwrap_or(false){return true;}
 crate::maintain_status_effect(player,stats,READY,false,base);
 // Release the reservation lock before either potentially reentrant API.
 // Native state275 emits behavior863030000 at dummy80 through its first-frame
 // update, retaining the original projectile/target/network behavior.
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);
 if apply(player,MIST,0)==0{
  if let Ok(mut gate)=GATE.lock(){*gate=Gate::new();}
  return true;
 }
 apply(player,COOLDOWN,0);
 true
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn fixed_thirty_second_reservation_rejects_multitarget_without_refresh(){
  let mut g=Gate::new();assert!(g.claim());
  for _ in 0..59{g.advance(0.5);for _ in 0..20{assert!(!g.claim());}}
  g.advance(0.499);assert!(!g.claim());g.advance(0.0011);assert!(g.claim());
 }
 #[test]fn invalid_or_zero_delta_cannot_skip_the_game_time_cooldown(){
  let mut g=Gate::new();g.claim();for dt in [0.,-1.,f32::NAN,f32::INFINITY,1.01]{g.advance(dt);assert_eq!(g.remaining,30.);}
  for _ in 0..30{g.advance(1.);}assert!(g.claim());
 }
}
#[cfg(test)]#[path="heartstealer_native_tests.rs"]mod native_tests;
