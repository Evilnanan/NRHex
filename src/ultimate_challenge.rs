use crate::{has_effect,is_local,native_actor,read_i32,read_ptr};
use std::sync::Mutex;
pub const OWNER:i32=95004000;
pub const BUFF:i32=OWNER+1;
pub const RECALCULATE_RVA:usize=0x428bf0;
pub const RECALCULATE_PATTERN:&str="40 53 57 41 57 48 83 EC 40 83 B9 40 01 00 00 00 44 0F B6 FA 48 8B F9 7F 2D";

#[derive(Clone,Copy)]
struct Pending{player:usize,stats:usize,amount:i32,remaining:f32}
static PENDING:Mutex<Option<Pending>>=Mutex::new(None);
unsafe fn recalculate(stats:usize,base:usize){
 let refresh:unsafe extern "system" fn(usize,u8)=std::mem::transmute(base+RECALCULATE_RVA);
 refresh(stats,1);
}
fn healing(maximum:i32,active:bool)->i32{maximum.max(0)/if active{3}else{2}}
pub unsafe fn activated(player:usize,base:usize){
 if player==0||!is_local(player,base)||!has_effect(player,OWNER,base)||has_effect(player,crate::reward_cycle_runtime::RESET,base){return;}
 let stats=native_actor::stats(player);
 if stats==0||native_actor::incapacitated(player,stats){return;}
 recalculate(stats,base);
 let amount=healing(read_i32(stats+0x144),has_effect(player,BUFF,base));
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);
 if apply(player,BUFF,0)==0||native_actor::incapacitated(player,stats){return;}
 if let Ok(mut pending)=PENDING.lock(){
  let amount=pending.filter(|old|old.player==player&&old.stats==stats).map_or(amount,|old|old.amount.saturating_add(amount));
  *pending=Some(Pending{player,stats,amount,remaining:1.});
 }
}
pub unsafe fn clear(player:usize,base:usize){
 if let Ok(mut pending)=PENDING.lock(){*pending=None;}
 if player==0||!has_effect(player,BUFF,base){return;}
 let manager=read_ptr(player+0x190);if manager==0{return;}
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);
 remove(manager,BUFF);
 let stats=native_actor::stats(player);
 if stats!=0{recalculate(stats,base);}
}
pub unsafe fn update(player:usize,base:usize,dt:f32){
 let stats=native_actor::stats(player);
 if stats==0||native_actor::incapacitated(player,stats)||!has_effect(player,OWNER,base)||has_effect(player,crate::reward_cycle_runtime::RESET,base){clear(player,base);return;}
 if !dt.is_finite()||dt<=0.||dt>1.{return;}
 let request=PENDING.lock().ok().and_then(|mut pending|pending.take());
 let Some(mut request)=request else{return;};
 if request.player!=player||request.stats!=stats{return;}
 if !has_effect(player,BUFF,base){
  request.remaining-=dt;
  if request.remaining>0.{if let Ok(mut pending)=PENDING.lock(){*pending=Some(request);}}
  return;
 }
 recalculate(stats,base);
 if native_actor::incapacitated(player,stats){clear(player,base);return;}
 let recover:unsafe extern "system" fn(usize,i32,u8,u8,f32,f32,u8)=std::mem::transmute(base+crate::death_dance::DEBIT_RVA);
 recover(stats,request.amount,0,0,0.,1.,1);
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn refreshed_cast_heals_from_the_unboosted_maximum(){
  assert_eq!(healing(1000,false),500);assert_eq!(healing(1500,true),500);
  assert_eq!(healing(1001,false),500);assert_eq!(healing(1501,true),500);
  assert_eq!(healing(1800,false),900);assert_eq!(healing(2700,true),900);
  assert_eq!(healing(0,false),0);assert_eq!(healing(-1,true),0);
 }
 #[test]fn challenge_is_available_to_all_heroes_and_party_sizes(){
  let index=crate::hex_pool::OWNER_IDS.iter().position(|&owner|owner==OWNER).unwrap();
  for hero in std::iter::once(None).chain((0..=10).map(Some)){
   for affinity in [None,Some(1100),Some(1600),Some(1700),Some(57),Some(61)]{
    for party in [None,Some(1),Some(2),Some(3)]{
     let filtered=crate::reward_choices::pool_exclusions(crate::reward_choices::party_exclusions(crate::reward_choices::weapon_exclusions(crate::reward_choices::role_exclusions(Default::default(),hero),affinity),party));
     assert!(!filtered.contains(&index));
     assert!(crate::reward_cycle::Round::new(&filtered,&Default::default()).candidates.contains(&index));
    }
   }
  }
  assert_eq!(crate::hex_pool::HISTORY_BITS[index],37);
 }
}
#[cfg(test)]#[path="ultimate_challenge_native_tests.rs"]mod native_tests;
