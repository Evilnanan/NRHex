//! Only the original critical victim of this particular wave bypasses immunity.
use crate::{ptr,read_ptr,read_i32,ACTIVE};
use std::{cell::Cell,sync::atomic::{AtomicPtr,Ordering}};
pub const RVA:usize=0x3df380;
pub const PATTERN:&str="40 55 56 41 56 48 83 EC 30 48 8B 81 B8 01 00 00 48 8B F1 45 0F B6 F0 48 8B EA 48 8B 48 08 F6 41 58 01";
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub const GATE_RVA:usize=0x436638;
pub const GATE_PATTERN:&str="80 BF 21 01 00 00 02 0F 85 96 00 00 00 C6 44 24 20 01";
pub static GATE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static GATE_RESUME:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
thread_local!{static CONTACT:Cell<Option<(usize,usize)>>=const{Cell::new(None)};}
pub struct Guard(Option<(usize,usize)>);
impl Drop for Guard{fn drop(&mut self){CONTACT.with(|s|s.set(self.0));}}
pub fn contact(target:Option<(usize,usize)>)->Guard{Guard(CONTACT.with(|s|s.replace(target)))}
// Supply only the comparison operand. Keep the hit's ordinary classification,
// all actor flags, and the native protected-hit continuation (extra=1) intact.
unsafe extern "system" fn gate_operand(module:usize,hit:usize)->u8{
 let actual=ptr::read((hit+0x121)as*const u8);
 if ACTIVE.load(Ordering::Acquire)&&module!=0&&CONTACT.with(|s|s.get()==Some((read_ptr(module+8),hit)))
  &&read_i32(hit+0x40)==crate::soul_critical::ATTACK&&read_i32(hit+0x44)==1{2}else{actual}
}
#[unsafe(naked)]
pub unsafe extern "system" fn gate_detour(){
 core::arch::naked_asm!(
  "pushfq", "sub rsp,0xc8",
  "mov [rsp+0x20],rax", "mov [rsp+0x28],rcx", "mov [rsp+0x30],rdx", "mov [rsp+0x38],r8",
  "mov [rsp+0x40],r9", "mov [rsp+0x48],r10", "mov [rsp+0x50],r11",
  "movdqu [rsp+0x60],xmm0", "movdqu [rsp+0x70],xmm1", "movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3", "movdqu [rsp+0xa0],xmm4", "movdqu [rsp+0xb0],xmm5",
  "mov rcx,rsi", "mov rdx,rdi", "call {operand}", "cmp al,2",
  "pushfq", "pop rax", "mov [rsp+0xc8],rax",
  "mov rax,[rsp+0x20]", "mov rcx,[rsp+0x28]", "mov rdx,[rsp+0x30]", "mov r8,[rsp+0x38]",
  "mov r9,[rsp+0x40]", "mov r10,[rsp+0x48]", "mov r11,[rsp+0x50]",
  "movdqu xmm0,[rsp+0x60]", "movdqu xmm1,[rsp+0x70]", "movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]", "movdqu xmm4,[rsp+0xa0]", "movdqu xmm5,[rsp+0xb0]",
  "add rsp,0xc8", "popfq", "jmp qword ptr [rip+{resume}]",
  operand=sym gate_operand,resume=sym GATE_RESUME
 );
}
// Three exact, finite f32 integers carry all 64 handle bits through the
// existing four-coefficient local/network transport. The last limb is tagged;
// old neutral coefficients cannot accidentally identify a victim.
pub fn encode(target:u64)->[f32;3]{[(target&0x3fffff)as f32,((target>>22)&0x3fffff)as f32,((target>>44)as u32+0x100000)as f32]}
pub fn decode(v:[f32;3])->Option<u64>{
 if v.iter().any(|v|!v.is_finite()||v.fract()!=0.)||!(0. ..4194304.).contains(&v[0])||!(0. ..4194304.).contains(&v[1])||!(1048576. ..2097152.).contains(&v[2]){return None;}
 let h=v[0]as u64|((v[1]as u64)<<22)|(((v[2]as u64)-0x100000)<<44);
 (h!=0&&h!=u64::MAX).then_some(h)
}
unsafe fn bypass(actor:usize,reference:usize)->bool{
 if !ACTIVE.load(Ordering::Acquire)||actor==0||reference==0{return false;}
 CONTACT.with(|s|s.get().is_some_and(|(target,hit)|target==actor&&hit!=0
  &&read_i32(hit+0x40)==crate::soul_critical::ATTACK&&read_i32(hit+0x44)==1
  &&read_i32(reference)==crate::soul_critical::ATTACK&&read_i32(reference+8)==1&&read_ptr(reference+0x10)!=0))
}
pub unsafe extern "system" fn immunity(actor:usize,reference:usize,force:u8)->u8{
 if bypass(actor,reference){return 0;}
 let original:unsafe extern "system" fn(usize,usize,u8)->u8=std::mem::transmute(ORIGINAL.load(Ordering::Acquire));
 original(actor,reference,force)
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn victim_transport_roundtrips_full_handles_and_rejects_old_or_malformed_payloads(){
  for h in [1,123,0xffff_ffff_10e0_0000,u64::MAX-1,1<<22,1<<44]{assert_eq!(decode(encode(h)),Some(h));}
  for h in [0,u64::MAX]{assert_eq!(decode(encode(h)),None);}
  for v in [[1.;3],[0.,0.,0.],[f32::NAN,0.,1048576.],[0.,f32::INFINITY,1048576.],[-1.,0.,1048576.],[0.5,0.,1048576.],[4194304.,0.,1048576.],[0.,0.,2097152.]]{assert_eq!(decode(v),None);}
 }
}
