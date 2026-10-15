//! Nearby Sword spells use native weapon classifications in on-hit selection.
//! The private descriptor changes no damage, element, projectile or shared data.
use crate::{has_effect,is_local,read_ptr,sword,spell_percentage,sword_hit_distance,ACTIVE,BASE};
use std::{ptr,sync::atomic::{AtomicPtr,Ordering}};
pub const RVA:usize=0x505e90;
pub const PATTERN:&str="40 53 55 56 57 48 81 EC 88 00 00 00 48 8B 05 C5 00 5E 03 48 33 C4 48 89 44 24 78";
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
const HIT_SIZE:usize=0x2c8;
fn weapon_hit(original:&[u8;HIT_SIZE])->Option<[u8;HIT_SIZE]>{
 if !matches!(original[0x122],3|4){return None;}
 let mut hit=*original;hit[0x122]=1;
 // Keep every original category, including charged-spell 110. Weapon charge
 // 100 is additive only when the hit itself already carries native charge 110.
 hit[0x124+130/8]|=1<<(130%8);
 if original[0x124+110/8]&(1<<(110%8))!=0{hit[0x124+100/8]|=1<<(100%8);}
 Some(hit)
}
unsafe fn dispatch(hit:usize,enabled:bool,call:impl FnOnce(usize)){
 if enabled&&hit!=0{
  let input=ptr::read_unaligned(hit as*const [u8;HIT_SIZE]);
  if let Some(copy)=weapon_hit(&input){call(copy.as_ptr()as usize);return;}
 }
 call(hit);
}
type Select=unsafe extern "system" fn(usize,usize,usize);
pub unsafe extern "system" fn detour(manager:usize,out:usize,hit:usize){
 let original:Select=std::mem::transmute(ORIGINAL.load(Ordering::Acquire));
 let base=BASE.load(Ordering::Acquire);
 let enabled=if ACTIVE.load(Ordering::Acquire)&&manager!=0&&hit!=0{
  let owner=read_ptr(manager+0x10);
  is_local(owner,base)&&has_effect(owner,sword::OWNER,base)&&spell_percentage(hit).is_some()
   &&sword::near(sword_hit_distance(owner,hit,base),sword::NEAR_DISTANCE)
 }else{false};
 // 505E90 consumes hit data synchronously and appends native effect IDs to
 // out. Eligibility/tiers/duplicates/delivery remain the game's own rules.
 dispatch(hit,enabled,|input|original(manager,out,input));
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn conversion_preserves_damage_elements_charge_and_all_unrelated_bytes(){
  for kind in 0..13{for charged in [false,true]{
   let mut hit=[0xabu8;HIT_SIZE];hit[0x122]=kind;hit[0x124..0x144].fill(0);
   hit[0x124+7/8]|=1<<(7%8);hit[0x124+111/8]|=1<<(111%8);
   if charged{hit[0x124+110/8]|=1<<(110%8);}
   let copy=weapon_hit(&hit);
   if !matches!(kind,3|4){assert!(copy.is_none());continue;}
   let copy=copy.unwrap();assert_eq!(copy[0x122],1);
   for bit in 0..256{
    let old=hit[0x124+bit/8]&(1<<(bit%8))!=0;
    assert_eq!(copy[0x124+bit/8]&(1<<(bit%8))!=0,old||bit==130||(charged&&bit==100));
   }
   for i in 0..HIT_SIZE{if i!=0x122&&!(0x124..0x144).contains(&i){assert_eq!(copy[i],hit[i]);}}
  }}
 }
 #[test]fn selector_gets_private_copy_once_and_keeps_native_outputs(){unsafe{
  let mut hit=[0u8;HIT_SIZE];hit[0x122]=3;let h=hit.as_mut_ptr()as usize;let saved=hit;
  let mut calls=0;let mut result=0;
  dispatch(h,true,|p|{assert_ne!(p,h);assert_eq!(ptr::read((p+0x122)as*const u8),1);calls+=1;result=8320401;});
  assert_eq!((calls,result),(1,8320401));assert_eq!(hit,saved);
  dispatch(h,false,|p|assert_eq!(p,h));
  hit[0x122]=1;dispatch(h,true,|p|assert_eq!(p,h));
  dispatch(0,true,|p|assert_eq!(p,0));
 }}
 #[test]fn selector_detour_retains_three_argument_abi_and_output(){unsafe{
  unsafe extern "system" fn native(manager:usize,out:usize,hit:usize){assert_eq!((manager,hit),(11,22));ptr::write(out as*mut u32,1234);}
  ORIGINAL.store(native as*mut u8,Ordering::Release);let mut out=0u32;
  detour(11,&mut out as*mut u32 as usize,22);assert_eq!(out,1234);
 }}
}
