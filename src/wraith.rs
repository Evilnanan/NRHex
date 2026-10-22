//! Royal Revenant: native item-use provenance and first-contact Bagcraft credit.
//! Inventory transitions are acknowledged before the previous native tool is removed.
use crate::{ptr,read_i32,read_ptr,has_effect,is_local,native_actor,ACTIVE,BASE};
use std::{cell::Cell,collections::BTreeMap,sync::{Mutex,OnceLock,atomic::{AtomicPtr,Ordering}}};
pub const OWNER:i32=95004300;
pub const ORIGINALS:[i32;3]=[3080,3010,3011];
pub const FRAME:i32=990043;pub const BUSY:i32=990044;
pub const CAN_RVA:usize=0x3d49a0;pub const USE_RVA:usize=0x3979b0;pub const PARAM_RVA:usize=0xe8d840;
pub const TOKEN_RVA:usize=0x66d3b0;pub const XP_RVA:usize=0x209490;
pub const CAN_PATTERN:&str="48 89 5C 24 08 48 89 6C 24 20 56 57 41 56 48 83 EC 50 48 8B FA 48 8B D9 45 32 F6 83 3A FF";
pub const USE_PATTERN:&str="48 89 5C 24 10 44 88 4C 24 20 55 56 57 41 54 41 55 41 56 41 57 48 81 EC 10 01 00 00 4D 8B E0 44";
pub const PARAM_PATTERN:&str="48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 57 48 83 EC 30 8B FA 48 8B F1 33 DB 85 D2 0F 88 27 01 00 00 48 8B 0D 16 B0 D9 02 48 85 C9 75";
pub const TOKEN_PATTERN:&str="4C 8B 81 48 07 00 00 49 8B 08 49 3B C8 74 0F 8B 12 3B 51 10 74 0B 48";
pub const XP_PATTERN:&str="48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 57 41 56 41 57 48 83 EC 40 0F B6 EA 41 0F B7 D8";
pub const REGISTER_PATTERN:&str="48 89 5C 24 10 57 48 83 EC 30 4C 8B 81 48 07 00 00 49 8B 00 49 3B C0 74 19";
pub const INVENTORY_PATTERN:&str="48 89 5C 24 18 48 89 7C 24 20 41 56 48 83 EC 20 83 3A FF 41 8B D8 4C 8B F2 48 8B F9 0F 84 B3 00";
pub const SLOT_PATTERN:&str="40 57 48 83 EC 20 8D 42 E0 41 8B F8 83 F8 11 77";
pub const CREDIT_PATTERN:&str="E8 DB B3 EB FF 41 B0 01 48 8B D7 48 8B CB E8 ED 5F 00 00";
pub static CAN_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static USE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static PARAM_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static TOKEN_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static XP_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
#[derive(Clone,Copy,Default)]struct Credit{actor:usize,good:i32}
thread_local!{static FREE:Cell<bool>=const{Cell::new(false)};static CREDIT:Cell<Option<Credit>>=const{Cell::new(None)};}
struct FreeGuard(bool);impl FreeGuard{fn enter(value:bool)->Self{Self(FREE.with(|s|s.replace(value)))}}impl Drop for FreeGuard{fn drop(&mut self){FREE.with(|s|s.set(self.0));}}
#[repr(C,align(16))]struct GoodRow([u8;0xe0]);
static SHADOWS:[OnceLock<Box<GoodRow>>;3]=[OnceLock::new(),OnceLock::new(),OnceLock::new()];
#[derive(Default)]struct State{actor:usize,handle:u64,units:Option<u8>,stage:u8,given:Option<u8>,tokens:BTreeMap<i32,i32>,ages:BTreeMap<i32,f64>,saw_item:bool,idle_frames:u8,ready:bool,requested:Option<u8>,request_frames:u8}
static STATE:Mutex<State>=Mutex::new(State{actor:0,handle:0,units:None,stage:0,given:None,tokens:BTreeMap::new(),ages:BTreeMap::new(),saw_item:false,idle_frames:0,ready:false,requested:None,request_frames:0});
fn tier(good:i32)->Option<usize>{ORIGINALS.iter().position(|&id|id==good)}
fn progressed(units:u8,stage:u8,good:i32)->u8{
 if tier(good)!=Some(stage as usize){return units;}
 match stage{0=>units.saturating_add(1).min(20),1=>units.saturating_add(4).min(60),_=>units}
}
impl State{
 fn bind(&mut self,actor:usize,handle:u64){if self.actor!=actor||self.handle!=handle{self.actor=actor;self.handle=handle;self.tokens.clear();self.ages.clear();self.saw_item=false;self.idle_frames=0;self.ready=false;self.requested=None;}}
 fn frame(&mut self){if self.requested.is_some(){self.request_frames=self.request_frames.saturating_add(1);}self.idle_frames=if self.saw_item{0}else{self.idle_frames.saturating_add(1)};self.saw_item=false;self.ready=self.idle_frames>=3;}
}
unsafe fn enabled(actor:usize,base:usize)->bool{actor!=0&&is_local(actor,base)&&has_effect(actor,OWNER,base)&&!has_effect(actor,crate::reward_cycle_runtime::RESET,base)}
unsafe fn local(base:usize)->usize{let w=read_ptr(base+crate::WORLD_RVA);if w==0{0}else{read_ptr(w+0x174e8)}}
unsafe fn set(actor:usize,id:i32,yes:bool,base:usize){let manager=read_ptr(actor+0x190);if manager==0{return;}if yes{let add:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);add(actor,id,0);}else{let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);remove(manager,id);}}
unsafe fn saved(actor:usize)->Vec<i32>{
 let mut out=Vec::new();let manager=read_ptr(actor+0x190);if manager==0{return out;}let mut node=read_ptr(manager+8);
 for _ in 0..65536{if node==0{break;}let id=read_i32(node+8);if (OWNER+11..=OWNER+70).contains(&id)&&ptr::read_unaligned((node+0x68)as*const u32)&0x80080003==0{out.push(id);}node=read_ptr(node+0x38);}out
}
pub unsafe fn reset(){if let Ok(mut s)=STATE.lock(){*s=State::default();}CREDIT.with(|s|s.set(None));}
pub unsafe extern "system" fn param(out:usize,id:i32)->usize{
 let original:unsafe extern "system" fn(usize,i32)->usize=std::mem::transmute(PARAM_ORIGINAL.load(Ordering::Acquire));let result=original(out,id);
 if out!=0&&FREE.with(Cell::get){if let Some(i)=ORIGINALS.iter().position(|&good|good==id){let row=read_ptr(out+8);if row!=0{
  let shadow=SHADOWS[i].get_or_init(||{let mut copy=Box::new(GoodRow(ptr::read_unaligned(row as*const[u8;0xe0])));copy.0[0x80..0x82].fill(0);copy});
  ptr::write_unaligned((out+8)as*mut usize,shadow.0.as_ptr()as usize);
 }}}
 result
}
pub unsafe extern "system" fn can_use(actor:usize,item:*const i32)->u8{
 let original:unsafe extern "system" fn(usize,*const i32)->u8=std::mem::transmute(CAN_ORIGINAL.load(Ordering::Acquire));let base=BASE.load(Ordering::Acquire);
 let free=ACTIVE.load(Ordering::Acquire)&&!item.is_null()&&tier(ptr::read_unaligned(item)&0x0fffffff).is_some()&&enabled(actor,base);
 let _guard=FreeGuard::enter(free);original(actor,item)
}
type Use=unsafe extern "system" fn(usize,i32,usize,u8,u8,f32)->u8;
pub unsafe extern "system" fn used(context:usize,good:i32,data:usize,flag:u8,entry:u8,elapsed:f32)->u8{
 let original:Use=std::mem::transmute(USE_ORIGINAL.load(Ordering::Acquire));let base=BASE.load(Ordering::Acquire);let actor=if context==0{0}else{read_ptr(context)};
 let free=ACTIVE.load(Ordering::Acquire)&&tier(good).is_some()&&enabled(actor,base);let _guard=FreeGuard::enter(free);original(context,good,data,flag,entry,elapsed)
}
// Shared root dispatch already preserves native source identity and networking.
pub unsafe fn emitted(descriptor:usize,out:usize,base:usize){
 if !ACTIVE.load(Ordering::Acquire)||descriptor==0||out==0||read_i32(descriptor+8)>=0{return;}
 let good=read_i32(descriptor+0x18);if tier(good).is_none(){return;}
 let actor=local(base);if !enabled(actor,base)||ptr::read_unaligned(descriptor as*const u64)!=ptr::read_unaligned((actor+8)as*const u64){return;}
 let token=read_i32(out);if token<0{return;}
 let register:unsafe extern "system" fn(usize,*const i32)=std::mem::transmute(base+0x66f040);register(actor,&token);
 if let Ok(mut s)=STATE.lock(){s.bind(actor,ptr::read_unaligned((actor+8)as*const u64));s.tokens.insert(token,good);s.ages.insert(token,0.);}
}
pub unsafe extern "system" fn token(actor:usize,id:*const i32)->u8{
 CREDIT.with(|s|s.set(None));let original:unsafe extern "system" fn(usize,*const i32)->u8=std::mem::transmute(TOKEN_ORIGINAL.load(Ordering::Acquire));let result=original(actor,id);
 let base=BASE.load(Ordering::Acquire);
 if result!=0&&!id.is_null()&&ACTIVE.load(Ordering::Acquire)&&is_local(actor,base){
  let good=STATE.lock().ok().and_then(|mut s|if s.actor==actor{let token=ptr::read_unaligned(id);s.ages.remove(&token);s.tokens.remove(&token)}else{None});
  if let Some(good)=good{CREDIT.with(|s|s.set(Some(Credit{actor,good})));}
 }result
}
#[unsafe(naked)]pub unsafe extern "system" fn xp(_:usize,_:u8,_:u16){core::arch::naked_asm!("mov r9,[rsp]","jmp {callback}",callback=sym credited);}
unsafe extern "system" fn credited(data:usize,category:u8,points:u16,caller:usize){
 let original:unsafe extern "system" fn(usize,u8,u16)=std::mem::transmute(XP_ORIGINAL.load(Ordering::Acquire));let base=BASE.load(Ordering::Acquire);
 if caller==base+0x34e0b5&&category==9&&ACTIVE.load(Ordering::Acquire){if let Some(c)=CREDIT.with(|s|s.take()){
  if enabled(c.actor,base){let markers=saved(c.actor);if let Ok(mut s)=STATE.lock(){let old=*s.units.get_or_insert_with(||markers.iter().map(|id|(id-OWNER-10)as u8).max().unwrap_or(0));s.units=Some(progressed(old,s.stage,c.good));}
   // Scholar keeps its own native category XP. Other heroes gain only our progress.
  }
  if crate::local_hero(base)!=Some(9){return;}
 }}original(data,category,points)
}
unsafe fn inventory(actor:usize)->Option<(usize,usize)>{let gd=read_ptr(actor+0x5a0);(gd!=0).then_some((gd+0x1d8+0x1c8,gd+0x1d8+0x248))}
unsafe fn find(root:usize,good:i32,base:usize)->i32{let find:unsafe extern "system" fn(usize,*const i32,i32)->i32=std::mem::transmute(base+0x1f5610);find(root,&(good|0x40000000),0)}
unsafe fn equip(slots:usize,slot:i32,index:i32,base:usize){let f:unsafe extern "system" fn(usize,i32,i32)=std::mem::transmute(base+0x1f7840);f(slots,slot+0x20,index);}
// HKS observes every item common/activation frame, including the whole chained
// animation. Three empty samples bridge ordering and same-frame blend callbacks.
pub unsafe fn query(actor:usize,code:i32,base:usize){
 if !enabled(actor,base){return;}
 if let Ok(mut s)=STATE.lock(){s.bind(actor,ptr::read_unaligned((actor+8)as*const u64));if code==BUSY{s.saw_item=true;s.ready=false;s.idle_frames=0;}else if code==FRAME{s.frame();}}
}
pub unsafe fn update(actor:usize,base:usize,dt:f32){
 if !enabled(actor,base){return;}let stats=native_actor::stats(actor);if stats==0{return;}
 // Native tool flights last at most ten seconds; retire only our missed
 // pending records after thirty game-time seconds, never on emission-set reset.
 let expired=if let Ok(mut s)=STATE.lock(){s.bind(actor,ptr::read_unaligned((actor+8)as*const u64));if dt.is_finite()&&dt>0.&&dt<=1.{for age in s.ages.values_mut(){*age+=f64::from(dt);}}let expired:Vec<_>=s.ages.iter().filter_map(|(&id,&age)|(age>=30.).then_some(id)).collect();for id in &expired{s.ages.remove(id);s.tokens.remove(id);}expired}else{Vec::new()};
 if !expired.is_empty(){let remove:unsafe extern "system" fn(usize,*const i32)=std::mem::transmute(base+0x669a20);for id in expired{remove(actor,&id);}}
 let markers=saved(actor);let native_stage=(0..3).rev().find(|&i|has_effect(actor,OWNER+90+i,base));
 let (units,ready,requested,request_frames,stage)={let Ok(mut s)=STATE.lock()else{return;};s.bind(actor,ptr::read_unaligned((actor+8)as*const u64));let units=*s.units.get_or_insert_with(||markers.iter().map(|id|(id-OWNER-10)as u8).max().unwrap_or(0));let stage=native_stage.map(|n|n as u8).max(s.given);s.given=stage;s.stage=stage.unwrap_or(0);(units,s.ready,s.requested,s.request_frames,stage.map(i32::from))};
 let wanted=OWNER+10+i32::from(units);if units>0&&!markers.contains(&wanted){set(actor,wanted,true,base);}for id in markers{if id!=wanted{set(actor,id,false,base);}}
 if !ready||native_actor::incapacitated(actor,stats){return;}
 let Some((root,slots))=inventory(actor)else{return;};
 let desired=match stage{None=>0,Some(0)if units>=20=>1,Some(1)if units>=60=>2,_=>return};
 let index=find(root,ORIGINALS[desired as usize],base);
 if index<0{
  if requested!=Some(desired as u8)||(request_frames>=30&&!has_effect(actor,OWNER+1+desired,base)){set(actor,OWNER+1+desired,true,base);if let Ok(mut s)=STATE.lock(){s.requested=Some(desired as u8);s.request_frames=0;}}
  return;
 }
 if let Some(old_stage)=stage{
  let old=find(root,ORIGINALS[old_stage as usize],base);
  if old>=0{let old_slots:Vec<_>=(0..18).filter(|&i|read_i32(slots+12+i*8)==old).collect();
   if !old_slots.is_empty(){if requested==Some(desired as u8){for i in 0..18{if read_i32(slots+12+i*8)==index{equip(slots,i as i32,-1,base);}}}for i in old_slots{equip(slots,i as i32,index,base);}}
  }
  set(actor,OWNER+7+old_stage,true,base);
 }
 set(actor,OWNER+1+desired,false,base);
 set(actor,OWNER+90+desired,true,base);
 if let Ok(mut s)=STATE.lock(){s.stage=desired as u8;s.given=Some(desired as u8);s.requested=None;}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn only_matching_tier_first_contact_counts_and_old_flights_cannot_skip_upgrades(){let mut n=0;for _ in 0..20{n=progressed(n,0,3080);}assert_eq!(n,20);assert_eq!(progressed(n,0,3080),20);assert_eq!(progressed(n,1,3080),20);for _ in 0..10{n=progressed(n,1,3010);}assert_eq!(n,60);assert_eq!(progressed(n,2,3011),60);assert_eq!(progressed(0,0,3011),0);assert_eq!(tier(OWNER+80),None);}
 #[test]fn chained_actions_and_first_two_empty_samples_cannot_replace_and_rebinding_keeps_progress(){let mut s=State::default();s.units=Some(20);s.bind(1,10);for _ in 0..100{s.saw_item=true;s.frame();assert!(!s.ready);}s.frame();assert!(!s.ready);s.frame();assert!(!s.ready);s.frame();assert!(s.ready);s.saw_item=true;s.frame();assert!(!s.ready);s.tokens.insert(7,3080);s.bind(2,11);assert_eq!(s.units,Some(20));assert!(s.tokens.is_empty());assert!(!s.ready);}
}
#[cfg(test)]#[path="wraith_native_tests.rs"]mod native_tests;
