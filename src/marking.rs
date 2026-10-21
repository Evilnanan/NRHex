//! Ironeye's direct Marking hit uses Stamp (Upward Cut)'s native reaction.
use crate::{ptr,read_i32};
pub const OWNER:i32=95004200;
pub const ATTACK:i32=600000110;
pub const IMPACT:u8=9;
pub unsafe fn constructed(hit:usize,context:usize,owned:impl FnOnce()->bool){
 // Caller has already matched context[0] against the local ChrIns. Never
 // change a shared AtkParam, poison-relic follow-up, projectile or other skill.
 if hit!=0&&context!=0&&read_i32(context+8)==ATTACK&&read_i32(context+0xc)==1&&owned(){
  ptr::write((hit+0x24)as*mut u8,IMPACT);
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn only_owned_pc_marking_changes_the_outgoing_reaction(){unsafe{
  for owned in [false,true]{for attack in [ATTACK,600000120,300000290,0,-1]{for table in [0,1,2]{
   let mut context=[0u32;8];context[2]=attack as u32;context[3]=table;
   for original in [0,1,2,3,7,9,255]{
    let mut hit=[0xabu8;0x300];hit[0x24]=original;let before=hit;
    constructed(hit.as_mut_ptr()as usize,context.as_ptr()as usize,||owned);
    for i in 0..hit.len(){assert_eq!(hit[i],if i==0x24&&owned&&attack==ATTACK&&table==1{IMPACT}else{before[i]});}
   }
  }}}
 }}
 #[test]fn only_ironeye_draws_marking_even_after_affinity_overrides(){
  let index=crate::hex_pool::OWNER_IDS.iter().position(|&id|id==OWNER).unwrap();
  for hero in std::iter::once(None).chain((0..=10).map(Some)){for affinity in [None,Some(1100),Some(57),Some(61),Some(1600),Some(1700)]{for party in [None,Some(1),Some(2),Some(3)]{
   let excluded=crate::reward_choices::party_exclusions(crate::reward_choices::weapon_exclusions(crate::reward_choices::role_exclusions(Default::default(),hero),affinity),party);
   assert_eq!(!excluded.contains(&index),hero==Some(3));
   let round=crate::reward_cycle::Round::new(&excluded,&Default::default());assert_eq!(round.candidates.contains(&index),hero==Some(3));
   assert_eq!(crate::reward_cycle::Single::new(&excluded,&Default::default(),&crate::reward_choices::mask([0,1])).candidates.contains(&index),hero==Some(3));
  }}}
  assert_eq!(crate::hex_pool::HISTORY_BITS[index],39);
 }
}
#[cfg(test)]#[path="marking_native_tests.rs"]mod native_tests;
