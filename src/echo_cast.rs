//! Delayed spell-root replay through native BulletMan, with retained native
//! target/snapshot references. Descendants and replay dispatch are not echoed.
use crate::{ptr,read_ptr,read_i32,has_effect,is_local,native_actor,ACTIVE,BASE};
use std::{cell::Cell,sync::{Mutex,atomic::{AtomicPtr,Ordering}}};
pub const OWNER:i32=95003500;
pub const EMIT_RVA:usize=0x35a460;
pub const FACTORY_RVA:usize=0x35da70;
pub const FACTORY_PATTERN:&str="48 89 5C 24 08 55 56 57 41 54 41 55 41 56 41 57 48 83 EC 40 4D 8B F0 48 8B EA 48 8B F1";
pub const TARGET_COPY_RVA:usize=0x3482e0;
pub const TARGET_COPY_PATTERN:&str="48 89 5C 24 08 57 48 83 EC 20 0F B6 02 48 8B D9 88 01 48 8B FA 48 8B 42 08";
pub const RELEASE_RVA:usize=0x33e650;
pub const RELEASE_PATTERN:&str="48 89 4C 24 08 53 48 83 EC 20 48 8B D9 48 8B 89 98 00 00 00 48 85 C9 74 20 8B 41 18 8D 50 FF 33 D0 81 E2 FF FF FF 3F 33 D0 89 51 18 F7 C2 FF FF FF 3F 75 05 E8 37 02 1F 00 48 8B 8B A0 00 00 00 48 85 C9 74 1F 8B 41 28 8D 50 FF 33 D0 0F BA F2 1F 33 D0 89";
pub const TARGET_VALID_RVA:usize=0x3494c0;
pub const TARGET_VALID_PATTERN:&str="48 89 5C 24 08 57 48 83 EC 30 48 8B 51 10 48 8B F9 48 85 D2";
pub static EMIT_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static FACTORY_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
type Emit=unsafe extern "system" fn(usize,usize,usize,usize)->usize;
type Factory=unsafe extern "system" fn(usize,usize,usize)->usize;
#[repr(C,align(16))]struct Descriptor([u8;0x110]);
struct Ticket{descriptor:Box<Descriptor>,pose:[u8;0x40],base:usize,wait:f64,area:bool,entity_target:bool}
impl Drop for Ticket{fn drop(&mut self){unsafe{
 let release:unsafe extern "system" fn(usize)=std::mem::transmute(self.base+RELEASE_RVA);
 release(self.descriptor.0.as_mut_ptr()as usize);
}}}
#[derive(Clone,Copy,Default,PartialEq,Eq)]struct Owner{actor:usize,handle:u64,stats:usize,context:usize}
#[derive(Default)]struct Queue{owner:Owner,pending:Vec<Ticket>}
static QUEUE:Mutex<Queue>=Mutex::new(Queue{owner:Owner{actor:0,handle:0,stats:0,context:0},pending:Vec::new()});
#[derive(Clone,Copy,Default)]struct Scope{descriptor:usize,owner:Owner,consumed:bool}
#[derive(Clone,Copy,Default)]struct Replay{ticket:usize,consumed:bool}
thread_local!{static SCOPE:Cell<Scope>=Cell::new(Scope::default());static REPLAY:Cell<Replay>=Cell::new(Replay::default());}
struct ScopeGuard(Scope);
impl Drop for ScopeGuard{fn drop(&mut self){SCOPE.with(|s|s.set(self.0));}}
struct ReplayGuard(Replay);
impl Drop for ReplayGuard{fn drop(&mut self){REPLAY.with(|s|s.set(self.0));}}
fn eligible(magic:i32,bullet:i32)->bool{
 area_replay(magic,bullet).is_some()||crate::echo_cast_data::ROOTS.binary_search_by_key(&magic,|r|r.0).ok().is_some_and(|i|crate::echo_cast_data::ROOTS[i].1.binary_search(&bullet).is_ok())
}
fn area_replay(magic:i32,bullet:i32)->Option<i32>{
 crate::echo_cast_data::AREAS.binary_search_by_key(&magic,|r|r.0).ok().and_then(|i|{
  let (_,root,replay)=crate::echo_cast_data::AREAS[i];(root==bullet).then_some(replay)
 })
}
unsafe fn owner(actor:usize,base:usize)->Option<Owner>{
 if !is_local(actor,base)||!has_effect(actor,OWNER,base){return None;}
 let stats=native_actor::stats(actor);if stats==0||native_actor::incapacitated(actor,stats){return None;}
 Some(Owner{actor,handle:ptr::read_unaligned((actor+8)as*const u64),stats,context:read_ptr(base+0x3c131c8)})
}
unsafe fn scope(descriptor:usize,base:usize)->Option<Owner>{
 if descriptor==0||read_i32(descriptor+8)>=0||read_ptr(descriptor+0x90)!=0{return None;}
 if !eligible(read_i32(descriptor+0x18),read_i32(descriptor+0x14)){return None;}
 let world=read_ptr(base+crate::WORLD_RVA);if world==0{return None;}
 let actor=read_ptr(world+0x174e8);let value=owner(actor,base)?;
 (ptr::read_unaligned(descriptor as*const u64)==value.handle).then_some(value)
}
pub unsafe extern "system" fn emit(manager:usize,out:usize,descriptor:usize,error:usize)->usize{
 let original:Emit=std::mem::transmute(EMIT_ORIGINAL.load(Ordering::Acquire));
 let value=if ACTIVE.load(Ordering::Acquire)&&REPLAY.with(Cell::get).ticket==0{scope(descriptor,BASE.load(Ordering::Acquire))}else{None};
 let _guard=ScopeGuard(SCOPE.with(|s|s.replace(value.map_or(Scope::default(),|owner|Scope{descriptor,owner,consumed:false}))));
 original(manager,out,descriptor,error)
}
unsafe fn retain_counter(pointer:usize,offset:usize,mask:u32){
 if pointer==0{return;}
 let address=(pointer+offset)as*mut u32;let old=ptr::read_unaligned(address);
 ptr::write_unaligned(address,(old&!mask)|(old.wrapping_add(1)&mask));
}
unsafe fn snapshot(scope:Scope,payload:usize,base:usize)->Option<Ticket>{
 if payload==0||ptr::read_unaligned((payload+0x10)as*const u64)!=scope.owner.handle{return None;}
 let replay=area_replay(read_i32(scope.descriptor+0x18),read_i32(scope.descriptor+0x14));
 let targets=read_ptr(payload+0x148);let end=read_ptr(payload+0x150);
 // Independent root shots use one native target descriptor. Multi-target
 // selection needs a separate replay representation rather than choosing one.
 if replay.is_none()&&(targets==0||end.wrapping_sub(targets)!=0x50){return None;}
 let mut d=Box::new(Descriptor(ptr::read_unaligned(scope.descriptor as*const[u8;0x110])));
 // The copied target is replaced with the target already resolved by native
 // calculation, not the player's lock-on at replay time.
 if let Some(bullet)=replay{
  d.0[0x14..0x18].copy_from_slice(&bullet.to_le_bytes());
  // These stationary radial areas use the captured world pose. Clear entity
  // targeting and its reference, with the native empty handle/index sentinels.
  d.0[0xb0..0x100].fill(0);
  d.0[0xb8..0xc0].fill(0xff);d.0[0xe0..0xe4].fill(0xff);d.0[0xf0..0xf4].fill(0xff);
 }else{
  ptr::write_unaligned(d.0.as_mut_ptr().add(0xc0)as*mut usize,0);
  let copy:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(base+TARGET_COPY_RVA);
  copy(d.0.as_mut_ptr().add(0xb0)as usize,targets);
 }
 retain_counter(read_ptr(scope.descriptor+0x98),0x18,0x3fffffff);
 retain_counter(read_ptr(scope.descriptor+0xa0),0x28,0x7fffffff);
 // Native flag0 requests an independent shot; bit3 retains native networking.
 let flags=read_i32(scope.descriptor+0x44)as u32|1;
 d.0[0x44..0x48].copy_from_slice(&flags.to_le_bytes());
 let entity_target=replay.is_none()&&(read_ptr(d.0.as_ptr()as usize+0xb8)!=usize::MAX||read_ptr(d.0.as_ptr()as usize+0xc0)!=0);
 Some(Ticket{descriptor:d,pose:ptr::read_unaligned((payload+0x190)as*const[u8;0x40]),base,wait:0.5,area:replay.is_some(),entity_target})
}
pub unsafe extern "system" fn factory(manager:usize,payload:usize,error:usize)->usize{
 let original:Factory=std::mem::transmute(FACTORY_ORIGINAL.load(Ordering::Acquire));
 let scope=SCOPE.with(|s|{let v=s.get();s.set(Scope{consumed:true,..v});v});
 let base=BASE.load(Ordering::Acquire);
 let replay=REPLAY.with(|r|{let v=r.get();r.set(Replay{consumed:true,..v});v});
 if replay.ticket!=0&&!replay.consumed{
  let ticket=&*(replay.ticket as*const Ticket);
  if !ticket.area{
   let targets=read_ptr(payload+0x148);let end=read_ptr(payload+0x150);
   if targets==0||end.wrapping_sub(targets)!=0x50{return 0;}
   // Native calculation may reselect a current lock for a forward-only
   // descriptor. Restore its original resolved target immediately before
   // allocation; native networking then serializes the restored result too.
   let copy:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(base+TARGET_COPY_RVA);
   copy(targets,ticket.descriptor.0.as_ptr().add(0xb0)as usize);
  }
  ptr::copy_nonoverlapping(ticket.pose.as_ptr(),(payload+0x190)as*mut u8,0x40);
 }
 let ticket=if replay.ticket==0&&scope.descriptor!=0&&!scope.consumed{snapshot(scope,payload,base)}else{None};
 let result=original(manager,payload,error);
 if result!=0{if let Some(ticket)=ticket{
  let stale=if let Ok(mut q)=QUEUE.lock(){let old=if q.owner!=scope.owner{q.owner=scope.owner;std::mem::take(&mut q.pending)}else{Vec::new()};q.pending.push(ticket);old}else{Vec::new()};drop(stale);
 }}
 result
}
pub unsafe fn update(actor:usize,base:usize,dt:f32){
 let current=owner(actor,base);
 let (mut ready,discard)=if let Ok(mut q)=QUEUE.lock(){
  if current!=Some(q.owner){(Vec::new(),std::mem::take(&mut q.pending))}else{
   let mut ready=Vec::new();let mut waiting=Vec::new();
   for mut ticket in std::mem::take(&mut q.pending){if dt.is_finite()&&dt>0.{ticket.wait-=f64::from(dt);}if ticket.wait<=1e-7{ready.push(ticket);}else{waiting.push(ticket);}}
   q.pending=waiting;(ready,Vec::new())
  }
 }else{return;};drop(discard);
 let manager=read_ptr(base+0x3c0ae30);if manager==0{return;}
 let emit:Emit=std::mem::transmute(EMIT_ORIGINAL.load(Ordering::Acquire));
 let valid:unsafe extern "system" fn(usize)->u8=std::mem::transmute(base+TARGET_VALID_RVA);
 for ticket in &mut ready{
  let descriptor=ticket.descriptor.0.as_mut_ptr()as usize;
  // A vanished target must not make native calculation choose a new lock-on.
  if ticket.entity_target&&valid(descriptor+0xb0)==0{continue;}
  let _replay=ReplayGuard(REPLAY.with(|r|r.replace(Replay{ticket:ticket as*const Ticket as usize,consumed:false})));
  let mut out=-1i32;let mut error=-1i32;emit(manager,&mut out as*mut i32 as usize,descriptor,&mut error as*mut i32 as usize);
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn root_filter_does_not_echo_children_or_sustained_casts(){
  assert!(eligible(4021,10402100));assert!(eligible(4021,10402150));assert!(!eligible(4021,10402151));assert!(!eligible(4200,10420000));assert!(!eligible(7000,10700000));assert!(!eligible(4021,10462000));
  for (magic,roots)in [(4120,&[10412000,10412001][..]),(4130,&[10413000][..]),(4510,&[10451000,10451010,10451020][..]),(4700,&[10470000,10470005,10470010,10470020,10470030,10470040][..]),(4701,&[10470100,10470110,10470120,10470150,10470151,10470152][..]),(7060,&[10706000,10706002][..]),(7090,&[10709000,10709001][..]),(7240,&[10724000][..]),(7330,&[10733000,10733001,10999999][..])]{
   for &root in roots{assert!(!eligible(magic,root));}
  }
 }
 #[test]fn retain_preserves_native_high_flag_bits(){unsafe{
  let mut a=[0u32;16];a[6]=0xc0000002;a[10]=0x80000005;let p=a.as_mut_ptr()as usize;retain_counter(p,0x18,0x3fffffff);retain_counter(p,0x28,0x7fffffff);assert_eq!(a[6],0xc0000003);assert_eq!(a[10],0x80000006);
 }}
 #[test]fn group_profiles_replace_only_the_native_cast_root(){
  assert_eq!(crate::echo_cast_data::AREAS.len(),11);
  for &(magic,root,replay) in crate::echo_cast_data::AREAS{
   assert!(eligible(magic,root));assert_eq!(area_replay(magic,root),Some(replay));
   assert!(!eligible(magic,replay));assert!(!eligible(magic,root+1));
  }
  for magic in [6420,6760,6971]{assert!(!eligible(magic,10000000+magic*100));}
 }
 #[test]fn volley_roots_include_no_cost_sides_but_not_helpers_or_descendants(){
  for root in [10409000,10409001,10409002,10409050,10409051,10409052]{assert!(eligible(4090,root));}
  for root in [10450010,10450011,10450012,10450060,10450061,10450062]{assert!(eligible(4500,root));}
  for root in [10682010,10682011,10682012,10682070,10682071,10682072]{assert!(eligible(6820,root));}
  assert!(eligible(4220,10422000));assert!(eligible(4220,10422050));assert!(!eligible(4220,10422015));
  assert!(!eligible(4220,10422098));assert!(!eligible(4220,10422099));assert!(!eligible(4050,10404099));
  assert!(!eligible(4060,10406000));assert!(!eligible(4520,10452000));assert!(!eligible(7900,10790001));
 }
}
#[cfg(test)]
#[path="echo_cast_native_tests.rs"]mod native_tests;
