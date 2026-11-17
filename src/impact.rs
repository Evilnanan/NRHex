//! Persist the native impact AND its wire override at fresh construction.
use crate::{ptr,read_ptr,has_effect,weapon_contact};
pub const REBUILD_RVA:usize=0x4350e6;
pub const REBUILD_PATTERN:&str="0F B6 83 8C 00 00 00 88 45 08 C6 45 4E 01";
pub fn override_id(level:u8)->Option<u8>{
 // Inverse of native E6D510; 0 means use the attack row, NOT impact zero.
 match level{0=>Some(1),1=>Some(3),2=>Some(4),3=>Some(5),4=>Some(7),5=>Some(10),6=>Some(8),7=>Some(6),8=>Some(2),9=>Some(9),10=>Some(11),11=>Some(12),_=>None}
}
pub unsafe fn set(hit:usize,level:u8){
 if let Some(id)=override_id(level){ptr::write((hit+0x24)as*mut u8,level);ptr::write((hit+0x1e)as*mut u8,id);}
}
pub unsafe fn boost(hit:usize){
 let original=ptr::read((hit+0x24)as*const u8);let level=crate::strength::remap(original);
 // Preserve native override/default behavior on tiers Mighty does not change.
 if level!=original{set(hit,level);}
}
pub unsafe fn constructed(local:usize,hit:usize,context:usize,base:usize){
 if local==0||hit==0||context==0{return;}
 // 434FA0 sets +10E before rebuilding the received descriptor. Preserve its
 // encoded tier even when the received source resolves back to this player.
 if ptr::read((context+0x10e)as*const u8)!=0{return;}
 let direct=read_ptr(context)==local;
 if !direct{
  if crate::spell_percentage(hit).is_none(){return;}
  let resolve:unsafe extern "system" fn(u64)->usize=std::mem::transmute(base+crate::CHR_HANDLE_RVA);
  if crate::ability_damage::source_actor(context,resolve)!=local{return;}
 }
 if crate::strength::applies(crate::read_i32(context+8),crate::read_i32(context+12))
  &&has_effect(local,crate::strength::OWNER,base)
  &&weapon_contact::from_source(local,hit,direct,base).is_some(){boost(hit);}
 if direct{crate::marking::constructed(hit,context,||has_effect(local,crate::marking::OWNER,base),||crate::marking::air_started(local));}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn paired_impact_updates_only_two_bytes_and_preserves_unknown_tiers(){unsafe{
  for level in 0..=255u8{
   let mut hit=[0xabu8;0x2c8];let before=hit;set(hit.as_mut_ptr()as usize,level);
   for i in 0..hit.len(){assert_eq!(hit[i],match (override_id(level),i){(Some(_),0x24)=>level,(Some(id),0x1e)=>id,_=>before[i]});}
  }
 }}
 #[test]fn unchanged_mighty_tiers_do_not_install_an_override(){unsafe{
  for level in 0..=255u8{for encoded in [0,1,6,12]{
   let mut hit=[0u8;0x2c8];hit[0x24]=level;hit[0x1e]=encoded;let before=hit;boost(hit.as_mut_ptr()as usize);
   if crate::strength::remap(level)==level{assert_eq!(hit,before);}else{assert_eq!(hit[0x24],crate::strength::remap(level));assert_eq!(Some(hit[0x1e]),override_id(hit[0x24]));}
  }}
 }}
}
#[cfg(test)]#[path="impact_native_tests.rs"]mod native_tests;
