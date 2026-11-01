//! Per-recipient Balloon input, before native offense, defense and guard.
//! The five-float vector is 439E80's stack-local copy, never shared hit data.
use crate::{balloon,native_actor,ptr,read_i32,read_ptr,ACTIVE};
use std::sync::atomic::{AtomicPtr,Ordering};
pub const RVA:usize=0x43a39f;
pub const PATTERN:&str="41 0F B6 9E 61 01 00 00 D0 EB 80 E3 01 4D 85 E4 74 3E 49 8B 8E 28 02 00 00 48 85 C9 74 32 48 8B 01 FF 50 10 83 F8 03 75 27 49 8B 04 24 49 8B CC FF 90 A0 01 00 00 48 85 C0 74 15 48 8B 00 48 85 C0 74 0B 0F B6 80 5D 01 00 00 D0 E8 24 01 0A D8 84 DB 74 30 49 8B CF E8 35 1D FF FF";
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());

unsafe fn add(active:bool,module:usize,source:usize,hit:usize,attack:*mut f32){
 if !active||module==0||source==0||hit==0||attack.is_null()
  ||read_i32(hit+0x40)!=balloon::ATTACK||read_i32(hit+0x44)!=1
  ||ptr::read((hit+0x2c1)as*const u8)&8!=0{return;}
 let target=read_ptr(module+8);if target==0||target==source{return;}
 let stats=native_actor::stats(target);if stats==0||read_i32(stats+0x140)<=0{return;}
 let maximum=read_i32(stats+0x144);if maximum<=0{return;}
 let physical=ptr::read_unaligned(attack);
 if !physical.is_finite()||physical<0.{return;}
 let value=physical+maximum as f32*0.07;
 if value.is_finite(){ptr::write_unaligned(attack,value);}
}
unsafe extern "system" fn input(module:usize,source:usize,hit:usize,attack:*mut f32){
 // Ownership may already be cleared by death. The private PC attack is the
 // provenance, also when a remote owner's hit is reconstructed locally.
 add(ACTIVE.load(Ordering::Acquire),module,source,hit,attack);
}
#[unsafe(naked)]
pub unsafe extern "system" fn detour(){
 core::arch::naked_asm!(
  "pushfq", "sub rsp,0xc8",
  "mov [rsp+0x20],rax", "mov [rsp+0x28],rcx", "mov [rsp+0x30],rdx",
  "mov [rsp+0x38],r8", "mov [rsp+0x40],r9", "mov [rsp+0x48],r10", "mov [rsp+0x50],r11",
  "movdqu [rsp+0x60],xmm0", "movdqu [rsp+0x70],xmm1", "movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3", "movdqu [rsp+0xa0],xmm4", "movdqu [rsp+0xb0],xmm5",
  "mov rcx,r15", "mov rdx,r12", "mov r8,r14", "lea r9,[rbp-0x78]", "call {callback}",
  "movdqu xmm0,[rsp+0x60]", "movdqu xmm1,[rsp+0x70]", "movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]", "movdqu xmm4,[rsp+0xa0]", "movdqu xmm5,[rsp+0xb0]",
  "mov rax,[rsp+0x20]", "mov rcx,[rsp+0x28]", "mov rdx,[rsp+0x30]",
  "mov r8,[rsp+0x38]", "mov r9,[rsp+0x40]", "mov r10,[rsp+0x48]", "mov r11,[rsp+0x50]",
  "add rsp,0xc8", "popfq", "jmp qword ptr [rip+{original}]",
  callback=sym input,original=sym ORIGINAL
 );
}
#[cfg(test)]#[path="balloon_target_tests.rs"]mod tests;
