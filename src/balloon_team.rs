use crate::{balloon,read_i32,read_ptr,ACTIVE,BASE,CHR_HANDLE_RVA};
use std::{cell::Cell,ptr,sync::atomic::{AtomicPtr,Ordering}};

pub const TEAM_RVA:usize=0x3d1b30;
pub const TEAM_PATTERN:&str="48 89 5C 24 10 57 48 83 EC 20 0F B6 41 74 48 8B F9 88 02 48 8B DA 48 8B 89 90 01 00 00";
pub const HIT_RVA:usize=0x52cef0;
pub const HIT_PATTERN:&str="48 89 5C 24 20 57 48 83 EC 20 48 8B B9 28 02 00 00 48 8B D9 48 8B CF 48 8B 07 FF 50 10";
pub const BULLET_RVA:usize=0x34d1a0;
pub const BULLET_PATTERN:&str="40 53 48 83 EC 20 48 8B DA C6 02 00 48 8B 91 B0 00 00 00 48 8B C2 48 C1 E8 1C 83 E0 0F";
pub const PARAM_RVA:usize=0x354c7a;
pub const PARAM_PATTERN:&str="8B B3 A4 00 00 00 4C 89 6C 24 40 C7 44 24 48 FF FF FF FF";
pub static TEAM_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static HIT_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static BULLET_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());

thread_local!{static SOURCE:Cell<usize>=const{Cell::new(0)};}
struct Scope(usize);
impl Scope{fn enter(actor:usize)->Self{Self(SOURCE.with(|source|source.replace(actor)))}}
impl Drop for Scope{fn drop(&mut self){SOURCE.with(|source|source.set(self.0));}}
unsafe fn kind(object:usize)->i32{
 if object==0{return -1;}
 let table=read_ptr(object);if table==0{return -1;}
 let getter=read_ptr(table+0x10);if getter==0{return -1;}
 let getter:unsafe extern "system" fn(usize)->i32=std::mem::transmute(getter);getter(object)
}
unsafe fn owner(bullet:usize,base:usize)->usize{
 if bullet==0||read_i32(bullet+0xa4)!=balloon::BULLET{return 0;}
 let resolve:unsafe extern "system" fn(u64)->usize=std::mem::transmute(base+CHR_HANDLE_RVA);
 let actor=resolve(ptr::read_unaligned((bullet+0xb0)as*const u64));
 if kind(actor)==1{actor}else{0}
}
unsafe fn hit_source(hit:usize,base:usize)->usize{
 if hit==0||read_i32(hit+0x40)!=balloon::ATTACK||read_i32(hit+0x44)!=1{return 0;}
 let source=read_ptr(hit+0x228);
 if kind(source)==3{owner(source,base)}else{0}
}
unsafe fn replace_near_death_team(actor:usize,result:usize){
 if actor==0||result==0||ptr::read(result as*const u8)!=77{return;}
 let native=ptr::read((actor+0x74)as*const u8);
 if native>0&&native<81&&native!=77{ptr::write(result as*mut u8,native);}
}
pub unsafe extern "system" fn team_detour(actor:usize,out:usize)->usize{
 let original:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(TEAM_ORIGINAL.load(Ordering::Acquire));
 let result=original(actor,out);
 if ACTIVE.load(Ordering::Acquire)&&actor!=0&&SOURCE.with(Cell::get)==actor{replace_near_death_team(actor,result);}
 result
}
pub unsafe extern "system" fn hit_detour(hit:usize)->u8{
 let original:unsafe extern "system" fn(usize)->u8=std::mem::transmute(HIT_ORIGINAL.load(Ordering::Acquire));
 let source=if ACTIVE.load(Ordering::Acquire){hit_source(hit,BASE.load(Ordering::Acquire))}else{0};
 let _scope=Scope::enter(source);original(hit)
}
pub unsafe extern "system" fn bullet_detour(bullet:usize,out:usize)->usize{
 let original:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(BULLET_ORIGINAL.load(Ordering::Acquire));
 let result=original(bullet,out);
 if ACTIVE.load(Ordering::Acquire)&&result!=0&&ptr::read(result as*const u8)==77{
  replace_near_death_team(owner(bullet,BASE.load(Ordering::Acquire)),result);
 }
 result
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn only_the_private_attack_scope_changes_its_near_death_source_team(){unsafe{
  let mut actor=[0u8;0x80];actor[0x74]=2;let actor=actor.as_ptr()as usize;
  let mut result=77u8;replace_near_death_team(actor,&mut result as*mut u8 as usize);assert_eq!(result,2);
  for team in [0,1,2,6,7,15,76,78,80]{result=team;replace_near_death_team(actor,&mut result as*mut u8 as usize);assert_eq!(result,team);}
  assert_eq!(SOURCE.with(Cell::get),0);
  {let _outer=Scope::enter(actor);assert_eq!(SOURCE.with(Cell::get),actor);
   {let _inner=Scope::enter(0);assert_eq!(SOURCE.with(Cell::get),0);}
   assert_eq!(SOURCE.with(Cell::get),actor);
   {let _other=Scope::enter(42);assert_eq!(SOURCE.with(Cell::get),42);}
   assert_eq!(SOURCE.with(Cell::get),actor);
  }
  assert_eq!(SOURCE.with(Cell::get),0);
 }}
 unsafe extern "system" fn native_team(actor:usize,out:usize)->usize{assert_ne!(actor,0);ptr::write(out as*mut u8,77);out}
 #[test]fn team_wrapper_preserves_pointer_return_and_does_not_change_target_or_unscoped_queries(){unsafe{
  let saved=TEAM_ORIGINAL.swap(native_team as*mut u8,Ordering::AcqRel);let active=ACTIVE.swap(true,Ordering::AcqRel);
  let mut actor=[0u8;0x80];actor[0x74]=2;let actor=actor.as_ptr()as usize;
  let mut other=[0u8;0x80];other[0x74]=6;let other=other.as_ptr()as usize;
  let mut result=0u8;let out=&mut result as*mut u8 as usize;
  assert_eq!(team_detour(actor,out),out);assert_eq!(result,77);
  {let _scope=Scope::enter(actor);assert_eq!(team_detour(actor,out),out);assert_eq!(result,2);
   assert_eq!(team_detour(other,out),out);assert_eq!(result,77);
   ACTIVE.store(false,Ordering::Release);assert_eq!(team_detour(actor,out),out);assert_eq!(result,77);
  }
  TEAM_ORIGINAL.store(saved,Ordering::Release);ACTIVE.store(active,Ordering::Release);
 }}
 #[test]fn attack_identity_rejects_unrelated_hits_before_resolving_a_source(){unsafe{
  let mut hit=[0u8;0x240];let pointer=hit.as_mut_ptr()as usize;
  assert_eq!(hit_source(0,0),0);
  for (attack,table) in [(12064105,1),(balloon::ATTACK,0),(balloon::ATTACK,2)]{
   ptr::write_unaligned((pointer+0x40)as*mut i32,attack);ptr::write_unaligned((pointer+0x44)as*mut i32,table);
   assert_eq!(hit_source(pointer,0),0);
  }
  let mut bullet=[0u8;0xc0];let pointer=bullet.as_mut_ptr()as usize;
  assert_eq!(owner(pointer,0),0);
 }}
}
#[cfg(test)]#[path="balloon_team/native_tests.rs"]mod native_tests;
