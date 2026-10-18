//! Only locomotion's periodic stamina debit is waived. Rescue accounting follows
//! both the native authority path and its original type-3 network notification.
use crate::{has_effect,is_local,native_actor,ptr,read_i32,read_ptr,ACTIVE,BASE};
use std::{cell::Cell,sync::{Mutex,atomic::{AtomicPtr,Ordering}}};
pub const OWNER:i32=95003400;
pub const QUERY:i32=990034;
pub const ENV_RVA:usize=0x3f7b50;
pub const ENV_PATTERN:&str="48 8B C4 48 89 58 10 55 56 57 41 54 41 55 41 56 41 57 48 8D 68 A1 48 81 EC 00 01 00 00";
pub const NUMBER_RVA:usize=0xd40560;
pub const NUMBER_PATTERN:&str="E9 AB 8E 88 00 0F 85 B8 DE 59 01 E9 83 DE 59 01 48 89 5C 24 08 57 48 83";
pub const COST_RVA:usize=0x5085a0;
pub const COST_PATTERN:&str="48 8B 41 08 45 33 C9 4C 89 0A 48 85 C0 74 5A 0F 57 C9";
pub const RESCUE_RVA:usize=0x66dec0;
pub const RESCUE_PATTERN:&str="48 8B C4 48 89 58 10 48 89 68 20 56 57 41 56 48 81 EC 90 00 00 00";
pub const RECEIVE_RVA:usize=0x4f37d0;
pub const RECEIVE_PATTERN:&str="48 89 5C 24 08 48 89 74 24 18 55 57 41 56 48 8B EC 48 83 EC 50";
pub const CREDIT_RVA:usize=0x3d7610;
pub const CREDIT_PATTERN:&str="48 85 D2 74 44 53 48 83 EC 20 48 8B D9 48 81 C1 38 03 00 00";
pub const REMOTE_RVA:usize=0x519750;
pub const REMOTE_PATTERN:&str="48 89 5C 24 10 48 89 6C 24 18 48 89 74 24 20 41 56 48 83 EC 20 48 8D B1";
pub static ENV_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static COST_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static RESCUE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static RECEIVE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static CREDIT_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
#[derive(Clone,Copy,Default,PartialEq,Eq)]struct Identity{actor:usize,handle:u64,stats:usize}
#[derive(Default)]struct State{owner:Identity,frame:u64,sample:Option<(u64,f32,f32)>,fraction:f64}
static STATE:Mutex<State>=Mutex::new(State{owner:Identity{actor:0,handle:0,stats:0},frame:0,sample:None,fraction:0.});
impl State{
 fn bind(&mut self,owner:Identity){if self.owner!=owner{*self=Self{owner,..Self::default()};}}
 fn frame(&mut self,owner:Identity){self.bind(owner);self.frame=self.frame.wrapping_add(1);}
 fn sample(&mut self,owner:Identity,angle:f32,speed:f32){self.bind(owner);self.sample=Some((self.frame,angle,speed));}
 fn movement(&self,owner:Identity)->Option<(f32,f32)>{
  let(frame,angle,speed)=self.sample?;
  (owner==self.owner&&self.frame.wrapping_sub(frame)<=1&&angle.is_finite()&&speed.is_finite()&&speed>0.001).then_some((angle,speed))
 }
 fn credit(&mut self,owner:Identity,delta:f32,current:i32,maximum:i32)->i32{
  self.bind(owner);
  if !delta.is_finite()||delta<=0.||maximum<=0{return 0;}
  let room=(i64::from(maximum)-i64::from(current)).max(0)as f64;
  if room<=0.{self.fraction=0.;return 0;}
  let value=self.fraction+f64::from(delta)*0.5;
  let amount=value.floor().min(room).min(i32::MAX as f64)as i32;
  self.fraction=if value>=room{0.}else{value-f64::from(amount)};amount
 }
}
unsafe fn identity(actor:usize)->Identity{Identity{actor,handle:ptr::read_unaligned((actor+8)as*const u64),stats:native_actor::stats(actor)}}
unsafe fn owned(actor:usize,base:usize)->Option<Identity>{
 if !is_local(actor,base)||!has_effect(actor,OWNER,base){return None;}
 let owner=identity(actor);
 (owner.stats!=0&&!native_actor::incapacitated(actor,owner.stats)).then_some(owner)
}
pub unsafe fn update(actor:usize,base:usize){
 let owner=owned(actor,base);
 if let Ok(mut s)=STATE.lock(){if let Some(owner)=owner{s.frame(owner);}else{*s=State::default();}}
}
// Coordinates use negative Z as forward; positive native yaw turns left.
// Havok MoveAngle is degrees around the character, independent of the camera.
fn within_cone(direction:[f32;2],offset:[f32;2])->bool{
 if !direction.iter().chain(offset.iter()).all(|v|v.is_finite()){return false;}
 let a=f64::from(direction[0]);let b=f64::from(direction[1]);let x=f64::from(offset[0]);let z=f64::from(offset[1]);
 let length=(a*a+b*b)*(x*x+z*z);let dot=a*x+b*z;
 length>1e-12&&dot>0.&&dot*dot>=length*(0.75-1e-7)
}
unsafe fn party_member(actor:usize,base:usize)->bool{
 let world=read_ptr(base+crate::WORLD_RVA);if world==0||actor==0{return false;}
 if read_ptr(world+0x174e8)==actor{return true;}
 let get:unsafe extern "system" fn(usize,i32)->usize=std::mem::transmute(base+0x5197e0);
 (1..=3).any(|slot|get(world,slot)==actor)
}
#[repr(C,align(16))]struct Vector([f32;4]);
unsafe fn heading(actor:usize,base:usize,angle:f32)->Option<[f32;2]>{
 if !crate::spatial_ready(actor)||!angle.is_finite(){return None;}
 let mut rotation=Vector([0.;4]);
 let get:unsafe extern "system" fn(usize,*mut f32)->*const f32=std::mem::transmute(base+crate::CHR_ROTATION_RVA);
 get(actor,rotation.0.as_mut_ptr());let yaw=rotation.0[1]-angle.to_radians();
 yaw.is_finite().then(||[-yaw.sin(),-yaw.cos()])
}
unsafe fn toward_down(owner:Identity,base:usize,angle:f32,speed:f32)->bool{
 if !speed.is_finite()||speed<=0.001{return false;}
 let Some(direction)=heading(owner.actor,base,angle)else{return false;};
 let world=read_ptr(base+crate::WORLD_RVA);if world==0{return false;}
 let origin=crate::position(owner.actor,base);
 let get:unsafe extern "system" fn(usize,i32)->usize=std::mem::transmute(base+0x5197e0);
 (1..=3).any(|slot|{
  let target=get(world,slot);
  if target==0||target==owner.actor||ptr::read((target+0x710)as*const u8)!=1||!crate::spatial_ready(target){return false;}
  let position=crate::position(target,base);
  within_cone(direction,[position[0]-origin[0],position[2]-origin[2]])
 })
}
type Env=unsafe extern "system" fn(usize,i32,usize)->i32;
pub unsafe extern "system" fn env(context:usize,code:i32,vm:usize)->i32{
 if code!=QUERY{return std::mem::transmute::<*mut u8,Env>(ENV_ORIGINAL.load(Ordering::Acquire))(context,code,vm);}
 if !ACTIVE.load(Ordering::Acquire)||context==0||vm==0{return 0;}
 let base=BASE.load(Ordering::Acquire);let actor=read_ptr(context);
 let Some(owner)=owned(actor,base)else{return 0;};
 let number:unsafe extern "system" fn(usize,i32)->f32=std::mem::transmute(base+NUMBER_RVA);
 let angle=number(vm,2);let speed=number(vm,3);
 if let Ok(mut s)=STATE.lock(){s.sample(owner,angle,speed);}
 i32::from(toward_down(owner,base,angle,speed))
}
// Same node eligibility and f32 addition order as 5085A0. Only two native
// locomotion effects are omitted; no effect flag or shared PARAM is mutated.
unsafe fn filtered_cost(manager:usize,out:usize)->usize{
 let mut rate=0f32;let mut points=0f32;let mut node=read_ptr(manager+8);
 while node!=0{
  let flags=ptr::read_unaligned((node+0x68)as*const u32);
  if flags&0x800c0003==0&&flags&4!=0&&!matches!(read_i32(node+8),100300|102022){
   let row=read_ptr(node);
   if row!=0{rate=ptr::read_unaligned((row+0xb0)as*const f32)+rate;points+=(read_i32(row+0xb4))as f32;}
  }
  node=read_ptr(node+0x38);
 }
 ptr::write_unaligned(out as*mut[f32;2],[rate,points]);out
}
pub unsafe extern "system" fn cost(manager:usize,out:usize)->usize{
 let original:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(COST_ORIGINAL.load(Ordering::Acquire));
 if !ACTIVE.load(Ordering::Acquire)||manager==0||out==0{return original(manager,out);}
 let base=BASE.load(Ordering::Acquire);let actor=read_ptr(manager+0x10);
 let Some(owner)=owned(actor,base)else{return original(manager,out);};
 if read_ptr(actor+0x190)!=manager{return original(manager,out);}
 let sample=STATE.lock().ok().and_then(|s|s.movement(owner));
 if sample.is_some_and(|(angle,speed)|toward_down(owner,base,angle,speed)){filtered_cost(manager,out)}else{original(manager,out)}
}
#[derive(Clone,Copy)]struct Rescue{recipient:Identity,before:f32,down:bool}
thread_local!{static RESCUE:Cell<Option<Rescue>>=const{Cell::new(None)};}
struct Scope(Option<Rescue>);
impl Scope{fn enter(value:Option<Rescue>)->Self{Self(RESCUE.with(|s|s.replace(value)))}}
impl Drop for Scope{fn drop(&mut self){RESCUE.with(|s|s.set(self.0));}}
unsafe fn snapshot(target:usize,base:usize)->Option<Rescue>{
 if !party_member(target,base){return None;}
 Some(Rescue{recipient:identity(target),before:ptr::read_unaligned((target+0x714)as*const f32),down:ptr::read((target+0x710)as*const u8)==1})
}
pub unsafe extern "system" fn rescue(target:usize,source:usize,hit:usize){
 let base=BASE.load(Ordering::Acquire);
 let _scope=Scope::enter(if ACTIVE.load(Ordering::Acquire){snapshot(target,base)}else{None});
 let original:unsafe extern "system" fn(usize,usize,usize)=std::mem::transmute(RESCUE_ORIGINAL.load(Ordering::Acquire));original(target,source,hit);
}
// Match the original packet parser's two identity formats. Resolve afresh; no
// remote actor or stream address survives the synchronous original dispatch.
unsafe fn received_target(stream:usize,id:usize,local_format:u8,base:usize)->usize{
 let world=read_ptr(base+crate::WORLD_RVA);if world==0{return 0;}
 if local_format!=0{
  if id==0{return 0;}
  let get:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(base+REMOTE_RVA);get(world,id)
 }else{
  if stream==0||ptr::read((stream+0xc)as*const u8)!=0||read_i32(stream+8)<8{return 0;}
  let data=read_ptr(stream);if data==0{return 0;}
  native_actor::resolve(base,ptr::read_unaligned(data as*const u64))
 }
}
pub unsafe extern "system" fn receive(a:usize,stream:usize,id:usize,local_format:u8)->u8{
 let base=BASE.load(Ordering::Acquire);
 let value=if ACTIVE.load(Ordering::Acquire){snapshot(received_target(stream,id,local_format,base),base)}else{None};
 let _scope=Scope::enter(value);
 let original:unsafe extern "system" fn(usize,usize,usize,u8)->u8=std::mem::transmute(RECEIVE_ORIGINAL.load(Ordering::Acquire));original(a,stream,id,local_format)
}
pub unsafe extern "system" fn credit(source:usize,target:usize){
 if ACTIVE.load(Ordering::Acquire){
  let base=BASE.load(Ordering::Acquire);
  if let Some(owner)=owned(source,base){
   let value=RESCUE.with(|s|s.get());
   if let Some(value)=value.filter(|v|v.down&&v.recipient.actor==target&&identity(target)==v.recipient&&source!=target){
    RESCUE.with(|s|s.set(None));
    let after=ptr::read_unaligned((target+0x714)as*const f32);
    let delta=if value.before.is_finite()&&after.is_finite()&&value.before>0.&&after>=0.{(value.before-after).max(0.)}else{0.};
    let amount=STATE.lock().ok().map_or(0,|mut s|s.credit(owner,delta,read_i32(owner.stats+0x15c),read_i32(owner.stats+0x160)));
    if amount>0{let add:unsafe extern "system" fn(usize,i32,u8)=std::mem::transmute(base+crate::STAMINA_ADD_RVA);add(owner.stats,amount,0);}
   }
  }
 }
 let original:unsafe extern "system" fn(usize,usize)=std::mem::transmute(CREDIT_ORIGINAL.load(Ordering::Acquire));original(source,target);
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn thirty_degree_boundary_and_invalid_or_coincident_positions(){
  for degrees in [0f32,29.9,30.,30.1,45.,90.,180.,-30.,-30.1]{let r=degrees.to_radians();assert_eq!(within_cone([0.,-1.],[r.sin()*10.,-r.cos()*10.]),degrees.abs()<=30.,"{degrees}");}
  for p in [[0.,0.],[f32::NAN,0.],[0.,f32::INFINITY]]{assert!(!within_cone([0.,-1.],p));}
 }
 #[test]fn fractional_rescues_and_cap_never_bank_overflow(){
  let a=Identity{actor:1,handle:2,stats:3};let mut s=State::default();
  assert_eq!(s.credit(a,1.,0,100),0);assert_eq!(s.credit(a,1.,0,100),1);
  assert_eq!(s.credit(a,40.,10,100),20);assert_eq!(s.credit(a,30.,99,100),1);assert_eq!(s.fraction,0.);
  assert_eq!(s.credit(a,1.,100,100),0);assert_eq!(s.credit(a,1.,0,100),0);
  let b=Identity{handle:3,..a};assert_eq!(s.credit(b,1.,0,100),0);
  for delta in [0.,-1.,f32::NAN,f32::INFINITY]{assert_eq!(s.credit(b,delta,0,100),0);}
 }
 #[test]fn movement_expires_and_generation_changes_clear_it(){
  let a=Identity{actor:1,handle:2,stats:3};let mut s=State::default();s.frame(a);s.sample(a,20.,2.);assert_eq!(s.movement(a),Some((20.,2.)));s.frame(a);assert!(s.movement(a).is_some());s.frame(a);assert!(s.movement(a).is_none());
  s.sample(a,0.,0.);assert!(s.movement(a).is_none());s.sample(a,0.,2.);s.frame(Identity{handle:3,..a});assert!(s.movement(a).is_none());
 }
 #[test]fn filtered_periodic_sum_keeps_other_debits_recovery_and_native_flags(){unsafe{
  let mut manager=[0usize;3];let mut nodes=[[0usize;0x70/8];6];let mut rows=[[0u32;0xb8/4];6];
  for i in 0..6{nodes[i][0]=rows[i].as_mut_ptr()as usize;nodes[i][1]=[100300,102022,100320,99,101,103][i];nodes[i][0x68/8]=4;rows[i][0xb4/4]=[1,1,1,5,20,-3i32][i]as u32;rows[i][0xb0/4]=[0.,0.,1.,2.,100.,-2f32][i].to_bits();if i+1<6{nodes[i][0x38/8]=nodes[i+1].as_ptr()as usize;}}
  nodes[4][0x68/8]=4|0x80000;manager[1]=nodes[0].as_ptr()as usize;let saved=nodes;let mut out=[0f32;2];
  assert_eq!(filtered_cost(manager.as_ptr()as usize,out.as_mut_ptr()as usize),out.as_mut_ptr()as usize);assert_eq!(out,[1.,3.]);assert_eq!(nodes,saved);
 }}
 #[test]fn nested_rescue_scopes_restore_outer_and_unmatched_calls_have_no_credit(){
  RESCUE.with(|s|assert!(s.get().is_none()));let a=Rescue{recipient:Identity{actor:1,handle:2,stats:3},before:40.,down:true};
  {let _outer=Scope::enter(Some(a));{let _inner=Scope::enter(None);RESCUE.with(|s|assert!(s.get().is_none()));}RESCUE.with(|s|assert_eq!(s.get().unwrap().before,40.));}RESCUE.with(|s|assert!(s.get().is_none()));
 }
}

#[cfg(test)]
#[path="ambulance_native_tests.rs"]mod native_tests;
