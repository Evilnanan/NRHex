//! Reuse the original ultimate gauge's inner Grayout timeline. No UI assets.
use crate::{ptr,read_ptr,ACTIVE,BASE};
use std::sync::atomic::{AtomicPtr,Ordering};
pub const RVA:usize=0x9a45dd;
pub const PATTERN:&str="E8 3E 5A FF FF 80 BE 08 14 00 00 00 0F 84 AD 00 00 00 80 BE 09 14 00 00 04";
pub const SITE_RVA:usize=0x9a45b6;
pub const SITE_PATTERN:&str="4C 8D 86 78 10 00 00 48 8D 8F C8 72 00 00 0F 28 CE E8 54 5A FF FF 4C 8D 86 D8 10 00 00 48 8D 8F F8 78 00 00 0F 28 CE";
pub const UPDATE_RVA:usize=0x99a020;
pub const UPDATE_PATTERN:&str="48 8B C4 48 89 58 10 55 56 57 41 56 41 57 48 81 EC 80 00 00 00 0F 29 70 C8 49 8B F0 0F 28 F1 48 8B F9 33 ED 66 89 68 08 48 83 C1 08 4C 8D 40 08 0F B6 16 E8 98 12 E2 FF";
pub const BIND_RVA:usize=0x9a1928;
pub const BIND_PATTERN:&str="4C 8D 05 51 6B 2B 02 48 8D 54 24 58 49 8B CC E8 64 16 DF FF 90 49 8D 8D F8 78 00 00 4C 8D 05 25 6B 2B 02 48 8B D0 E8 4D 81 FF FF";
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static CONTINUE:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());

unsafe extern "system" fn update(gauge:usize,dt:f32,state:usize){
 let base=BASE.load(Ordering::Acquire);
 let native:unsafe extern "system" fn(usize,f32,usize)=std::mem::transmute(base+UPDATE_RVA);
 if ACTIVE.load(Ordering::Acquire)&&state!=0{
  let world=read_ptr(base+crate::WORLD_RVA);
  if world!=0&&crate::basics::sealed(read_ptr(world+0x174e8),base){
   // The two contiguous native skill/ultimate display records are 0x60 bytes.
   // The native renderer consumes this snapshot synchronously, including its
   // existing values/references. Only the inner Grayout selector is changed.
   let mut copy=ptr::read_unaligned(state as*const [u64;12]);
   ptr::write((copy.as_mut_ptr()as*mut u8).add(3),1);
   native(gauge,dt,copy.as_ptr()as usize);return;
  }
 }
 native(gauge,dt,state);
}
// This replaces a CALL, not a function entry. Its caller already owns the
// aligned shadow space; preserve the original call's ABI and continue exactly
// after it. The neighbouring skill call never passes through this wrapper.
#[unsafe(naked)]pub unsafe extern "system" fn detour(){core::arch::naked_asm!(
 "call {callback}", "jmp qword ptr [rip+{next}]",callback=sym update,next=sym CONTINUE
);}
#[cfg(test)]#[path="basics_hud_tests.rs"]mod tests;
