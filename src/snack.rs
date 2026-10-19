//! Successful local shop payments feed a persistent personal reward queue.
//! Native shop callbacks supply the settled price * quantity, after success.
use crate::{has_effect,read_i32,read_ptr,ptr};
use std::sync::{Mutex,atomic::{AtomicPtr,AtomicU64,Ordering}};
pub const OWNER:i32=95003600;
pub const LOT:i32=95420000;
pub const FIRST:i32=96110000;
const COST:u32=20_000;
pub const SPEND_RVA:usize=0x1fc0c0;
pub const SPEND_PATTERN:&str="4C 8B 05 09 B8 A0 03 B8 3F 42 0F 00 41 8B 90 90 01 00 00 03 D1 3B D0 0F 42 C2 41 89 80 90 01 00 00 C3";
pub const BUY_A_RVA:usize=0xa7cbc6;
pub const BUY_A_PATTERN:&str="8B 08 0F AF 8B 18 01 00 00 E8 EC F4 77 FF";
pub const BUY_B_RVA:usize=0xa7cfc6;
pub const BUY_B_PATTERN:&str="8B 08 0F AF 8B 10 01 00 00 E8 EC F0 77 FF";
pub const SHOP_RVA:usize=0xa7b600;
pub const SHOP_PATTERN:&str="48 89 5C 24 10 48 89 74 24 18 57 48 83 EC 30 48 8B D9 E8 F9 5A F4 FF";
// Base dialog's native closing predicate, also used by ShopDialog update.
pub const CLOSING_RVA:usize=0x938ca0;
pub const CLOSING_PATTERN:&str="48 83 B9 98 00 00 00 01 0F 97 C0 C3 CC 47 77 87 E9 7B D5 07 00 0F 84 D5 13 98 01 E9 DD 13 98 01";
pub static SPEND_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static SHOP_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static SHOP_SEEN:AtomicU64=AtomicU64::new(0);
fn purchase(caller:usize)->bool{matches!(caller,0xa7cbd4|0xa7cfd4)}
pub fn bonus_lot(lot:i32)->bool{(LOT..LOT+3).contains(&lot)}
pub fn pending_for(lot:i32,owner:u16,slot:u8)->bool{
 let recipient=(owner>>7)&3;
 bonus_lot(lot)&&owner&0x40!=0&&(1..=3).contains(&slot)&&(recipient==0||recipient==u16::from(slot))
}
#[derive(Default)]
struct Credit{amount:Option<u32>,resetting:bool,wait:f32}
impl Credit{
 const fn new()->Self{Self{amount:None,resetting:false,wait:0.}}
 fn observe(&mut self,saved:u32)->u32{*self.amount.get_or_insert(saved)}
 fn spend(&mut self,saved:u32,amount:i32){
  if amount<=0||self.resetting{return;}
  let current=self.observe(saved);self.amount=Some(current.saturating_add(amount as u32));self.wait=0.;
 }
 fn reserve(&mut self)->bool{
  if self.resetting{return false;}
  let Some(amount)=self.amount.filter(|&n|n>=COST)else{return false;};
  self.amount=Some(amount-COST);self.wait=0.25;true
 }
 fn complete(&mut self,created:u32){if created==0{self.amount=self.amount.map(|n|n.saturating_add(COST));}self.wait=0.25;}
 fn reset(&mut self){self.amount=Some(0);self.resetting=true;self.wait=0.;}
}
static CREDIT:Mutex<Credit>=Mutex::new(Credit::new());
fn encode(amount:u32)->Vec<i32>{amount.to_le_bytes().into_iter().enumerate().filter_map(|(i,b)|(b!=0).then_some(FIRST+i as i32*256+i32::from(b))).collect()}
fn decode(ids:&[i32])->u32{
 let mut bytes=[0u8;4];let mut seen=[false;4];
 for &id in ids{let offset=id-FIRST;if !(0..1024).contains(&offset)||offset%256==0{continue;}
  let digit=offset as usize/256;if !seen[digit]{bytes[digit]=offset as u8;seen[digit]=true;}
 }u32::from_le_bytes(bytes)
}
unsafe fn records(player:usize)->Vec<i32>{
 let mut ids=Vec::new();let manager=read_ptr(player+0x190);if manager==0{return ids;}
 let mut node=read_ptr(manager+8);
 for _ in 0..65536{if node==0{break;}
  let id=read_i32(node+8);
  if (FIRST..FIRST+1024).contains(&id)&&(id-FIRST)%256!=0&&ptr::read_unaligned((node+0x68)as*const u32)&0x80080003==0{ids.push(id);}
  node=read_ptr(node+0x38);
 }ids
}
// Apply replacement first, wait for native acknowledgement, then delete old
// byte records. RAM remains authoritative while native operations are deferred.
unsafe fn persist(player:usize,base:usize,amount:u32)->bool{
 let manager=read_ptr(player+0x190);if manager==0{return false;}
 let saved=records(player);let wanted=encode(amount);
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);
 for digit in 0..4{
  let desired=wanted.iter().copied().find(|id|(id-FIRST)/256==digit);
  if let Some(id)=desired{if !has_effect(player,id,base){apply(player,id,0);}if !has_effect(player,id,base){continue;}}
  for &id in &saved{if (id-FIRST)/256==digit&&Some(id)!=desired{remove(manager,id);}}
 }
 let current=records(player);
 current.len()==wanted.len()&&wanted.iter().all(|id|current.contains(id)&&has_effect(player,*id,base))
}
#[unsafe(naked)]
pub unsafe extern "system" fn spent(_amount:i32)->u32{
 core::arch::naked_asm!("mov rdx,[rsp]","jmp {callback}",callback=sym settled);
}
unsafe extern "system" fn settled(amount:i32,caller:usize)->u32{
 let original:unsafe extern "system" fn(i32)->u32=std::mem::transmute(SPEND_ORIGINAL.load(Ordering::Acquire));
 let result=original(amount);
 let base=crate::BASE.load(Ordering::Acquire);
 if amount<=0||!crate::ACTIVE.load(Ordering::Acquire)||base==0||!purchase(caller.wrapping_sub(base)){return result;}
 let world=read_ptr(base+crate::WORLD_RVA);if world==0{return result;}
 let player=read_ptr(world+0x174e8);
 if player==0||!has_effect(player,OWNER,base)||!has_effect(player,crate::level_rewards::EXPEDITION,base)||has_effect(player,crate::reward_cycle_runtime::RESET,base){return result;}
 let saved=decode(&records(player));
 let amount=if let Ok(mut state)=CREDIT.lock(){state.spend(saved,amount);state.amount}else{None};
 if let Some(amount)=amount{persist(player,base,amount);}
 SHOP_SEEN.store(crate::GetTickCount64(),Ordering::Relaxed);result
}
unsafe extern "system" fn shop_frame(dialog:usize){
 if dialog!=0&&crate::ACTIVE.load(Ordering::Acquire)&&read_ptr(dialog+0x98)<=1{
  SHOP_SEEN.store(crate::GetTickCount64(),Ordering::Relaxed);
 }
}
// Transparent pre-hook: do not guess ShopDialog::Update's return ABI.
#[unsafe(naked)]
pub unsafe extern "system" fn shop(){core::arch::naked_asm!(
 "sub rsp,0xc8",
 "mov [rsp+0x20],rax","mov [rsp+0x28],rcx","mov [rsp+0x30],rdx","mov [rsp+0x38],r8","mov [rsp+0x40],r9","mov [rsp+0x48],r10","mov [rsp+0x50],r11",
 "movdqu [rsp+0x60],xmm0","movdqu [rsp+0x70],xmm1","movdqu [rsp+0x80],xmm2","movdqu [rsp+0x90],xmm3","movdqu [rsp+0xa0],xmm4","movdqu [rsp+0xb0],xmm5",
 "call {callback}",
 "movdqu xmm0,[rsp+0x60]","movdqu xmm1,[rsp+0x70]","movdqu xmm2,[rsp+0x80]","movdqu xmm3,[rsp+0x90]","movdqu xmm4,[rsp+0xa0]","movdqu xmm5,[rsp+0xb0]",
 "mov rax,[rsp+0x20]","mov rcx,[rsp+0x28]","mov rdx,[rsp+0x30]","mov r8,[rsp+0x38]","mov r9,[rsp+0x40]","mov r10,[rsp+0x48]","mov r11,[rsp+0x50]",
 "add rsp,0xc8","jmp qword ptr [rip+{original}]",callback=sym shop_frame,original=sym SHOP_ORIGINAL);
}
pub unsafe fn reset(player:usize,base:usize){
 if let Ok(mut state)=CREDIT.lock(){state.reset();}
 persist(player,base,0);SHOP_SEEN.store(0,Ordering::Relaxed);
}
pub unsafe fn update(player:usize,base:usize,dt:f32){
 if has_effect(player,crate::reward_cycle_runtime::RESET,base){reset(player,base);return;}
 let owned=has_effect(player,OWNER,base);
 let (amount,resetting)={
  let Ok(mut state)=CREDIT.lock()else{return;};
  if !owned&&!state.resetting{return;}
  state.wait=(state.wait-dt).max(0.);if state.wait>0.{return;}
  state.wait=0.25;(state.observe(decode(&records(player))),state.resetting)
 };
 let saved=persist(player,base,amount);
 if resetting{if saved{if let Ok(mut state)=CREDIT.lock(){state.resetting=false;}}return;}
 if !owned||!saved||amount<COST{return;}
 let now=crate::GetTickCount64();let seen=SHOP_SEEN.load(Ordering::Relaxed);
 let stats=crate::native_actor::stats(player);
 if (seen!=0&&now.saturating_sub(seen)<500)||stats==0||crate::native_actor::incapacitated(player,stats)||!crate::spatial_ready(player)
  ||!has_effect(player,crate::level_rewards::EXPEDITION,base)||crate::unclaimed_hex_ball(base){return;}
 if crate::CALLBACK_LOCK.swap(true,Ordering::AcqRel){return;}
 let reserved=CREDIT.lock().map(|mut state|state.amount==Some(amount)&&state.reserve()).unwrap_or(false);
 if reserved{
  let created=crate::spawn_personal_lot(player,base,LOT,None);
  let remaining=CREDIT.lock().ok().and_then(|mut state|{state.complete(created);state.amount});
  if let Some(remaining)=remaining{persist(player,base,remaining);}
  if created!=3{crate::nlog_error!("Extra Helping created {created} choices; reserved={}",created>0);}
 }
 crate::CALLBACK_LOCK.store(false,Ordering::Release);
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn actual_spending_carries_remainders_and_never_uses_native_stat_cap(){
  let mut s=Credit::new();s.spend(0,8000);assert!(!s.reserve());s.spend(0,42000);
  assert!(s.reserve());s.complete(3);assert!(s.reserve());s.complete(3);assert!(!s.reserve());assert_eq!(s.amount,Some(10000));
  s.spend(0,2_000_000);assert_eq!(s.amount,Some(2_010_000));
  s.spend(0,0);s.spend(0,-50);assert_eq!(s.amount,Some(2_010_000));
 }
 #[test]fn deferred_records_revival_and_only_explicit_reset_control_credit(){
  let mut s=Credit::new();assert_eq!(s.observe(19000),19000);s.spend(0,5000);
  for saved in [0,19000,24000]{assert_eq!(s.observe(saved),24000);}
  assert!(s.reserve());s.complete(0);assert_eq!(s.amount,Some(24000));
  assert!(s.reserve());s.complete(1);assert_eq!(s.amount,Some(4000));assert!(!s.reserve());
  let mut restored=Credit::new();assert_eq!(restored.observe(decode(&encode(4000))),4000);
  s.reset();s.spend(24000,50000);assert!(!s.reserve());assert_eq!(s.observe(24000),0);
 }
 #[test]fn packed_credit_roundtrips_and_newest_byte_wins(){
  for amount in [0,1,255,256,19999,20000,50000,999999,2_000_000,u32::MAX]{let ids=encode(amount);assert_eq!(decode(&ids),amount);assert!(ids.len()<=4);}
  assert_eq!(decode(&[FIRST+2,FIRST+1]),2);assert_eq!(decode(&[FIRST+256,FIRST-1,FIRST+1025]),0);
 }
 #[test]fn only_successful_shop_callers_and_local_three_choices_qualify(){
  for caller in [0xa7cbd4,0xa7cfd4]{assert!(purchase(caller));}
  for caller in [0,0xa403ef,0xa7cbcf,0xa7cfcf]{assert!(!purchase(caller));}
  for slot in 1..=3{for other in 1..=3{for lot in LOT..LOT+3{assert_eq!(pending_for(lot,0x40|(other<<7),slot),other==u16::from(slot));}}}
  assert!(!pending_for(LOT+3,0xc0,1));assert!(!pending_for(LOT,0x80,1));
 }
}
#[cfg(test)]#[path="snack_native_tests.rs"]mod native_tests;
