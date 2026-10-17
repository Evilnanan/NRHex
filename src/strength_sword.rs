//! Scope the spell/projectile impact bonus across native damage/reaction dispatch.
//! The collision point exists here, after projectile construction. Restore only
//! the input after ALL consumers; keep native damage, reaction and copied outputs.
use crate::{has_effect,is_local,read_ptr,sword,spell_percentage,sword_hit_distance,ACTIVE,BASE};
use std::{cell::RefCell,ptr,sync::atomic::{AtomicPtr,Ordering}};
pub const RVA:usize=0x43d260;
pub const PATTERN:&str="48 8B C4 48 89 58 20 55 56 57 41 54 41 55 41 56 41 57 48 8D 68 A9 48 81 EC 00 01 00 00 0F 29 70 B8 0F 29 78 A8";
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
thread_local!{static SCOPES:RefCell<Vec<usize>>=const{RefCell::new(Vec::new())};}
fn eligible(local:bool,other:bool,mighty:bool,sword:bool,ranged:bool,distance:Option<f32>)->bool{
 local&&other&&mighty&&sword&&ranged&&crate::sword::near(distance,crate::sword::NEAR_DISTANCE)
}
unsafe fn projectile(hit:usize,actor:usize)->bool{
 // Native 5363A0 copies the original collision source into hit+228; the
 // dispatch's resolved actor can instead be its owner. Direct weapon hits
 // have already received Mighty at construction and must not get it twice.
 let source=read_ptr(hit+0x228);
 if source==0||source==actor{return false;}
 let table=read_ptr(source);if table==0{return false;}
 let getter=read_ptr(table+0x10);if getter==0{return false;}
 let kind:unsafe extern "system" fn(usize)->i32=std::mem::transmute(getter);
 kind(source)==3
}
unsafe fn with_impact(hit:usize,enabled:bool,call:impl FnOnce()){
 let change=enabled&&hit!=0&&SCOPES.with(|s|!s.borrow().contains(&hit));
 let original=if change{Some(ptr::read((hit+0x24)as*const u8))}else{None};
 SCOPES.with(|s|s.borrow_mut().push(if change{hit}else{0}));
 if let Some(level)=original{ptr::write((hit+0x24)as*mut u8,crate::strength::remap(level));}
 call();
 if let Some(level)=original{ptr::write((hit+0x24)as*mut u8,level);}
 SCOPES.with(|s|{s.borrow_mut().pop();});
}
type Dispatch=unsafe extern "system" fn(usize,usize,usize,u8,u8);
pub unsafe extern "system" fn detour(module:usize,source:usize,hit:usize,a:u8,b:u8){
 let original:Dispatch=std::mem::transmute(ORIGINAL.load(Ordering::Acquire));
 let base=BASE.load(Ordering::Acquire);
 let enabled=if ACTIVE.load(Ordering::Acquire)&&module!=0&&hit!=0&&is_local(source,base)
  &&ptr::read((hit+0x2c1)as*const u8)&8==0{
  eligible(true,read_ptr(module+8)!=source,has_effect(source,crate::strength::OWNER,base),
   has_effect(source,sword::OWNER,base),spell_percentage(hit).is_some()||projectile(hit,source),sword_hit_distance(source,hit,base))
 }else{false};
 with_impact(hit,enabled,||original(module,source,hit,a,b));
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn only_both_owners_spells_or_projectiles_and_sword_contact_range_qualify(){
  for bits in 0..32{for distance in [None,Some(f32::NAN),Some(0.),Some(4.),Some(4.001),Some(5.),Some(5.001),Some(f32::INFINITY)]{
   assert_eq!(eligible(bits&1!=0,bits&2!=0,bits&4!=0,bits&8!=0,bits&16!=0,distance),bits==31&&matches!(distance,Some(d)if d.is_finite()&&d<=5.));
  }}
 }
 #[test]fn native_bullet_sources_qualify_without_double_boosting_direct_weapons(){unsafe{
  unsafe extern "system" fn kind(source:usize)->i32{crate::read_i32(source+8)}
  let mut vt=[0usize;3];vt[2]=kind as*const ()as usize;
  let mut source=[vt.as_ptr()as usize,3];let mut hit=[0u64;0x2c8/8];
  let h=hit.as_mut_ptr()as usize;let actor=123;
  assert!(!projectile(h,actor));
  hit[0x228/8]=actor as u64;assert!(!projectile(h,actor));
  hit[0x228/8]=source.as_ptr()as u64;
  // All BulletIns share kind 3: no spell table lookup excludes arrows,
  // bolts, skill missiles or their child bullets.
  for k in 0..9{source[1]=k;assert_eq!(projectile(h,actor),k==3);}
  source[0]=0;assert!(!projectile(h,actor));
 }}
 #[test]fn impact_scope_handles_repeated_copied_and_nested_hits_without_compounding(){unsafe{
  for level in 0..=255u8{
   let mut hit=[0u8;0x2c8];hit[0x24]=level;let h=hit.as_mut_ptr()as usize;
   for _ in 0..3{
    with_impact(h,true,||{
     assert_eq!(ptr::read((h+0x24)as*const u8),crate::strength::remap(level));
     with_impact(h,true,||assert_eq!(ptr::read((h+0x24)as*const u8),crate::strength::remap(level)));
     // Serialized/native copies retain the once-adjusted value; foreign
     // owners do not enter the combination scope on a receiving machine.
     let mut copy=ptr::read_unaligned(h as*const [u8;0x2c8]);
     let c=copy.as_mut_ptr()as usize;
     with_impact(c,false,||assert_eq!(ptr::read((c+0x24)as*const u8),crate::strength::remap(level)));
     ptr::write_unaligned((h+0x280)as*mut i32,123);
     ptr::write((h+0x2b6)as*mut u8,99); // target's native reaction mapping survives
    });
    assert_eq!(hit[0x24],level);assert_eq!(hit[0x2b6],99);assert_eq!(crate::read_i32(h+0x280),123);
   }
   with_impact(h,false,||assert_eq!(ptr::read((h+0x24)as*const u8),level));
  }
  assert!(SCOPES.with(|s|s.borrow().is_empty()));
 }}
 #[test]fn five_native_dispatch_arguments_and_outputs_survive_detour(){unsafe{
  unsafe extern "system" fn capture(m:usize,s:usize,h:usize,a:u8,b:u8){
   assert_eq!((m,s,a,b),(123,456,7,9));ptr::write_unaligned((h+0x280)as*mut u64,0x12345678);
  }
  ORIGINAL.store(capture as*mut u8,Ordering::Release);
  let mut hit=[0u8;0x2c8];hit[0x24]=8;
  detour(123,456,hit.as_mut_ptr()as usize,7,9);
  assert_eq!(hit[0x24],8);assert_eq!(ptr::read_unaligned(hit.as_ptr().add(0x280)as*const u64),0x12345678);
 }}
}
