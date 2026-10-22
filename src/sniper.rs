//! Experienced Sniper: native attack/range penalty plus distance-scaled hit damage.
use crate::{has_effect,ptr,read_i32};
pub const OWNER:i32=95004500;
pub const CAP_DISTANCE:f32=50.;
pub const MAX_BONUS:f32=1.5;
pub fn arrow_rain(attack:i32,table:i32)->bool{
 table==1&&matches!(attack,5100850|5100855|5100880|5100885)
}
pub fn multiplier(distance:Option<f32>,attack:i32,table:i32)->f32{
 let Some(distance)=distance.filter(|v|v.is_finite()&&*v>=0.)else{return 1.;};
 let bonus=MAX_BONUS*(distance/CAP_DISTANCE).min(1.);
 1.+bonus*if arrow_rain(attack,table){0.6}else{1.}
}
pub unsafe fn calculated(local:usize,target:usize,source:usize,hit:usize,base:usize){
 if local==0||target==0||source!=local||source==target||hit==0||read_i32(hit+0x280)<=0
  ||!has_effect(local,OWNER,base)||has_effect(local,crate::reward_cycle_runtime::RESET,base){return;}
 // Current shooter-to-contact distance. Never use the target pivot, the rain's
 // airborne child origin or projectile path length as distance to the enemy.
 let distance=if crate::spatial_ready(local){crate::sword::distance(crate::position(local,base),crate::sword_hit_point(hit))}else{None};
 let factor=multiplier(distance,read_i32(hit+0x40),read_i32(hit+0x44));
 if factor>1.{ptr::write_unaligned((hit+0x280)as*mut i32,crate::sword::boosted(read_i32(hit+0x280),factor));}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn distance_curve_penalty_and_rain_increment(){
  for(d,normal,rain)in [(0.,1.,1.),(10.,1.3,1.18),(25.,1.75,1.45),(50.,2.5,1.9),(100.,2.5,1.9)]{
   assert!((multiplier(Some(d),40000,1)-normal).abs()<1e-6);
   for attack in [5100850,5100855,5100880,5100885]{assert!((multiplier(Some(d),attack,1)-rain).abs()<1e-6);assert_eq!(multiplier(Some(d),attack,0),normal);}
  }
  for distance in [None,Some(-1.),Some(f32::NAN),Some(f32::INFINITY)]{assert_eq!(multiplier(distance,40000,1),1.);}
  assert!((0.7*multiplier(Some(50.),40000,1)-1.75).abs()<1e-6);
  assert!((0.7*multiplier(Some(50.),5100850,1)-1.33).abs()<1e-6);
  for attack in [5100000,5100100,5100800,5100860,600000110,40210]{assert!(!arrow_rain(attack,1));}
 }
 #[test]fn sword_conflict_uses_actual_ownership_in_all_draw_paths(){
  use crate::reward_choices::*;
  let index=|id|crate::hex_pool::OWNER_IDS.iter().position(|&v|v==id).unwrap();
  let s=index(OWNER);let sword=index(crate::sword::OWNER);
  for owned in [mask([]),mask([s]),mask([sword]),mask([s,sword])]{
   let blocked=ownership_exclusions(owned.clone());
   assert_eq!(blocked.contains(&s),!owned.is_empty());assert_eq!(blocked.contains(&sword),!owned.is_empty());
   for seen in [mask([]),all(crate::hex_pool::HEX_COUNT)]{
    let round=crate::reward_cycle::Round::new(&blocked,&seen);let single=crate::reward_cycle::Single::new(&blocked,&seen,&mask([0,1]));
    for id in [s,sword]{assert_eq!(round.candidates.contains(&id),owned.is_empty());assert_eq!(single.candidates.contains(&id),owned.is_empty());}
    let focused=round.preferred([s,sword,0]).unwrap();if !owned.is_empty(){assert!(!focused.contains(&s)&&!focused.contains(&sword));}
   }
  }
  // A caster restriction alone must not exclude Sniper as if Sword were owned.
  let blocked=weapon_exclusions(ownership_exclusions(mask([])),Some(51));assert!(!blocked.contains(&s));assert!(blocked.contains(&sword));
  for hero in [None,Some(1),Some(7)]{for affinity in [None,Some(57),Some(51)]{
   let caster=affinity==Some(57);let physical=affinity==Some(51);
   for owned in [mask([]),mask([s]),mask([sword])]{
    let blocked=combat_exclusions(owned.clone(),hero,affinity);
    assert_eq!(blocked.contains(&s),!owned.is_empty());
    assert_eq!(blocked.contains(&sword),!owned.is_empty()||!caster);
    for id in [95001100,95002600]{assert_eq!(blocked.contains(&index(id)),!physical&&!owned.contains(&sword));}
   }
  }}
  for hero in 1..=10{assert!(!role_exclusions(mask([]),Some(hero)).contains(&s));}
  assert_eq!(crate::hex_pool::HISTORY_BITS[s],42);
 }
}
#[cfg(test)]#[path="sniper_native_tests.rs"]mod native_tests;
