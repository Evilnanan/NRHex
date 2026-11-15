//! Keep real Havok contacts; remove walking-only constraints during managed rolling.
use crate::{ptr, BASE, lightning_ram};
use std::sync::atomic::{AtomicPtr,Ordering};
pub const CONTACTS_RVA:usize=0x1a6c980;
pub const CONTACTS_PATTERN:&str="48 8B C4 53 57 48 83 EC 68 48 89 68 10 49 8B F9 48 89 70 18 33 ED 4C 89 68 E8";
pub static CONTACTS_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub const QUERY_RVA:usize=0x1a35a90;
pub const QUERY_PATTERN:&str="48 89 5C 24 08 57 48 83 EC 20 0F B7 42 4C 48 8B FA 4C 6B C8 70 48 8B 81 48 01 00 00";
pub static QUERY_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
// hknpCharacterProxyInternals::QueryCollector::addHit. +140 is the
// delegated collector supplied by D96A50 (+A8/+B0 of the owner wrapper).
// Reject before it can shorten the sweep fraction, not after movement has
// already been clamped by the NPC's broad body capsule. No TLS or actor-wide
// collision flag is used: projectile/scene/other actors' collectors differ.
pub unsafe extern "system" fn query(collector:usize,hit:usize){
 let base=BASE.load(Ordering::Acquire);
 if hit!=0&&lightning_ram::managed_collector(crate::read_ptr(collector+0x140),base)
  &&npc_body(base,ptr::read_unaligned((hit+0x48)as*const u32)){return;}
 let original:unsafe extern "system" fn(usize,usize)=std::mem::transmute(QUERY_ORIGINAL.load(Ordering::Acquire));
 original(collector,hit);
}
// The proxy manifold contains 0x40-byte physical contacts. 1A6C980 builds
// one solver plane per contact, then D985D0 dispatches body kind 0 (map),
// 2 (character) and 5 (asset). Remove only NPC body contacts BEFORE plane
// construction so manifold/plane indices stay aligned. Attack hurt shapes
// belong to a different pipeline and are never changed here.
unsafe fn npc_body(base:usize,id:u32)->bool{
 if id&0x00ff_ffff==0x00ff_ffff{return false;}
 let lookup:unsafe extern "system" fn(u32)->usize=std::mem::transmute(base+lightning_ram::BODY_RVA);
 let entry=lookup(id);
 if entry==0||ptr::read_unaligned((entry+8)as*const u16)!=2{return false;}
 let actor=crate::read_ptr(entry+0x10);
 crate::native_actor::npc_id(actor,base).is_some()
}
unsafe fn filter_contacts(proxy:usize,base:usize){
 let count=crate::read_i32(proxy+0x20);
 let capacity=crate::read_i32(proxy+0x24)as u32&0x3fff_ffff;
 let data=crate::read_ptr(proxy+0x18);
 if data==0||count<=0||count>256||count as u32>capacity{return;}
 let mut kept=0usize;
 for i in 0..count as usize{
  let source=(data+i*0x40)as*const u8;
  let id=ptr::read_unaligned(source.add(0x28)as*const u32);
  if npc_body(base,id){continue;}
  if i!=kept{ptr::copy(source,(data+kept*0x40)as*mut u8,0x40);}
  kept+=1;
 }
 ptr::write_unaligned((proxy+0x20)as*mut i32,kept as i32);
}
pub unsafe extern "system" fn contacts(proxy:usize,step:usize,delta:f32,planes:usize){
 let base=BASE.load(Ordering::Acquire);
 if lightning_ram::managed_proxy(proxy,base){filter_contacts(proxy,base);}
 let original:unsafe extern "system" fn(usize,usize,f32,usize)=std::mem::transmute(CONTACTS_ORIGINAL.load(Ordering::Acquire));
 original(proxy,step,delta,planes);
}
pub const SLOPE_RVA:usize=0x1a6c4b0;
pub const SLOPE_PATTERN:&str="48 89 5C 24 08 48 89 6C 24 10 56 57 41 56 48 83 EC 40 48 63 84 24 80 00 00 00";
pub static SLOPE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
// 1A6C980 creates the real contact first, then calls this routine to append a
// horizontal blocking plane when 0 < dot(normal,up) < maxSlopeCosine.
// A zero cosine suppresses only that extra plane, without editing the proxy.
pub unsafe extern "system" fn slope(proxy:usize,result:*mut u8,cosine:f32,up:*const f32,index:i32,planes:usize,distance:f32)->*mut u8{
 let original:unsafe extern "system" fn(usize,*mut u8,f32,*const f32,i32,usize,f32)->*mut u8=std::mem::transmute(SLOPE_ORIGINAL.load(Ordering::Acquire));
 let cosine=if lightning_ram::managed_proxy(proxy,BASE.load(Ordering::Acquire)){0.}else{cosine};
 original(proxy,result,cosine,up,index,planes,distance)
}
pub const SNAP_RVA:usize=0x471dc8;
pub const SNAP_PATTERN:&str="41 0F 2F F0 76 23 84 C0 75 1F 45 0F 57 C5 C6 87 92 00 00 00 01";
pub static SNAP_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
unsafe extern "system" fn snap_limit(physics:usize,limit:f32,lift:f32)->f32{
 let Some(airborne)=lightning_ram::managed_physics(physics,BASE.load(Ordering::Acquire))else{return limit;};
 if !limit.is_finite()||!lift.is_finite()||lift<0.{return limit;}
 // XMM7 is the already applied capsule lift, NOT extra ground adhesion.
 // Preserve its removal and upward depenetration. Only cap the extra downward
 // reach (physics+354). Airborne motion must not acquire distant support.
 limit.min(lift+if airborne{0.}else{0.015})
}
#[unsafe(naked)]pub unsafe extern "system" fn snap_detour(){
 core::arch::naked_asm!(
  "pushfq","sub rsp,0xc8",
  "mov [rsp+0x20],rax","mov [rsp+0x28],rcx","mov [rsp+0x30],rdx","mov [rsp+0x38],r8",
  "mov [rsp+0x40],r9","mov [rsp+0x48],r10","mov [rsp+0x50],r11",
  "movdqu [rsp+0x60],xmm0","movdqu [rsp+0x70],xmm1","movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3","movdqu [rsp+0xa0],xmm4","movdqu [rsp+0xb0],xmm5",
  "mov rcx,rdi","movaps xmm1,xmm6","movaps xmm2,xmm7","call {callback}","movss xmm6,xmm0",
  "mov rax,[rsp+0x20]","mov rcx,[rsp+0x28]","mov rdx,[rsp+0x30]","mov r8,[rsp+0x38]",
  "mov r9,[rsp+0x40]","mov r10,[rsp+0x48]","mov r11,[rsp+0x50]",
  "movdqu xmm0,[rsp+0x60]","movdqu xmm1,[rsp+0x70]","movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]","movdqu xmm4,[rsp+0xa0]","movdqu xmm5,[rsp+0xb0]",
  "add rsp,0xc8","popfq","jmp qword ptr [rip+{original}]",
  callback=sym snap_limit,original=sym SNAP_ORIGINAL
 );
}
