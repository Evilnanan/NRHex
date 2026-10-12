//! Supply Balloon's base physical attack; native code owns every modifier.
use crate::{balloon, native_actor, ptr, read_i32, read_ptr, ACTIVE, BASE, BALLOON, CHR_HANDLE_RVA};
use std::sync::atomic::{AtomicPtr, Ordering};

// PC fixed-attack branch of 6C3200: ECX is AtkParam.atkPhys before rates.
// RDI is the attack context; replay MOVD XMM8,ECX with only ECX substituted.
pub const BASE_RVA:usize=0x6c49bb;
pub const BASE_PATTERN:&str="66 44 0F 6E C1 48 8D 55 A8 49 8B CD 45 33 C9 4C 8B C7 45 0F 5B C0 E8 ? ? ? ?";
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());

unsafe fn fixed_attack(context:usize){
 if context!=0&&read_i32(context+8)==balloon::ATTACK&&read_i32(context+0xc)==1 {
  // Native 6C3200 selects its fixed-attack branch with context+F8. The
  // ordinary branch uses equipped-weapon attack, unsuitable for HP scaling.
  // This per-attack context is copied by E70350 before cached/network hits.
  ptr::write((context+0xf8) as *mut u8,1);
 }
}
pub unsafe extern "system" fn before_construct(context:usize){
 if ACTIVE.load(Ordering::Acquire){fixed_attack(context);}
}

unsafe fn source_actor(context:usize,resolve:unsafe extern "system" fn(u64)->usize)->usize {
 let source=read_ptr(context);if source==0{return 0;}
 let vtable=read_ptr(source);if vtable==0{return 0;}
 let get=read_ptr(vtable+0x10);if get==0{return 0;}
 let kind:unsafe extern "system" fn(usize)->i32=std::mem::transmute(get);
 // Same source kinds as native 6C2DD0. Only bullets have owner handle +B0.
 match kind(source){1=>source,3=>resolve(ptr::read_unaligned((source+0xb0) as *const u64)),_=>0}
}
unsafe fn select_base(context:usize,original:u32,resolve:unsafe extern "system" fn(u64)->usize,maximum:impl FnOnce(usize)->Option<i32>)->u32 {
 if context==0||read_i32(context+8)!=balloon::ATTACK||read_i32(context+0xc)!=1{return original;}
 let actor=source_actor(context,resolve);if actor==0{return original;}
 maximum(actor).filter(|v|*v>0).map_or(original,|v|v as u32)
}
unsafe extern "system" fn base_attack(context:usize,original:u32)->u32 {
 if !ACTIVE.load(Ordering::Acquire){return original;}
 let base=BASE.load(Ordering::Acquire);
 select_base(context,original,std::mem::transmute(base+CHR_HANDLE_RVA),|actor|{
  let stats=native_actor::stats(actor);if stats==0{return None;}
  // Local emission retains HP through death cleanup. Remote/native rebuilt
  // contexts use their own actor's HP, never a global last-emitter value.
  let captured=BALLOON.get().and_then(|s|s.lock().ok()?.base_hp_for(actor,stats));
  captured.or_else(||Some(read_i32(stats+0x144)))
 })
}
#[unsafe(naked)]
pub unsafe extern "system" fn base_detour(){
 core::arch::naked_asm!(
  "pushfq", "sub rsp,0xc8",
  "mov [rsp+0x20],rax", "mov [rsp+0x30],rdx", "mov [rsp+0x38],r8",
  "mov [rsp+0x40],r9", "mov [rsp+0x48],r10", "mov [rsp+0x50],r11",
  "movdqu [rsp+0x60],xmm0", "movdqu [rsp+0x70],xmm1", "movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3", "movdqu [rsp+0xa0],xmm4", "movdqu [rsp+0xb0],xmm5",
  "mov edx,ecx", "mov rcx,rdi", "call {callback}",
  "mov ecx,eax", "mov rax,[rsp+0x20]", "mov rdx,[rsp+0x30]", "mov r8,[rsp+0x38]",
  "mov r9,[rsp+0x40]", "mov r10,[rsp+0x48]", "mov r11,[rsp+0x50]",
  "movdqu xmm0,[rsp+0x60]", "movdqu xmm1,[rsp+0x70]", "movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]", "movdqu xmm4,[rsp+0xa0]", "movdqu xmm5,[rsp+0xb0]",
  "add rsp,0xc8", "popfq", "jmp qword ptr [rip+{original}]",
  callback=sym base_attack,original=sym ORIGINAL
 );
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn only_balloon_context_selects_native_fixed_attack(){unsafe{
  let mut context=[0xabu8;0x120];context[8..12].copy_from_slice(&balloon::ATTACK.to_le_bytes());
  context[12..16].copy_from_slice(&1i32.to_le_bytes());let before=context;
  fixed_attack(context.as_mut_ptr() as usize);
  for i in 0..context.len(){assert_eq!(context[i],if i==0xf8{1}else{before[i]});}
  context[8..12].copy_from_slice(&12064105i32.to_le_bytes());context[0xf8]=0;
  let before=context;fixed_attack(context.as_mut_ptr() as usize);assert_eq!(context,before);
 }}
 #[unsafe(naked)]
 unsafe extern "system" fn fixture(_:usize,_:u32,_:f64)->u32{
  core::arch::naked_asm!(
   "push rdi", "sub rsp,0x30", "movdqu [rsp+0x20],xmm8", "mov rdi,rcx", "mov ecx,edx", "mov eax,0x12345678",
   "mov r8,0x8877", "mov r9,0x9988", "mov r10,0xaa99", "mov r11,0xbbaa",
   "movaps xmm0,xmm2", "movaps xmm1,xmm2", "movaps xmm3,xmm2", "movaps xmm4,xmm2", "movaps xmm5,xmm2",
   "jmp {detour}",detour=sym base_detour
  );
 }
 #[unsafe(naked)]
 unsafe extern "system" fn fixture_site(){
  core::arch::naked_asm!(
   "movd xmm8,ecx",
   "mov [rdi+0x120],rdx", "mov [rdi+0x128],r8", "mov [rdi+0x130],r9",
   "mov [rdi+0x138],r10", "mov [rdi+0x140],r11", "mov [rdi+0x148],rax",
   "movq [rdi+0x150],xmm0", "movq [rdi+0x158],xmm1", "movq [rdi+0x160],xmm2",
   "movq [rdi+0x168],xmm3", "movq [rdi+0x170],xmm4", "movq [rdi+0x178],xmm5",
   "movd eax,xmm8", "movdqu xmm8,[rsp+0x20]", "add rsp,0x30", "pop rdi", "ret"
  );
 }
 #[test]fn physical_base_mid_hook_preserves_native_row_and_registers(){unsafe{
  ACTIVE.store(false,Ordering::Release);
  let p=crate::hook::prepare(fixture_site as *mut u8,base_detour as *const u8,64).unwrap();
  assert_eq!(p.stolen,5);ORIGINAL.store(p.trampoline,Ordering::Release);
  let mut context=[0u64;0x200/8];let addr=context.as_mut_ptr() as usize;
  for value in [0u32,1,65535,70000]{
   assert_eq!(fixture(addr,value,17.25),value);
   assert_eq!(&context[0x120/8..0x150/8],&[value as u64,0x8877,0x9988,0xaa99,0xbbaa,0x12345678]);
   for i in (0x150..0x180).step_by(8){assert_eq!(f64::from_bits(context[i/8]),17.25);}
  }
 }}
 unsafe extern "system" fn kind(p:usize)->i32{read_i32(p+8)}
 unsafe extern "system" fn resolve(handle:u64)->usize{if handle==77{123}else{0}}
 #[test]fn base_attack_resolves_each_source_and_never_changes_unrelated_attacks(){unsafe{
  let mut actor=[0usize;0x200/8];let mut vt=[0usize;4];vt[2]=kind as *const () as usize;
  actor[0]=vt.as_ptr() as usize;actor[1]=1;let mut ctx=[0u32;0x120/4];
  let context=ctx.as_mut_ptr() as usize;ptr::write_unaligned(context as *mut usize,actor.as_ptr() as usize);
  ctx[2]=balloon::ATTACK as u32;ctx[3]=1;
  assert_eq!(select_base(context,1,resolve,|p|{assert_eq!(p,actor.as_ptr() as usize);Some(70000)}),70000);
  actor[1]=3;actor[0xb0/8]=77;
  assert_eq!(select_base(context,1,resolve,|p|{assert_eq!(p,123);Some(2345)}),2345);
  actor[0xb0/8]=88;assert_eq!(select_base(context,1,resolve,|_|panic!("stale handle")),1);
  actor[1]=2;assert_eq!(select_base(context,1,resolve,|_|panic!("other source kind")),1);
  actor[1]=1;assert_eq!(select_base(context,1,resolve,|_|Some(0)),1);
  ctx[2]=12064105;assert_eq!(select_base(context,42,resolve,|_|panic!("native golden wave")),42);
  ctx[2]=balloon::ATTACK as u32;ctx[3]=0;assert_eq!(select_base(context,42,resolve,|_|panic!("NPC attack table")),42);
  assert_eq!(select_base(0,42,resolve,|_|panic!()),42);
 }}
}
