//! Movement charge -> two-second recipient mark -> native lightning discharge.
use crate::{ptr,read_ptr,read_i32,has_effect,ACTIVE,BASE};
use std::{cell::{RefCell,Cell},collections::{BTreeMap,BTreeSet},sync::{Arc,Mutex},sync::atomic::Ordering};
pub const OWNER:i32=95005100;
pub const READY:i32=OWNER+1;
pub const MARK:i32=OWNER+2;
pub const ATTACK:i32=OWNER+60;
pub const BULLET:i32=OWNER+50;
pub const LEAK_BULLET:i32=BULLET+4;
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
#[derive(Clone,Copy)]struct Pending{handle:u64,damage:f32,point:[f32;4],leak:bool}
#[derive(Clone,Copy)]struct Mark{owner:u64,target:u64,sum:f64,deadline:f64,level:i32,leak:bool,point:[f32;4]}
impl Mark{fn damage(&self)->f32{((self.sum+f64::from(self.level.max(1))*10.)*if self.leak{0.25}else{1.}).min(f64::from(f32::MAX))as f32}}
struct State{meter:Meter,marks:Vec<Mark>,pending:Vec<Pending>,scene:usize,time:f64}
impl State{
 fn advance(&mut self,dt:f32)->Vec<Mark>{
  if !dt.is_finite()||dt<=0.||dt>1.{return Vec::new();}self.time+=f64::from(dt);
  let now=self.time;let mut due=Vec::new();self.marks.retain(|m|{if m.deadline<=now{due.push(*m);false}else{true}});due
 }
 fn credit(&mut self,target:u64,damage:f32){for mark in &mut self.marks{if mark.target==target&&self.time<mark.deadline{mark.sum=(mark.sum+f64::from(damage)).min(f64::from(f32::MAX));}}}
 fn mark(&mut self,owner:u64,target:u64,damage:f32,level:i32,leak:bool,point:[f32;4])->bool{
  if !self.meter.take(){return false;}self.marks.push(Mark{owner,target,sum:f64::from(damage),deadline:self.time+2.,level,leak,point});true
 }
}
static STATE:Mutex<State>=Mutex::new(State{meter:Meter{distance:0.,last:None},marks:Vec::new(),pending:Vec::new(),scene:0,time:0.});
pub fn suspend(){if let Ok(mut s)=STATE.lock(){s.meter.last=None;}}
pub fn reset(){if let Ok(mut s)=STATE.lock(){s.meter=Meter::default();s.marks.clear();s.pending.clear();s.time=0.;}if let Ok(mut s)=SAMPLES.lock(){s.clear();}}
thread_local!{static CONTACT:Cell<bool>=const{Cell::new(false)};}
struct ContactGuard(bool);
impl Drop for ContactGuard{fn drop(&mut self){CONTACT.with(|s|s.set(self.0));}}
pub fn active_contact()->bool{CONTACT.with(Cell::get)}
pub fn private_attack(id:i32,table:i32)->bool{table==1&&(ATTACK..=ATTACK+3).contains(&id)}
pub fn private_bullet(id:i32)->bool{(BULLET..=BULLET+7).contains(&id)}
unsafe fn visual(actor:usize,base:usize,ready:bool){
 let current=has_effect(actor,READY,base);if ready==current{return;}
 if ready{let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);apply(actor,READY,0);}
 else{let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);remove(read_ptr(actor+0x190),READY);}
}
unsafe fn level(base:usize)->i32{let game=read_ptr(base+crate::GAME_DATA_RVA);let data=if game!=0{read_ptr(game+8)}else{0};if data!=0{read_i32(data+0x68).clamp(1,15)}else{1}}
unsafe fn mark_visual(target:usize,base:usize){let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);apply(target,MARK,0);}
pub unsafe fn update(actor:usize,base:usize,dt:f32){
 if has_effect(actor,crate::reward_cycle_runtime::RESET,base){reset();visual(actor,base,false);return;}
 let owned=has_effect(actor,OWNER,base);
 let coord=read_ptr(actor+0x60);let handle=read_ptr(actor+8)as u64;
 let context=read_ptr(base+crate::MAP_CONTEXT_RVA);
 let map=if context!=0{ptr::read_unaligned((context+0x2c)as*const u32)}else{(handle>>32)as u32};
 let p=crate::spatial_ready(actor).then(||crate::position(actor,base));
 let tracked=STATE.lock().map(|s|s.marks.iter().map(|m|m.target).collect::<BTreeSet<_>>()).unwrap_or_default();
 let positions=tracked.into_iter().filter_map(|h|{let t=crate::native_actor::resolve(base,h);(t!=0&&crate::spatial_ready(t)).then(||(h,crate::position(t,base)))}).collect::<BTreeMap<_,_>>();
 let (ready,marks,mut pending)=if let Ok(mut s)=STATE.lock(){
  let scene=read_ptr(base+0x3c131c8);
  if s.scene!=scene{s.meter.last=None;s.scene=scene;s.marks.clear();s.pending.clear();}
  for mark in &mut s.marks{if let Some(p)=positions.get(&mark.target){if p[..3].iter().all(|v|v.is_finite()){mark.point=*p;}}}
  if owned{s.meter.observe(coord,handle,map,p);}else{s.meter.last=None;}
  let due=s.advance(dt);
  (owned&&s.meter.distance>=DISTANCE,due,std::mem::take(&mut s.pending))
 }else{return;};
 visual(actor,base,ready);
 for mark in marks{
  let target=crate::native_actor::resolve(base,mark.target);
  // Keep the last valid location if a killed target has already despawned.
  // Resolve the recipient again at detonation; the mark travels with them.
  let feet=if target!=0&&crate::spatial_ready(target){crate::position(target,base)}else{mark.point};
  let point=crate::noisy_ground::ground_at(feet,base).unwrap_or(feet);
  pending.push(Pending{handle:mark.owner,damage:mark.damage(),point,leak:mark.leak});
 }
 for ticket in pending{if crate::native_actor::resolve(base,ticket.handle)!=0&&!emit(ticket,base){if let Ok(mut s)=STATE.lock(){s.pending.push(ticket);}}}
}
// Native dispatch can contain preview calculations and nested dispatches. Only
// its accepted-hit notification commits, once per dispatch, not once per frame.
thread_local!{static DISPATCH:RefCell<Vec<(usize,usize,bool)>>=const{RefCell::new(Vec::new())};}
pub struct DispatchGuard;
impl Drop for DispatchGuard{fn drop(&mut self){DISPATCH.with(|s|{s.borrow_mut().pop();});}}
pub fn dispatch_scope(module:usize,hit:usize)->DispatchGuard{DISPATCH.with(|s|s.borrow_mut().push((module,hit,false)));DispatchGuard}
fn first_contact(module:usize,hit:usize)->bool{DISPATCH.with(|s|{let mut s=s.borrow_mut();let Some(v)=s.last_mut().filter(|v|v.0==module&&v.1==hit)else{return false;};if v.2{return false;}v.2=true;true})}
// Capture before all custom late offensive multipliers. Also observe preview
// calculations: native contact can reuse a previously computed hit structure.
#[derive(Clone,PartialEq,Eq)]struct Key{source:u64,target:u64,attack:i32,table:i32,components:[u32;5],point:[u32;3]}
unsafe fn key(source:usize,target:usize,hit:usize)->Key{Key{source:if source!=0{read_ptr(source+8)as u64}else{0},target:read_ptr(target+8)as u64,attack:read_i32(hit+0x40),table:read_i32(hit+0x44),components:ptr::read_unaligned(hit as*const[u32;5]),point:ptr::read_unaligned((hit+0x1f0)as*const[u32;3])}}
struct Sample{key:Key,value:f32}
static SAMPLES:Mutex<BTreeMap<usize,Sample>>=Mutex::new(BTreeMap::new());
#[derive(Default)]struct Calculation{key:Option<Key>,hit:usize,delta:[f32;5],sampled:bool,lightning:Option<f32>,object:f32}
thread_local!{static CALC:RefCell<Vec<Calculation>>=const{RefCell::new(Vec::new())};}
pub unsafe fn enter(module:usize,source:usize,hit:usize){
 let mut c=Calculation{object:1.,..Default::default()};
 if ACTIVE.load(Ordering::Acquire)&&module!=0&&hit!=0{
  let target=read_ptr(module+8);let base=BASE.load(Ordering::Acquire);
  let private=private_attack(read_i32(hit+0x40),read_i32(hit+0x44));
  if private{c.lightning=Some(ptr::read_unaligned((hit+0xc)as*const f32));}
  // Capture any source while the local owner holds the augment. A later
  // accepted hit can reuse a preview made before the first mark was applied.
  let world=read_ptr(base+crate::WORLD_RVA);let local=if world!=0{read_ptr(world+0x174e8)}else{0};
  if target!=0&&local!=0&&has_effect(local,OWNER,base){
   c.key=Some(key(source,target,hit));c.hit=hit;
   if let Some(value)=c.lightning{c.sampled=true;if let Ok(mut samples)=SAMPLES.lock(){samples.insert(hit,Sample{key:c.key.clone().unwrap(),value});}}
  }
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
 if !ACTIVE.load(Ordering::Acquire)||module==0||hit==0{return;}
 let base=BASE.load(Ordering::Acquire);let target=read_ptr(module+8);if target==0{return;}
 let k=key(source,target,hit);
 let damage=SAMPLES.lock().ok().and_then(|s|s.get(&hit).filter(|v|v.key==k).or_else(||s.values().find(|v|v.key==k)).map(|v|v.value));
 let Some(damage)=damage.filter(|v|v.is_finite()&&*v>0.)else{return;};
 if !first_contact(module,hit){return;}
 let target_handle=read_ptr(target+8)as u64;
 // Already marked recipients collect all accepted attack sources. Private
 // lightning contributes to existing marks but cannot spend another charge.
 if let Ok(mut s)=STATE.lock(){s.credit(target_handle,damage);}
 if private_attack(read_i32(hit+0x40),read_i32(hit+0x44))||source==target{return;}
 let outgoing=crate::is_local(source,base)&&has_effect(source,OWNER,base);
 let incoming=crate::is_local(target,base)&&has_effect(target,OWNER,base);
 let owner=if outgoing{
  if crate::ambulance::party_member(target,base)&&!crate::ambulance::static_rescue(target,hit,base){return;}source
 }else if incoming{
  // Friendly rescue is not an incoming attack against the recipient.
  if source!=0&&crate::ambulance::party_member(source,base){return;}target
 }else{return;};
 let point=if crate::spatial_ready(target){crate::position(target,base)}else{ptr::read_unaligned((hit+0x1f0)as*const[f32;4])};
 if !point[..3].iter().all(|v|v.is_finite()){return;}
 let applied=STATE.lock().map(|mut s|s.mark(read_ptr(owner+8)as u64,target_handle,damage,level(base),incoming,point)).unwrap_or(false);
 if applied{visual(owner,base,false);mark_visual(target,base);}
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
 ptr::write_unaligned((at+0x14)as*mut i32,if p.leak{LEAK_BULLET}else{BULLET});
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
  (matches!(read_i32(payload+4),BULLET|LEAK_BULLET)&&damage.is_finite()&&damage>0.).then(||Arc::new(Discharge{damage,targets:Mutex::new(BTreeSet::new())}))
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
 fn state()->State{State{meter:Meter::default(),marks:Vec::new(),pending:Vec::new(),scene:0,time:0.}}
 #[test]fn marked_window_counts_first_hit_and_later_sources_but_not_unmarked_targets(){
  let mut s=state();s.meter.distance=30.;assert!(s.mark(1,2,100.,15,false,[0.;4]));
  s.credit(2,200.);s.credit(3,999.);assert_eq!(s.marks[0].damage(),450.);
  s.time=1.999;s.credit(2,50.);assert_eq!(s.marks[0].damage(),500.);
  s.time=2.;s.credit(2,1000.);assert_eq!(s.marks[0].damage(),500.);
 }
 #[test]fn detonation_uses_game_time_once_and_invalid_steps_do_not_advance(){
  let mut s=state();s.meter.distance=30.;s.mark(1,2,100.,15,false,[0.;4]);
  for dt in [0.,-1.,f32::NAN,2.]{assert!(s.advance(dt).is_empty());}assert_eq!(s.time,0.);
  assert!(s.advance(1.).is_empty());s.credit(2,50.);let due=s.advance(1.);assert_eq!(due.len(),1);assert_eq!(due[0].damage(),300.);
  assert!(s.advance(1.).is_empty());assert!(s.marks.is_empty());
 }
 #[test]fn leakage_scales_total_including_level_and_overlapping_marks_keep_own_deadlines(){
  let mut s=state();s.meter.distance=30.;assert!(s.mark(1,1,400.,15,true,[0.;4]));
  assert_eq!(s.marks[0].damage(),137.5);s.time=0.5;s.credit(1,100.);
  s.meter.distance=30.;assert!(s.mark(1,1,100.,15,true,[0.;4]));
  assert_eq!(s.marks[0].sum,500.);assert_eq!(s.marks[1].sum,100.);
  s.time=2.;s.credit(1,200.);assert_eq!(s.marks[0].damage(),162.5);assert_eq!(s.marks[1].damage(),112.5);
 }
 #[test]fn each_charge_is_spent_once_and_can_be_spent_again_on_a_later_hit(){
  let mut s=state();s.meter.distance=30.;assert!(s.mark(1,2,10.,1,false,[0.;4]));
  assert!(!s.mark(1,3,20.,1,false,[0.;4]));s.meter.distance=30.;assert!(s.mark(1,3,20.,1,false,[0.;4]));assert_eq!(s.marks.len(),2);
 }
 #[test]fn accepted_scope_deduplicates_notifications_not_same_frame_attacks(){
  assert!(!first_contact(1,2));{let _a=dispatch_scope(1,2);assert!(first_contact(1,2));assert!(!first_contact(1,2));
   {let _b=dispatch_scope(3,4);assert!(first_contact(3,4));assert!(!first_contact(1,2));}assert!(!first_contact(1,2));}
  let _next=dispatch_scope(1,2);assert!(first_contact(1,2));
 }
 #[test]fn both_modes_have_private_root_and_wave_rows(){
  for id in ATTACK..=ATTACK+3{assert!(private_attack(id,1));assert!(!private_attack(id,0));}
  for id in BULLET..=BULLET+7{assert!(private_bullet(id));}assert!(!private_bullet(BULLET+8));
 }
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
