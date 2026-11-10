//! Retain native locomotion under an already mobile item/spell/art layer.
//! TAE JumpTable 90/91 limit speed; 89 (immobility) is never intercepted.
use crate::{has_effect,is_local,native_actor,ptr,read_i32,read_ptr,ACTIVE,BASE};
use std::sync::{Mutex,atomic::{AtomicPtr,Ordering}};
pub const OWNER:i32=95005400;
pub const QUERY:i32=990058;
pub const TAE_RVA:usize=0x40d390;
pub const TAE_PATTERN:&str="48 89 5C 24 10 48 89 74 24 18 48 89 7C 24 20 55 41 54 41 55 41 56 41 57 48 8B EC 48 83 EC 70 48 8B 71 18 48 8B F9 44 0F B6 6A 18 48 8B CE 4C 8B FA 48 8B 86 B8 01 00";
pub static TAE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
#[derive(Clone,Copy,Default,PartialEq,Eq)]struct Identity{actor:usize,handle:u64,stats:usize}
#[derive(Default)]struct State{owner:Identity,active:bool,missed:u8}
static STATE:Mutex<State>=Mutex::new(State{owner:Identity{actor:0,handle:0,stats:0},active:false,missed:0});
fn mobile(upper:i32,lower:i32)->bool{lower==1&&matches!(upper,11|12|104)}
impl State{
 fn observe(&mut self,owner:Identity,active:bool){*self=Self{owner,active,missed:0};}
 fn tick(&mut self,owner:Option<Identity>){
  if owner!=Some(self.owner){*self=Self::default();return;}
  self.missed=self.missed.saturating_add(1);
  if self.missed>1{self.active=false;}
 }
 fn allows(&self,owner:Identity,flag:i32)->bool{self.active&&self.owner==owner&&matches!(flag,90|91)}
}
unsafe fn identity(actor:usize,base:usize)->Option<Identity>{
 if !is_local(actor,base)||!has_effect(actor,OWNER,base)||has_effect(actor,crate::reward_cycle_runtime::RESET,base){return None;}
 let stats=native_actor::stats(actor);
 if stats==0||native_actor::incapacitated(actor,stats){return None;}
 Some(Identity{actor,stats,handle:ptr::read_unaligned((actor+8)as*const u64)})
}
unsafe fn unrestricted(owner:Identity,base:usize)->bool{
 crate::parry::grounded(owner.actor)&&!has_effect(owner.actor,100200,base)
  &&read_i32(owner.stats+0x15c)>0
  &&![100020,701530,503520,5520,425,426,42312,4101,4100,46328,46329].into_iter().any(|id|has_effect(owner.actor,id,base))
}
/// Script supplies current layer states. Every ordinary event transition
/// invalidates the observation before firing, so the old action cannot lend
/// permission to a new full-body attack, roll, hurt, mount or death state.
pub unsafe fn query(actor:usize,upper:i32,lower:i32,base:usize)->i32{
 if !is_local(actor,base){return 0;}
 let owner=identity(actor,base);
 let enabled=mobile(upper,lower)&&owner.is_some_and(|o|unrestricted(o,base));
 if let Ok(mut s)=STATE.lock(){if let Some(owner)=owner{s.observe(owner,enabled);}else{*s=State::default();}}
 i32::from(enabled)
}
pub unsafe fn update(actor:usize,base:usize){let owner=identity(actor,base);if let Ok(mut s)=STATE.lock(){s.tick(owner);}}
pub fn reset(){if let Ok(mut s)=STATE.lock(){*s=State::default();}}
type Tae=unsafe extern "system" fn(usize,usize);
pub unsafe extern "system" fn tae(event:usize,descriptor:usize){
 let original:Tae=std::mem::transmute(TAE_ORIGINAL.load(Ordering::Acquire));
 if ACTIVE.load(Ordering::Acquire)&&event!=0&&descriptor!=0{
  let data=read_ptr(descriptor+8);
  if data!=0&&matches!(read_i32(data),90|91){
   let actor=read_ptr(event+0x18);let base=BASE.load(Ordering::Acquire);
   if let Some(owner)=identity(actor,base){
    if STATE.lock().is_ok_and(|s|s.allows(owner,read_i32(data)))&&unrestricted(owner,base){return;}
   }
  }
 }
 original(event,descriptor);
}
#[cfg(test)]mod tests{
 use super::*;
 fn id()->Identity{Identity{actor:1,handle:2,stats:3}}
 #[test]fn only_existing_mobile_layers_qualify(){
  for u in 0..160{for l in 0..160{assert_eq!(mobile(u,l),l==1&&[11,12,104].contains(&u));}}
 }
 #[test]fn speed_limits_only_and_transitions_clear_immediately(){
  let mut s=State::default();s.observe(id(),true);
  for flag in -1..170{assert_eq!(s.allows(id(),flag),matches!(flag,90|91));}
  for changed in [Identity{actor:4,..id()},Identity{handle:4,..id()},Identity{stats:4,..id()}]{assert!(!s.allows(changed,90));}
  s.observe(id(),false);assert!(!s.allows(id(),90));
  s.observe(id(),true);s.tick(Some(id()));assert!(s.allows(id(),91));s.tick(Some(id()));assert!(!s.allows(id(),91));
  s.observe(id(),true);s.tick(None);assert!(!s.allows(id(),90));
 }
 #[test]fn unowned_dispatch_preserves_descriptor_and_native_call(){unsafe{
  unsafe extern "system" fn native(event:usize,descriptor:usize){ptr::write_unaligned(event as*mut usize,descriptor);}
  TAE_ORIGINAL.store(native as*mut u8,Ordering::Release);let mut slot=0usize;
  tae(&mut slot as*mut usize as usize,123);assert_eq!(slot,123);
 }}
}

#[cfg(test)]#[path="light_footed_native_tests.rs"]mod native_tests;
