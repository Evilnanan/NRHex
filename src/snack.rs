//! Successful local merchant weapon purchases fund immediate personal rewards.
//! Keep the historical owner/lot IDs; no spending threshold or price filter.
use crate::{has_effect,read_i32,read_ptr,ptr};
use std::sync::{Mutex,atomic::{AtomicPtr,Ordering}};
pub const OWNER:i32=95003600;
pub const LOT:i32=95420000;
pub const SPEND_RVA:usize=0x1fc0c0;
pub const SPEND_PATTERN:&str="4C 8B 05 09 B8 A0 03 B8 3F 42 0F 00 41 8B 90 90 01 00 00 03 D1 3B D0 0F 42 C2 41 89 80 90 01 00 00 C3";
pub const BUY_A_RVA:usize=0xa7cbc6;
pub const BUY_A_PATTERN:&str="8B 08 0F AF 8B 18 01 00 00 E8 EC F4 77 FF";
pub const BUY_B_RVA:usize=0xa7cfc6;
pub const BUY_B_PATTERN:&str="8B 08 0F AF 8B 10 01 00 00 E8 EC F0 77 FF";
pub const ITEM_A_RVA:usize=0xa7caec;
pub const ITEM_A_PATTERN:&str="8B 41 64 89 44 24 30 48 83 3D 4D A7 19 03 00";
pub const ITEM_B_RVA:usize=0xa7ceec;
pub const ITEM_B_PATTERN:&str="8B 41 5C 89 44 24 30 48 83 3D 4D A3 19 03 00";
pub static SPEND_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
fn fields(caller:usize)->Option<(usize,usize)>{match caller{0xa7cbd4=>Some((0x64,0x118)),0xa7cfd4=>Some((0x5c,0x110)),_=>None}}
fn weapon(encoded:u32)->bool{encoded>0&&encoded<0x0fff_ffff} // Native item kind 0; includes shields, bows and catalysts.
pub fn bonus_lot(lot:i32)->bool{(LOT..LOT+3).contains(&lot)}
pub fn pending_for(lot:i32,owner:u16,slot:u8)->bool{
 let recipient=(owner>>7)&3;
 bonus_lot(lot)&&owner&0x40!=0&&(1..=3).contains(&slot)&&(recipient==0||recipient==u16::from(slot))
}
#[derive(Default)]
struct Pending{count:u32}
impl Pending{
 const fn new()->Self{Self{count:0}}
 fn bought(&mut self,quantity:u32){self.count=self.count.saturating_add(quantity);}
 fn reserve(&mut self)->bool{if self.count==0{return false;}self.count-=1;true}
 fn complete(&mut self,created:u32){if created==0{self.count=self.count.saturating_add(1);}}
}
// Only rewards waiting for a safe creation frame are queued. No currency total
// or cross-version saved credit is read. Death/actor reconstruction keeps RAM.
static PENDING:Mutex<Pending>=Mutex::new(Pending::new());
#[unsafe(naked)]
pub unsafe extern "system" fn spent(_amount:i32)->u32{
 core::arch::naked_asm!("mov rdx,[rsp]","mov r8,rbx","jmp {callback}",callback=sym settled);
}
unsafe extern "system" fn settled(amount:i32,caller:usize,shop:usize)->u32{
 let original:unsafe extern "system" fn(i32)->u32=std::mem::transmute(SPEND_ORIGINAL.load(Ordering::Acquire));
 let base=crate::BASE.load(Ordering::Acquire);
 // Only the two signature-guarded successful purchase callers keep the item
 // record in RBX. All other spending forwards without dereferencing RBX.
 let quantity=if amount>=0&&crate::ACTIVE.load(Ordering::Acquire)&&base!=0&&shop!=0{
  fields(caller.wrapping_sub(base)).and_then(|(item,count)|{
   let quantity=read_i32(shop+count);
   (quantity>0&&weapon(read_i32(shop+item)as u32)).then_some(quantity as u32)
  }).unwrap_or(0)
 }else{0};
 let result=original(amount);
 if quantity==0{return result;}
 let world=read_ptr(base+crate::WORLD_RVA);if world==0{return result;}
 let player=read_ptr(world+0x174e8);
 if player==0||!has_effect(player,OWNER,base)||!has_effect(player,crate::level_rewards::EXPEDITION,base)||has_effect(player,crate::reward_cycle_runtime::RESET,base){return result;}
 if let Ok(mut state)=PENDING.lock(){state.bought(quantity);}
 result
}
pub unsafe fn reset(_player:usize,_base:usize){if let Ok(mut state)=PENDING.lock(){state.count=0;}}
pub unsafe fn update(player:usize,base:usize,_dt:f32){
 if has_effect(player,crate::reward_cycle_runtime::RESET,base){reset(player,base);return;}
 if !has_effect(player,OWNER,base)||PENDING.lock().map(|s|s.count==0).unwrap_or(true){return;}
 let stats=crate::native_actor::stats(player);
 if stats==0||crate::native_actor::incapacitated(player,stats)||!crate::spatial_ready(player)
  ||!has_effect(player,crate::level_rewards::EXPEDITION,base){return;}
 if crate::CALLBACK_LOCK.swap(true,Ordering::AcqRel){return;}
 let count=PENDING.lock().map(|s|s.count).unwrap_or(0);
 for _ in 0..count{
  if !PENDING.lock().map(|mut s|s.reserve()).unwrap_or(false){break;}
  let created=crate::spawn_personal_lot(player,base,LOT,None);
  if let Ok(mut state)=PENDING.lock(){state.complete(created);}
  if created!=3{crate::nlog_error!("Extra Practice created {created} choices; reserved={}",created>0);break;}
 }
 crate::CALLBACK_LOCK.store(false,Ordering::Release);
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn weapon_purchases_count_items_instead_of_currency(){let mut s=Pending::new();s.bought(1);s.bought(2);for _ in 0..3{assert!(s.reserve());s.complete(3);}assert!(!s.reserve());assert_eq!(s.count,0);}
 #[test]fn zero_creation_refunds_but_partial_creation_never_duplicates(){let mut s=Pending::new();s.bought(1);assert!(s.reserve());s.complete(0);assert_eq!(s.count,1);assert!(s.reserve());s.complete(1);assert_eq!(s.count,0);s.bought(u32::MAX);s.bought(1);assert_eq!(s.count,u32::MAX);}
 #[test]fn only_weapon_codes_and_valid_purchase_callers_qualify(){
  for id in [1_000_000,12_000_000,33_000_000,40_000_000]{assert!(weapon(id));}
  for id in [0,0x0fff_ffff,0x1000_0000,0x2000_0000,0x4000_0000|100,0x6000_0000|100,u32::MAX]{assert!(!weapon(id));}
  assert_eq!(fields(0xa7cbd4),Some((0x64,0x118)));assert_eq!(fields(0xa7cfd4),Some((0x5c,0x110)));
  for caller in [0,0xa403ef,0xa7cbcf,0xa7cfcf]{assert_eq!(fields(caller),None);}
 }
 #[test]fn bonus_choices_keep_their_private_recipient_rules(){
  for slot in 1..=3{for other in 1..=3{for lot in LOT..LOT+3{assert_eq!(pending_for(lot,0x40|(other<<7),slot),other==u16::from(slot));}}}
  assert!(!pending_for(LOT+3,0xc0,1));assert!(!pending_for(LOT,0x80,1));
 }
}
#[cfg(test)]#[path="snack_native_tests.rs"]mod native_tests;
