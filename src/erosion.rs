//! Native attack-added ailments. Selection happens per recipient, before the
//! native apply/authority/network path; native motion values and buildup remain.
use crate::{has_effect,has_state,native_actor,read_i32,read_ptr,ptr,ACTIVE,BASE};
use std::sync::atomic::{AtomicPtr,Ordering};
pub const OWNER:i32=95003100;
pub const CARRIER:i32=OWNER+1;
pub const FIRST:i32=OWNER+10;
pub const RVA:usize=0x3c80a0;
pub const PATTERN:&str="48 89 5C 24 10 55 56 57 41 54 41 55 41 56 41 57 48 8D AC 24 60 FF FF FF 48 81 EC A0 01 00 00 45 0F B6 E1 4D 8B E8 44 8B FA 48 8B F9 33 DB";
pub const IMMUNE_RVA:usize=0x509790;
pub const IMMUNE_PATTERN:&str="48 89 5C 24 08 57 48 83 EC 20 48 8B 59 08 8B FA 48 85 DB 74 20 F7 43 68 03 00 0C 80";
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
// CSChrResistModule order: poison, rot, blood loss, death, frost, sleep, madness.
const STATES:[u16;7]=[2,5,6,116,260,436,437];
#[derive(Clone,Copy,Debug,Default)]
struct Candidate{threshold:i32,remaining:i32,immune:bool,active:bool}
fn weakest(candidates:&[Candidate;7])->Option<usize>{
 let mut best:Option<usize>=None;
 for (i,c) in candidates.iter().enumerate(){
  if c.immune||c.active||c.threshold<=0{continue;}
  if best.is_none_or(|b|{
   let previous=candidates[b];
   c.threshold<previous.threshold||c.threshold==previous.threshold
    && c.remaining.clamp(0,c.threshold)<previous.remaining.clamp(0,previous.threshold)
  }){best=Some(i);}
 }
 best
}
unsafe fn select(target:usize,source:usize,context:usize,base:usize)->Option<i32>{
 if target==0||source==0||target==source||context==0{return None;}
 // Carrier is emitted only by the source's native stateInfo152 effect. Do not
 // turn unrelated effect applications or generic projectile pointers into hits.
 let vt=read_ptr(source);if vt==0{return None;}
 let kind:unsafe extern "system" fn(usize)->i32=std::mem::transmute(read_ptr(vt+0x10));
 if kind(source)!=1||!has_effect(source,OWNER,base){return None;}
 let factor=ptr::read_unaligned((context+0x38)as*const f32);
 if !factor.is_finite()||factor<=0.{return None;}
 let stats=native_actor::stats(target);
 if stats==0||read_i32(stats+0x140)<=0{return None;}
 let modules=read_ptr(target+0x1b8);let resist=read_ptr(modules+0x20);
 if resist==0||read_ptr(resist)!=base+0x2c113c8{return None;}
 let manager=read_ptr(target+0x190);if manager==0{return None;}
 let immune:unsafe extern "system" fn(usize,u32)->u8=std::mem::transmute(base+IMMUNE_RVA);
 let mut candidates=[Candidate::default();7];
 for i in 0..7{
  candidates[i]=Candidate{threshold:read_i32(resist+0x2c+i*4),remaining:read_i32(resist+0x10+i*4),
   immune:immune(manager,i as u32)!=0,active:has_state(target,STATES[i],base)};
 }
 weakest(&candidates).map(|i|FIRST+i as i32)
}
// 50F230 supplies the native attack-table key at descriptor +34/+38.
// These PC rows are body/locomotion contacts, including roll 304. Their
// native status correction is 100 despite having no weapon attack. Filter
// only Erosion's state152 owner, before it can emit a carrier; leave native
// attack-added effects and actual attacks performed after a roll untouched.
pub unsafe fn body_contact(node:usize,descriptor:usize)->bool{
 if node==0||descriptor==0{return false;}
 let param=read_ptr(node);if param==0{return false;}
 ptr::read_unaligned((param+0x156)as*const u16)==152
  &&read_i32(param+0x12c)==CARRIER&&read_i32(descriptor+0x38)==1
  &&matches!(read_i32(descriptor+0x34),300..=307|310..=313|322|550|551|560|561)
}
// Nine stack arguments follow the four register arguments: a5 is attribution metadata,
// a6 is the native buildup context. Preserve all 13 arguments and the AL result.
type Apply=unsafe extern "system" fn(usize,i32,usize,u8,usize,usize,u8,u8,u8,u8,u8,u8,i32)->u8;
pub unsafe extern "system" fn detour(target:usize,mut effect:i32,source:usize,a4:u8,a5:usize,a6:usize,a7:u8,a8:u8,a9:u8,a10:u8,a11:u8,a12:u8,a13:i32)->u8{
 let original:Apply=std::mem::transmute(ORIGINAL.load(Ordering::Acquire));
 if effect==CARRIER&&ACTIVE.load(Ordering::Acquire){
  let Some(chosen)=select(target,source,a6,BASE.load(Ordering::Acquire))else{return 0;};effect=chosen;
 }
 original(target,effect,source,a4,a5,a6,a7,a8,a9,a10,a11,a12,a13)
}
#[cfg(test)]
#[path="erosion_native_tests.rs"]
mod native_tests;
#[cfg(test)]mod tests{
 use super::*;
 fn c(threshold:i32,remaining:i32)->Candidate{Candidate{threshold,remaining,..Default::default()}}
 #[test]fn immunity_and_active_ailments_are_excluded_but_death_is_a_normal_candidate(){
  let mut a=[c(999,999);7];for v in &mut a{v.immune=true;}
  assert_eq!(weakest(&a),None);
  a[3]=c(84,84);assert_eq!(weakest(&a),Some(3));
  a[3].active=true;assert_eq!(weakest(&a),None);
  a[1]=c(154,154);assert_eq!(weakest(&a),Some(1));
 }
 #[test]fn compares_current_full_threshold_then_existing_progress_and_keeps_ties_stable(){
  let mut a=[c(542,1);7];a[1]=c(154,154);a[2]=c(154,100);
  assert_eq!(weakest(&a),Some(2));a[1].remaining=100;assert_eq!(weakest(&a),Some(1));
  a[1].threshold=252;assert_eq!(weakest(&a),Some(2));
  a[2].active=true;assert_eq!(weakest(&a),Some(1));
  a[1].immune=true;assert_eq!(weakest(&a),Some(0));
 }
 #[test]fn invalid_thresholds_and_not_the_number_999_define_unavailable_candidates(){
  let mut a=[c(0,0);7];a[0].threshold=-1;assert_eq!(weakest(&a),None);
  a[6]=c(999,999);assert_eq!(weakest(&a),Some(6));
 }
 #[test]fn body_contact_filter_requires_the_erosion_owner_and_pc_table(){unsafe{
  let mut row=[0u8;0x160];let param=row.as_mut_ptr()as usize;
  let node=[param];let mut descriptor=[0u8;0x3c];
  ptr::write_unaligned((param+0x156)as*mut u16,152);ptr::write_unaligned((param+0x12c)as*mut i32,CARRIER);
  for attack in [304,307,301,550,560,10000,40000,300208910,303401305]{for table in [0,1,2]{
   descriptor[0x34..0x38].copy_from_slice(&i32::to_le_bytes(attack));descriptor[0x38..].copy_from_slice(&i32::to_le_bytes(table));
   assert_eq!(body_contact(node.as_ptr()as usize,descriptor.as_ptr()as usize),table==1&&attack<1000);
  }}
  descriptor[0x34..0x38].copy_from_slice(&304i32.to_le_bytes());descriptor[0x38..].copy_from_slice(&1i32.to_le_bytes());
  ptr::write_unaligned((param+0x12c)as*mut i32,8110002);assert!(!body_contact(node.as_ptr()as usize,descriptor.as_ptr()as usize));
  assert!(!body_contact(0,descriptor.as_ptr()as usize));assert!(!body_contact(node.as_ptr()as usize,0));
 }}
 #[test]fn native_apply_forwards_all_thirteen_arguments_and_return_when_not_remapping(){unsafe{
  unsafe extern "system" fn native(t:usize,e:i32,s:usize,b:u8,d:usize,c:usize,x:u8,y:u8,z:u8,j:u8,k:u8,l:u8,n:i32)->u8{
   assert_eq!((t,e,s,b,d,c),(11,-22,33,0x81,55,66));
   assert_eq!((x,y,z,j,k,l,n),(0x82,0x83,0x84,0x85,0x86,0x87,-99));0x91
  }
  ORIGINAL.store(native as *mut u8,Ordering::Release);
  assert_eq!(detour(11,-22,33,0x81,55,66,0x82,0x83,0x84,0x85,0x86,0x87,-99),0x91);
 }}
}
