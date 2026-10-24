//! Metallization: original Ironjar Aromatic, local use-only quantity exemption.
use crate::{ptr,read_ptr,has_effect,is_local,ACTIVE,BASE,native_actor,wraith};
use std::{cell::Cell,sync::{Mutex,OnceLock,atomic::{AtomicPtr,Ordering}}};
pub const OWNER:i32=95004600;
pub const GOOD:i32=3520;
pub const GROUND_LOT:i32=95546000;
pub const REQUEST:i32=OWNER+1;
pub const RECEIPT:i32=OWNER+2;
pub const CONSUME_RVA:usize=0x669ae0;
pub const CONSUME_PATTERN:&str="48 89 5C 24 10 56 57 41 56 48 83 EC 60 41 8B F0 4C 8B F2 48 8B D9 48 8B 01 FF 90 90 01 00 00 48";
pub const QUANTITY_PATTERN:&str="40 53 48 83 EC 30 44 8B C2 48 8B D1 48 8D 4C 24 20 E8 6A E1 55 00 90 48 8B C8 E8 41 E2 55 00 8B";
pub static CONSUME_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
thread_local!{static FREE:Cell<bool>=const{Cell::new(false)};}
struct Guard(bool);impl Guard{fn enter(value:bool)->Self{Self(FREE.with(|v|v.replace(value)))}}impl Drop for Guard{fn drop(&mut self){FREE.with(|v|v.set(self.0));}}
#[repr(C,align(16))]struct Row([u8;0xe0]);
static SHADOW:OnceLock<Box<Row>>=OnceLock::new();
unsafe fn enabled(actor:usize,base:usize)->bool{actor!=0&&is_local(actor,base)&&has_effect(actor,OWNER,base)&&!has_effect(actor,crate::reward_cycle_runtime::RESET,base)}
pub unsafe fn parameter(out:usize,id:i32){
 if id!=GOOD||out==0||!FREE.with(Cell::get){return;}
 let row=read_ptr(out+8);if row==0{return;}
 let shadow=SHADOW.get_or_init(||{let mut copy=Box::new(Row(ptr::read_unaligned(row as*const[u8;0xe0])));copy.0[0x48]&=0x7f;copy});
 ptr::write_unaligned((out+8)as*mut usize,shadow.0.as_ptr()as usize);
}
// TAE65 calls this PlayerIns method only after native use/effect succeeds.
// Its native1F12B0 quantity path respects isConsume. Scope the parameter query
// here, not at3979B0, whose return precedes the separate deduction call.
type Consumed=unsafe extern "system" fn(usize,*const i32,i32);
pub unsafe extern "system" fn consumed(actor:usize,good:*const i32,index:i32){
 let original:Consumed=std::mem::transmute(CONSUME_ORIGINAL.load(Ordering::Acquire));let base=BASE.load(Ordering::Acquire);
 let free=ACTIVE.load(Ordering::Acquire)&&!good.is_null()&&ptr::read_unaligned(good)==(0x40000000|GOOD)&&enabled(actor,base);
 let _guard=Guard::enter(free);original(actor,good,index);
}
#[derive(Clone,Default)]struct State{given:bool,requested:Option<Vec<(u32,u32)>>,actor:usize,handle:u64,saw_item:bool,idle_frames:u8}
static STATE:Mutex<State>=Mutex::new(State{given:false,requested:None,actor:0,handle:0,saw_item:false,idle_frames:0});
impl State{
 fn bind(&mut self,actor:usize,handle:u64){if self.actor!=actor||self.handle!=handle{self.actor=actor;self.handle=handle;self.saw_item=false;self.idle_frames=0;}}
 fn observe(&mut self,code:i32){if code==wraith::BUSY{self.saw_item=true;self.idle_frames=0;}else if code==wraith::FRAME{self.idle_frames=if self.saw_item{0}else{self.idle_frames.saturating_add(1)};self.saw_item=false;}}
}
fn received(items:&[(u32,u32)],previous:&[(u32,u32)])->bool{items.iter().any(|(serial,count)|*count>previous.iter().find(|(old,_)|old==serial).map_or(0,|(_,n)|*n))}
pub unsafe fn reset(){if let Ok(mut state)=STATE.lock(){*state=State::default();}}
pub unsafe fn observe(actor:usize,code:i32,base:usize){if enabled(actor,base){if let Ok(mut state)=STATE.lock(){state.bind(actor,ptr::read_unaligned((actor+8)as*const u64));state.observe(code);}}}
unsafe fn snapshot(root:usize,base:usize)->Vec<(u32,u32)>{
 let quantity:unsafe extern "system" fn(usize,i32)->u32=std::mem::transmute(base+0x1f5ca0);
 wraith::instances(root,GOOD,base).into_iter().map(|(index,serial)|(serial,quantity(root,index))).collect()
}
pub unsafe fn update(actor:usize,base:usize){
 if !enabled(actor,base){return;}let stats=native_actor::stats(actor);if stats==0{return;}
 let acknowledged=has_effect(actor,RECEIPT,base);
 let (given,requested,ready)={let Ok(mut state)=STATE.lock()else{return;};state.bind(actor,ptr::read_unaligned((actor+8)as*const u64));state.given|=acknowledged;(state.given,state.requested.clone(),state.idle_frames>=3)};
 if given||!ready||native_actor::incapacitated(actor,stats){return;}
 let Some((root,slots))=wraith::inventory(actor)else{return;};let items=snapshot(root,base);
 if let Some(previous)=requested{if !received(&items,&previous){return;}}
 else if wraith::bag_full(slots,base)||items.iter().any(|(_,count)|*count>=2){
  // A native Ironjar stack holds two. Do not silently lose a gift to a full
  // stack even when another tool slot is free.
  if !crate::spatial_ready(actor)||crate::CALLBACK_LOCK.swap(true,Ordering::AcqRel){return;}
  let created=crate::spawn_personal_lot_kind(actor,base,GROUND_LOT,None,1);crate::CALLBACK_LOCK.store(false,Ordering::Release);
  if created==0{return;}
 }else{
  if let Ok(mut state)=STATE.lock(){state.requested=Some(items);}else{return;}
  wraith::set(actor,REQUEST,true,base);return;
 }
 // Record successful delivery before accepting any further update callbacks.
 if let Ok(mut state)=STATE.lock(){state.given=true;state.requested=None;}
 wraith::set(actor,REQUEST,false,base);wraith::set(actor,RECEIPT,true,base);
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn receipt_accepts_stack_growth_and_new_serial_but_not_old_copies(){
  assert!(!received(&[(10,1)],&[(10,1)]));assert!(received(&[(10,2)],&[(10,1)]));assert!(received(&[(11,1)],&[(10,1)]));assert!(!received(&[],&[(10,1)]));
 }
 #[test]fn pending_gift_survives_rebinding_but_waits_for_item_chain_exit(){
  let mut state=State::default();state.bind(1,1);state.requested=Some(vec![(1,1)]);
  for _ in 0..10{state.observe(wraith::BUSY);state.observe(wraith::FRAME);assert!(state.idle_frames<3);}
  for _ in 0..3{state.observe(wraith::FRAME);}assert!(state.idle_frames>=3);
  state.bind(2,2);assert_eq!(state.requested,Some(vec![(1,1)]));assert_eq!(state.idle_frames,0);
 }
}
#[cfg(test)]#[path="metal_native_tests.rs"]mod native_tests;
