//! Ram supplies a velocity-derived base attack, then keeps native melee/skill
//! aggregation, source buffs, network copies and recipient defenses intact.
use crate::{ptr,read_i32,ACTIVE};
use std::sync::atomic::{AtomicPtr,Ordering};
pub const RVA:usize=0x6c49d6;
pub const PATTERN:&str="0F B6 4F 44 8B D1 0F 28 C8 85 C9 74 5F 83 EA 01 74 49";
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
// 6C3200 has converted the fixed physical base to XMM8; 6D1CC0 has
// returned its native scale in XMM0. Preserve both fractional base precision
// and that independent scale. RDI is the current attack context.
unsafe extern "system" fn base(context:usize,native:f32)->f32{
 if !ACTIVE.load(Ordering::Acquire){return native;}
 crate::static_charge::ram_base(context).unwrap_or(native)
}
pub unsafe fn categories(context:usize){
 if context==0||read_i32(context+0xc)!=1||!crate::lightning_ram::attack(read_i32(context+8)){return;}
 // Same categories as native rolling attack 300000711, before 502830
 // caches source AttackRate. No local-owner gate: peers rebuild their own hit.
 let mut mask=ptr::read_unaligned((context+0x14)as*const[u32;8]);
 for tag in [112usize,128,130]{mask[tag/32]|=1<<(tag%32);}
 ptr::write_unaligned((context+0x14)as*mut[u32;8],mask);
}
#[unsafe(naked)]
pub unsafe extern "system" fn detour(){
 core::arch::naked_asm!(
  "pushfq", "sub rsp,0xc8",
  "mov [rsp+0x20],rax", "mov [rsp+0x28],rcx", "mov [rsp+0x30],rdx", "mov [rsp+0x38],r8",
  "mov [rsp+0x40],r9", "mov [rsp+0x48],r10", "mov [rsp+0x50],r11",
  "movdqu [rsp+0x60],xmm0", "movdqu [rsp+0x70],xmm1", "movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3", "movdqu [rsp+0xa0],xmm4", "movdqu [rsp+0xb0],xmm5",
  "mov rcx,rdi", "movaps xmm1,xmm8", "call {callback}", "movss xmm8,xmm0",
  "mov rax,[rsp+0x20]", "mov rcx,[rsp+0x28]", "mov rdx,[rsp+0x30]", "mov r8,[rsp+0x38]",
  "mov r9,[rsp+0x40]", "mov r10,[rsp+0x48]", "mov r11,[rsp+0x50]",
  "movdqu xmm0,[rsp+0x60]", "movdqu xmm1,[rsp+0x70]", "movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]", "movdqu xmm4,[rsp+0xa0]", "movdqu xmm5,[rsp+0xb0]",
  "add rsp,0xc8", "popfq", "jmp qword ptr [rip+{original}]",
  callback=sym base,original=sym ORIGINAL
 );
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn native_skill_categories_are_early_idempotent_and_preserve_other_tags(){unsafe{
  for (id,table,wanted) in [(crate::lightning_ram::ATTACK,1,true),(crate::lightning_ram::FAST_ATTACK,1,true),(crate::lightning_ram::EXTREME_ATTACK,1,true),(crate::lightning_ram::ATTACK,0,false),(crate::static_charge::ATTACK,1,false),(crate::soul_critical::ATTACK,1,false),(300000711,1,false)]{
   let mut c=[0u8;0x120];let p=c.as_mut_ptr()as usize;ptr::write_unaligned((p+8)as*mut i32,id);ptr::write_unaligned((p+12)as*mut i32,table);c[0x14]=2;let before=c;categories(p);let after=c;categories(p);assert_eq!(c,after);
   for bit in 0..256{assert_eq!(c[0x14+bit/8]&(1<<(bit%8))!=0,bit==1||(wanted&&[112,128,130].contains(&bit)));}
   if !wanted{assert_eq!(c,before);}
   assert_eq!(&c[..0x14],&before[..0x14]);assert_eq!(&c[0x34..],&before[0x34..]);
  }
 }}
}
#[cfg(test)]#[path="ram_damage_native_tests.rs"]mod native_tests;
