//! Scoped native art quotes/reservations: preserve mastered Onyx costs
//! and distinguish Lightning Ram activation from its free continuations.
use crate::{ptr,read_ptr,read_i32,ACTIVE,BASE};
use std::{cell::Cell,sync::atomic::{AtomicPtr,Ordering}};
pub const CHECK_RVA:usize=0x48f9b0;
pub const CHECK_PATTERN:&str="48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 50 41 8B D8 8B FA 48 8B F1 E8 54 C7 F9 FF";
pub const RESERVE_RVA:usize=0x48fa90;
pub const RESERVE_PATTERN:&str="48 89 5C 24 10 48 89 74 24 18 57 48 83 EC 70 41 8B D8 8B FA 48 8B F1 E8 74 C6 F9 FF";
pub const RATE_RVA:usize=0x5061b0;
pub const RATE_PATTERN:&str="48 8B 41 08 F3 0F 10 15 E0 D4 DE 02 0F 28 CA 48 85 C0 74 33 0F 57 DB F7 40 68 03 00 0C 80 75 1E";
pub static CHECK_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static RESERVE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static RATE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
const L2:i32=3;
thread_local! {
 static FILTER:Cell<usize>=const{Cell::new(0)};
 static RAM:Cell<Option<(usize,i32)>>=const{Cell::new(None)};
}
pub struct Guard(usize,Option<(usize,i32)>);
impl Guard{
 pub unsafe fn enter(actor:usize,input:Option<(i32,i32)>,base:usize)->Self{
  let hand=input.map(|(hand,_)|hand);
  let enabled=ACTIVE.load(Ordering::Acquire);
  let manager=if enabled&&crate::onyx_mastery::fp_exception(actor,hand,base){read_ptr(actor+0x190)}else{0};
  let ram=input.filter(|&(hand,_)|enabled&&crate::lightning_ram::managed_art(actor,hand,base))
   .map(|(_,button)|(read_ptr(actor+0x190),button));
  Self(FILTER.with(|s|s.replace(manager)),RAM.with(|s|s.replace(ram)))
 }
}
impl Drop for Guard{fn drop(&mut self){FILTER.with(|s|s.set(self.0));RAM.with(|s|s.set(self.1));}}
type Check=unsafe extern "system" fn(usize,i32,i32)->u8;
pub unsafe extern "system" fn check(module:usize,button:i32,hand:i32)->u8{
 let original:Check=std::mem::transmute(CHECK_ORIGINAL.load(Ordering::Acquire));
 let actor=if module!=0{read_ptr(module+8)}else{0};
 let _scope=Guard::enter(actor,Some((hand,button)),BASE.load(Ordering::Acquire));
 original(module,button,hand)
}
pub unsafe extern "system" fn reserve(module:usize,button:i32,hand:i32){
 let original:unsafe extern "system" fn(usize,i32,i32)=std::mem::transmute(RESERVE_ORIGINAL.load(Ordering::Acquire));
 let actor=if module!=0{read_ptr(module+8)}else{0};
 let _scope=Guard::enter(actor,Some((hand,button)),BASE.load(Ordering::Acquire));
 if matches!(RAM.with(Cell::get),Some((_,L2))){
  // Capture native eligibility before the reservation can spend the last FP.
  let enough:Check=std::mem::transmute(CHECK_ORIGINAL.load(Ordering::Acquire));
  crate::lightning_ram::record_startup(actor,hand,enough(module,button,hand)!=0);
 }
 original(module,button,hand)
}
pub unsafe extern "system" fn rate(manager:usize)->f32{
 let original:unsafe extern "system" fn(usize)->f32=std::mem::transmute(RATE_ORIGINAL.load(Ordering::Acquire));
 if manager==0||!ACTIVE.load(Ordering::Acquire){return original(manager);}
 if let Some((_,button))=RAM.with(Cell::get).filter(|&(owner,_)|owner==manager){
  // SwordArtsParam 504 has a base L2 fee of 5. Preserve native cost modifiers and rounding.
  return if button==L2{2.*original(manager)}else{0.};
 }
 if FILTER.with(Cell::get)!=manager{return original(manager);}
 // Match native 5061B0's eligibility, multiplication order, null-row default
 // and negative/NaN exclusion; omit only the Wildcard Arts owner effect.
 let mut result=1f32;let mut node=read_ptr(manager+8);
 while node!=0{
  if ptr::read_unaligned((node+0x68)as*const u32)&0x800c0003==0&&read_i32(node+8)!=crate::arts::OWNER{
   let row=read_ptr(node);let value=if row!=0{ptr::read_unaligned((row+0x2e4)as*const f32)}else{1.};
   if value>=0.{result*=value;}
  }
  node=read_ptr(node+0x38);
 }
 result
}
