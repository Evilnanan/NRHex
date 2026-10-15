//! Native on-kill notifications, one aggregate max-FP multiplier, compact saved count.
//! A byte occupies one hidden saved record; kills never allocate visible FP buffs.
use crate::{has_effect,is_local,read_i32,read_ptr};
use std::sync::{Mutex,atomic::{AtomicPtr,Ordering}};

pub const RATE_RVA:usize=0x50c230;
pub const RATE_PATTERN:&str="48 89 5C 24 08 57 48 83 EC 40 48 8B 59 08 0F B6 FA 0F 29 74 24 30 F3 0F 10 35 ? ? ? ? 48 85 DB 74 39 0F 29 7C 24 20 0F 57 FF 0F 1F 44 00 00 F7 43 68 03 00 0C 80 75 15 40 0F B6 D7 48 8B CB E8 5B 41 00 00";
pub static RATE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(std::ptr::null_mut());

// The original getter multiplies active SpEffect maxMpRate values (row+0x14).
// Extend its return value once. No shared PARAM writes or transient row pointers.
fn combined_rate(native:f32,count:u32)->f32{
 if count==0||!native.is_finite()||native<=0.0{return native;}
 ((native as f64)*1.01f64.powf(count as f64)).min(f32::MAX as f64)as f32
}
unsafe fn rate_for(manager:usize,native:f32,base:usize)->f32{
 if manager==0{return native;}
 let player=read_ptr(manager+0x10);
 if !is_local(player,base)||read_ptr(player+0x190)!=manager{return native;}
 if !has_effect(player,OWNER,base)||has_effect(player,RESET,base){return native;}
 let count={let Ok(s)=STATE.lock()else{return native;};s.count};
 combined_rate(native,count.unwrap_or_else(||decode(&snapshot(player).markers)))
}
pub unsafe extern "system" fn rate_detour(manager:usize,mode:u8)->f32{
 let original:unsafe extern "system" fn(usize,u8)->f32=std::mem::transmute(RATE_ORIGINAL.load(Ordering::Acquire));
 let native=original(manager,mode);
 if !crate::ACTIVE.load(Ordering::Acquire){return native;}
 rate_for(manager,native,crate::BASE.load(Ordering::Relaxed))
}

pub const OWNER:i32=95002900;
pub const PULSE:i32=95002901;
pub const RESET:i32=95002903;
pub const FIRST:i32=95006000;
const DIGITS:usize=4;
static STATE:Mutex<Counter>=Mutex::new(Counter{count:None});

#[derive(Default)]
struct Counter{count:Option<u32>}
impl Counter{
 fn observe(&mut self,saved:u32,reset:bool)->u32{
  if reset{self.count=Some(0);}
  *self.count.get_or_insert(saved)
 }
 fn kill(&mut self,saved:u32){let current=self.observe(saved,false);self.count=Some(current.saturating_add(1));}
}
fn marker(digit:usize,value:u8)->Option<i32>{(value!=0).then_some(FIRST+digit as i32*256+value as i32)}
fn decode(ids:&[i32])->u32{
 let mut bytes=[0u8;DIGITS];
 for &id in ids{if (FIRST..FIRST+1024).contains(&id){let offset=(id-FIRST) as usize;bytes[offset/256]=offset as u8;}}
 u32::from_le_bytes(bytes)
}
#[derive(Default)]
struct Native{markers:Vec<i32>}
unsafe fn snapshot(player:usize)->Native{
 let mut result=Native::default();let manager=read_ptr(player+0x190);if manager==0{return result;}
 let mut node=read_ptr(manager+8);
 // Include pending saved records; exclude nodes already removed by the engine.
 for _ in 0..65536{
  if node==0{break;}
  if std::ptr::read_unaligned((node+0x68)as *const u32)&0x80080003==0{
   let id=read_i32(node+8);
   if (FIRST..FIRST+1024).contains(&id){result.markers.push(id);}
  }
  node=read_ptr(node+0x38);
 }
 result
}
pub unsafe fn pulse(player:usize,effect:i32,base:usize)->bool{
 if effect!=PULSE||!is_local(player,base){return false;}
 if !has_effect(player,OWNER,base)||has_effect(player,RESET,base){return true;}
 let saved=decode(&snapshot(player).markers);
 if let Ok(mut s)=STATE.lock(){s.kill(saved);}
 true
}
pub unsafe fn update(player:usize,base:usize){
 let manager=read_ptr(player+0x190);if manager==0{return;}
 let reset=has_effect(player,RESET,base);let owned=has_effect(player,OWNER,base);
 if !owned&&!reset{return;}
 let native=snapshot(player);
 let count={let Ok(mut s)=STATE.lock()else{return;};if owned||reset{s.observe(decode(&native.markers),reset)}else{s.count.unwrap_or(0)}};
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);
 if reset{
  for id in native.markers{remove(manager,id);}
  let remaining=snapshot(player);
  if remaining.markers.is_empty(){remove(manager,RESET);}
  return;
 }
 // Keep the count across temporary ownership loss during revival/rebuilding.
 // Explicit guarded expedition/hub notices are the only reset authority.
 for (digit,value) in count.to_le_bytes().into_iter().enumerate(){
  let wanted=marker(digit,value);
  if let Some(id)=wanted{
   if !has_effect(player,id,base){apply(player,id,0);}
   if !has_effect(player,id,base){continue;}
  }
  for &id in &native.markers{
   if (id-FIRST)/256==digit as i32&&Some(id)!=wanted{remove(manager,id);}
  }
 }

}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn aggregate_preserves_native_bonuses_and_remains_finite(){
  for count in [0,1,50,100,256,1000]{for native in [0.8,1.0,1.2]{
   let expected=native as f64*1.01f64.powi(count as i32);
   assert!((combined_rate(native,count)as f64/expected-1.0).abs()<0.000001);
  }}
  assert!(combined_rate(1.2,u32::MAX).is_finite());
 }
 #[test]fn compact_count_roundtrips_carries_and_never_uses_more_than_four_save_slots(){
  for count in [0,1,2,18,50,99,255,256,257,65535,65536,16777215,16777216,u32::MAX]{
   let ids:Vec<_>=count.to_le_bytes().into_iter().enumerate().filter_map(|(d,b)|marker(d,b)).collect();
   assert_eq!(decode(&ids),count);assert!(ids.len()<=4);
  }
 }
 #[test]fn kills_before_update_are_all_counted_and_revival_does_not_erase_partial_writes(){
  let mut s=Counter::default();for _ in 0..100{s.kill(0);}assert_eq!(s.count,Some(100));
  assert_eq!(s.observe(0,false),100);assert_eq!(s.observe(18,false),100);
  let mut loaded=Counter::default();assert_eq!(loaded.observe(100,false),100);
  loaded.kill(100);assert_eq!(loaded.count,Some(101));
  assert_eq!(s.observe(100,true),0);assert_eq!(s.observe(0,false),0);s.kill(0);assert_eq!(s.count,Some(1));
 }
 #[test]fn saved_records_include_pending_but_exclude_removed_nodes(){unsafe{
  let mut actor=vec![0u8;0x198];let mut manager=vec![0u64;3];let mut nodes=vec![[0u64;17];6];
  for i in 0..6{nodes[i][1]=marker(i%4,7).unwrap()as u64;nodes[i][7]=if i<5{nodes[i+1].as_ptr()as u64}else{0};}
  nodes[1][13]=0x40000;nodes[2][13]=0x80000;nodes[3][13]=0x80000000;
  manager[1]=nodes[0].as_ptr()as u64;std::ptr::write_unaligned(actor.as_mut_ptr().add(0x190)as *mut usize,manager.as_ptr()as usize);
  let n=snapshot(actor.as_ptr()as usize);assert_eq!(n.markers.len(),4);assert_eq!(decode(&n.markers),0x707);
 }}
 #[test]fn actual_runtime_uses_only_compact_records_and_restores_and_resets_count(){unsafe{
  use crate::win::*;
  use std::{cell::RefCell,ptr};
  thread_local!{static NODES:RefCell<Vec<Box<[u64;17]>>>=const{RefCell::new(Vec::new())};}
  unsafe fn link(manager:usize,nodes:&mut [Box<[u64;17]>]){
   ptr::write_unaligned((manager+8)as *mut usize,nodes.first().map_or(0,|n|n.as_ptr()as usize));
   for i in 0..nodes.len(){nodes[i][7]=nodes.get(i+1).map_or(0,|n|n.as_ptr()as u64);}
  }
  unsafe extern "system" fn has(manager:usize,id:i32)->u8{
   let mut node=read_ptr(manager+8);while node!=0{if read_i32(node+8)==id{return 1;}node=read_ptr(node+0x38);}0
  }
  unsafe extern "system" fn add(player:usize,id:i32,_flag:u8)->u8{
   let manager=read_ptr(player+0x190);
   if has(manager,id)!=0{return 1;}
   NODES.with(|s|{let mut s=s.borrow_mut();let mut node=Box::new([0u64;17]);node[1]=id as u64;s.push(node);link(manager,&mut s);});1
  }
  unsafe extern "system" fn remove(manager:usize,id:i32)->u8{
   NODES.with(|s|{let mut s=s.borrow_mut();let Some(i)=s.iter().position(|n|n[1]==id as u64)else{return 0;};s.remove(i);link(manager,&mut s);1})
  }
  let memory=VirtualAlloc(ptr::null_mut(),crate::WORLD_RVA+16,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE)as usize;assert_ne!(memory,0);
  for (rva,function) in [(crate::HAS_EFFECT_RVA,has as *const u8),(crate::APPLY_SELF_RVA,add as *const u8),(crate::REMOVE_EFFECT_RVA,remove as *const u8)]{
   let code=[vec![0x48,0xb8],(function as u64).to_le_bytes().to_vec(),vec![0xff,0xe0]].concat();ptr::copy_nonoverlapping(code.as_ptr(),(memory+rva)as *mut u8,code.len());
  }
  let mut actor=vec![0u64;0x1c0/8];let player=actor.as_mut_ptr()as usize;
  let mut manager=[0u64;3];let m=manager.as_mut_ptr()as usize;actor[0x190/8]=m as u64;manager[2]=player as u64;
  let mut stats=vec![0i32;0x180/4];stats[0x140/4]=100;
  let mut modules=[stats.as_mut_ptr()as usize];actor[0x1b8/8]=modules.as_mut_ptr()as u64;
  let mut world=vec![0u64;0x17500/8];world[0x174e8/8]=player as u64;ptr::write_unaligned((memory+crate::WORLD_RVA)as *mut usize,world.as_ptr()as usize);
  *STATE.lock().unwrap()=Counter::default();add(player,OWNER,0);
  for _ in 0..100{assert!(pulse(player,PULSE,memory));}
  for _ in 0..10{update(player,memory);}
  assert!(snapshot(player).markers.len()<=4);assert!((rate_for(m,1.2,memory)-1.2*1.01f32.powi(100)).abs()<0.0001);assert_eq!(decode(&snapshot(player).markers),100);
  for _ in 0..10{update(player,memory);}assert!(snapshot(player).markers.len()<=4);assert!((rate_for(m,1.2,memory)-1.2*1.01f32.powi(100)).abs()<0.0001);
  // Save/reload restores the aggregate immediately, without recreating layers.
  *STATE.lock().unwrap()=Counter::default();
  stats[0x140/4]=0;update(player,memory);assert!(snapshot(player).markers.len()<=4);
  stats[0x140/4]=100;update(player,memory);assert!(snapshot(player).markers.len()<=4);assert!((rate_for(m,1.0,memory)-1.01f32.powi(100)).abs()<0.0001);
  assert!(pulse(player,PULSE,memory));assert!(pulse(player,PULSE,memory));
  for _ in 0..10{update(player,memory);}assert!(snapshot(player).markers.len()<=4);assert!((rate_for(m,1.0,memory)-1.01f32.powi(102)).abs()<0.0001);
  assert_eq!(decode(&snapshot(player).markers),102);
  assert!(!pulse(player,12345,memory));assert!(!pulse(1,PULSE,memory));
  // Reset blocks the multiplier immediately and clears the compact records.
  add(player,RESET,0);update(player,memory);assert_eq!(rate_for(m,1.2,memory),1.2);
  for _ in 0..10{update(player,memory);}
  assert!(snapshot(player).markers.len()<=4);assert!(snapshot(player).markers.is_empty());assert!(!has_effect(player,RESET,memory));
  assert!(pulse(player,PULSE,memory));update(player,memory);assert!(snapshot(player).markers.len()<=4);assert!((rate_for(m,1.0,memory)-1.01).abs()<0.000001);
  remove(m,OWNER);assert!(pulse(player,PULSE,memory));update(player,memory);assert!(snapshot(player).markers.len()<=4);assert_eq!(rate_for(m,1.2,memory),1.2);
  add(player,OWNER,0);assert!((rate_for(m,1.0,memory)-1.01).abs()<0.000001);
  manager[2]=1;assert_eq!(rate_for(m,1.2,memory),1.2);
  NODES.with(|s|s.borrow_mut().clear());*STATE.lock().unwrap()=Counter::default();assert_eq!(VirtualFree(memory as *mut _,0,MEM_RELEASE),1);
 }}
}
