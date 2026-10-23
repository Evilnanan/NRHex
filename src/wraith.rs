//! Royal Revenant: native item-use provenance and first-contact Bagcraft credit.
//! Inventory transitions are acknowledged before the previous native tool is removed.
use crate::{ptr,read_i32,read_ptr,has_effect,is_local,native_actor,ACTIVE,BASE};
use std::{cell::Cell,collections::BTreeMap,sync::{Mutex,OnceLock,atomic::{AtomicPtr,Ordering}}};
pub const OWNER:i32=95004300;
pub const GROUND_LOT:i32=95543000;
pub fn ground_tier(lot:i32)->Option<usize>{let offset=lot.checked_sub(GROUND_LOT)?;(matches!(offset,0|10|20)).then_some((offset/10)as usize)}
pub const ORIGINALS:[i32;3]=[3080,3010,3011];
pub const FRAME:i32=990043;pub const BUSY:i32=990044;
pub const CAN_RVA:usize=0x6d0470;pub const USE_RVA:usize=0x3979b0;pub const PARAM_RVA:usize=0xe8d840;
pub const TOKEN_RVA:usize=0x66d3b0;pub const XP_RVA:usize=0x209490;
pub const CAN_PATTERN:&str="48 89 5C 24 20 55 56 57 41 54 41 55 41 56 41 57 48 8D AC 24 10 FF FF FF 48 81 EC F0 01 00 00 48 8B 05 D2 5A 41 03 48 33 C4 48 89 85 E0 00 00 00 41 8B D9 4C 89 45 90 4C 8B E2 44 8B E9";
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
#[derive(Clone,Copy,Default)]struct Credit{actor:usize,good:i32,cycle:u64}
thread_local!{static FREE:Cell<bool>=const{Cell::new(false)};static CREDIT:Cell<Option<Credit>>=const{Cell::new(None)};}
struct FreeGuard(bool);impl FreeGuard{fn enter(value:bool)->Self{Self(FREE.with(|s|s.replace(value)))}}impl Drop for FreeGuard{fn drop(&mut self){FREE.with(|s|s.set(self.0));}}
#[repr(C,align(16))]struct GoodRow([u8;0xe0]);
static SHADOWS:[OnceLock<Box<GoodRow>>;3]=[OnceLock::new(),OnceLock::new(),OnceLock::new()];
#[derive(Default)]struct State{actor:usize,handle:u64,units:Option<[u8;2]>,cycles:[u64;2],given:Option<u8>,removing:Option<(usize,i32)>,tokens:BTreeMap<i32,Credit>,ages:BTreeMap<i32,f64>,saw_item:bool,idle_frames:u8,ready:bool,requested:Option<u8>,request_frames:u8}
static STATE:Mutex<State>=Mutex::new(State{actor:0,handle:0,units:None,cycles:[0;2],given:None,removing:None,tokens:BTreeMap::new(),ages:BTreeMap::new(),saw_item:false,idle_frames:0,ready:false,requested:None,request_frames:0});
fn tier(good:i32)->Option<usize>{ORIGINALS.iter().position(|&id|id==good)}
const LIMITS:[u8;2]=[20,10];
// Reuse the existing saved marker range: bell 11..30, bairn 31..40.
fn marker(kind:usize,count:u8)->i32{OWNER+10+if kind==0{0}else{20}+i32::from(count)}
fn restored(markers:&[i32])->[u8;2]{
 let mut counts=[0;2];for &id in markers{let n=id-OWNER-10;if (1..=20).contains(&n){counts[0]=counts[0].max(n as u8);}else if (21..=30).contains(&n){counts[1]=counts[1].max((n-20)as u8);}}counts
}
fn progressed(mut counts:[u8;2],good:i32)->[u8;2]{if let Some(i)=tier(good).filter(|&i|i<2){counts[i]=counts[i].saturating_add(1).min(LIMITS[i]);}counts}
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
// Shared eligibility is called directly by selected-item input (3CFE48),
// action checks (3D4A3F) and UI checks (6D0437). Scoping only 3D49A0 misses
// the earlier input gate, so an empty FP bar never reaches actual use.
type CanUse=unsafe extern "system" fn(i32,usize,usize,i32,i32,i32,u8)->u8;
pub unsafe extern "system" fn can_use(good:i32,actor:usize,manager:usize,kind:i32,right:i32,left:i32,flag:u8)->u8{
 let original:CanUse=std::mem::transmute(CAN_ORIGINAL.load(Ordering::Acquire));let base=BASE.load(Ordering::Acquire);
 let free=ACTIVE.load(Ordering::Acquire)&&tier(good).is_some()&&enabled(actor,base);
 let _guard=FreeGuard::enter(free);original(good,actor,manager,kind,right,left,flag)
}
type Use=unsafe extern "system" fn(usize,i32,usize,u8,u8,f32)->u8;
pub unsafe extern "system" fn used(context:usize,good:i32,data:usize,flag:u8,entry:u8,elapsed:f32)->u8{
 let original:Use=std::mem::transmute(USE_ORIGINAL.load(Ordering::Acquire));let base=BASE.load(Ordering::Acquire);let actor=if context==0{0}else{read_ptr(context)};
 let free=ACTIVE.load(Ordering::Acquire)&&tier(good).is_some()&&enabled(actor,base);let _guard=FreeGuard::enter(free);original(context,good,data,flag,entry,elapsed)
}
// Turn only our single-tool lottery result into the ordinary placed-item format.
// Native 574B50 uses kind 1, ItemMan+1C drop UIDs (bit29), enabled bit6,
// no source/lot and option index FF. Kind 2/bit30 belong to choice rewards.
// Keep native item data and map/position, and never remove the preowned upgrade target.
pub unsafe fn place(manager:usize,descriptor:usize,original:unsafe extern "system" fn(usize,usize)->usize)->Option<usize>{
 let scope=crate::PERSONAL_REWARD.with(Cell::get);
 let kind=ground_tier(scope.lot)?;
 if descriptor==0||scope.manager==0||manager!=scope.manager||read_i32(descriptor+0xb8)!=scope.lot{return None;}
 let owner=ptr::read_unaligned((descriptor+0xaa)as*const u16);
 if ((owner>>7)&3)as u8!=scope.slot||read_i32(descriptor+0x58)&15!=2||read_i32(descriptor+8)!=(ORIGINALS[kind]|0x40000000)||ptr::read((descriptor+0xa9)as*const u8)!=0||scope.attempted!=0{return Some(0);}
 crate::PERSONAL_REWARD.with(|s|{let mut state=s.get();state.attempted=1;s.set(state);});
 let mut copy=ptr::read_unaligned(descriptor as*const[u64;24]);let p=copy.as_mut_ptr()as usize;
 let counter=(read_i32(manager+0x1c)as u32).wrapping_add(1);ptr::write_unaligned((manager+0x1c)as*mut u32,counter%0xffffff);
 let native_uid=ptr::read_unaligned((descriptor+0x5c)as*const u32);
 ptr::write_unaligned((p+0x58)as*mut u32,1); // ordinary placed item, local creation
 ptr::write_unaligned((p+0x5c)as*mut u32,(native_uid&0x18000000)|0x20000000|(counter&0x7ffffff));
 ptr::write_unaligned((p+0x60)as*mut u64,u64::MAX);ptr::write_unaligned((p+0x68)as*mut u64,u64::MAX);
 ptr::write_unaligned((p+0x70)as*mut u32,0);ptr::write_unaligned((p+0xac)as*mut i32,-1);ptr::write_unaligned((p+0xbc)as*mut u32,0);
 ptr::write((p+0xa9)as*mut u8,0xff);ptr::write_unaligned((p+0xaa)as*mut u16,0x40);ptr::write_unaligned((p+0xb8)as*mut i32,-1);
 let node=original(manager,p);
 if node!=0{crate::PERSONAL_REWARD.with(|s|{let mut state=s.get();state.created+=1;state.nodes[0]=node;s.set(state);});}
 Some(node)
}
// Shared root dispatch already preserves native source identity and networking.
pub unsafe fn emitted(descriptor:usize,out:usize,base:usize){
 if !ACTIVE.load(Ordering::Acquire)||descriptor==0||out==0||read_i32(descriptor+8)>=0{return;}
 let good=read_i32(descriptor+0x18);if tier(good).is_none(){return;}
 let actor=local(base);if !enabled(actor,base)||ptr::read_unaligned(descriptor as*const u64)!=ptr::read_unaligned((actor+8)as*const u64){return;}
 let token=read_i32(out);if token<0{return;}
 let register:unsafe extern "system" fn(usize,*const i32)=std::mem::transmute(base+0x66f040);register(actor,&token);
 if let Ok(mut s)=STATE.lock(){s.bind(actor,ptr::read_unaligned((actor+8)as*const u64));let cycle=tier(good).filter(|&i|i<2).map_or(0,|i|if s.removing.is_some_and(|(kind,_)|kind==i){s.cycles[i].wrapping_sub(1)}else{s.cycles[i]});s.tokens.insert(token,Credit{actor,good,cycle});s.ages.insert(token,0.);}
}
pub unsafe extern "system" fn token(actor:usize,id:*const i32)->u8{
 CREDIT.with(|s|s.set(None));let original:unsafe extern "system" fn(usize,*const i32)->u8=std::mem::transmute(TOKEN_ORIGINAL.load(Ordering::Acquire));let result=original(actor,id);
 let base=BASE.load(Ordering::Acquire);
 if result!=0&&!id.is_null()&&ACTIVE.load(Ordering::Acquire)&&is_local(actor,base){
  let good=STATE.lock().ok().and_then(|mut s|if s.actor==actor{let token=ptr::read_unaligned(id);s.ages.remove(&token);s.tokens.remove(&token)}else{None});
  if let Some(credit)=good{CREDIT.with(|s|s.set(Some(credit)));}
 }result
}
#[unsafe(naked)]pub unsafe extern "system" fn xp(_:usize,_:u8,_:u16){core::arch::naked_asm!("mov r9,[rsp]","jmp {callback}",callback=sym credited);}
unsafe extern "system" fn credited(data:usize,category:u8,points:u16,caller:usize){
 let original:unsafe extern "system" fn(usize,u8,u16)=std::mem::transmute(XP_ORIGINAL.load(Ordering::Acquire));let base=BASE.load(Ordering::Acquire);
 if caller==base+0x34e0b5&&category==9&&ACTIVE.load(Ordering::Acquire){if let Some(c)=CREDIT.with(|s|s.take()){
  if enabled(c.actor,base){let markers=saved(c.actor);if let Ok(mut s)=STATE.lock(){if let Some(i)=tier(c.good).filter(|&i|i<2){if c.cycle==s.cycles[i]&&!s.removing.is_some_and(|(kind,_)|kind==i){let old=*s.units.get_or_insert_with(||restored(&markers));s.units=Some(progressed(old,c.good));}}}
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
 let (units,ready,requested,request_frames,given,removing)={let Ok(mut s)=STATE.lock()else{return;};s.bind(actor,ptr::read_unaligned((actor+8)as*const u64));let units=*s.units.get_or_insert_with(||restored(&markers));s.given=native_stage.map(|n|n as u8).max(s.given);(units,s.ready,s.requested,s.request_frames,s.given,s.removing)};
 let wanted:Vec<_>=(0..2).filter(|&i|units[i]>0).map(|i|marker(i,units[i])).collect();for &id in &wanted{if !markers.contains(&id){set(actor,id,true,base);}}for id in markers{if !wanted.contains(&id){set(actor,id,false,base);}}
 if !ready||native_actor::incapacitated(actor,stats){return;}
 let Some((root,slots))=inventory(actor)else{return;};
 // Removal is asynchronous. Do not turn the same physical item into a second
 // upgrade while its previous remove command is still waiting to execute.
 if let Some((kind,index))=removing{
  if has_effect(actor,OWNER+7+kind as i32,base)||find(root,ORIGINALS[kind],base)==index{return;}
  if let Ok(mut s)=STATE.lock(){s.removing=None;}
 }
 let desired=if let Some(desired)=requested{i32::from(desired)}else if given.is_none(){0}else{
  let Some(kind)=(0..2).find(|&i|units[i]>=LIMITS[i]&&find(root,ORIGINALS[i],base)>=0)else{return;};kind as i32+1
 };
 let index=find(root,ORIGINALS[desired as usize],base);
 if index>=0&&requested.is_none(){
  // Preserve the preowned tool. Every new copy beyond native capacity is an
  // ordinary placed item, including repeated upgrades with a preowned target.
  if !crate::spatial_ready(actor)||crate::CALLBACK_LOCK.swap(true,Ordering::AcqRel){return;}
  let created=crate::spawn_personal_lot_kind(actor,base,GROUND_LOT+desired*10,None,1);
  crate::CALLBACK_LOCK.store(false,Ordering::Release);
  if created==0{return;}
  if desired==0{set(actor,OWNER+90,true,base);if let Ok(mut s)=STATE.lock(){s.given=Some(0);}return;}
 }
 if index<0{
  if requested!=Some(desired as u8)||(request_frames>=30&&!has_effect(actor,OWNER+1+desired,base)){set(actor,OWNER+1+desired,true,base);if let Ok(mut s)=STATE.lock(){s.requested=Some(desired as u8);s.request_frames=0;}}
  return;
 }
 if desired>0{
  let old_stage=desired-1;let old=find(root,ORIGINALS[old_stage as usize],base);
  if old>=0{let old_slots:Vec<_>=(0..18).filter(|&i|read_i32(slots+12+i*8)==old).collect();
   if !old_slots.is_empty(){if requested==Some(desired as u8){for i in 0..18{if read_i32(slots+12+i*8)==index{equip(slots,i as i32,-1,base);}}}for i in old_slots{equip(slots,i as i32,index,base);}}
  }
  if old>=0{set(actor,OWNER+7+old_stage,true,base);}
  if let Ok(mut s)=STATE.lock(){
   let kind=old_stage as usize;let mut counts=s.units.unwrap_or([0;2]);counts[kind]=0;s.units=Some(counts);s.cycles[kind]=s.cycles[kind].wrapping_add(1);
   // Keep provenance until contact/expiry, including obsolete cycles. The
   // native XP hook still needs it to suppress Scholar-only XP on other heroes.
   // The advanced cycle rejects old flights without deleting native records.
   s.removing=(old>=0).then_some((kind,old));
  }
  for count in 1..=LIMITS[old_stage as usize]{set(actor,marker(old_stage as usize,count),false,base);}

 }
 set(actor,OWNER+1+desired,false,base);
 set(actor,OWNER+90+desired,true,base);
 if let Ok(mut s)=STATE.lock(){s.given=s.given.max(Some(desired as u8));s.requested=None;}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn independent_repeatable_progress_and_saved_markers(){
  let mut counts=[0;2];for _ in 0..20{counts=progressed(counts,3080);}assert_eq!(counts,[20,0]);
  for _ in 0..4{counts=progressed(counts,3010);}assert_eq!(counts,[20,4]);assert_eq!(progressed(counts,3011),counts);
  counts[0]=0;counts=progressed(counts,3080);assert_eq!(counts,[1,4]);
  for _ in 0..20{counts=progressed(counts,3010);}assert_eq!(counts,[1,10]);
  assert_eq!(restored(&[marker(0,1),marker(1,10)]),counts);counts[1]=0;
  assert_eq!(restored(&[marker(0,1)]),counts);assert_eq!(restored(&[]),[0;2]);assert_eq!(tier(OWNER+80),None);
 }
 #[test]fn chained_actions_and_first_two_empty_samples_cannot_replace_and_rebinding_keeps_progress(){let mut s=State::default();s.units=Some([20,4]);s.bind(1,10);for _ in 0..100{s.saw_item=true;s.frame();assert!(!s.ready);}s.frame();assert!(!s.ready);s.frame();assert!(!s.ready);s.frame();assert!(s.ready);s.saw_item=true;s.frame();assert!(!s.ready);s.tokens.insert(7,Credit{actor:1,good:3080,cycle:0});s.bind(2,11);assert_eq!(s.units,Some([20,4]));assert!(s.tokens.is_empty());assert!(!s.ready);}
}
#[cfg(test)]#[path="wraith_native_tests.rs"]mod native_tests;
