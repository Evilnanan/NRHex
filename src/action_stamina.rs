//! Route only accepted native action costs. Ordinary fees retain their native
//! immunity checks; selected fees enter the same setter after those checks.
use crate::{ptr,read_i32,read_ptr,ACTIVE,BASE};
use std::sync::atomic::{AtomicPtr,Ordering};
pub const RVA:usize=0x384aa1;
pub const PATTERN:&str="E8 2A 17 0A 00 48 8B 03 F3 0F 10 8B C4 00 00 00 48 8B 88 B8 01 00 00";
pub const CLAMP_RVA:usize=0x427f69;
pub const CLAMP_PATTERN:&str="48 8D 8B 60 01 00 00 83 FF CE 7D 16 B8 CE FF FF FF 89 83 5C 01 00 00 48 8B 5C 24 30 48 83 C4 20 5F C3 3B 39 48 8D 44 24 38 48 0F 4F C1 8B 00 89 83 5C 01 00 00 48";
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static CONTINUE:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static CLAMP:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub const MELEE_RVA:usize=0x434783;
pub const MELEE_PATTERN:&str="E8 48 1A FF FF 48 8B CE E8 90 79 FF FF 8B 54 24 48 48 8B C8 4C 8B 00 41";
pub static MELEE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static MELEE_CONTINUE:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());

unsafe extern "system" fn melee(actor:usize,stats:usize,delta:i32,weapon:i32,behavior:i32){
 let base=BASE.load(Ordering::Acquire);
 let add:unsafe extern "system" fn(usize,i32,u8)=std::mem::transmute(base+crate::STAMINA_ADD_RVA);
 let eligible=ACTIVE.load(Ordering::Acquire)&&delta<0&&delta!=i32::MIN&&stats!=0&&crate::is_local(actor,base)&&crate::native_actor::stats(actor)==stats;
 let adjusted=if eligible{-crate::onyx_mastery::melee_cost(actor,weapon,behavior,-delta,base)}else{delta};
 add(stats,adjusted,0);
}
// Native melee hitbox creation pays separately from bullet/spell contexts.
// RAX is the owner, [RBP+98] the resolved weapon in the attack descriptor,
// and [original RSP+50] the native BehaviorParam reference's ID.
#[unsafe(naked)]pub unsafe extern "system" fn melee_detour(){core::arch::naked_asm!(
 "sub rsp,0xd0",
 "mov [rsp+0x30],rax", "mov [rsp+0x38],rcx", "mov [rsp+0x40],rdx",
 "mov [rsp+0x48],r8", "mov [rsp+0x50],r9", "mov [rsp+0x58],r10", "mov [rsp+0x60],r11",
 "movdqu [rsp+0x70],xmm0", "movdqu [rsp+0x80],xmm1", "movdqu [rsp+0x90],xmm2",
 "movdqu [rsp+0xa0],xmm3", "movdqu [rsp+0xb0],xmm4", "movdqu [rsp+0xc0],xmm5",
 "mov r8d,edx", "mov rdx,rcx", "mov rcx,rax", "mov r9d,[rbp+0x98]",
 "mov eax,[rsp+0x120]", "mov [rsp+0x20],eax", "call {callback}",
 "movdqu xmm0,[rsp+0x70]", "movdqu xmm1,[rsp+0x80]", "movdqu xmm2,[rsp+0x90]",
 "movdqu xmm3,[rsp+0xa0]", "movdqu xmm4,[rsp+0xb0]", "movdqu xmm5,[rsp+0xc0]",
 "mov rax,[rsp+0x30]", "mov rcx,[rsp+0x38]", "mov rdx,[rsp+0x40]",
 "mov r8,[rsp+0x48]", "mov r9,[rsp+0x50]", "mov r10,[rsp+0x58]", "mov r11,[rsp+0x60]",
 "add rsp,0xd0", "jmp qword ptr [rip+{next}]",callback=sym melee,next=sym MELEE_CONTINUE
);}

// Recreate 427F00's native frame, then enter its original clamp/write/return.
// RCX=stats, EDX=absolute stamina. Neither actor nor global immunity is changed.
#[unsafe(naked)]unsafe extern "system" fn force_set(_:usize,_:i32){core::arch::naked_asm!(
 "mov [rsp+8],rbx", "mov [rsp+0x10],edx", "push rdi", "sub rsp,0x20",
 "mov edi,edx", "mov rbx,rcx", "jmp qword ptr [rip+{clamp}]",clamp=sym CLAMP
);}
unsafe fn force_debit(stats:usize,cost:i32){
 if cost<=0{return;}
 let next=(i64::from(read_i32(stats+0x15c))-i64::from(cost)).max(i64::from(i32::MIN)) as i32;
 force_set(stats,next);
}
unsafe fn storm(context:usize,actor:usize,base:usize)->bool{
 if !storm_behavior(read_i32(context+8))||read_i32(context+0xc)>=0||!crate::has_effect(actor,crate::arts::OWNER,base){return false;}
 let Some(w)=crate::arts_source_weapon(actor,read_i32(context+0x38),base)else{return false;};
 w.original!=crate::arts::STORM&&crate::arts_actor_selection(actor,w,base)==Some(crate::arts::STORM)
}
fn storm_behavior(behavior:i32)->bool{
 // Native common behavior families for different weapon categories. All four
 // carry the same Storm normal/empty-FP branches in standalone and MMV.
 matches!(behavior/1000,400000|400100|400500|400900)&&matches!(behavior%1000,220|225|230|235|240|245)
}
unsafe extern "system" fn consume(context:usize,stats:usize,delta:i32,force:u8){
 let base=BASE.load(Ordering::Acquire);
 let add:unsafe extern "system" fn(usize,i32,u8)=std::mem::transmute(base+crate::STAMINA_ADD_RVA);
 let actor=if context!=0{read_ptr(context)}else{0};
 if !ACTIVE.load(Ordering::Acquire)||delta>=0||delta==i32::MIN||stats==0||!crate::is_local(actor,base)||crate::native_actor::stats(actor)!=stats{
  add(stats,delta,force);return;
 }
 let delta=-crate::onyx_mastery::cost(context,actor,-delta,base);
 if storm(context,actor,base){force_debit(stats,-delta);return;}
 let extra=crate::echo_cost::extra(context,-delta,base);
 add(stats,delta,force);
 force_debit(stats,extra);
}
// Native CALL has not pushed its return address: RSP is 16-byte aligned.
// RBX is the exact context whose cached C0 fee the native caller just loaded.
#[unsafe(naked)]pub unsafe extern "system" fn detour(){core::arch::naked_asm!(
 "sub rsp,0xc0",
 "mov [rsp+0x20],rax", "mov [rsp+0x28],rcx", "mov [rsp+0x30],rdx",
 "mov [rsp+0x38],r8", "mov [rsp+0x40],r9", "mov [rsp+0x48],r10", "mov [rsp+0x50],r11",
 "movdqu [rsp+0x60],xmm0", "movdqu [rsp+0x70],xmm1", "movdqu [rsp+0x80],xmm2",
 "movdqu [rsp+0x90],xmm3", "movdqu [rsp+0xa0],xmm4", "movdqu [rsp+0xb0],xmm5",
 "mov r9,r8", "mov r8,rdx", "mov rdx,rcx", "mov rcx,rbx", "call {callback}",
 "movdqu xmm0,[rsp+0x60]", "movdqu xmm1,[rsp+0x70]", "movdqu xmm2,[rsp+0x80]",
 "movdqu xmm3,[rsp+0x90]", "movdqu xmm4,[rsp+0xa0]", "movdqu xmm5,[rsp+0xb0]",
 "mov rax,[rsp+0x20]", "mov rcx,[rsp+0x28]", "mov rdx,[rsp+0x30]",
 "mov r8,[rsp+0x38]", "mov r9,[rsp+0x40]", "mov r10,[rsp+0x48]", "mov r11,[rsp+0x50]",
 "add rsp,0xc0", "jmp qword ptr [rip+{next}]",callback=sym consume,next=sym CONTINUE
);}
#[cfg(test)]#[path="action_stamina_tests.rs"]mod tests;
