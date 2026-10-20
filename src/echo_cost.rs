//! Spell cost context, after the actor's native stamina modifiers and before
//! caching its negative cost. Replays enter BulletMan, not this constructor.
use crate::{has_effect,is_local,read_ptr,ACTIVE,BASE};
use std::sync::atomic::{AtomicPtr,Ordering};

pub const RVA:usize=0x3849cf;
pub const PATTERN:&str="0F B6 D3 48 8D 4D C7 F7 D8 89 87 C0 00 00 00 E8 4D D8 B0 00";
pub const GETTER_RVA:usize=0xe92250;
pub const GETTER_PATTERN:&str="48 8B 41 08 48 85 C0 74 13 84 D2 75 05 0F BF 40 12 C3 80 FA 01 75 05 0F";
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(std::ptr::null_mut());

fn adjusted(cost:i32,kind:u8,owned:bool)->i32{
 if owned&&matches!(kind,0|1)&&cost>0{cost.saturating_mul(2)}else{cost}
}
unsafe extern "system" fn stamina(context:usize,row:usize,cost:i32)->i32{
 let base=BASE.load(Ordering::Acquire);
 if !ACTIVE.load(Ordering::Acquire)||base==0||context==0||row==0||cost<=0{return cost;}
 let kind=std::ptr::read((row+0x26)as*const u8);
 if !matches!(kind,0|1){return cost;}
 let actor=read_ptr(context);
 adjusted(cost,kind,actor!=0&&is_local(actor,base)&&has_effect(actor,crate::echo_cast::OWNER,base))
}

// Interior RSP is 16-byte aligned. Save every volatile register; only EAX is
// replaced. The stolen MOVZX/LEA and original NEG/store still run afterwards.
#[unsafe(naked)]
pub unsafe extern "system" fn detour(){
 core::arch::naked_asm!(
  "sub rsp,0xc0",
  "mov [rsp+0x20],rax", "mov [rsp+0x28],rcx", "mov [rsp+0x30],rdx",
  "mov [rsp+0x38],r8", "mov [rsp+0x40],r9", "mov [rsp+0x48],r10", "mov [rsp+0x50],r11",
  "movdqu [rsp+0x60],xmm0", "movdqu [rsp+0x70],xmm1", "movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3", "movdqu [rsp+0xa0],xmm4", "movdqu [rsp+0xb0],xmm5",
  "mov rcx,rdi", "mov rdx,[rbp-0x31]", "mov r8d,eax", "call {callback}",
  "mov [rsp+0x20],rax",
  "movdqu xmm0,[rsp+0x60]", "movdqu xmm1,[rsp+0x70]", "movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]", "movdqu xmm4,[rsp+0xa0]", "movdqu xmm5,[rsp+0xb0]",
  "mov rax,[rsp+0x20]", "mov rcx,[rsp+0x28]", "mov rdx,[rsp+0x30]",
  "mov r8,[rsp+0x38]", "mov r9,[rsp+0x40]", "mov r10,[rsp+0x48]", "mov r11,[rsp+0x50]",
  "add rsp,0xc0", "jmp qword ptr [rip+{original}]",
  callback=sym stamina,original=sym ORIGINAL
 );
}
#[cfg(test)]#[path="echo_cost_native_tests.rs"]mod native_tests;
