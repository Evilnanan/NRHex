//! Turn only after an accepted guard; a mixed-attacker chain stays mixed.
//! Retain generation-bearing identities and copied positions, never enemy pointers.
use crate::{has_effect, native_actor, parry, ptr, read_ptr, ACTIVE, BASE};
use std::sync::{Mutex, atomic::Ordering};

pub const SET_ROTATION_RVA:usize=0x467c90;
pub const SET_ROTATION_PATTERN:&str="48 89 5C 24 10 57 48 83 EC 60 F3 0F 10 0A 48 8B DA 0F 28 05 ? ? ? ? 48 8B F9 0F 29 74 24 50";
#[derive(Clone,Copy,Default,PartialEq,Eq)]
struct Owner { actor:usize, handle:u64, stats:usize }
#[derive(Clone,Copy,Default)]
struct Chain { active:bool, first:Option<u64>, mixed:bool }
impl Chain {
 fn accept(&mut self,continuing:bool,source:Option<u64>)->bool{
  if !continuing { *self=Self{active:true,first:source,mixed:source.is_none()}; }
  else if !self.active {
   // We missed the start (loading/ownership transition). Never assume that
   // the current attacker also supplied the earlier successful guard.
   *self=Self{active:true,first:source,mixed:true};
  } else if source.is_none()||source!=self.first { self.mixed=true; }
  !self.mixed
 }
}
#[derive(Clone,Copy)]
struct Contact { module:usize, hit:usize, source:Option<u64>, yaw:Option<f32> }
#[derive(Default)]
struct State { owner:Owner, chain:Chain, contacts:Vec<Contact> }
impl State {
 fn bind(&mut self,owner:Owner){if self.owner!=owner { *self=Self{owner,..Self::default()}; }}
 fn update(&mut self,owner:Owner){
  self.bind(owner);self.contacts.clear();
  // The update hook runs before native effect/action updates. A newly
  // accepted guard's CHAIN/REACTION can still be invisible here. Keep its
  // provenance until the NEXT accepted guard decides `continuing` using
  // the native guard eligibility at the hit itself; an idle poll must not
  // manufacture a missing chain start and disable same-enemy turning.
 }
 fn record(&mut self,contact:Contact){
  // Calculated hits which are rejected later never reach the final notice.
  // Bound their storage; an evicted/unknown contact fails closed for turning.
  self.contacts.retain(|c|c.module!=contact.module||c.hit!=contact.hit);
  if self.contacts.len()==64 { self.contacts.remove(0); }
  self.contacts.push(contact);
 }
 fn accepted(&mut self,module:usize,hit:usize,continuing:bool)->Option<Turn>{
  let index=self.contacts.iter().rposition(|c|c.module==module&&c.hit==hit);
  let contact=index.map(|i|self.contacts.remove(i));
  let eligible=self.chain.accept(continuing,contact.and_then(|c|c.source));
  eligible.then(||contact.and_then(|c|c.yaw)).flatten().map(|yaw|Turn{owner:self.owner,yaw})
 }
}
static STATE:Mutex<State>=Mutex::new(State{owner:Owner{actor:0,handle:0,stats:0},chain:Chain{active:false,first:None,mixed:false},contacts:Vec::new()});
pub struct Turn { owner:Owner, yaw:f32 }

fn yaw_to(from:[f32;4],to:[f32;4])->Option<f32>{
 let x=to[0]-from[0];let z=to[2]-from[2];
 if !x.is_finite()||!z.is_finite()||x.hypot(z)<0.001 { return None; }
 // Verified against native 467C90 -> 3CFC20: yaw zero faces negative Z.
 Some((-x).atan2(-z))
}
unsafe fn identity(player:usize)->Owner{
 Owner{actor:player,handle:ptr::read_unaligned((player+8) as *const u64),stats:native_actor::stats(player)}
}
pub unsafe fn update(player:usize,eligible:bool){
 let owner=if eligible{identity(player)}else{Owner::default()};
 let Ok(mut s)=STATE.lock() else{return;};
 if !eligible { *s=State::default();return; }
 s.update(owner);
}
// 43D4BE passes the resolved character source to 439E80 (not the generic
// bullet at hit+228). Capture after calculation; don't rotate before native
// facing/unblockable/guard-break and final notification have accepted the hit.
pub unsafe fn calculated(module:usize,source:usize,hit:usize){
 if !ACTIVE.load(Ordering::Acquire)||module==0||hit==0{return;}
 let base=BASE.load(Ordering::Acquire);let player=read_ptr(module+8);
 if !crate::is_local(player,base)||!has_effect(player,parry::OWNER,base){return;}
 if ptr::read((hit+0x2b4) as *const u8)==0||ptr::read((hit+0x2c1) as *const u8)&(2|8)!=0{return;}
 let owner=identity(player);
 if owner.stats==0||native_actor::incapacitated(player,owner.stats){return;}
 let handle=if source==0||source==player {None}else{
  let h=ptr::read_unaligned((source+8) as *const u64);
  (h!=0&&h!=u64::MAX).then_some(h)
 };
 let yaw=if handle.is_some()&&crate::spatial_ready(player)&&crate::spatial_ready(source){
  yaw_to(crate::position(player,base),crate::position(source,base))
 }else{None};
 if let Ok(mut s)=STATE.lock(){s.bind(owner);s.record(Contact{module,hit,source:handle,yaw});}
}
pub unsafe fn accepted(module:usize,hit:usize,continuing:bool)->Option<Turn>{
 let mut s=STATE.lock().ok()?;
 if s.owner.actor==0||module==0||s.owner.actor!=read_ptr(module+8){return None;}
 s.accepted(module,hit,continuing)
}
#[repr(C,align(16))]struct Vector([f32;4]);
pub unsafe fn apply(turn:Turn){
 if !ACTIVE.load(Ordering::Acquire){return;}
 let base=BASE.load(Ordering::Acquire);let player=turn.owner.actor;
 if !crate::is_local(player,base)||identity(player)!=turn.owner||turn.owner.stats==0||
  native_actor::incapacitated(player,turn.owner.stats)||!has_effect(player,parry::OWNER,base)||!parry::two_handed(player)||!parry::grounded(player)||!crate::spatial_ready(player){return;}
 let mut rotation=Vector([0.;4]);
 let get:unsafe extern "system" fn(usize,*mut f32)->*const f32=std::mem::transmute(base+crate::CHR_ROTATION_RVA);
 get(player,rotation.0.as_mut_ptr());
 if !rotation.0[..3].iter().all(|v|v.is_finite()){return;}
 rotation.0[1]=turn.yaw;
 let physics=read_ptr(read_ptr(player+0x1b8)+0x68);
 let set:unsafe extern "system" fn(usize,*const f32)=std::mem::transmute(base+SET_ROTATION_RVA);
 set(physics,rotation.0.as_ptr());
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn first_and_same_enemy_turn_but_mixed_chain_never_rearms(){
  let mut c=Chain::default();
  assert!(c.accept(false,Some(10)));assert!(c.accept(true,Some(10)));
  for source in [20,20,10,10,30]{assert!(!c.accept(true,Some(source)));}
  assert!(c.accept(false,Some(20)));assert!(c.accept(true,Some(20)));
 }
 #[test]fn unknown_source_and_missing_start_fail_closed(){
  let mut c=Chain::default();assert!(!c.accept(true,Some(1)));assert!(!c.accept(true,Some(1)));
  assert!(c.accept(false,Some(1)));assert!(!c.accept(true,None));assert!(!c.accept(true,Some(1)));
  assert!(!c.accept(false,None));assert!(!c.accept(true,Some(1)));
 }
 #[test]fn native_yaw_axes_and_degenerate_positions(){
  for (x,z,expected) in [(0.,-1.,0.),(-1.,0.,std::f32::consts::FRAC_PI_2),(1.,0.,-std::f32::consts::FRAC_PI_2)]{
   assert!((yaw_to([0.;4],[x,99.,z,1.]).unwrap()-expected).abs()<0.00001);
  }
  for x in [0.,f32::NAN,f32::INFINITY]{assert!(yaw_to([0.;4],[x,2.,0.,1.]).is_none());}
 }
 #[test]fn nested_contacts_are_consumed_by_identity_not_latest_pointer(){
  let mut s=State::default();s.bind(Owner{actor:1,handle:1,stats:2});
  s.record(Contact{module:3,hit:10,source:Some(1),yaw:Some(0.2)});
  s.record(Contact{module:3,hit:20,source:Some(2),yaw:Some(0.4)});
  assert_eq!(s.accepted(3,20,false).unwrap().yaw,0.4);
  assert!(s.accepted(3,10,true).is_none());
  s.record(Contact{module:3,hit:30,source:Some(2),yaw:Some(0.4)});
  assert!(s.accepted(3,30,true).is_none());
 }
 #[test]fn generation_change_clears_chain_and_stale_contacts(){
  let mut s=State::default();s.bind(Owner{actor:1,handle:1,stats:2});
  s.chain.accept(false,Some(1));s.chain.accept(true,Some(2));
  s.record(Contact{module:3,hit:10,source:Some(1),yaw:Some(0.2)});
  s.bind(Owner{actor:1,handle:2,stats:2});assert!(!s.chain.active);assert!(s.contacts.is_empty());
  assert!(s.chain.accept(false,Some(1)));
 }
 #[test]fn update_between_guard_and_native_marker_visibility_preserves_provenance(){
  let owner=Owner{actor:1,handle:2,stats:3};let mut s=State::default();s.bind(owner);
  s.record(Contact{module:4,hit:5,source:Some(10),yaw:Some(0.2)});
  assert!(s.accepted(4,5,false).is_some());
  // Live 0.17.109: next update reset active before the reaction marker
  // became visible; the next same-source guard was misclassified mixed.
  s.update(owner);
  s.record(Contact{module:4,hit:6,source:Some(10),yaw:Some(0.4)});
  assert_eq!(s.accepted(4,6,true).unwrap().yaw,0.4);
  s.record(Contact{module:4,hit:7,source:Some(20),yaw:Some(0.6)});
  assert!(s.accepted(4,7,true).is_none());
  s.update(owner);
  s.record(Contact{module:4,hit:8,source:Some(20),yaw:Some(0.8)});
  assert!(s.accepted(4,8,true).is_none());
  // Native acceptance, not time spent idle, authoritatively starts a new chain.
  s.record(Contact{module:4,hit:9,source:Some(20),yaw:Some(1.)});
  assert_eq!(s.accepted(4,9,false).unwrap().yaw,1.);
 }
}
