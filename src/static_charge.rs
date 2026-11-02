//! One movement charge, one immutable discharge, native recipient damage/rescue.
use crate::{ptr,read_ptr,read_i32,has_effect,ACTIVE,BASE};
use std::{cell::{RefCell,Cell},collections::{BTreeMap,BTreeSet},sync::{Arc,Mutex},sync::atomic::Ordering};
pub const OWNER:i32=95005100;
pub const READY:i32=OWNER+1;
pub const ATTACK:i32=OWNER+60;
pub const BULLET:i32=OWNER+50;
pub const TRANSPORT_RVA:usize=0x34058e;
pub const TRANSPORT_PATTERN:&str="0F 10 42 30 0F 11 81 24 01 00 00";
pub const PACKET_RVA:usize=0x33e1a3;
pub const PACKET_PATTERN:&str="0F 10 47 30 0F 11 46 70";
const DISTANCE:f64=30.;
#[derive(Default)]struct Meter{distance:f64,last:Option<(usize,u64,u32,[f32;4])>}
impl Meter{
 fn observe(&mut self,identity:usize,handle:u64,map:u32,point:Option<[f32;4]>){
  let Some(p)=point.filter(|p|p[..3].iter().all(|v|v.is_finite()))else{self.last=None;return;};
  if let Some((a,h,m,previous))=self.last{if (a,h,m)==(identity,handle,map){
   let d=(0..3).map(|i|(f64::from(p[i])-f64::from(previous[i])).powi(2)).sum::<f64>().sqrt();
   self.distance=(self.distance+d).min(DISTANCE);
  }}
  self.last=Some((identity,handle,map,p));
 }
 fn take(&mut self)->bool{if self.distance<DISTANCE{return false;}self.distance=0.;true}
}
#[derive(Clone,Copy)]struct Pending{handle:u64,damage:f32,point:[f32;4]}
struct State{meter:Meter,pending:Vec<Pending>,scene:usize}
static STATE:Mutex<State>=Mutex::new(State{meter:Meter{distance:0.,last:None},pending:Vec::new(),scene:0});
pub fn suspend(){if let Ok(mut s)=STATE.lock(){s.meter.last=None;}}
pub fn reset(){if let Ok(mut s)=STATE.lock(){s.meter=Meter::default();s.pending.clear();}if let Ok(mut s)=SAMPLES.lock(){s.clear();}}
thread_local!{static CONTACT:Cell<bool>=const{Cell::new(false)};}
struct ContactGuard(bool);
impl Drop for ContactGuard{fn drop(&mut self){CONTACT.with(|s|s.set(self.0));}}
pub fn active_contact()->bool{CONTACT.with(Cell::get)}
pub fn private_attack(id:i32,table:i32)->bool{table==1&&id==ATTACK}
pub fn private_bullet(id:i32)->bool{(BULLET..=BULLET+3).contains(&id)}
unsafe fn visual(actor:usize,base:usize,ready:bool){
 let current=has_effect(actor,READY,base);if ready==current{return;}
 if ready{let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);apply(actor,READY,0);}
 else{let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);remove(read_ptr(actor+0x190),READY);}
}
pub unsafe fn update(actor:usize,base:usize){
 if has_effect(actor,crate::reward_cycle_runtime::RESET,base){reset();visual(actor,base,false);return;}
 if !has_effect(actor,OWNER,base){if let Ok(mut s)=STATE.lock(){s.meter.last=None;}visual(actor,base,false);return;}
 let coord=read_ptr(actor+0x60);let handle=read_ptr(actor+8)as u64;
 let context=read_ptr(base+crate::MAP_CONTEXT_RVA);
 let map=if context!=0{ptr::read_unaligned((context+0x2c)as*const u32)}else{(handle>>32)as u32};
 let p=crate::spatial_ready(actor).then(||crate::position(actor,base));
 let (ready,pending)=if let Ok(mut s)=STATE.lock(){let scene=read_ptr(base+0x3c131c8);if s.scene!=scene{s.meter.last=None;s.scene=scene;if !s.pending.is_empty(){s.meter.distance=DISTANCE;s.pending.clear();}}s.meter.observe(coord,handle,map,p);(s.meter.distance>=DISTANCE,std::mem::take(&mut s.pending))}else{return;};
 visual(actor,base,ready);
 for ticket in pending{if ticket.handle==handle&&!emit(ticket,base){if let Ok(mut s)=STATE.lock(){s.pending.push(ticket);}}}
}
// Capture before all custom late offensive multipliers. Also observe preview
// calculations: native contact can reuse a previously computed hit structure.
#[derive(Clone,PartialEq,Eq)]struct Key{source:u64,target:u64,attack:i32,table:i32,components:[u32;5],point:[u32;3]}
unsafe fn key(source:usize,target:usize,hit:usize)->Key{Key{source:read_ptr(source+8)as u64,target:read_ptr(target+8)as u64,attack:read_i32(hit+0x40),table:read_i32(hit+0x44),components:ptr::read_unaligned(hit as*const[u32;5]),point:ptr::read_unaligned((hit+0x1f0)as*const[u32;3])}}
struct Sample{key:Key,value:f32}
static SAMPLES:Mutex<BTreeMap<usize,Sample>>=Mutex::new(BTreeMap::new());
#[derive(Default)]struct Calculation{key:Option<Key>,hit:usize,delta:[f32;5],sampled:bool,lightning:Option<f32>,object:f32}
thread_local!{static CALC:RefCell<Vec<Calculation>>=const{RefCell::new(Vec::new())};}
pub unsafe fn enter(module:usize,source:usize,hit:usize){
 let mut c=Calculation{object:1.,..Default::default()};
 if ACTIVE.load(Ordering::Acquire)&&module!=0&&source!=0&&hit!=0{
  let target=read_ptr(module+8);let base=BASE.load(Ordering::Acquire);
  if private_attack(read_i32(hit+0x40),read_i32(hit+0x44)){c.lightning=Some(ptr::read_unaligned((hit+0xc)as*const f32));}
  else if target!=0&&source!=target&&crate::is_local(source,base)&&has_effect(source,OWNER,base){c.key=Some(key(source,target,hit));c.hit=hit;}
 }
 CALC.with(|s|s.borrow_mut().push(c));
}
pub fn leave(){CALC.with(|s|{s.borrow_mut().pop();});}
pub unsafe fn object_delta(a:*const f32,rate:f32){CALC.with(|s|{if let Some(c)=s.borrow_mut().last_mut(){c.object=rate;if c.key.is_some(){c.delta=std::array::from_fn(|i|*a.add(i)*(1.-rate));}}});}
pub fn scale_delta(rate:f32){CALC.with(|s|{if let Some(c)=s.borrow_mut().last_mut(){for d in &mut c.delta{*d*=rate;}}});}
pub unsafe fn raw_attack(a:*const f32)->Option<[f32;5]>{if a.is_null(){return None;}CALC.with(|s|{let mut s=s.borrow_mut();let c=s.last_mut()?;if c.key.is_none()||c.sampled{return None;}c.sampled=true;Some(std::array::from_fn(|i|*a.add(i)+c.delta[i]))})}
pub fn candidate(value:f32){if !value.is_finite()||value<0.{return;}CALC.with(|s|{if let Some(c)=s.borrow().last(){if let Some(key)=&c.key{if let Ok(mut samples)=SAMPLES.lock(){if samples.len()>256{samples.clear();}samples.insert(c.hit,Sample{key:key.clone(),value});}}}});}
// These inputs have already passed the attacker's native aggregation. A new
// elemental strike must not aggregate those bonuses again. Preserve only the
// new target's defensive coefficients, including object defense (possibly 0).
pub fn lightning()->Option<f32>{CALC.with(|s|s.borrow().last().and_then(|c|c.lightning.map(|d|d*c.object)))}
pub unsafe fn outgoing(module:usize,source:usize,hit:usize){
 if !ACTIVE.load(Ordering::Acquire)||module==0||source==0||hit==0{return;}
 let base=BASE.load(Ordering::Acquire);if !crate::is_local(source,base)||!has_effect(source,OWNER,base)||private_attack(read_i32(hit+0x40),read_i32(hit+0x44)){return;}
 let target=read_ptr(module+8);if target==0||source==target{return;}
 let rescue=crate::ambulance::static_rescue(target,hit,base);
 // Accepted guard contacts count too; invulnerable rejected contacts never
 // reach this hook. Ordinary live teammates do not consume a charge.
 if !rescue&&crate::ambulance::party_member(target,base){return;}
 let k=key(source,target,hit);
 let damage=SAMPLES.lock().ok().and_then(|s|s.get(&hit).filter(|v|v.key==k).or_else(||s.values().find(|v|v.key==k)).map(|v|v.value));
 let Some(damage)=damage.filter(|v|*v>0.)else{return;};
 let mut point=ptr::read_unaligned((hit+0x1f0)as*const[f32;4]);
 if !point[..3].iter().all(|v|v.is_finite()){return;}
 // Probe around the victim's floor at the actual contact X/Z. A local sweep
 // cannot jump to a basement beneath the supported floor.
 if crate::spatial_ready(target){let feet=crate::position(target,base);let origin=[point[0],feet[1],point[2],1.];if let Some(ground)=crate::noisy_ground::ground_at(origin,base){point=ground;}}
 if let Ok(mut s)=STATE.lock(){if s.meter.take(){s.pending.push(Pending{handle:read_ptr(source+8)as u64,damage,point});}}
}
// Native root descriptors serialize the four float coefficients at +30.
// Private roots carry an immutable snapshot in the first coefficient; the
// factory captures it and restores neutral coefficients before native use.
// Thus network-recreated roots do not depend on a process-global last damage.
#[repr(C,align(16))]struct Descriptor([u8;0x110]);
unsafe fn emit(p:Pending,base:usize)->bool{
 let manager=read_ptr(base+0x3c0ae30);if manager==0{return false;}
 let mut d=Descriptor([0;0x110]);let at=d.0.as_mut_ptr()as usize;
 ptr::write_unaligned(at as*mut u64,p.handle);
 for offset in [8,0xc,0x14,0x18,0x1c,0x24,0xe0,0xf0]{ptr::write_unaligned((at+offset)as*mut i32,-1);}
 ptr::write_unaligned((at+0x14)as*mut i32,BULLET);
 for offset in [0x28,0x2c]{ptr::write_unaligned((at+offset)as*mut i32,0x10);}
 d.0[0xa8]=0xff;d.0[0x105]=0xff;
 ptr::write_unaligned((at+0x30)as*mut[f32;4],[p.damage,1.,1.,1.]);
 ptr::write_unaligned((at+0x44)as*mut u32,15); // fixed independent pose + direct row + network
 ptr::write_unaligned((at+0x50)as*mut[[f32;4];4],[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[p.point[0],p.point[1],p.point[2],1.]]);
 ptr::write_unaligned((at+0xb8)as*mut u64,u64::MAX);
 let mut out=-1i32;let mut error=-1i32;
 let emit:unsafe extern "system" fn(usize,usize,usize,usize)->usize=std::mem::transmute(crate::echo_cast::EMIT_ORIGINAL.load(Ordering::Acquire));
 emit(manager,&mut out as*mut i32 as usize,at,&mut error as*mut i32 as usize);out>=0
}
struct Discharge{damage:f32,targets:Mutex<BTreeSet<u64>>}
#[derive(Default)]struct Flights{groups:BTreeMap<usize,(i32,Arc<Discharge>)>,instances:BTreeMap<usize,(u64,Arc<Discharge>)>}
static FLIGHTS:Mutex<Flights>=Mutex::new(Flights{groups:BTreeMap::new(),instances:BTreeMap::new()});
thread_local!{static SEED:RefCell<Option<Arc<Discharge>>>=const{RefCell::new(None)};static CHILD:RefCell<Option<Arc<Discharge>>>=const{RefCell::new(None)};}
pub struct SeedGuard(Option<Arc<Discharge>>,Option<(usize,[f32;4])>);
impl Drop for SeedGuard{fn drop(&mut self){if let Some((p,v))=self.1{unsafe{ptr::write_unaligned(p as*mut[f32;4],v);}}SEED.with(|s|*s.borrow_mut()=self.0.take());}}
pub struct ChildGuard(Option<Arc<Discharge>>);
impl Drop for ChildGuard{fn drop(&mut self){CHILD.with(|s|*s.borrow_mut()=self.0.take());}}
pub unsafe fn factory(payload:usize)->SeedGuard{
 let private=ACTIVE.load(Ordering::Acquire)&&payload!=0&&private_bullet(read_i32(payload+4));
 let seed=if private{CHILD.with(|s|s.borrow().clone()).or_else(||{
  let damage=ptr::read_unaligned((payload+0x124)as*const f32);
  (read_i32(payload+4)==BULLET&&damage.is_finite()&&damage>0.).then(||Arc::new(Discharge{damage,targets:Mutex::new(BTreeSet::new())}))
 })}else{None};
 let restore=if private{let p=payload+0x124;let v=ptr::read_unaligned(p as*const[f32;4]);ptr::write_unaligned(p as*mut[f32;4],[1.;4]);Some((p,v))}else{None};
 SeedGuard(SEED.with(|s|s.replace(seed)),restore)
}
pub unsafe fn factory_done(group:usize){if group==0{return;}if let Ok(mut f)=FLIGHTS.lock(){f.groups.remove(&group);if let Some(seed)=SEED.with(|s|s.borrow().clone()){f.groups.insert(group,(read_i32(group),seed));}}}
pub unsafe fn initialize(group:usize)->SeedGuard{
 let seed=FLIGHTS.lock().ok().and_then(|f|f.groups.get(&group).filter(|(id,_)|group!=0&&*id==read_i32(group)).map(|(_,s)|s.clone())).or_else(||SEED.with(|s|s.borrow().clone()));
 SeedGuard(SEED.with(|s|s.replace(seed)),None)
}
pub unsafe fn bound(instance:usize){if instance==0{return;}if let Ok(mut f)=FLIGHTS.lock(){f.instances.remove(&instance);if let Some(s)=SEED.with(|s|s.borrow().clone()){f.instances.insert(instance,(read_ptr(instance+8)as u64,s));}}}
unsafe fn seed(instance:usize)->Option<Arc<Discharge>>{if instance==0{return None;}FLIGHTS.lock().ok()?.instances.get(&instance).filter(|(h,_)|*h==read_ptr(instance+8)as u64).map(|(_,s)|s.clone())}
pub unsafe fn child(instance:usize,bullet:i32)->ChildGuard{ChildGuard(CHILD.with(|s|s.replace(if private_bullet(bullet){seed(instance)}else{None})))}
pub fn finished(group:usize){if let Ok(mut f)=FLIGHTS.lock(){f.groups.remove(&group);}}
pub fn retired(instance:usize){if let Ok(mut f)=FLIGHTS.lock(){f.instances.remove(&instance);}}
pub unsafe fn constructed(hit:usize,context:usize){
 if let Ok(mut s)=SAMPLES.lock(){s.remove(&hit);}
 if !private_attack(read_i32(hit+0x40),read_i32(hit+0x44)){return;}
 let damage=seed(read_ptr(context)).or_else(||SEED.with(|s|s.borrow().clone())).map_or(0.,|s|s.damage);
 ptr::write_unaligned(hit as*mut[f32;5],[0.,0.,0.,damage,0.]);
 // Native attack context is fixed, no equipped-weapon contribution. Recipient
 // calculation independently suppresses offensive factors already captured.
}
pub unsafe fn contact(original:unsafe extern "system" fn(usize,usize,usize)->u8,hit:usize,target:usize,bullet:usize)->u8{
 if !ACTIVE.load(Ordering::Acquire)||bullet==0||!private_bullet(read_i32(bullet+0xa4)){return original(hit,target,bullet);}
 let Some(seed)=seed(bullet)else{return 0;};if target==0{return 0;}
 let handle=read_ptr(target+8)as u64;
 {let Ok(mut targets)=seed.targets.lock()else{return 0;};if !targets.insert(handle){return 0;}}
 let _guard=ContactGuard(CONTACT.with(|s|s.replace(true)));
 let result=original(hit,target,bullet);
 // Native rejection (e.g. dodge) must leave that target eligible for the wave.
 if result==0{if let Ok(mut targets)=seed.targets.lock(){targets.remove(&handle);}}
 result
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn all_axes_accumulate_and_rebinding_only_rebases(){
  let mut m=Meter::default();m.observe(1,2,3,Some([0.,0.,0.,1.]));
  for p in [[10.,0.,0.,1.],[10.,10.,0.,1.],[10.,10.,10.,1.]]{m.observe(1,2,3,Some(p));}
  assert!(m.take());assert!(!m.take());
  m.observe(1,2,3,Some([10.,10.,15.,1.]));assert_eq!(m.distance,5.);
  m.observe(4,5,6,Some([999.,999.,999.,1.]));assert_eq!(m.distance,5.);
  m.observe(4,5,6,None);m.observe(4,5,6,Some([0.;4]));assert_eq!(m.distance,5.);
 }
 #[test]fn full_charge_is_one_token_and_does_not_bank_overflow(){let mut m=Meter::default();m.observe(1,2,3,Some([0.;4]));m.observe(1,2,3,Some([100.,0.,0.,1.]));assert_eq!(m.distance,30.);assert!(m.take());assert!(!m.take());}
}
#[cfg(test)]#[path="static_charge_tests.rs"]mod native_tests;
