//! Ram supplies a velocity-derived base attack, then keeps native melee/skill
//! aggregation, source buffs, network copies and recipient defenses intact.
use crate::{ptr,read_i32,ACTIVE};
use std::sync::atomic::{AtomicPtr,Ordering};
pub const RVA:usize=0x6c49d6;
pub const PATTERN:&str="0F B6 4F 44 8B D1 0F 28 C8 85 C9 74 5F 83 EA 01 74 49";
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
// These are the native low-speed/start contacts, plus the three swept tiers.
// Exclude the harmless object-break carrier (OWNER+69) and crash self-damage.
fn collision_attack(id:i32,table:i32)->bool{
 table==1&&(crate::lightning_ram::attack(id)||matches!(id,300000710|300000711|300000715|300000716))
}
fn ironjar_enabled(mut has:impl FnMut(i32)->bool)->bool{
 has(crate::lightning_ram::OWNER)&&has(crate::metal::IRONJAR)&&!has(crate::reward_cycle_runtime::RESET)
}
unsafe fn ironjar_hit(hit:usize,context:usize,resolve:unsafe extern "system" fn(u64)->usize,enabled:impl FnOnce(usize)->bool){
 if hit==0||context==0||!collision_attack(read_i32(context+8),read_i32(context+12)){return;}
 let actor=crate::ability_damage::source_actor(context,resolve);
 if actor==0||!enabled(actor){return;}
 // E70350 has finished constructing a fresh hit. Multiply its five attack
 // components and poise once, before target defenses/network copies; never
 // touch the velocity seed, cached AttackRate, impact level or rescue amount.
 for offset in [0usize,4,8,12,16,0x148]{
  let p=(hit+offset)as*mut f32;
  ptr::write_unaligned(p,ptr::read_unaligned(p)*2.);
 }
}
pub unsafe fn ironjar_constructed(hit:usize,context:usize){
 let base=crate::BASE.load(Ordering::Acquire);if base==0{return;}
 ironjar_hit(hit,context,std::mem::transmute(base+crate::CHR_HANDLE_RVA),|actor|{
  ironjar_enabled(|id|crate::has_effect(actor,id,base))
 });
}
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
 unsafe extern "system" fn kind(source:usize)->i32{read_i32(source+0x10)}
 unsafe extern "system" fn resolve(handle:u64)->usize{if handle==77{123}else{0}}
 #[test]fn ironjar_requires_actual_effect_and_ram_but_not_metallization(){
  for (ids,wanted) in [
   (vec![crate::lightning_ram::OWNER,crate::metal::IRONJAR],true),
   (vec![crate::lightning_ram::OWNER,crate::metal::IRONJAR,crate::metal::OWNER],true),
   (vec![crate::metal::IRONJAR],false),(vec![crate::lightning_ram::OWNER,crate::metal::OWNER],false),
   (vec![crate::lightning_ram::OWNER,503521],false),
   (vec![crate::lightning_ram::OWNER,crate::metal::IRONJAR,crate::reward_cycle_runtime::RESET],false),
  ]{assert_eq!(ironjar_enabled(|id|ids.contains(&id)),wanted);}
 }
 #[test]fn ironjar_scales_each_fresh_source_hit_only_preserving_impact_rescue_and_context(){unsafe{
  let mut source=[0usize;0x200/8];let mut vt=[0usize;4];vt[2]=kind as*const()as usize;
  source[0]=vt.as_ptr()as usize;source[0xb0/8]=77;
  let mut ctx=[0usize;0x180/8];let c=ctx.as_mut_ptr()as usize;
  ctx[0]=source.as_ptr()as usize;
  for id in [crate::lightning_ram::ATTACK,crate::lightning_ram::FAST_ATTACK,crate::lightning_ram::EXTREME_ATTACK,300000710,300000711,300000715,300000716,crate::lightning_ram::OWNER+69,crate::static_charge::ATTACK,crate::soul_critical::ATTACK,100000000]{
   for table in [0i32,1]{for source_kind in [1i32,3,2]{for enabled in [false,true]{
    ptr::write_unaligned((c+8)as*mut i32,id);ptr::write_unaligned((c+12)as*mut i32,table);
    ptr::write_unaligned((source.as_mut_ptr()as usize+0x10)as*mut i32,source_kind);
    let before_context=ctx;
    let mut hit=[0xabu8;0x300];let h=hit.as_mut_ptr()as usize;
    for (offset,value) in [(0,123.5f32),(4,30.),(8,0.),(12,40.),(16,20.),(0x148,45.),(0x150,60.)]{ptr::write_unaligned((h+offset)as*mut f32,value);}
    let mut expected=hit;
    if enabled&&source_kind!=2&&collision_attack(id,table){for offset in [0usize,4,8,12,16,0x148]{let value=f32::from_le_bytes(expected[offset..offset+4].try_into().unwrap())*2.;expected[offset..offset+4].copy_from_slice(&value.to_le_bytes());}}
    ironjar_hit(h,c,resolve,|actor|{assert_eq!(actor,if source_kind==1{source.as_ptr()as usize}else{123});enabled});
    assert_eq!(hit,expected,"attack {id}, table {table}, source {source_kind}, enabled {enabled}");assert_eq!(ctx,before_context);
   }}}
  }
  ptr::write_unaligned((c+8)as*mut i32,crate::lightning_ram::ATTACK);ptr::write_unaligned((c+12)as*mut i32,1);
  source[0xb0/8]=88;ptr::write_unaligned((source.as_mut_ptr()as usize+0x10)as*mut i32,3);
  let mut hit=[0u8;0x300];ironjar_hit(hit.as_mut_ptr()as usize,c,resolve,|_|panic!("unresolved owner"));assert_eq!(hit,[0;0x300]);
 }}
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
