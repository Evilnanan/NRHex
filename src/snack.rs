//! Successful local merchant weapon purchases fund immediate personal rewards.
//! Keep the historical owner/lot IDs; no spending threshold or price filter.
use crate::{has_effect,read_i32,read_ptr,ptr};
use std::sync::{Mutex,atomic::{AtomicPtr,Ordering}};
use std::collections::VecDeque;
pub const OWNER:i32=95003600;
pub const LOT:i32=95420000;
pub const PREMIUM_LOT:i32=95420010;
pub const RARITY_RVA:usize=0x8bb780;
pub const RARITY_PATTERN:&str="48 89 5C 24 10 48 89 74 24 18 57 48 81 EC A0 00 00 00 48 8B 05 CF A7 22 03 48 33 C4 48 89 84 24 90 00 00 00 48 8B D9 48 8D 79 54 F7 07 00 00 00 F0";
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
pub fn bonus_lot(lot:i32)->bool{[LOT,PREMIUM_LOT].iter().any(|start|(*start..*start+3).contains(&lot))}
fn premium(rarity:u8)->bool{matches!(rarity,2|3|9)}
pub fn pending_for(lot:i32,owner:u16,slot:u8)->bool{
 let recipient=(owner>>7)&3;
 bonus_lot(lot)&&owner&0x40!=0&&(1..=3).contains(&slot)&&(recipient==0||recipient==u16::from(slot))
}
#[derive(Default)]
struct Pending{count:u32,batches:VecDeque<(i32,u32)>}
impl Pending{
 const fn new()->Self{Self{count:0,batches:VecDeque::new()}}
 fn bought(&mut self,quantity:u32,high:bool){
  let quantity=quantity.min(u32::MAX-self.count);if quantity==0{return;}
  let lot=if high{PREMIUM_LOT}else{LOT};self.count+=quantity;
  if let Some(batch)=self.batches.back_mut().filter(|batch|batch.0==lot){batch.1+=quantity;}else{self.batches.push_back((lot,quantity));}
 }
 fn reserve(&mut self)->Option<i32>{
  let batch=self.batches.front_mut()?;let lot=batch.0;batch.1-=1;self.count-=1;
  if batch.1==0{self.batches.pop_front();}Some(lot)
 }
 fn complete(&mut self,lot:i32,created:u32){
  if created!=0||self.count==u32::MAX{return;}
  self.count+=1;
  if let Some(batch)=self.batches.front_mut().filter(|batch|batch.0==lot){batch.1+=1;}else{self.batches.push_front((lot,1));}
 }
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
 let purchase=if amount>=0&&crate::ACTIVE.load(Ordering::Acquire)&&base!=0&&shop!=0{
  fields(caller.wrapping_sub(base)).and_then(|(item,count)|{
   let quantity=read_i32(shop+count);
   (quantity>0&&weapon(read_i32(shop+item)as u32)).then_some((quantity as u32,shop+item-0x54))
  })
 }else{None};
 let result=original(amount);
 let Some((quantity,record))=purchase else{return result;};
 let world=read_ptr(base+crate::WORLD_RVA);if world==0{return result;}
 let player=read_ptr(world+0x174e8);
 if player==0||!has_effect(player,OWNER,base)||!has_effect(player,crate::level_rewards::EXPEDITION,base)||has_effect(player,crate::reward_cycle_runtime::RESET,base){return result;}
 let rarity:unsafe extern "system" fn(usize)->u8=std::mem::transmute(base+RARITY_RVA);
 let high=premium(rarity(record));
 if let Ok(mut state)=PENDING.lock(){state.bought(quantity,high);}
 result
}
pub unsafe fn reset(_player:usize,_base:usize){if let Ok(mut state)=PENDING.lock(){*state=Pending::new();}}
pub unsafe fn update(player:usize,base:usize,_dt:f32){
 if has_effect(player,crate::reward_cycle_runtime::RESET,base){reset(player,base);return;}
 if !has_effect(player,OWNER,base)||PENDING.lock().map(|s|s.count==0).unwrap_or(true){return;}
 let stats=crate::native_actor::stats(player);
 if stats==0||crate::native_actor::incapacitated(player,stats)||!crate::spatial_ready(player)
  ||!has_effect(player,crate::level_rewards::EXPEDITION,base){return;}
 if crate::CALLBACK_LOCK.swap(true,Ordering::AcqRel){return;}
 let count=PENDING.lock().map(|s|s.count).unwrap_or(0);
 for _ in 0..count{
  let Some(lot)=PENDING.lock().ok().and_then(|mut state|state.reserve())else{break;};
  let created=crate::spawn_personal_lot(player,base,lot,None);
  if let Ok(mut state)=PENDING.lock(){state.complete(lot,created);}
  if created!=3{crate::nlog_error!("Extra Practice created {created} choices; reserved={}",created>0);break;}
 }
 crate::CALLBACK_LOCK.store(false,Ordering::Release);
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn weapon_purchases_count_items_instead_of_currency(){let mut state=Pending::new();state.bought(1,false);state.bought(2,true);state.bought(1,false);for lot in [LOT,PREMIUM_LOT,PREMIUM_LOT,LOT]{assert_eq!(state.reserve(),Some(lot));state.complete(lot,3);}assert_eq!(state.reserve(),None);assert_eq!(state.count,0);}
 #[test]fn zero_creation_refunds_but_partial_creation_never_duplicates(){let mut state=Pending::new();state.bought(1,true);assert_eq!(state.reserve(),Some(PREMIUM_LOT));state.complete(PREMIUM_LOT,0);assert_eq!(state.count,1);assert_eq!(state.reserve(),Some(PREMIUM_LOT));state.complete(PREMIUM_LOT,1);assert_eq!(state.count,0);state.bought(u32::MAX,false);state.bought(1,true);assert_eq!(state.count,u32::MAX);}
 #[test]fn only_purple_gold_and_cursed_red_use_premium_odds(){for rarity in 0..=255{assert_eq!(premium(rarity),[2,3,9].contains(&rarity));}}
 #[test]fn only_weapon_codes_and_valid_purchase_callers_qualify(){
  for id in [1_000_000,12_000_000,33_000_000,40_000_000]{assert!(weapon(id));}
  for id in [0,0x0fff_ffff,0x1000_0000,0x2000_0000,0x4000_0000|100,0x6000_0000|100,u32::MAX]{assert!(!weapon(id));}
  assert_eq!(fields(0xa7cbd4),Some((0x64,0x118)));assert_eq!(fields(0xa7cfd4),Some((0x5c,0x110)));
  for caller in [0,0xa403ef,0xa7cbcf,0xa7cfcf]{assert_eq!(fields(caller),None);}
 }
 #[test]fn bonus_choices_keep_their_private_recipient_rules(){
  for first in [LOT,PREMIUM_LOT]{for slot in 1..=3{for other in 1..=3{for lot in first..first+3{assert_eq!(pending_for(lot,0x40|(other<<7),slot),other==u16::from(slot));}}}}
  assert!(!pending_for(LOT+3,0xc0,1));assert!(!pending_for(LOT,0x80,1));assert!(!bonus_lot(PREMIUM_LOT+3));
  for first in [LOT,PREMIUM_LOT]{for lot in first..first+3{assert!(crate::reward_recovery::eligible(2,0x8c0,lot,0,1));assert!(!crate::reward_recovery::eligible(2,0x8c0,lot,0,2));}}
 }
}
#[cfg(test)]#[path="snack_native_tests.rs"]mod native_tests;
