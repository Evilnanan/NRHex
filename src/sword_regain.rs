use crate::{has_effect,is_local,read_i32,read_ptr,sword,spell_percentage,ACTIVE,BASE};
use std::{ptr,sync::atomic::{AtomicPtr,Ordering}};

pub const DIRECT_RVA:usize=0x33c4b0;
pub const DIRECT_PATTERN:&str="4C 8B C6 8B D0 48 8B CF E8 53 E0 09 00 41 0F B6 C6";
pub const RECEIVED_RVA:usize=0x43f3e5;
pub const RECEIVED_PATTERN:&str="4C 8B C6 8B D0 49 8B CE E8 1E B1 F9 FF B0 01";
pub const DESCRIPTOR_RVA:usize=0x50f230;
pub const DESCRIPTOR_PATTERN:&str="0F B6 82 22 01 00 00 88 01 0F 10 82 24 01 00 00 0F 11 41 04";
pub const RATE_RVA:usize=0x507bc0;
pub const RATE_PATTERN:&str="48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 40 48 8B 59 08 48 8B F2 0F 29 74 24 30 48 8B F9 F3 0F 10 35";
pub static DIRECT_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static RECEIVED_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());

fn select(original:u32,percentage:Option<f32>,damage:i32,nullified:bool,rate:impl FnOnce()->f32)->u32{
 let Some(percentage)=percentage else{return original;};
 let requested=sword::heal_amount(damage,percentage,nullified);
 if requested==0{return 0;}
 let rate=rate();let rate=if rate.is_finite()&&rate>=0.{rate}else{1.};
 (requested as f64*rate as f64).floor().clamp(0.,65535.) as u32
}

unsafe fn native_rate(source:usize,hit:usize,base:usize)->f32{
 let manager=read_ptr(source+0x190);if manager==0{return 1.;}
 let describe:unsafe extern "system" fn(*mut u8,usize)->*mut u8=std::mem::transmute(base+DESCRIPTOR_RVA);
 let calculate:unsafe extern "system" fn(usize,*const u8)->f32=std::mem::transmute(base+RATE_RVA);
 let mut descriptor=[0u8;0x3c];describe(descriptor.as_mut_ptr(),hit);descriptor[0]=1;
 calculate(manager,descriptor.as_ptr())
}

unsafe extern "system" fn amount(source:usize,hit:usize,original:u32)->u32{
 if !ACTIVE.load(Ordering::Acquire)||hit==0{return original;}
 let base=BASE.load(Ordering::Acquire);
 if !is_local(source,base)||!has_effect(source,sword::OWNER,base){return original;}
 select(original,spell_percentage(hit),read_i32(hit+0x280),ptr::read((hit+0x2c3)as*const u8)&2!=0,||native_rate(source,hit,base))
}

macro_rules! regain_detour {
 ($name:ident,$source:literal,$hit:literal,$original:ident)=>{
  #[unsafe(naked)]
  pub unsafe extern "system" fn $name(){
   core::arch::naked_asm!(
    "pushfq","sub rsp,0xc8",
    "mov [rsp+0x28],rcx","mov [rsp+0x30],rdx","mov [rsp+0x38],r8",
    "mov [rsp+0x40],r9","mov [rsp+0x48],r10","mov [rsp+0x50],r11",
    "movdqu [rsp+0x60],xmm0","movdqu [rsp+0x70],xmm1","movdqu [rsp+0x80],xmm2",
    "movdqu [rsp+0x90],xmm3","movdqu [rsp+0xa0],xmm4","movdqu [rsp+0xb0],xmm5",
    "mov r8d,eax",concat!("mov rdx,",$hit),concat!("mov rcx,",$source),"call {callback}",
    "mov rcx,[rsp+0x28]","mov rdx,[rsp+0x30]","mov r8,[rsp+0x38]",
    "mov r9,[rsp+0x40]","mov r10,[rsp+0x48]","mov r11,[rsp+0x50]",
    "movdqu xmm0,[rsp+0x60]","movdqu xmm1,[rsp+0x70]","movdqu xmm2,[rsp+0x80]",
    "movdqu xmm3,[rsp+0x90]","movdqu xmm4,[rsp+0xa0]","movdqu xmm5,[rsp+0xb0]",
    "add rsp,0xc8","popfq","jmp qword ptr [rip+{original}]",
    callback=sym amount,original=sym $original
   );
  }
 };
}
regain_detour!(direct,"rdi","rbx",DIRECT_ORIGINAL);
regain_detour!(received,"r14","rdi",RECEIVED_ORIGINAL);

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn native_weapon_amounts_are_untouched(){
  for original in [0,20,40,140,65535]{assert_eq!(select(original,None,300,false,||panic!("Weapon rate must not be applied twice")),original);}
 }
 #[test]fn accepted_sword_hits_replace_native_regain_without_caster_distance_gate(){
  for original in [0,20,25,27,140]{assert_eq!(select(original,Some(30.),300,false,||1.),180);}
  assert_eq!(select(20,Some(0.),300,false,||panic!("Zero regain")),0);
  for damage in [-1,0,300]{assert_eq!(select(20,Some(30.),damage,true,||panic!("Nullified")),0);}
  assert_eq!(select(20,Some(30.),0,false,||panic!("No damage")),0);
 }
 #[test]fn spell_regain_uses_native_tier_once_and_retains_cap(){
  for original in [0,20,25,27,140]{for (rate,expected) in [(1.,180),(1.25,225),(1.35,243)]{
   let mut calls=0;assert_eq!(select(original,Some(30.),300,false,||{calls+=1;rate}),expected);assert_eq!(calls,1);
  }}
  assert_eq!(select(20,Some(30.),3,false,||1.35),1);
  assert_eq!(select(20,Some(100.),100000,false,||1.35),65535);
  assert_eq!(select(20,Some(30.),300,false,||0.),0);
  for rate in [f32::NAN,f32::INFINITY,-1.]{assert_eq!(select(20,Some(30.),300,false,||rate),180);}
 }
 #[unsafe(naked)]
 unsafe extern "system" fn fixture_direct(_:usize)->u32{
  core::arch::naked_asm!("push rbx","push rdi","sub rsp,0x28","mov rdi,rcx","xor ebx,ebx","mov eax,123","mov r8,456","mov r9,789","jmp {detour}",detour=sym direct);
 }
 #[unsafe(naked)]
 unsafe extern "system" fn fixture_received(_:usize)->u32{
  core::arch::naked_asm!("push rdi","push r14","sub rsp,0x28","mov r14,rcx","xor edi,edi","mov eax,123","mov r8,456","mov r9,789","jmp {detour}",detour=sym received);
 }
 #[unsafe(naked)]
 unsafe extern "system" fn resume_direct(){
  core::arch::naked_asm!("mov [rdi],rax","mov [rdi+8],rcx","mov [rdi+16],r8","mov [rdi+24],r9","add rsp,0x28","pop rdi","pop rbx","ret");
 }
 #[unsafe(naked)]
 unsafe extern "system" fn resume_received(){
  core::arch::naked_asm!("mov [r14],rax","mov [r14+8],rcx","mov [r14+16],r8","mov [r14+24],r9","add rsp,0x28","pop r14","pop rdi","ret");
 }
 #[test]fn both_mid_hooks_preserve_native_registers_when_inapplicable(){unsafe{
  DIRECT_ORIGINAL.store(resume_direct as*mut u8,Ordering::Release);
  RECEIVED_ORIGINAL.store(resume_received as*mut u8,Ordering::Release);
  for fixture in [fixture_direct,fixture_received]{let mut values=[0usize;4];let address=values.as_mut_ptr()as usize;assert_eq!(fixture(address),123);assert_eq!(values,[123,address,456,789]);}
 }}
}
