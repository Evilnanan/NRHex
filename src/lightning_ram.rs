//! Experimental Lightning Ram: private HKS control + native capsule velocity submission.
use crate::{ptr,read_ptr,read_i32,has_effect,native_actor,ACTIVE,BASE,ram_motion::{Motion,Travel,V}};
use std::{collections::BTreeMap,sync::{Mutex,atomic::{AtomicPtr,AtomicU64,Ordering}}};
pub const OWNER:i32=95005600;
pub const FAST:i32=OWNER+1;
pub const QUERY:i32=990060;
pub const STOP_QUERY:i32=990061;
pub const CRASH_QUERY:i32=990062;
pub const BULLET:i32=OWNER+50;
pub const ATTACK:i32=OWNER+60;
pub const EXTREME_ATTACK:i32=OWNER+61;
pub const FAST_ATTACK:i32=OWNER+62;
pub fn attack(id:i32)->bool{matches!(id,ATTACK|EXTREME_ATTACK|FAST_ATTACK)}
pub fn attack_for_damage(damage:f32)->i32{
 match contact_tier(damage){Some((_,10,_))=>EXTREME_ATTACK,Some((_,7,_))=>FAST_ATTACK,_=>ATTACK}
}
const CONTACT_SPEED:f32=6.;
#[derive(Clone,Copy,Debug,PartialEq)]enum EnemyResponse{Suppressed,Yield,Deflect,Rebound}
fn contact_tier(damage:f32)->Option<(f32,u8,u8)>{
 if !damage.is_finite()||damage<=0.{return None;}
 let speed=(damage/1.5).sqrt();
 if speed<CONTACT_SPEED{return None;}
 Some((speed,if speed>=15.{10}else if speed>=10.{7}else{3},if speed>=15.{11}else if speed>=10.{6}else{5}))
}
fn enemy_response(kind:i32,impact:u8)->EnemyResponse{
 if kind==3{return EnemyResponse::Rebound;}
 match impact{4|6|7|9|10=>EnemyResponse::Yield,1|2|3|5|8|11=>EnemyResponse::Deflect,_=>EnemyResponse::Rebound}
}
fn slide_retention(closing:f32,speed:f32)->f32{
 let angle=(closing/speed.max(0.00001)).clamp(0.,1.).acos().to_degrees();
 let t=((angle-15.)/45.).clamp(0.,1.);
 0.25+0.60*t*t*(3.-2.*t)
}
fn enemy_velocity(v:V,normal:V,response:EnemyResponse)->V{
 let n=normal;let closing=(-v.dot(n)).max(0.);let tangent=v.add(n.scale(closing));
 let next=match response{
  EnemyResponse::Suppressed|EnemyResponse::Yield=>v.add(n.scale(closing*0.05)),
  EnemyResponse::Deflect=>tangent.scale(slide_retention(closing,v.len())),
  EnemyResponse::Rebound=>tangent.scale(0.65).add(n.scale(closing*0.35)).add(V(0.,2.5,0.)),
 };
 next.scale((v.len()/next.len().max(0.00001)).min(1.))
}
fn rebound_velocity(incoming:V,normal:V,speed:f32)->V{
 let outward=V(normal.0,0.,normal.2).unit();
 let outward=if outward.len()>0.5{outward}else{V(-incoming.0,0.,-incoming.2).unit()};
 let speed=speed.clamp(0.,crate::ram_motion::SAFETY_SPEED);
 // The model surface determines "away". Do not let a downward normal cancel
 // the lift, or use the already slowed movement as the available impulse.
 let tangent=V(incoming.0,0.,incoming.2).plane(outward).scale(0.35);
 let next=outward.scale(speed*0.35).add(tangent).add(V(0.,2.5,0.));
 next.scale((speed/next.len().max(0.00001)).min(1.))
}
#[derive(Clone,Copy)]struct PendingStagger{target:usize,handle:u64,contact:EnemyContact,incoming:V,expected:crate::ram_stagger::Expected,age:f32,consumed:bool}
#[derive(Clone,Copy)]struct EnemyContact{speed:f32,normal:V,medium:Option<u8>}
#[derive(Clone,Copy)]struct Resolution{hit:usize,target:usize,value:Option<u8>,suppressed:bool}
thread_local!{static RESOLUTION:std::cell::Cell<Option<Resolution>>=const{std::cell::Cell::new(None)};}
pub const IMPACT_RVA:usize=0x6d1be0;
pub const IMPACT_PATTERN:&str="48 89 5C 24 10 57 48 83 EC 20 41 80 B8 21 01 00 00 03 48 8B F9 0F B6 DA";
pub static IMPACT_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
// 4381F0 may preserve an older action+1C when kind=99999 and the new
// resolved impact is zero. Capture the real return, never infer resistance
// from that notification kind or a previously queued action.
pub unsafe extern "system" fn resolved_impact(actor:usize,level:u8,hit:usize)->u8{
 let original:unsafe extern "system" fn(usize,u8,usize)->u8=std::mem::transmute(IMPACT_ORIGINAL.load(Ordering::Acquire));
 let incoming = crate::light_footed::incoming_impact(actor, level, hit);
 let result=original(actor,incoming,hit);
 if ACTIVE.load(Ordering::Acquire)&&CONTACT.with(std::cell::Cell::get){RESOLUTION.with(|s|{
  if let Some(mut r)=s.get().filter(|r|r.hit==hit&&r.target==actor){r.value=Some(result);r.suppressed=level==0;s.set(Some(r));}
 });}
 result
}
thread_local!{static ENEMY_CONTACT:std::cell::Cell<Option<EnemyContact>>=const{std::cell::Cell::new(None)};}
pub struct ImpactGuard(Option<(usize,u8,u8,Option<EnemyContact>,Option<Resolution>)>);
impl Drop for ImpactGuard{fn drop(&mut self){if let Some((hit,level,override_id,previous,resolution))=self.0{unsafe{
 ptr::write((hit+0x24)as*mut u8,level);ptr::write((hit+0x1e)as*mut u8,override_id);
 }ENEMY_CONTACT.with(|s|s.set(previous));RESOLUTION.with(|s|s.set(resolution));}}}
// Query native current poise/effect reaction mapping before the real hit can
// spend poise. This calls only the resolver, never damage/action submission.
// Zero descriptor deliberately disables the explicit-result shortcut (+121=3)
// and bypass bit (+2B4); the resolver reads only those two hit fields.
unsafe fn medium_impact(target:usize)->Option<u8>{
 if target==0{return None;}
 let modules=read_ptr(target+0x1b8);let original=IMPACT_ORIGINAL.load(Ordering::Acquire);
 if modules==0||read_ptr(modules+0x40)==0||read_ptr(target+0x190)==0||original.is_null(){return None;}
 let descriptor=[0u8;0x2b8];
 let query:unsafe extern "system" fn(usize,u8,usize)->u8=std::mem::transmute(original);
 let level=query(target,2,descriptor.as_ptr()as usize);
 (level<=11).then_some(level)
}
unsafe fn impact_scope(hit:usize,target:usize,damage:f32,normal:V,medium:Option<u8>)->ImpactGuard{
 let Some((speed,level,override_id))=contact_tier(damage)else{return ImpactGuard(None);};
 let old=(hit,ptr::read((hit+0x24)as*const u8),ptr::read((hit+0x1e)as*const u8),ENEMY_CONTACT.with(|s|s.replace(Some(EnemyContact{speed,normal,medium}))),RESOLUTION.with(|s|s.replace(Some(Resolution{hit,target,value:None,suppressed:false}))));
 ptr::write((hit+0x24)as*mut u8,level);ptr::write((hit+0x1e)as*mut u8,override_id);ImpactGuard(Some(old))
}
// Explicit override IDs are the original E6D510 mapping: 5->3, 6->7, 11->10.
// Snapshot damage travels with each root, so remote peers use the same tier.
unsafe fn model_contact_normal(hit:usize)->V{
 // 5363A0 copies the model collision callback's normal to +200. Orient it
 // toward the bullet origin (+1E0), away from its surface point (+1F0).
 // Never substitute the target centre or flip against current velocity:
 // a trailing sphere can hit after the owner has already passed the surface.
 let read=|offset|V::from(ptr::read_unaligned((hit+offset)as*const[f32;4]));
 let normal=read(0x200);let outward=read(0x1e0).sub(read(0x1f0));
 if !normal.finite()||!outward.finite()||normal.len()<0.5{return V::default();}
 let n=normal.unit();let side=n.dot(outward);
 if side.abs()<0.0001{return V::default();}
 if side<0.{n.scale(-1.)}else{n}
}
pub unsafe fn contact_impact(hit:usize,target:usize,damage:f32,_base:usize)->ImpactGuard{
 if hit==0||!attack(read_i32(hit+0x40))||read_i32(hit+0x44)!=1{return ImpactGuard(None);}
 let (normal,medium)=if CONTACT.with(std::cell::Cell::get){
  (model_contact_normal(hit),medium_impact(target))
 }else{(V::default(),None)};
 impact_scope(hit,target,damage,normal,medium)
}
pub const MOTION_RVA:usize=0x4702bb;
pub const MOTION_PATTERN:&str="48 8D 55 80 48 8B 8F 98 00 00 00 E8 25 75 92 00";
pub static MOTION_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub const REBASE_RVA:usize=0x468910;
pub const REBASE_PATTERN:&str="48 89 5C 24 18 57 48 83 EC 50 0F 29 74 24 40 48 8B 05 ? ? ? ? 48 33 C4 48 89 44 24 30 F3 0F 10 02 48 8B D9";
pub static REBASE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub const BODY_RVA:usize=0xda26c0;
pub const BODY_PATTERN:&str="40 53 48 83 EC 20 8B C1 8B D9 25 FF FF FF 00 3D FF FF FF 00 74 5D 48 8B 0D ? ? ? ? 48 85 C9";
// Ground power 160 -> 20 stamina/s; airborne power 80 -> at most 10/s.
#[derive(Debug)]struct MotorFee{started:bool,remainder:f64,weapon:i32}
impl Default for MotorFee{fn default()->Self{Self{started:false,remainder:0.,weapon:110000}}}
impl MotorFee{
 fn action(&mut self,cost:i32,weapon:i32)->i32{self.weapon=weapon;if std::mem::replace(&mut self.started,true){0}else{cost}}
 fn charge(&mut self,work:f32,rate:f32)->i32{
  if !work.is_finite()||work<=0.||!rate.is_finite()||rate<0.{return 0;}
  self.remainder+=f64::from(work)*0.125*f64::from(rate);
  let whole=(self.remainder+1e-6).floor().min(f64::from(i32::MAX))as i32;
  self.remainder=(self.remainder-f64::from(whole)).max(0.);whole
 }
}
fn roll_behavior(id:i32)->bool{matches!(id,300000710|300000711|300000715|300000716)||id==OWNER+90}
pub(crate) unsafe fn action_cost(actor:usize,behavior:i32,weapon:i32,cost:i32,base:usize)->Option<i32>{
 if !roll_behavior(behavior)||!eligible(actor,base){return None;}
 let mut s=STATE.lock().ok()?;
 if s.actor!=actor||s.handle!=read_ptr(actor+8)as u64||!s.active||s.interrupted{return None;}
 Some(s.fee.action(cost,weapon))
}
// HKS act(110), SetStaminaRecoveryDisabled: one-update action flag, cleared
// by the native action lifecycle. Never clear other callers' recovery flags.
pub const RECOVERY_FLAG_RVA:usize=0x3f5ace;
pub const RECOVERY_FLAG_PATTERN:&str="48 8B 07 48 8B 48 60 48 8B 81 C8 00 00 00 83 48 18 40";
unsafe fn disable_recovery(actor:usize){
 let coord=read_ptr(actor+0x60);let flags=if coord!=0{read_ptr(coord+0xc8)}else{0};
 if flags!=0{let p=(flags+0x18)as*mut u32;ptr::write(p,ptr::read(p)|0x40);}
}
unsafe fn motor_cost_rate(actor:usize,weapon:i32)->f32{
 // Same PlayerIns virtual cost calculation used by both native payment paths.
 // A large reference fee preserves fractional reductions before integer debit.
 let vt=read_ptr(actor);if vt==0{return 1.;}let address=read_ptr(vt+0x348);if address==0{return 1.;}
 let getter:unsafe extern "system" fn(usize,i32,i32,u8,i32)->i32=std::mem::transmute(address);
 getter(actor,10000,weapon,0,1).max(0)as f32/10000.
}
// Three exact finite f32 limbs carry generation + emission-time level.
// Keep velocity damage separate: tiers and recoil always use physical speed.
pub(crate) fn encode_roll(generation:u64,level:i32)->[f32;3]{
 [(generation&0xffffff)as f32,((generation>>24)&0xffffff)as f32,((generation>>48)as u32|((level.clamp(1,15)as u32)<<16)|0x400000)as f32]
}
pub(crate) fn decode_roll(v:[f32;3])->Option<(u64,i32)>{
 if v.iter().all(|x|x.is_finite()&&x.fract()==0.)&&(0. ..16777216.).contains(&v[0])&&(0. ..16777216.).contains(&v[1])&&(4259840. ..5242880.).contains(&v[2]){
  let high=v[2]as u32;let generation=v[0]as u64|((v[1]as u64)<<24)|((u64::from(high&0xffff))<<48);
  if generation!=0{return Some((generation,((high>>16)&15)as i32));}
 }
 // Accept already in-flight snapshots from the previous format without bonus.
 crate::soul_immunity::decode(v).map(|g|(g,0))
}
pub(crate) fn contact_damage(velocity_damage:f32,level:i32)->f32{
 velocity_damage+if attack_for_damage(velocity_damage)==EXTREME_ATTACK{30.*level.clamp(0,15)as f32}else{0.}
}
// a713_040010 travels 3.700m before the 22/30s continuation window.
const ROLL_SPEED:f32=3.700/(22./30.);
const DEFLECT_TIME:f32=0.1;
fn remove_inward(v:V,outward:V)->V{v.sub(outward.scale(v.dot(outward).min(0.)))}
#[derive(Default)]struct State{startup_hand:Option<i32>,deflection:Option<(V,f32)>,pending:Vec<PendingStagger>,actor:usize,handle:u64,stats:usize,motion:Motion,travel:Travel,yielding:crate::ram_motion::Yielding,effective_speed:f32,detach:f32,step_body:Option<u32>,step_grace:f32,push:bool,active:bool,interrupted:bool,queried:bool,missed_updates:u8,drive:bool,fee:MotorFee,wish:V,window:bool,recoil:bool,ending:bool,crash:bool,crash_pending:bool,rebound:f32,started:bool,rolling:bool,visual_speed:f32,trail:Option<V>,generation:u64,sync_cycle:u32}
static GENERATION:AtomicU64=AtomicU64::new(1);
static CONTACTS:Mutex<BTreeMap<(u64,u64,u64),V>>=Mutex::new(BTreeMap::new());
static STATE:Mutex<State>=Mutex::new(State{startup_hand:None,deflection:None,pending:Vec::new(),actor:0,handle:0,stats:0,motion:Motion{motor_work:0.,velocity:V(0.,0.,0.),spin:V(0.,0.,0.),initialized:false,locked:false,airborne:false,support:None},travel:Travel{previous:None,blocked:0.},yielding:crate::ram_motion::Yielding{body:None,age:0.,knocked:0.},effective_speed:0.,detach:0.,step_body:None,step_grace:0.,push:false,active:false,interrupted:false,queried:false,missed_updates:0,drive:false,fee:MotorFee{started:false,remainder:0.,weapon:110000},wish:V(0.,0.,0.),window:false,recoil:false,ending:false,crash:false,crash_pending:false,rebound:0.,started:false,rolling:false,visual_speed:0.,trail:None,generation:0,sync_cycle:0});
// No game calls under STATE or on a Havok worker. Match the current local
// actor, generation handle, physics module and the exact live proxy object.
unsafe fn managed_actor(s:&State,base:usize)->Option<usize>{
 if !ACTIVE.load(Ordering::Acquire)||!s.started||s.actor==0
  ||!(s.rebound>0.||(s.active&&s.rolling&&!s.interrupted)){return None;}
 let world=read_ptr(base+crate::WORLD_RVA);if world==0||read_ptr(world+0x174e8)!=s.actor{return None;}
 if read_ptr(s.actor+8)as u64!=s.handle{return None;}
 let modules=read_ptr(s.actor+0x1b8);if modules==0{return None;}
 let stats=read_ptr(modules);if stats==0||read_i32(stats+0x140)<=0{return None;}
 let physics=read_ptr(modules+0x68);
 (physics!=0&&read_ptr(physics+8)==s.actor).then_some(physics)
}
pub(crate) unsafe fn managed_physics(physics:usize,base:usize)->Option<bool>{
 let s=STATE.lock().ok()?;
 (managed_actor(&s,base)?==physics).then_some(s.motion.airborne)
}
pub(crate) unsafe fn managed_proxy(proxy:usize,base:usize)->bool{
 if proxy==0{return false;}
 let Ok(s)=STATE.lock()else{return false;};let Some(physics)=managed_actor(&s,base)else{return false;};
 [0x98,0xa0].into_iter().any(|offset|{
  let wrapper=read_ptr(physics+offset);
  wrapper!=0&&read_ptr(wrapper)==base+0x2c9e6a0&&read_ptr(wrapper+0x88)==proxy
 })
}
pub(crate) unsafe fn managed_collector(collector:usize,base:usize)->bool{
 if collector==0||!ACTIVE.load(Ordering::Acquire){return false;}
 let Ok(s)=STATE.lock()else{return false;};let Some(physics)=managed_actor(&s,base)else{return false;};
 [0x98,0xa0].into_iter().any(|offset|{
  let wrapper=read_ptr(physics+offset);
  wrapper!=0&&read_ptr(wrapper)==base+0x2c9e6a0&&
   [0xa8,0xb0].into_iter().any(|field|read_ptr(wrapper+field)==collector)
 })
}
impl State{
 fn enemy_collision(&mut self,contact:EnemyContact,response:EnemyResponse,staggered:bool){
  let incoming=self.motion.velocity;
  // Only a newly confirmed ultimate-stagger animation may rebound us.
  // Native level-2 immunity still separates large targets from small mobs.
  let bounce=staggered&&response!=EnemyResponse::Suppressed&&contact.medium==Some(0)&&contact.speed>=15.;
  let surface=contact.normal;
  let valid_normal=surface.finite()&&surface.len()>0.5;
  if !bounce&&valid_normal&&incoming.dot(surface)>= -0.5{return;}
  let response=if bounce{EnemyResponse::Rebound}
   else if contact.medium.is_some_and(|level|level!=0){EnemyResponse::Yield}
   else if response==EnemyResponse::Rebound{EnemyResponse::Deflect}else{response};
  // No surface to slide along: a confirmed hit still pays the head-on pass cost.
  let response=if !bounce&&!valid_normal{EnemyResponse::Yield}else{response};
  let mut normal=if valid_normal{surface.unit()}else{incoming.unit().scale(-1.)};
  if response==EnemyResponse::Deflect{
   // Stay in the support plane when striking the inclined flank of a model.
   if let Some(ground)=self.motion.support.filter(|_|!self.motion.airborne){
    let tangent=normal.plane(ground);
    if tangent.len()>0.2{normal=tangent.unit();}
   }
  }
  self.motion.velocity=if bounce{rebound_velocity(incoming,normal,contact.speed)}else{enemy_velocity(incoming,normal,response)};
  self.travel=Travel::default();self.detach=0.;self.step_body=None;self.step_grace=0.;self.yielding=Default::default();
  match response{
   EnemyResponse::Suppressed|EnemyResponse::Yield=>{self.yielding.knocked=0.12;}
   EnemyResponse::Deflect=>{
    let spin=remove_inward(self.motion.spin,normal);
    let tangent=spin.plane(normal);
    let limit=self.motion.velocity.plane(normal).len();
    self.motion.spin=spin.sub(tangent).add(tangent.scale((limit/tangent.len().max(0.00001)).min(1.)));
    self.deflection=Some((normal,DEFLECT_TIME));
    if self.motion.velocity.len()<=crate::ram_motion::RELEASE{self.motion.locked=false;}
   }
   EnemyResponse::Rebound=>{
    self.deflection=None;self.active=false;self.drive=false;self.ending=true;self.crash_pending=false;
    self.crash=true;self.rebound=0.45;self.motion.airborne=true;self.motion.support=None;
    self.motion.spin=V::default();self.motion.locked=false;self.visual_speed=0.;self.pending.clear();
   }
  }
 }
 fn advance_roll(&mut self,dt:f32,drive:bool,wish:V,ground:Option<V>,budget:f32)->bool{
  let Some((outward,left))=self.deflection else{return self.motion.advance_with_budget(dt,drive,wish,ground,budget);};
  let held=dt.min(left);
  self.motion.velocity=remove_inward(self.motion.velocity,outward);
  self.motion.spin=remove_inward(self.motion.spin,outward);
  let sliding_wish=remove_inward(wish,outward);
  if !self.motion.advance_with_budget(held,drive,sliding_wish,ground,budget){return false;}
  self.motion.velocity=remove_inward(self.motion.velocity,outward);
  self.motion.spin=remove_inward(self.motion.spin,outward);
  self.deflection=(left>held+1e-6).then_some((outward,left-held));
  if dt>held{
   let work=self.motion.motor_work;
   if !self.motion.advance_with_budget(dt-held,drive,wish,ground,(budget-work).max(0.)){return false;}
   self.motion.motor_work+=work;
  }
  true
 }
 fn rebase(&mut self,delta:V){
  if let Some((position,_,_))=&mut self.travel.previous{*position=position.add(delta);}
  if let Some(position)=&mut self.trail{*position=position.add(delta);}
 }
 fn query_expired(&mut self)->bool{
  if std::mem::take(&mut self.queried){self.missed_updates=0;false}
  else{self.missed_updates=self.missed_updates.saturating_add(1);self.missed_updates>=2}
 }
 fn wall_stop(&mut self,speed:f32){
  self.deflection=None;self.active=false;self.drive=false;self.ending=true;self.motion=Motion::default();
  self.travel=Travel::default();self.detach=0.;self.step_body=None;self.step_grace=0.;self.effective_speed=0.;self.visual_speed=0.;
  self.yielding=Default::default();self.pending.clear();
  self.push=(6. ..crate::ram_motion::CRASH_SPEED).contains(&speed);self.rebound=0.;self.crash=speed>=crate::ram_motion::CRASH_SPEED;self.crash_pending=self.crash;
 }
}
// 3D7AF0 -> 468910 translates the native physics position, cached positions
// and capsules by the same delta. Keep our position history in that frame;
// velocities, time, energy and contact generation are invariant to translation.
pub unsafe extern "system" fn rebase(physics:usize,delta:*const f32){
 let shift=(!delta.is_null()).then(||V(ptr::read_unaligned(delta),ptr::read_unaligned(delta.add(1)),ptr::read_unaligned(delta.add(2))));
 let original:unsafe extern "system" fn(usize,*const f32)=std::mem::transmute(REBASE_ORIGINAL.load(Ordering::Acquire));
 original(physics,delta);
 if !ACTIVE.load(Ordering::Acquire)||physics==0{return;}
 let Some(shift)=shift.filter(|v|v.finite())else{return;};
 let actor=read_ptr(physics+8);if actor==0||read_ptr(read_ptr(actor+0x1b8)+0x68)!=physics{return;}
 let handle=read_ptr(actor+8)as u64;
 crate::ram_peer_visual::rebase(actor,handle,shift);
 if let Ok(mut s)=STATE.lock(){if s.actor==actor&&s.handle==handle{s.rebase(shift);}}
 // Contact history is expressed in the owning character's coordinate frame.
 // Remote owners receive their own native rebase callback exactly once too.
 if let Ok(mut contacts)=CONTACTS.lock(){for (&(owner,_,_),position)in contacts.iter_mut(){if owner==handle{*position=position.add(shift);}}}
}
fn crash_loss(maximum:i32)->i32{((i64::from(maximum.max(0))+2)/4)as i32}
// Native HP-cost settlement: fixed max-HP loss, no offensive scaling and no
// self-hit (which could trigger rescue, retaliation or the rolling contact logic).
pub unsafe fn crash_damage(actor:usize,base:usize){
 if !eligible(actor,base){return;}
 let pending=STATE.lock().is_ok_and(|mut s|{if s.actor!=actor{return false;}std::mem::take(&mut s.crash_pending)});
 if !pending{return;}
 let stats=native_actor::stats(actor);let module=read_ptr(read_ptr(actor+0x1b8)+crate::DAMAGE_MODULE_SLOT);
 if module==0||read_ptr(module+8)!=actor||read_ptr(module)!=base+crate::ominous_pact::PLAYER_DAMAGE_VTABLE_RVA{return;}
 let amount=crash_loss(read_i32(stats+0x144));let before=read_i32(stats+0x140);
 let debit:unsafe extern "system" fn(usize,i32,u8,u8,f32,f32,u8)=std::mem::transmute(base+crate::death_dance::DEBIT_RVA);
 let finish:unsafe extern "system" fn(usize,i32,usize,usize,i32,u8)=std::mem::transmute(base+crate::death_dance::FINISH_RVA);
 debit(stats,-amount,1,0,0.,0.,0);
 let lost=before.saturating_sub(read_i32(stats+0x140)).max(0);
 if lost>0{let flags=(module+0x78)as*mut u8;ptr::write(flags,ptr::read(flags)|8);finish(module,lost,0,0,before,0);}
}
#[repr(C,align(16))]struct Point([f32;4]);
pub fn reset(){if let Ok(mut s)=STATE.lock(){*s=State::default();}}
pub(crate) fn ground_feedback(actor:usize)->Option<(u64,f32,bool)>{
 if !ACTIVE.load(Ordering::Acquire){return None;}
 let s=STATE.lock().ok()?;
 (s.actor==actor&&s.active&&s.started&&s.rolling&&!s.interrupted).then_some((s.generation,s.visual_speed,!s.motion.airborne))
}
pub(crate) fn awaiting_stagger(target:usize)->bool{
 STATE.lock().is_ok_and(|s|s.active&&s.pending.iter().any(|p|p.target==target))
}
pub(crate) fn stagger_consumed(target:usize,handle:u64){
 if let Ok(mut s)=STATE.lock(){for p in &mut s.pending{if p.target==target&&p.handle==handle{p.consumed=true;}}}
}
unsafe fn confirm_staggers(base:usize,dt:f32){
 let (generation,pending)=match STATE.lock(){Ok(mut s)=>{
  if !s.active||s.interrupted{s.pending.clear();return;}
  for p in &mut s.pending{p.age+=dt;}
  s.pending.retain(|p|p.age<=0.25);
  (s.generation,s.pending.clone())
 },_=>return};
 for p in pending{
  if !p.consumed{continue;}
  // No dereference of a saved actor after unload/reuse. Native identity lookup
  // supplies the live instance; graph identity is checked again separately.
  let target=crate::static_charge::resolve_actor(base,p.handle);
  if target!=p.target||!crate::ram_stagger::confirmed(target,p.expected,base){continue;}
  if let Ok(mut s)=STATE.lock(){
   if s.active&&!s.interrupted&&s.generation==generation&&s.pending.iter().any(|q|q.handle==p.handle&&q.expected==p.expected){
    s.motion.velocity=p.incoming;
    s.enemy_collision(p.contact,EnemyResponse::Rebound,true);
   }
  }
  break;
 }
}
unsafe fn owns(actor:usize,base:usize)->bool{actor!=0&&has_effect(actor,OWNER,base)&&!has_effect(actor,crate::reward_cycle_runtime::RESET,base)&&native_actor::stats(actor)!=0&&!native_actor::incapacitated(actor,native_actor::stats(actor))}
unsafe fn eligible(actor:usize,base:usize)->bool{crate::is_local(actor,base)&&owns(actor,base)}
pub fn reserved(owned:bool,weapon:i32)->bool{owned&&weapon==110000}
pub unsafe fn selected(actor:usize,w:crate::arts::Weapon,base:usize)->bool{
 reserved(actor!=0&&has_effect(actor,OWNER,base)&&!has_effect(actor,crate::reward_cycle_runtime::RESET,base),w.base)
}
pub unsafe fn art(actor:usize,hand:i32,base:usize)->bool{
 matches!(hand,0|1)&&owns(actor,base)&&source_art(actor,if hand==0{-2}else{-1},base)
}
unsafe fn source_art(actor:usize,source:i32,base:usize)->bool{
 let Some(w)=crate::arts_source_weapon(actor,source,base)else{return false;};
 crate::arts_actor_selection(actor,w,base).unwrap_or(w.original)==504
}
// TurnAngle's sign was verified in 0.17.313 controller testing: subtracting
// it mirrors left/right. This is not the MoveAngle convention used elsewhere.
fn steering_wish(yaw:f32,angle:f32)->V{let yaw=yaw+angle.to_radians();V(-yaw.sin(),0.,-yaw.cos())}
fn visual_speed(previous:f32,distance:f32,dt:f32)->f32{
 if !distance.is_finite()||!dt.is_finite()||dt<=0.||dt>0.25{return previous;}
 let measured=(distance/dt).clamp(0.,crate::ram_motion::SAFETY_SPEED);
 previous+(measured-previous)*(1.-(-dt/0.06).exp())
}
fn roll_rate(speed:f32)->f32{if speed.is_finite(){(speed/ROLL_SPEED).clamp(0.25,8.)}else{1.}}
// Diagnostic snapshots never invoke native code or change motor state.
#[cfg(feature="ram-sync-probe")]
pub fn sync_probe(actor:usize)->Option<(u64,bool,bool,bool,f32)>{
 let s=STATE.try_lock().ok()?;
 (s.actor==actor).then(||(s.generation,s.active,s.started,s.rolling,roll_rate(s.visual_speed)))
}
pub(crate) fn sync_frame(actor:usize)->Option<(u64,u32,f32,bool)>{
 let s=STATE.try_lock().ok()?;
 (s.actor==actor).then(||(s.generation,s.sync_cycle,roll_rate(s.visual_speed),s.active&&s.started&&s.rolling&&!s.interrupted))
}
pub unsafe fn speed(context:usize,native:f32)->f32{
 if !ACTIVE.load(Ordering::Acquire){return native;}
 let base=BASE.load(Ordering::Acquire);
 if let Some(rate)=crate::ram_sync::speed(context,base).or_else(||crate::ram_peer_visual::speed(context,base)){return native*rate;}
 let world=read_ptr(base+crate::WORLD_RVA);if world==0{return native;}
 let actor=read_ptr(world+0x174e8);if !eligible(actor,base){return native;}
 let animation=read_ptr(read_ptr(actor+0x1b8)+0x28);
 if animation==0||context!=animation+0x1800{return native;}
 let rate=STATE.lock().ok().filter(|s|s.actor==actor&&s.active&&s.started&&s.rolling).map(|s|roll_rate(s.visual_speed)).unwrap_or(1.);
 native*rate
}
pub unsafe fn behavior(actor:usize,judge:i32,source:i32,base:usize)->Option<i32>{
 if !matches!(judge,3710|3711|3715|3716)||!owns(actor,base)||!source_art(actor,source,base){return None;}
 let high=if crate::is_local(actor,base){
  STATE.lock().is_ok_and(|mut s|{if s.actor==actor&&s.active{s.rolling=true;}s.actor==actor&&s.active&&s.started&&s.motion.velocity.len()>=CONTACT_SPEED})
 }else{has_effect(actor,FAST,base)};
 if high{Some(OWNER+90)}else{Some(300000000+judge-3000)}
}
pub unsafe fn managed_art(actor:usize,hand:i32,base:usize)->bool{eligible(actor,base)&&art(actor,hand,base)}
impl State{
 unsafe fn bind(&mut self,actor:usize,stats:usize){
  let handle=read_ptr(actor+8)as u64;
  if (self.actor,self.handle,self.stats)!=(actor,handle,stats){*self=State{actor,handle,stats,..Default::default()};}
 }
}
pub(super) unsafe fn record_startup(actor:usize,hand:i32,allowed:bool){
 let stats=native_actor::stats(actor);
 if let Ok(mut s)=STATE.lock(){
  s.bind(actor,stats);
  // A new native L2 reservation supersedes any previous drive, even on a quick restart.
  s.active=false;s.drive=false;s.started=false;s.rolling=false;
  s.startup_hand=allowed.then_some(hand);
 }
}
// 0 native / 1 coast / 2 next roll / 3 recovery / 4 damaging crash / 5 enemy recoil / 6 harmless wall push.
pub unsafe fn query(actor:usize,art_id:i32,hand:i32,held:bool,window:bool,angle:f32,base:usize)->i32{
 if art_id!=113||!eligible(actor,base)||!art(actor,hand,base){return 0;}
 if has_effect(actor,crate::arts::OWNER,base){
  let w=crate::arts_hand_weapon(actor,hand,base).filter(|w|!selected(actor,*w,base));
  if let Some(state)=crate::ARTS.get(){if let Ok(mut state)=state.lock(){state.notify(w,true);}}
 }
 let stats=native_actor::stats(actor);let can_drive=held&&read_i32(stats+0x15c)>0;
 let mut rotation=Point([0.;4]);let get:unsafe extern "system" fn(usize,*mut f32)->*const f32=std::mem::transmute(base+crate::CHR_ROTATION_RVA);get(actor,rotation.0.as_mut_ptr());
 let wish=steering_wish(rotation.0[1],angle);
 let result={let Ok(mut s)=STATE.lock()else{return 0;};
  s.bind(actor,stats);
  s.queried=true;
  if s.recoil{s.recoil=false;s.active=false;s.interrupted=true;return 5;}
  if s.crash{s.crash=false;s.interrupted=true;return 4;}
  if s.push{s.push=false;s.interrupted=true;return 6;}
  if s.ending{return 3;}
  if s.interrupted{return 0;}if wish.finite(){s.wish=wish;}
  // Activation only arms the simulation. The real a713 attack behavior
  // opens rolling; the windup has substantial root motion of its own.
  let initial=!s.active;
  if initial{if !can_drive||s.rebound>0.||s.startup_hand!=Some(hand){return 0;}s.startup_hand=None;s.active=true;s.deflection=None;s.fee=MotorFee::default();s.started=false;s.rolling=false;s.window=false;s.motion=Motion::default();s.detach=0.;s.step_body=None;s.step_grace=0.;s.travel=Travel::default();s.yielding=Default::default();s.effective_speed=0.;s.generation=GENERATION.fetch_add(1,Ordering::Relaxed);s.sync_cycle=0;}
  let cycle=window&&!s.window;s.window=window;
  // Recheck every update, not only at animation-cycle boundaries.
  s.drive=can_drive;
  if !s.drive&&!s.motion.locked&&!s.motion.airborne&&s.started{s.active=false;s.ending=true;return 3;}
  if cycle{s.sync_cycle=s.sync_cycle.wrapping_add(1);2}else if s.started&&(s.motion.locked||s.motion.airborne){1}else{0}
 };
 if STATE.lock().is_ok_and(|s|s.actor==actor&&s.active&&s.drive&&s.started){disable_recovery(actor);}
 #[cfg(feature="ram-sync-probe")]
 if result==2{crate::ram_sync_probe::cycle(actor);}
 result
}
#[derive(Clone,Copy)]struct Contact{normal:V,point:V,fraction:f32,body:u32}
// DB1590 / DA26C0 resolves the fresh sweep ID. The asset branch matches
// DA2760 and 700130; HP alone is insufficient. Native 385985 / 49CF3D
// additionally require object attack > defense (equal defense blocks damage).
// Use our actual contact attacks, not that native path's fixed power of 90:
// partial-static buildings/trees can have HP=1 and defense=90.
unsafe fn object_power(base:usize)->u16{
 let lookup:unsafe extern "system" fn(*mut usize,i32,i32)=std::mem::transmute(base+crate::devils_gaze_hit::LOOKUP_RVA);
 [ATTACK,OWNER+69].into_iter().map(|id|{
  // Borrowed native AttackRef: id/cache, table/padding, row. No retention.
  let mut reference=[0usize;3];lookup(reference.as_mut_ptr(),1,id);
  if reference[2]==0{0}else{ptr::read_unaligned((reference[2]+0x62)as*const u16)}
 }).max().unwrap_or(0)
}
unsafe fn breakable_entry(entry:usize,power:impl FnOnce()->u16)->bool{
 if entry==0||ptr::read_unaligned((entry+8)as*const u16)!=5{return false;}
 let asset=read_ptr(entry+0x10);if asset==0||ptr::read((asset+0x190)as*const u8)!=0{return false;}
 let row=read_ptr(asset+0x20);row!=0&&ptr::read_unaligned((row+0x10)as*const i16)>0&&ptr::read((row+0x3c)as*const u8)&0x40==0
  &&power()>ptr::read_unaligned((row+0x12)as*const u16)
}
unsafe fn breakable(base:usize,body:u32)->bool{
 let get:unsafe extern "system" fn(u32)->usize=std::mem::transmute(base+BODY_RVA);breakable_entry(get(body),||object_power(base))
}
unsafe fn sweep(base:usize,position:V,motion:V)->Option<Contact>{sweep_shape(base,position,motion,0.45,0.3)}
unsafe fn sweep_shape(base:usize,position:V,motion:V,lift:f32,radius:f32)->Option<Contact>{
 let geometry=read_ptr(base+0x3c1e028);if geometry==0||read_ptr(geometry+0x98)==0||read_ptr(base+0x3c1d9a8)==0{return None;}
 let start=Point([position.0,position.1+lift,position.2,1.]);let delta=Point(motion.array());
 let mut hit=u32::MAX;let mut flags=0;let mut fraction=1.;let mut point=Point([0.;4]);let mut normal=Point([0.;4]);
 let cast:crate::spirit_placement::Sweep=std::mem::transmute(base+crate::SPIRIT_SWEEP_RVA);
 let result=cast(read_ptr(geometry+0x98),&mut hit,0x79,start.0.as_ptr(),delta.0.as_ptr(),radius,0,&mut flags,point.0.as_mut_ptr(),normal.0.as_mut_ptr(),&mut fraction,ptr::null_mut());
 let n=V::from(normal.0);(result==&mut hit as*mut u32&&hit&0xffffff!=0xffffff&&n.finite()&&n.len()>0.5&&fraction.is_finite()&&(0. ..=1.).contains(&fraction)).then_some(Contact{normal:n.unit(),point:V::from(start.0).add(motion.scale(fraction)).sub(n.unit().scale(radius)),fraction,body:hit})
}
unsafe fn capsule_dimensions(physics:usize,base:usize)->Option<(f32,f32)>{
 let wrapper=read_ptr(physics+0x98);
 if wrapper==0||read_ptr(wrapper)!=base+0x2c9e6a0{return None;}
 let radius=ptr::read_unaligned((wrapper+0xc0)as*const f32);
 if !radius.is_finite()||!(0.1..=2.).contains(&radius){return None;}
 let proxy=read_ptr(wrapper+0x88);
 let skin=if proxy!=0&&read_ptr(proxy)==base+0x2c9e9b8{ptr::read_unaligned((proxy+0x130)as*const f32)}else{0.};
 Some((radius,if skin.is_finite()&&(0. ..=0.2).contains(&skin){skin}else{0.}))
}
unsafe fn wall_probe(physics:usize,base:usize)->(f32,f32){
 let Some((radius,skin))=capsule_dimensions(physics,base)else{return (0.95,0.25);};let radius=radius+skin;
 // Match the capsule's side contact envelope, including native keep-distance.
 // Keep the lower edge at .7m so widening the torso probe cannot turn low
 // risers into crash walls. Native travel rejection is still mandatory.
 (0.7+radius,radius)
}
fn wall_extension(incoming:V,dt:f32,low:&Contact,lift:f32,radius:f32)->f32{
 if low.normal.0.hypot(low.normal.2)<0.25||low.normal.1<=0.{return 0.;}
 let closing=-incoming.unit().dot(low.normal);
 if closing<=0.05{return 0.;}
 // Same plane, using the actual height and radius difference. Account for the full
 // incoming direction, including descent; never extend along world-horizontal.
 (incoming.len()*dt*low.fraction+((lift-0.45)*low.normal.1+0.3-radius)/closing-(incoming.len()*dt+0.08)).clamp(0.,0.75)
}
fn same_wall_surface(low:&Contact,upper:&Contact)->bool{
 low.normal.dot(upper.normal)>0.92&&upper.point.sub(low.point).dot(low.normal).abs()<0.12
}
fn step_top_height(from:V,edge:&Contact,top:&Contact)->Option<f32>{
 let rise=top.point.1-from.1;
 (top.point.finite()&&top.normal.1>=0.7&&(0.03..=0.7).contains(&rise)
  &&top.point.1>=edge.point.1-0.03).then_some(rise)
}
unsafe fn step_top(base:usize,from:V,incoming:V,edge:Contact,lift:f32,radius:f32)->Option<f32>{
 let forward=V(incoming.0,0.,incoming.2).unit();
 if forward.len()<0.5||edge.point.1-from.1>0.7||edge.point.1<from.1-0.05{return None;}
 // Probe just beyond the lip, not at arbitrary distances from the player.
 // Two nearby tops exclude a bare underside, thin trim or a tall obstacle.
 let points=[0.12,0.35].map(|d|edge.point.add(forward.scale(d)));
 let mut rise=0f32;
 for p in points{
  let origin=V(p.0,from.1,p.2);
  let top=sweep_shape(base,origin,V(0.,-1.5,0.),1.25,0.05)?;
  rise=rise.max(step_top_height(from,&edge,&top)?);
 }
 // Keep the actual torso envelope clear throughout the diagonal ascent.
 let destination=V(points[1].0,from.1+rise+0.12,points[1].2);
 if sweep_shape(base,from,destination.sub(from),lift,radius).is_some(){return None;}
 Some(rise)
}
// Probe sphere radius 0.1, centre +0.22, cast 0.36 down. Recover the
// support plane's vertical gap directly from swept fraction and normal.
fn support_contact(normal:V,fraction:f32,airborne:bool,native_ground:bool)->bool{
 if normal.1<crate::ram_motion::SUPPORT_Y{return false;}
 let gap=0.36*fraction-0.22+0.1/normal.1;
 let _=(airborne,native_ground);
 gap>= -0.18&&gap<=0.035
}
fn near_surface(position:V,radius:f32,skin:f32,contact:&Contact)->bool{
 let gap=position.add(V(0.,radius,0.)).sub(contact.point).dot(contact.normal)-radius;
 (-0.08..=skin+0.015).contains(&gap)
}
unsafe fn bottom_support(base:usize,physics:usize,position:V)->Option<V>{
 let (radius,skin)=capsule_dimensions(physics,base)?;
 // The bottom of a capsule contacting a slope is above the plane directly
 // below the feet. Measure normal separation of the real bottom sphere,
 // including native keep-distance, rather than a tiny vertical foot ray.
 // An initial overlap is not a usable sweep hit. Native slope contact can
 // place our smaller query sphere 1-8mm inside the surface. Start outside
 // the whole accepted penetration range; near_surface still tests the
 // original capsule position, so this does not extend ground adhesion.
 let lift=0.1/crate::ram_motion::SUPPORT_Y;
 let depth=(skin+0.035)/crate::ram_motion::SUPPORT_Y+lift;
 sweep_shape(base,position,V(0.,-depth,0.),radius+lift,radius-0.02)
  .filter(|c|c.normal.1>=crate::ram_motion::SUPPORT_Y&&near_surface(position,radius,skin,c)).map(|c|c.normal)
}
unsafe fn curved_support(base:usize,physics:usize,position:V,previous:Option<V>,velocity:V,encounter:Option<Contact>)->Option<V>{
 let old=previous?;
 // D97810 builds the bottom capsule sphere from this per-character radius.
 let (radius,skin)=capsule_dimensions(physics,base)?;
 let valid=|c:&Contact|near_surface(position,radius,skin,c)&&crate::ram_motion::continuous_surface(previous,velocity,c.normal);
 // Along the previous support normal, not world-down: a near-vertical
 // continuation is still a contact, even though a downward ray misses it.
 sweep_shape(base,position.add(old.scale(0.1)),old.scale(-0.28),radius,radius-0.02).filter(valid)
  .or_else(||encounter.filter(valid)).map(|c|c.normal)
}
unsafe extern "system" fn velocity(physics:usize,vector:*mut f32,time:usize){
 if !ACTIVE.load(Ordering::Acquire)||physics==0||vector.is_null()||time==0{return;}
 let base=BASE.load(Ordering::Acquire);let actor=read_ptr(physics+8);
 if !eligible(actor,base)||read_ptr(read_ptr(actor+0x1b8)+0x68)!=physics{return;}
 let dt=ptr::read_unaligned((time+8)as*const f32);let position=V::from(ptr::read_unaligned((physics+0x70)as*const[f32;4]));
 if !position.finite()||!dt.is_finite()||dt<=0.||dt>0.25{return;}
 // Do not hold gameplay state across a native scene query.
 let (active,started,rolling,v,rebound,airborne,support)=match STATE.lock(){Ok(s)if s.actor==actor=>(s.active,s.started,s.rolling,s.motion.velocity,s.rebound,s.motion.airborne,s.motion.support),_=>return};
 if (!active||!rolling)&&rebound<=0.{return;}
 if rebound>0.{
  let mut s=match STATE.lock(){Ok(s)=>s,_=>return};
  let mut v=s.motion.velocity;
  if let Some(resolved)=s.travel.resolve(position){
   // Use accepted lateral displacement. Never carry energy through real walls.
   v=V(resolved.velocity.0,v.1.min(resolved.velocity.1.max(0.)),resolved.velocity.2);
  }
  v.1-=crate::ram_motion::GRAVITY*dt;s.motion.velocity=v;s.travel.submit(position,v,dt);
  ptr::write_unaligned(vector as*mut[f32;4],v.array());return;
 }
 if !started{
  let native=V::from(ptr::read_unaligned(vector as*const[f32;4]));
  if native.0.hypot(native.2)<0.5{return;}
  if let Ok(mut s)=STATE.lock(){s.motion.velocity=native;s.effective_speed=native.len();s.visual_speed=native.len();s.started=true;}
 }
 // Read the previous submission's accepted travel before adding power.
 // The probe only supplies a wall normal; it cannot invent a collision when
 // the native capsule actually accepted the requested movement.
 let previous=STATE.lock().ok().and_then(|s|s.travel.previous);
 // The low sweep identifies breakable contacts / ceilings. A separate torso
 // sweep supplies wall normals, so a short riser cannot cause a crash animation.
 let low=previous.and_then(|(from,incoming,elapsed)|sweep(base,from,incoming.scale(elapsed)));
 let (wall_lift,wall_radius)=wall_probe(physics,base);
 let mut upper_found=false;
 let wall=previous.and_then(|(from,incoming,elapsed)|{
  let heading=incoming.unit();
  let length=incoming.len()*elapsed+0.08;
  let extension=low.as_ref().map_or(0.,|c|wall_extension(incoming,elapsed,c,wall_lift,wall_radius));
  let path=heading.scale(length+extension);
  let upper=sweep_shape(base,from,path,wall_lift,wall_radius)
   .filter(|c|c.normal.0.hypot(c.normal.2)>0.25)
   .filter(|c|c.fraction*path.len()<=length+0.02||low.as_ref().is_some_and(|lo|same_wall_surface(lo,c)));
  upper_found=upper.is_some();
  // An overhang's down-facing side cannot be a traversable step top. It
  // remains a wall even when the upper probe starts inside the overhang.
  upper.or_else(||low.filter(|c|crate::ram_motion::wall_normal(c.normal)&&c.normal.1< -0.05))
 });
 let step=previous.and_then(|(from,incoming,elapsed)|{
  if upper_found||incoming.0.hypot(incoming.2)<6.||incoming.1< -1.{return None;}
  let edge=low?;
  let rejected=incoming.sub(position.sub(from).scale(1./elapsed));
  if -rejected.dot(edge.normal)<=0.5||edge.normal.0.hypot(edge.normal.2)<0.5{return None;}
  step_top(base,from,incoming,edge,wall_lift,wall_radius).map(|rise|(edge,((from.1+rise)-position.1).max(0.01)))
 });
 let soft_body=wall.as_ref().or(low.as_ref()).filter(|c|c.normal.0.hypot(c.normal.2)>0.25&&breakable(base,c.body)).map(|c|c.body);
 let ceiling=low.as_ref().is_some_and(|c|c.normal.1< -0.45);
 let native_ground=ptr::read((physics+0x92)as*const u8)!=0;
 let ground=curved_support(base,physics,position,support,v,low).or_else(||if !airborne||v.1<=0.5{
  if capsule_dimensions(physics,base).is_some(){bottom_support(base,physics,position)}else{sweep_shape(base,position,V(0.,-0.36,0.),0.22,0.1)
   .filter(|c|support_contact(c.normal,c.fraction,airborne,native_ground)).map(|c|c.normal)
  }
 }else{None});
 let mut obstructed=false;
 let mut step_air=false;
 let stopped={let Ok(mut s)=STATE.lock()else{return;};
  s.step_grace=(s.step_grace-dt).max(0.);
  if support.is_some()&&ground.is_none(){s.detach=0.10;}
  else{s.detach=(s.detach-dt).max(0.);}
  if let Some(resolved)=s.travel.resolve(position){
   if step.is_none()&&ground.is_some_and(|n|resolved.incoming.dot(n)<=0.5){s.step_body=None;}
   let continuing=step.is_some_and(|(edge,_)|s.step_body==Some(edge.body)&&s.step_grace>0.&&resolved.incoming.1>0.);
   let launch=if s.step_grace<=0.{step.filter(|(edge,_)|s.step_body!=Some(edge.body)).and_then(|(edge,rise)|crate::ram_motion::step_launch(resolved.incoming,resolved.velocity,edge.normal,rise))}else{None};
   step_air=continuing||launch.is_some();
   let tau=s.yielding.step(soft_body,wall.is_some()&&soft_body.is_none(),previous.map(|p|p.2).unwrap_or(dt));
   obstructed=resolved.rejected.0.hypot(resolved.rejected.2)>0.5;
   let mut velocity=if let Some(tau)=tau{crate::ram_motion::compliant_velocity(resolved.incoming,resolved.velocity,previous.unwrap().2,tau)}else{resolved.velocity};
   if step_air{
    velocity=launch.unwrap_or_else(||crate::ram_motion::flight_feedback(resolved.incoming,velocity,true,false,false));
    if launch.is_some(){s.step_body=step.map(|(edge,_)|edge.body);s.step_grace=0.18;s.detach=0.12;}
    s.motion.airborne=true;s.motion.support=None;
   }else if ground.is_none()&&!resolved.stopped{
    velocity=crate::ram_motion::flight_feedback(resolved.incoming,velocity,s.detach>0.,wall.is_some(),ceiling);
   }else if airborne{
    if let Some(n)=ground{
     // Native travel already stops at the floor. Reconstruct only the inward
     // component to settle this landing once, before advance consumes it.
     let inward=resolved.incoming.dot(n).min(0.);
     velocity=velocity.plane(n).add(n.scale(inward));
     let limit=resolved.incoming.len();velocity=velocity.scale((limit/velocity.len().max(0.00001)).min(1.));
    }
   }
   s.motion.velocity=velocity;s.effective_speed=resolved.velocity.len();

   if s.motion.velocity.len()<=crate::ram_motion::RELEASE{s.motion.locked=false;}
   let impact=wall.as_ref().filter(|_|tau.is_none())
    .filter(|c|!crate::ram_motion::continuous_surface(support,resolved.incoming,c.normal))
    .and_then(|c|crate::ram_motion::wall_impact(resolved.incoming,resolved.rejected,c.normal));
   if !step_air{if let Some(speed)=impact{s.wall_stop(speed);}else if resolved.stopped{s.wall_stop(0.);}}
  }
  !s.active&&s.rebound<=0.
 };
 let stats=native_actor::stats(actor);
 let fee_context=STATE.lock().ok().filter(|s|s.actor==actor&&s.active).map(|s|s.fee.weapon);
 let rate=fee_context.map(|weapon|motor_cost_rate(actor,weapon)).unwrap_or(0.);
 let stamina=if stats!=0{read_i32(stats+0x15c).max(0)}else{0};
 let available=stamina>0;
 let exempt=stats!=0&&(ptr::read((stats+0x189)as*const u8)&0x21!=0||ptr::read((base+0x3c0f2f7)as*const u8)!=0);
 let (desired,cost,working)={let Ok(mut s)=STATE.lock()else{return;};let drive=s.drive&&available;let wish=s.wish;
  if stopped{(V::default(),0,false)}else{
   let budget=if exempt||rate<=0.{f32::INFINITY}else{((f64::from(stamina)-s.fee.remainder).max(0.)/(0.125*f64::from(rate)))as f32};
   if !s.advance_roll(dt,drive&&rebound<=0.&&!obstructed,wish,if step_air{None}else{ground},budget){s.active=false;return;}
   // No independent wall bounce. Compliance expires promptly and blocked
   // motion cannot receive engine power, so corners never store new energy.
   let spin=s.motion.spin.len();s.visual_speed=visual_speed(s.visual_speed,spin*dt,dt);
   let candidate=s.motion.velocity;let work=s.motion.motor_work;
   let cost=s.fee.charge(work,if exempt{0.}else{rate});s.travel.submit(position,candidate,dt);(candidate,cost,work>0.)
  }
 };
 if working&&rate>0.{disable_recovery(actor);}
 if cost>0{let add:unsafe extern "system" fn(usize,i32,u8)=std::mem::transmute(base+crate::STAMINA_ADD_RVA);add(stats,-cost.min(read_i32(stats+0x15c).max(0)),0);}
 // Only the stack-local native velocity is replaced. Both capsules, normal
 // collision resolution and native movement/network updates consume it.
 ptr::write_unaligned(vector as*mut[f32;4],desired.array());
 if active&&desired.0.hypot(desired.2)>0.5{
  let mut rotation=Point([0.;4]);let get:unsafe extern "system" fn(usize,*mut f32)->*const f32=std::mem::transmute(base+crate::CHR_ROTATION_RVA);
  get(actor,rotation.0.as_mut_ptr());rotation.0[1]=(-desired.0).atan2(-desired.2);
  let set:unsafe extern "system" fn(usize,*const f32)=std::mem::transmute(base+crate::parry_turn::SET_ROTATION_RVA);set(physics,rotation.0.as_ptr());
 }
}
unsafe fn fast_marker(actor:usize,base:usize,enabled:bool){
 if actor==0||!crate::is_local(actor,base){return;}
 let present=has_effect(actor,FAST,base);if present==enabled{return;}
 if enabled{let add:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);add(actor,FAST,0);}
 else{let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);remove(read_ptr(actor+0x190),FAST);}
}
pub unsafe fn update(actor:usize,base:usize,dt:f32){
 if !eligible(actor,base){fast_marker(actor,base,false);reset();return;}
 if !dt.is_finite()||dt<=0.{return;}
 if !STATE.lock().is_ok_and(|s|s.actor==actor){fast_marker(actor,base,false);return;}
 confirm_staggers(base,dt);
 let position=V::from(crate::position(actor,base));
 let pulses={let Ok(mut s)=STATE.lock()else{return;};if s.actor!=actor{return;}
  s.rebound=(s.rebound-dt).max(0.);
  if s.query_expired(){s.active=false;if s.rebound==0.{s.started=false;s.interrupted=false;s.ending=false;s.crash=false;s.crash_pending=false;s.push=false;s.travel=Travel::default();s.effective_speed=0.;}s.trail=None;}
  if !s.active||!s.started||s.motion.velocity.len()<CONTACT_SPEED{s.trail=None;Vec::new()}else{
   let from=s.trail.unwrap_or(position);let displacement=position.sub(from);let length=displacement.len();
   if length>10.{s.active=false;Vec::new()}else{
    s.trail=Some(position);
    let count=(length/0.65).ceil().clamp(1.,16.)as usize;
    let damage=s.motion.velocity.len().powi(2)*1.5;
    (1..=count).map(|i|(from.add(displacement.scale(i as f32/count as f32)),damage,s.handle,s.generation)).collect()
   }
  }
 };
 let high=STATE.lock().is_ok_and(|s|s.active&&s.started&&s.motion.velocity.len()>=CONTACT_SPEED);fast_marker(actor,base,high);
 for (p,damage,handle,generation)in pulses{crate::static_charge::emit_ram(handle,generation,damage,[p.0,p.1+0.5,p.2,1.],base);}
 // Submit one short leading contact, rather than waiting until the capsule
 // has already been blocked. Never spawn the centre across scene geometry.
 let lead=STATE.lock().ok().filter(|s|s.active&&s.started&&s.rolling&&s.motion.velocity.len()>=CONTACT_SPEED)
  .map(|s|(s.motion.velocity,s.handle,s.generation));
 if let Some((v,handle,generation))=lead{
  let offset=leading_offset(v,dt);
  let fraction=sweep(base,position,offset).map(|c|c.fraction).unwrap_or(1.);
  let p=position.add(offset.scale(fraction));
  crate::static_charge::emit_ram(handle,generation,v.len().powi(2)*1.5,[p.0,p.1+0.5,p.2,1.],base);
 }
}
fn leading_offset(v:V,dt:f32)->V{let horizontal=V(v.0,0.,v.2);horizontal.unit().scale((horizontal.len()*dt.min(1./30.)).min(0.5))}
// The serialized roll generation also deduplicates separate pulses on peers.
// Native ownership/authority still decides which contacts apply damage.
// Bullet +B0 stores ChrIns +8 entity identity, not the pool handle accepted
// by 519140. Reuse the discharge resolver (63F840 -> 518F30), including its
// full-identity validation for local and remote owners.
pub unsafe fn allow_contact(target:usize,bullet:usize,generation:Option<u64>,base:usize)->bool{
 let Some(generation)=generation else{return false;};
 let owner=read_ptr(bullet+0xb0)as u64;let handle=read_ptr(target+8)as u64;
 if handle==owner||!crate::spatial_ready(target){return false;}
 // Native rescue owns downed allies; exclude standing/reviving allies and
 // retain the same per-pass dedup as hostile contacts.
 if crate::ambulance::party_member(target,base)&&ptr::read((target+0x710)as*const u8)!=1{return false;}
 if STATE.lock().is_ok_and(|s|s.handle==owner&&(!s.active||s.generation!=generation)){return false;}
 let actor=crate::static_charge::resolve_actor(base,owner);if !crate::spatial_ready(actor){return false;}
 let position=V::from(crate::position(actor,base));
 let Ok(mut contacts)=CONTACTS.lock()else{return false;};
 contacts.retain(|&(h,g,_),p|h!=owner||(g==generation&&p.sub(position).len()<3.));
 !contacts.contains_key(&(owner,generation,handle))
}
pub unsafe fn accepted_contact(target:usize,bullet:usize,generation:Option<u64>,result:u8,base:usize){
 if result==0{return;}let Some(generation)=generation else{return;};
 let owner=read_ptr(bullet+0xb0)as u64;let actor=crate::static_charge::resolve_actor(base,owner);
 if !crate::spatial_ready(actor){return;}
 // An accepted hit can deal damage without sending a reaction notification.
 // reaction() already took the contact if it applied movement feedback.
 if let Some(contact)=ENEMY_CONTACT.with(std::cell::Cell::take).filter(|_|CONTACT.with(std::cell::Cell::get)){
  if let Ok(mut s)=STATE.lock(){
   if s.active&&s.actor==actor&&s.handle==owner&&s.generation==generation{
    s.enemy_collision(contact,EnemyResponse::Yield,false);
   }
  }
 }
 let position=V::from(crate::position(actor,base));
 if let Ok(mut contacts)=CONTACTS.lock(){
  if contacts.len()>1024{contacts.clear();}
  contacts.insert((owner,generation,read_ptr(target+8)as u64),position);
 }
}
// Called after native 4381F0 has written the resolved impact into action+1C.
pub unsafe fn reaction(module:usize,hit:usize,kind:i32){
 if !ACTIVE.load(Ordering::Acquire)||module==0||hit==0||!STATE.lock().is_ok_and(|s|s.active){return;}
 if ptr::read((hit+0x2c1)as*const u8)&8!=0{return;}
 let target=read_ptr(module+8);if target==0{return;}let modules=read_ptr(target+0x1b8);if modules==0{return;}let action=read_ptr(modules+8);if action==0{return;}
 let impact=ptr::read((action+0x1c)as*const u8);
 let local=CONTACT.with(std::cell::Cell::get)&&attack(read_i32(hit+0x40))&&read_i32(hit+0x44)==1;
 let resolution=RESOLUTION.with(|r|r.get().filter(|r|r.hit==hit&&r.target==target&&r.value.is_some()));
 let contact=if local&&resolution.is_some(){ENEMY_CONTACT.with(std::cell::Cell::take)}else{None};
 let expected=if contact.is_some_and(|c|c.medium==Some(0)&&c.speed>=15.)&&resolution.is_some_and(|r|!r.suppressed){crate::ram_stagger::prepare(target,BASE.load(Ordering::Acquire))}else{None};
 let mut s=match STATE.lock(){Ok(s)=>s,_=>return};
 // Any subsequent hit invalidates attribution to the queued Ram contact.
 s.pending.retain(|p|p.target!=target);
 if target==s.actor&&kind!=3&&impact>=4{s.active=false;s.interrupted=true;s.started=false;s.motion=Motion::default();s.pending.clear();}
 if local&&s.active&&target!=s.actor{
  if let (Some(r),Some(contact))=(resolution,contact){
   let response=if r.suppressed{EnemyResponse::Suppressed}else{enemy_response(kind,r.value.unwrap())};
   let incoming=s.motion.velocity;
   s.enemy_collision(contact,response,false);
   if let Some(expected)=expected{
    if s.pending.len()<8{s.pending.push(PendingStagger{target,handle:read_ptr(target+8)as u64,contact,incoming,expected,age:0.,consumed:false});}
   }
  }
 }
}
#[unsafe(naked)]
pub unsafe extern "system" fn motion_detour(){
 core::arch::naked_asm!(
  "pushfq", "sub rsp,0xc8",
  "mov [rsp+0x20],rax", "mov [rsp+0x28],rcx", "mov [rsp+0x30],rdx", "mov [rsp+0x38],r8",
  "mov [rsp+0x40],r9", "mov [rsp+0x48],r10", "mov [rsp+0x50],r11",
  "movdqu [rsp+0x60],xmm0", "movdqu [rsp+0x70],xmm1", "movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3", "movdqu [rsp+0xa0],xmm4", "movdqu [rsp+0xb0],xmm5",
  "mov rcx,rdi", "lea rdx,[rbp-0x80]", "mov r8,r14", "call {callback}",
  "mov rax,[rsp+0x20]", "mov rcx,[rsp+0x28]", "mov rdx,[rsp+0x30]", "mov r8,[rsp+0x38]",
  "mov r9,[rsp+0x40]", "mov r10,[rsp+0x48]", "mov r11,[rsp+0x50]",
  "movdqu xmm0,[rsp+0x60]", "movdqu xmm1,[rsp+0x70]", "movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]", "movdqu xmm4,[rsp+0xa0]", "movdqu xmm5,[rsp+0xb0]",
  "add rsp,0xc8", "popfq", "jmp qword ptr [rip+{original}]",
  callback=sym velocity,original=sym MOTION_ORIGINAL
 );
}

thread_local!{static CONTACT:std::cell::Cell<bool>=const{std::cell::Cell::new(false)};}
pub struct ContactScope(bool,Option<EnemyContact>,Option<Resolution>);
impl ContactScope{pub fn enter(local:bool)->Self{Self(CONTACT.with(|s|s.replace(local)),ENEMY_CONTACT.with(std::cell::Cell::take),RESOLUTION.with(std::cell::Cell::take))}}
impl Drop for ContactScope{fn drop(&mut self){CONTACT.with(|s|s.set(self.0));ENEMY_CONTACT.with(|s|s.set(self.1));RESOLUTION.with(|s|s.set(self.2));}}
pub unsafe fn local_bullet(bullet:usize)->bool{STATE.lock().is_ok_and(|s|s.handle!=0&&s.handle==read_ptr(bullet+0xb0)as u64)}

#[cfg(test)]#[path="lightning_ram_native_tests.rs"]mod native_tests;

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn outward_wall_confirmation_follows_the_plane_not_a_distant_parallel_obstacle(){
  let n=V(0.,0.5,0.8660254);let low=Contact{normal:n,point:V::default(),fraction:0.5,body:1};
  let along=V(0.,0.5,-0.5*n.1/n.2);let upper=Contact{point:along,..low};
  assert!(same_wall_surface(&low,&upper));
  assert!(!same_wall_surface(&low,&Contact{point:along.add(n.scale(0.2)),..upper}));
  assert!(!same_wall_surface(&low,&Contact{normal:V(0.,1.,0.),..upper}));
  assert!((wall_extension(V(0.,0.,-20.),1./60.,&low,0.95,0.25)-0.099744).abs()<0.001);
  for y in [-0.8,-0.5,0.,0.5,0.54]{assert!(crate::ram_motion::wall_normal(V(0.,y,(1f32-y*y).sqrt())));}
  for n in [V(0.,-1.,0.),V(0.,1.,0.),V(0.,0.6,0.8)]{assert!(!crate::ram_motion::wall_normal(n));}
 }
 #[test]fn support_probe_does_not_treat_a_nearby_floor_or_cliff_side_as_landed(){
  let n=V(0.,1.,0.);
  assert!(support_contact(n,1./3.,true,false));
  assert!(!support_contact(n,0.65,true,false));
  assert!(!support_contact(n,0.65,false,false));
  assert!(!support_contact(V(0.9,0.4,0.).unit(),0.5,false,true));
 }
 #[test]fn contact_speed_tiers_and_native_override_ids(){
  for (speed,level,id) in [(6.,3,5),(9.999,3,5),(10.,7,6),(14.999,7,6),(15.,10,11),(30.,10,11)]{
   let (actual,l,o)=contact_tier(speed*speed*1.5).unwrap();assert!((actual-speed).abs()<0.001);assert_eq!((l,o),(level,id));
  }
  for damage in [0.,-1.,5.99*5.99*1.5,f32::NAN,f32::INFINITY]{assert!(contact_tier(damage).is_none());}
 }
 #[test]fn resolved_enemy_response_is_not_a_numeric_impact_threshold(){
  for level in [4,6,7,9,10]{assert_eq!(enemy_response(0,level),EnemyResponse::Yield);}
  for level in [1,2,3,5,8,11]{assert_eq!(enemy_response(0,level),EnemyResponse::Deflect);}
  for level in [0,12,255]{assert_eq!(enemy_response(0,level),EnemyResponse::Rebound);}
  for level in 0..12{assert_eq!(enemy_response(3,level),EnemyResponse::Rebound);assert_eq!(enemy_response(99999,level),enemy_response(0,level));}
 }
 #[test]fn collision_response_loses_energy_and_preserves_symmetric_glancing_motion(){
  for response in [EnemyResponse::Suppressed,EnemyResponse::Yield,EnemyResponse::Deflect,EnemyResponse::Rebound]{
   for speed in [6.,10.,15.,30.]{for side in [0.,0.3,0.7,1.]{
    let v=V(speed,0.,speed*side);let out=enemy_velocity(v,V(-1.,0.,0.),response);
    assert!(out.len()<=v.len()+0.0001);assert!(out.2>=0.);
    let mirror=enemy_velocity(V(v.0,0.,-v.2),V(-1.,0.,0.),response);
    assert_eq!(out,V(mirror.0,mirror.1,-mirror.2));
   }}
  }
  assert_eq!(enemy_velocity(V(20.,0.,0.),V(-1.,0.,0.),EnemyResponse::Yield),V(19.,0.,0.));
  assert_eq!(enemy_velocity(V(20.,0.,10.),V(-1.,0.,0.),EnemyResponse::Yield),V(19.,0.,10.));
  let glance=enemy_velocity(V(10.,0.,10.),V(-1.,0.,0.),EnemyResponse::Deflect);
  assert!(glance.sub(V(0.,0.,6.94444)).len()<0.001);
  for (angle,expected) in [(0f32,0.25),(15.,0.25),(30.,0.40556),(60.,0.85),(89.,0.85)]{
   assert!((slide_retention(angle.to_radians().cos(),1.)-expected).abs()<0.0001);
  }
  assert_eq!(enemy_velocity(V::default(),V(-1.,0.,0.),EnemyResponse::Rebound),V::default());
 }
 #[test]fn resistant_enemy_rebound_never_arms_wall_hp_cost(){
  for speed in [6.,9.99,10.,15.,30.]{
   let mut s=State{active:true,drive:true,motion:Motion{motor_work:0.,velocity:V(speed,0.,0.),..Default::default()},..Default::default()};
   s.enemy_collision(EnemyContact{speed,normal:V(-1.,0.,0.),medium:Some(0)},EnemyResponse::Rebound,true);
   assert_eq!(s.active,speed<15.);assert_eq!(s.drive,speed<15.);assert_eq!(s.ending,speed>=15.);assert!(!s.crash_pending&&!s.push);
   assert_eq!(s.crash,speed>=15.);assert_eq!(s.rebound>0.,speed>=15.);
  }
  for response in [EnemyResponse::Suppressed,EnemyResponse::Yield,EnemyResponse::Deflect]{
   let mut s=State{active:true,drive:true,motion:Motion{motor_work:0.,velocity:V(15.,0.,3.),..Default::default()},..Default::default()};
   s.enemy_collision(EnemyContact{speed:15.,normal:V(-1.,0.,0.),medium:Some(0)},response,true);
   let suppressed=response==EnemyResponse::Suppressed;
   assert_eq!(s.active,suppressed);assert_eq!(s.drive,suppressed);assert_eq!(s.crash,!suppressed);assert!(!s.push&&!s.crash_pending);
  }
 }
 #[test]fn pass_cost_depends_on_incidence_for_all_targets(){
  let velocity=V(20.,0.,10.);
  for medium in [Some(0),Some(2),None]{
   for (normal,expected) in [(V(-1.,0.,0.),V(19.,0.,10.)),(V(1.,0.,0.),velocity),(V::default(),velocity.scale(0.95))]{
    let mut s=State{motion:Motion{velocity,spin:velocity,..Default::default()},..Default::default()};
    s.enemy_collision(EnemyContact{speed:velocity.len(),normal,medium},EnemyResponse::Yield,false);
    assert!(s.motion.velocity.sub(expected).len()<1e-4);
    assert_eq!(s.motion.spin,velocity);
    assert!(s.deflection.is_none());
   }
  }
 }
 #[test]fn deflection_slides_and_accepts_side_or_outward_input_immediately(){
  let ground=V(0.,1.,0.);let forward=V(0.,0.,1.);let normal=forward.scale(-1.);
  for hz in [30.,60.,120.]{
   for side in [0.,8.]{
    for wish in [forward,V(1.,0.,0.),normal]{
     let velocity=V(side,0.,12.);
     let mut s=State{motion:Motion{velocity,spin:velocity,initialized:true,support:Some(ground),..Default::default()},..Default::default()};
     s.enemy_collision(EnemyContact{speed:velocity.len(),normal,medium:Some(0)},EnemyResponse::Deflect,false);
     assert_eq!(s.motion.velocity,V(side*slide_retention(12.,velocity.len()),0.,0.));
     assert!(s.motion.spin.len()<=s.motion.velocity.len()+1e-5);
     let mut pos=V::default();
     for _ in 0..(hz/10.) as usize{
      assert!(s.advance_roll(1./hz,true,wish,Some(ground),f32::INFINITY));
      pos=pos.add(s.motion.velocity.scale(1./hz));
      assert!(pos.2<1e-5);
     }
     if side>0.||wish.0>0.{assert!(pos.0>0.);}
     if side==0.&&wish==normal{assert!(pos.2<0.);}
     assert!(s.deflection.is_none());
     if side==0.&&wish==forward{
      assert!(s.advance_roll(1./hz,true,wish,Some(ground),f32::INFINITY));
      assert!(s.motion.velocity.2>0.);
     }
    }
   }
  }
 }
 #[test]fn medium_reaction_precheck_gates_extreme_rebound_but_not_damage(){
  for medium in [None,Some(0),Some(1),Some(2),Some(5),Some(10)]{
   for response in [EnemyResponse::Suppressed,EnemyResponse::Yield,EnemyResponse::Deflect,EnemyResponse::Rebound]{
    let mut s=State{active:true,drive:true,motion:Motion{motor_work:0.,velocity:V(20.,0.,0.),..Default::default()},..Default::default()};
    s.enemy_collision(EnemyContact{speed:20.,normal:V(-1.,0.,0.),medium},response,true);
    let bounce=medium==Some(0)&&response!=EnemyResponse::Suppressed;
    assert_eq!(s.crash,bounce);assert_eq!(s.active,!bounce);assert!(!s.push&&!s.crash_pending);
   }
  }
 }
 #[test]fn enemy_model_surface_sets_direction_without_vetoing_extreme_rebound(){unsafe{
  let mut hit=[0u8;0x240];let h=hit.as_mut_ptr()as usize;
  ptr::write_unaligned((h+0x1e0)as*mut[f32;4],[10.,2.,10.,0.]);
  ptr::write_unaligned((h+0x1f0)as*mut[f32;4],[10.,2.,10.5,0.]);
  for sign in [-1.,1.]{
   ptr::write_unaligned((h+0x200)as*mut[f32;4],[0.,0.,sign,0.]);
   let normal=model_contact_normal(h);assert_eq!(normal,V(0.,0.,-1.));
   for (v,seed,bounce)in [(V(30.,0.,16.),34.,true),(V(0.,0.,14.99),30.,true),(V(0.,0.,20.),14.,false),(V(0.,0.,15.),15.,true)]{
    let mut s=State{active:true,drive:true,motion:Motion{motor_work:0.,velocity:v,..Default::default()},..Default::default()};
    s.enemy_collision(EnemyContact{speed:seed,normal,medium:Some(0)},EnemyResponse::Rebound,true);
    assert_eq!(s.crash,bounce);assert_eq!(s.active,!bounce);assert!(!s.push&&!s.crash_pending);
    assert!(s.motion.velocity.len()<=v.len()+0.0001);
   }
  }
  // Native suppressed hits on a prone model remain non-blocking.
  let mut s=State{active:true,drive:true,motion:Motion{motor_work:0.,velocity:V(20.,0.,0.),..Default::default()},..Default::default()};
  s.enemy_collision(EnemyContact{speed:20.,normal:V(0.,1.,0.),medium:Some(0)},EnemyResponse::Suppressed,true);
  assert!(s.active&&!s.crash);assert_eq!(s.motion.velocity,V(20.,0.,0.));
  for normal in [[0.,0.,0.,0.],[f32::NAN,0.,0.,0.]]{
   ptr::write_unaligned((h+0x200)as*mut[f32;4],normal);assert_eq!(model_contact_normal(h),V::default());
  }
 }}
  #[test]fn hippo_extreme_hit_rebounds_without_a_second_speed_or_angle_gate(){
   // Captured 50110010 contact: emitted tier 10, level-2 preflight zero,
   // native ultimate stagger, but only 11.998669 m/s along the contact normal.
   let v=V(-14.900168,0.0004190615,5.336222);
   let normal=V(0.7386203,-0.6479381,-0.18605481);
   assert!(-v.dot(normal)<15.);
   for (velocity,n)in [(v,normal),(v.scale(0.1),normal),(v,V::default()),(v,normal.scale(-1.))]{
    for (medium,response,bounce)in [(Some(0),EnemyResponse::Rebound,true),(Some(0),EnemyResponse::Yield,true),(Some(2),EnemyResponse::Rebound,false),(None,EnemyResponse::Rebound,false),(Some(0),EnemyResponse::Suppressed,false)]{
     let mut s=State{active:true,drive:true,motion:Motion{velocity,..Default::default()},..Default::default()};
     s.enemy_collision(EnemyContact{speed:15.826884,normal:n,medium},response,true);
     assert_eq!(s.crash,bounce);assert_eq!(s.active,!bounce);assert!(!s.crash_pending);
     assert!(s.motion.velocity.finite());assert!(s.motion.velocity.len()<=if bounce{15.826984}else{velocity.len()+0.0001});
    }
   }
  }
  #[test]fn unconfirmed_ultimate_stagger_never_rebounds_even_if_level_two_immune(){
   for response in [EnemyResponse::Rebound,EnemyResponse::Deflect,EnemyResponse::Yield,EnemyResponse::Suppressed]{
    let mut s=State{active:true,drive:true,motion:Motion{motor_work:0.,velocity:V(20.,0.,0.),..Default::default()},..Default::default()};
    s.enemy_collision(EnemyContact{speed:20.,normal:V(-1.,0.,0.),medium:Some(0)},response,false);
    assert!(s.active&&!s.crash&&s.rebound==0.&&!s.crash_pending);
   }
  }
  #[test]fn confirmed_rebound_moves_outward_even_after_contact_has_stopped_the_owner(){
   for incoming in [V::default(),V(0.1,0.,0.),V(20.,0.,0.)]{
    for n in [V(-1.,0.,0.),V(-0.6,-0.8,0.)]{
     let v=rebound_velocity(incoming,n,20.);
     assert!(v.0<= -6.9&&v.1>2.4&&v.len()<=20.);
    }
   }
  }
  #[test]fn trailing_soft_target_hits_do_not_redirect_the_owner(){
  let mut s=State{active:true,drive:true,motion:Motion{motor_work:0.,velocity:V(15.,0.,0.),..Default::default()},..Default::default()};
  s.enemy_collision(EnemyContact{speed:15.,normal:V(1.,0.,0.),medium:Some(2)},EnemyResponse::Rebound,true);
  assert!(s.active&&s.drive&&!s.crash&&!s.push);assert_eq!(s.motion.velocity,V(15.,0.,0.));
 }
 #[test]fn contact_override_restores_fields_and_masks_nested_non_ram_attacks(){unsafe{
  let mut hit=[0xa5u8;0x50];let original=hit;let h=hit.as_mut_ptr()as usize;
  let outer=ContactScope::enter(true);
  {let _impact=impact_scope(h,123,15.*15.*1.5,V(-1.,0.,0.),Some(0));
   assert_eq!((hit[0x24],hit[0x1e]),(10,11));assert!(ENEMY_CONTACT.with(|s|s.get().is_some()));
   {let _ordinary=ContactScope::enter(false);assert!(!CONTACT.with(std::cell::Cell::get));assert!(ENEMY_CONTACT.with(|s|s.get().is_none()));assert!(RESOLUTION.with(|s|s.get().is_none()));}
   assert!(CONTACT.with(std::cell::Cell::get));
   {let _inner=impact_scope(h,124,6.*6.*1.5,V(0.,0.,-1.),Some(0));assert_eq!((hit[0x24],hit[0x1e]),(3,5));assert_eq!(RESOLUTION.with(|s|s.get().unwrap().target),124);}
   assert_eq!(RESOLUTION.with(|s|s.get().unwrap().target),123);
   assert_eq!((hit[0x24],hit[0x1e]),(10,11));
   assert_eq!(ENEMY_CONTACT.with(std::cell::Cell::take).unwrap().speed,15.);
   assert!(ENEMY_CONTACT.with(std::cell::Cell::take).is_none());
  }
  assert_eq!(hit,original);assert!(ENEMY_CONTACT.with(|s|s.get().is_none()));assert!(RESOLUTION.with(|s|s.get().is_none()));drop(outer);
 }}
 #[test]fn breakable_assets_require_native_identity_valid_hp_and_sufficient_object_power(){unsafe{
  let mut entry=[0usize;8];let mut asset=[0usize;0x1a0/8];let mut row=[0u8;0x50];
  let e=entry.as_mut_ptr()as usize;let a=asset.as_mut_ptr()as usize;entry[2]=a;asset[4]=row.as_ptr()as usize;
  for kind in [0u16,1,3,5,6]{ptr::write_unaligned((e+8)as*mut u16,kind);row[0x10]=1;assert_eq!(breakable_entry(e,||20),kind==5);}
  ptr::write_unaligned((e+8)as*mut u16,5);
  for hp in [-1i16,0,1,100]{row[0x10..0x12].copy_from_slice(&hp.to_le_bytes());assert_eq!(breakable_entry(e,||20),hp>0);}
  for defense in [0u16,10,19,20,21,90]{row[0x12..0x14].copy_from_slice(&defense.to_le_bytes());assert_eq!(breakable_entry(e,||20),defense<20);}
  row[0x12..0x14].copy_from_slice(&0u16.to_le_bytes());assert!(!breakable_entry(e,||0));
  row[0x3c]=0x40;assert!(!breakable_entry(e,||20));row[0x3c]=0;
  ptr::write((a+0x190)as*mut u8,1);assert!(!breakable_entry(e,||20));ptr::write((a+0x190)as*mut u8,0);
  asset[4]=0;assert!(!breakable_entry(e,||20));assert!(!breakable_entry(0,||20));
 }}
 #[test]fn predictive_contact_is_short_horizontal_and_bounded(){
  assert_eq!(leading_offset(V(0.,-50.,20.),1./60.),V(0.,0.,20./60.));
  assert_eq!(leading_offset(V(0.,0.,40.),0.2),V(0.,0.,0.5));
  assert_eq!(leading_offset(V::default(),0.02),V::default());
 }
 #[test]fn origin_translation_preserves_speed_and_all_position_histories(){
  for shift in [V(0.,0.,-32.),V(-24.,0.,-32.),V(0.,8.,0.)]{
   let v=V(8.,-1.,14.);let from=V(31.2,59.7,32.3);let dt=0.0234;
   let mut s=State{active:true,drive:true,started:true,trail:Some(from),generation:7,effective_speed:v.len(),motion:Motion{motor_work:0.,velocity:v,locked:true,..Default::default()},..Default::default()};
   s.travel.submit(from,v,dt);s.rebase(shift);
   let resolved=s.travel.resolve(from.add(v.scale(dt)).add(shift)).unwrap();
   assert!((resolved.velocity.len()-v.len()).abs()<0.001);assert!(!resolved.stopped);
   assert_eq!(s.trail,Some(from.add(shift)));assert_eq!(s.motion.velocity,v);assert!(s.motion.locked&&s.active&&s.drive);assert_eq!(s.generation,7);
   s.travel.submit(from.add(shift),v,dt);let blocked=s.travel.resolve(from.add(shift)).unwrap();assert_eq!(blocked.velocity,V::default());
  }
 }
 #[test]fn controller_turn_angle_sign_and_rotated_world_heading(){
  let left=steering_wish(0.,90.);let right=steering_wish(0.,-90.);
  assert!(left.0< -0.99&&right.0>0.99);
  let turned=steering_wish(std::f32::consts::FRAC_PI_2,-90.);assert!(turned.2< -0.99);
 }
 #[test]fn action_handoff_watchdog_counts_missing_updates_not_animation_seconds(){
  let mut s=State::default();
  for _ in 0..100{s.queried=true;assert!(!s.query_expired());}
  assert!(!s.query_expired());s.queried=true;assert!(!s.query_expired());
  assert!(!s.query_expired());assert!(s.query_expired());
 }
 #[test]fn head_on_crash_threshold_and_cost_are_once_per_roll(){
  for speed in [2.,5.999,6.,14.999,15.,20.]{let mut s=State{active:true,drive:true,..Default::default()};s.wall_stop(speed);
   assert!(!s.active&&!s.drive&&s.ending);assert_eq!(s.motion.velocity,V::default());
   assert_eq!(s.push,(6. ..15.).contains(&speed));assert_eq!(s.crash,speed>=15.);assert_eq!(std::mem::take(&mut s.crash_pending),speed>=15.);assert!(!s.crash_pending);
  }
  assert_eq!(crash_loss(300),75);assert_eq!(crash_loss(306),77);assert_eq!(crash_loss(0),0);assert_eq!(crash_loss(i32::MAX),536870912);
 }
 #[test]fn displayed_roll_rate_tracks_observed_travel_and_stops_without_sticky_speed(){
  let mut speed=0.;for _ in 0..120{speed=visual_speed(speed,10./60.,1./60.);}
  assert!((speed-10.).abs()<0.001);assert!((roll_rate(speed)-10./ROLL_SPEED).abs()<0.001);
  for _ in 0..120{speed=visual_speed(speed,0.,1./60.);}assert!(speed<0.001);
  assert_eq!(visual_speed(5.,1.,0.),5.);assert_eq!(visual_speed(5.,f32::NAN,0.01),5.);
  assert_eq!(roll_rate(f32::NAN),1.);assert_eq!(roll_rate(1000.),8.);
 }
}

#[cfg(test)]#[path="ram_cost_tests.rs"]mod cost_tests;
