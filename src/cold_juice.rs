//! Flask-time frost and two-hand native enchantment; replicated moving storm.
use crate::{ptr,read_ptr,read_i32,has_effect,native_actor,ACTIVE,BASE};
use std::{cell::Cell,collections::BTreeMap,sync::{Mutex,atomic::Ordering}};
pub const OWNER:i32=95005700;
pub const FROST:i32=OWNER+1;
pub const RIGHT:i32=OWNER+2;
pub const LEFT:i32=OWNER+4;
pub const BULLET:i32=OWNER+50;
pub const START_VISUAL:i32=OWNER+51;
pub const ATTACK:i32=OWNER+60;
type Point=[f32;4];
#[derive(Clone,Copy,Debug,PartialEq)]struct Seed{serial:u16,level:u8,life:f32,target:u64}
fn damage(level:u8)->f32{40.+40.*(level.clamp(1,15)-1)as f32/14.}
impl Seed{
 fn encode(self)->[f32;4]{let target=crate::soul_immunity::encode(self.target);let time=(self.life.clamp(0.,6.)*20.).round()as u32;[(u32::from(self.serial&4095)|(u32::from(self.level.clamp(1,15))<<12)|(time<<16))as f32,target[0],target[1],target[2]]}
 fn decode(v:[f32;4])->Option<Self>{
  if !v[0].is_finite()||v[0].fract()!=0.||!(0. ..8388608.).contains(&v[0]){return None;}
  let bits=v[0]as u32;let serial=(bits&4095)as u16;let level=((bits>>12)&15)as u8;let time=bits>>16;
  if serial==0||level==0||time==0||time>120{return None;}
  Some(Self{serial,level,life:time as f32/20.,target:if v[1..]==[0.,0.,1048576.]{0}else{crate::soul_immunity::decode([v[1],v[2],v[3]])?}})
 }
}
fn newer(next:Seed,old:Seed)->bool{let d=next.serial.wrapping_sub(old.serial)&4095;(d>0&&d<2048)||(d==0&&old.target==0&&next.target!=0)}
fn toward(from:Point,to:Point,dt:f32)->Point{
 let d:[f32;3]=std::array::from_fn(|i|to[i]-from[i]);let len=d.iter().map(|v|v*v).sum::<f32>().sqrt();
 if !len.is_finite()||!dt.is_finite()||dt<=0.{return [0.;4];}let rate=(3./len.max(0.0001)).min(1./dt);
 [d[0]*rate,d[1]*rate,d[2]*rate,0.]
}
#[derive(Clone,Copy)]struct Flight{handle:u64,owner:u64,seed:Seed,left:f32,valid:bool}
struct State{groups:BTreeMap<usize,(i32,Seed)>,flights:BTreeMap<usize,Flight>,latest:BTreeMap<u64,(Seed,usize,u64)>,serial:u16,gap:f32,actor:u64,scene:usize,pending:Option<u64>,time:f64,hits:BTreeMap<(u64,u16,u64),f64>}
static STATE:Mutex<State>=Mutex::new(State{groups:BTreeMap::new(),flights:BTreeMap::new(),latest:BTreeMap::new(),serial:0,gap:0.,actor:0,scene:0,pending:None,time:0.,hits:BTreeMap::new()});
thread_local!{static SEED:Cell<Option<Seed>>=const{Cell::new(None)};}
pub struct Guard(Option<Seed>,Option<(usize,[f32;4])>);
impl Drop for Guard{fn drop(&mut self){if let Some((p,v))=self.1{unsafe{ptr::write_unaligned(p as*mut[f32;4],v);}}SEED.with(|s|s.set(self.0));}}
pub unsafe fn factory(payload:usize)->Guard{
 let ours=ACTIVE.load(Ordering::Acquire)&&payload!=0&&read_i32(payload+4)==BULLET;
 let restore=ours.then(||{let p=payload+0x124;(p,ptr::read_unaligned(p as*const[f32;4]))});
 let seed=restore.and_then(|(_,v)|Seed::decode(v));
 if let Some((p,_))=restore{ptr::write_unaligned(p as*mut[f32;4],[1.;4]);}
 Guard(SEED.with(|s|s.replace(seed)),restore)
}
pub unsafe fn group(group:usize){if group!=0{if let Ok(mut s)=STATE.lock(){s.groups.remove(&group);if let Some(seed)=SEED.with(Cell::get){s.groups.insert(group,(read_i32(group),seed));}}}}
pub unsafe fn initialize(group:usize)->Guard{let seed=STATE.lock().ok().and_then(|s|s.groups.get(&group).filter(|(id,_)|group!=0&&*id==read_i32(group)).map(|(_,v)|*v));Guard(SEED.with(|s|s.replace(seed)),None)}
pub fn finish(group:usize){if let Ok(mut s)=STATE.lock(){s.groups.remove(&group);}}
pub unsafe fn bound(ins:usize){
 if ins==0||read_i32(ins+0xa4)!=BULLET{return;}let Some(seed)=SEED.with(Cell::get)else{return;};
 let handle=read_ptr(ins+8)as u64;let owner=read_ptr(ins+0xb0)as u64;
 if let Ok(mut s)=STATE.lock(){let valid=s.latest.get(&owner).is_none_or(|(old,_,_)|newer(seed,*old));
  if valid{s.latest.insert(owner,(seed,ins,handle));}s.flights.insert(ins,Flight{handle,owner,seed,left:seed.life,valid});
 }
}
pub fn retired(ins:usize){if let Ok(mut s)=STATE.lock(){s.flights.remove(&ins);}}
unsafe fn flight(ins:usize)->Option<Flight>{STATE.lock().ok()?.flights.get(&ins).copied().filter(|f|read_ptr(ins+8)as u64==f.handle&&read_i32(ins+0xa4)==BULLET)}
unsafe fn alive(actor:usize)->bool{let stats=if actor!=0{native_actor::stats(actor)}else{0};stats!=0&&!native_actor::incapacitated(actor,stats)}
unsafe fn valid(f:Flight,base:usize)->bool{
 let latest=STATE.lock().ok().and_then(|s|s.latest.get(&f.owner).copied());
 if !f.valid||f.left<=0.||!latest.is_some_and(|(_,_,h)|h==f.handle){return false;}
 let actor=crate::static_charge::resolve_actor(base,f.owner);
 alive(actor)&&has_effect(actor,OWNER,base)&&!has_effect(actor,crate::reward_cycle_runtime::RESET,base)
}
pub unsafe fn motion(ins:usize,dt:f32){
 if !dt.is_finite()||dt<=0.||dt>1.{return;}let Some(mut f)=flight(ins)else{return;};let base=BASE.load(Ordering::Acquire);
 f.left=(f.left-dt).max(0.);let mut velocity=[0.;4];
 if valid(f,base){
  let target=crate::static_charge::resolve_actor(base,if f.seed.target==0{f.owner}else{f.seed.target});
  if alive(target)&&crate::spatial_ready(target){let point=crate::position(target,base);
   if f.seed.target==0{ptr::write_unaligned((ins+0x10)as*mut Point,point);}
   else{velocity=toward(ptr::read_unaligned((ins+0x10)as*const Point),point,dt);}
  }
 }
 for off in [0x30,0x40]{ptr::write_unaligned((ins+off)as*mut Point,velocity);}
 if let Ok(mut s)=STATE.lock(){s.flights.insert(ins,f);}
}
pub unsafe fn after_move(ins:usize)->bool{
 if ins==0||read_i32(ins+0xa4)!=BULLET{return false;}let base=BASE.load(Ordering::Acquire);
 if flight(ins).is_some_and(|f|valid(f,base)){return false;}
 ptr::write((ins+0x583)as*mut u8,0);
 let retire:unsafe extern "system" fn(usize,i32)=std::mem::transmute(base+crate::sword_projectile::MODE_RVA);retire(ins,4);true
}
pub unsafe fn constructed(hit:usize,context:usize){
 if read_i32(context+8)!=ATTACK||read_i32(context+12)!=1{return;}
 let seed=flight(read_ptr(context)).map(|f|f.seed).or_else(||SEED.with(Cell::get));
 if let Some(seed)=seed{ptr::write_unaligned(hit as*mut[f32;5],[0.,damage(seed.level),0.,0.,0.]);}
}

// Preserve the 0.4 s per-enemy interval when follow becomes chase; the
// replacement native root must not create an extra immediate damage pulse.
pub unsafe fn contact(original:unsafe extern "system" fn(usize,usize,usize)->u8,hit:usize,target:usize,bullet:usize)->u8{
 let Some(f)=flight(bullet)else{return 0;};
 if target==0||!valid(f,BASE.load(Ordering::Acquire)){return 0;}
 let key=(f.owner,f.seed.serial,read_ptr(target+8)as u64);
 let previous={let Ok(mut s)=STATE.lock()else{return 0;};let now=s.time;if s.hits.get(&key).is_some_and(|t|now-*t<0.3999){return 0;}s.hits.insert(key,now)};
 let result=original(hit,target,bullet);
 if result==0{if let Ok(mut s)=STATE.lock(){if let Some(t)=previous{s.hits.insert(key,t);}else{s.hits.remove(&key);}}}result
}

#[repr(C,align(16))]struct Descriptor([u8;0x110]);
fn emission(seed:Seed,visual:bool)->(i32,[f32;4]){if visual{(START_VISUAL,[1.;4])}else{(BULLET,seed.encode())}}
unsafe fn emit(owner:u64,seed:Seed,point:Point,base:usize,visual:bool)->bool{
 let manager=read_ptr(base+0x3c0ae30);let original=crate::echo_cast::EMIT_ORIGINAL.load(Ordering::Acquire);if manager==0||original.is_null()||!point[..3].iter().all(|x|x.is_finite()){return false;}
 let mut d=Descriptor([0;0x110]);let p=d.0.as_mut_ptr()as usize;
 ptr::write_unaligned(p as*mut u64,owner);
 for off in [8,0xc,0x14,0x18,0x1c,0x24,0xe0,0xf0]{ptr::write_unaligned((p+off)as*mut i32,-1);}
 let (bullet,coefficients)=emission(seed,visual);
 ptr::write_unaligned((p+0x14)as*mut i32,bullet);
 for off in [0x28,0x2c]{ptr::write_unaligned((p+off)as*mut i32,0x10);}
 d.0[0xa8]=0xff;d.0[0x105]=0xff;
 ptr::write_unaligned((p+0x30)as*mut[f32;4],coefficients);ptr::write_unaligned((p+0x44)as*mut u32,15);
 ptr::write_unaligned((p+0x50)as*mut[Point;4],[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],point]);ptr::write_unaligned((p+0xb8)as*mut u64,u64::MAX);
 let mut out=-1i32;let mut error=-1i32;let emit:unsafe extern "system" fn(usize,usize,usize,usize)->usize=std::mem::transmute(original);
 emit(manager,&mut out as*mut i32 as usize,p,&mut error as*mut i32 as usize);out>=0
}
pub unsafe fn flask(actor:usize,base:usize){
 if !crate::is_local(actor,base)||!has_effect(actor,OWNER,base)||has_effect(actor,crate::reward_cycle_runtime::RESET,base)||!alive(actor)||!crate::spatial_ready(actor){return;}
 let owner=read_ptr(actor+8)as u64;
 let serial={let Ok(mut s)=STATE.lock()else{return;};if s.actor==owner&&s.gap>0.{return;}s.actor=owner;s.gap=0.2;s.pending=None;s.serial=(s.serial%4095)+1;s.serial};
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);
 // Native 825/827 wrappers select ONE casting hand via style 323/322.
 // Resolve each actual equipped hand before applying its native enchantment.
 if crate::bartender::can_enchant(actor,0.,base){apply(actor,RIGHT+1,0);}
 if crate::bartender::can_enchant(actor,1.,base){apply(actor,LEFT+1,0);}
 if !crate::has_state(actor,260,base){apply(actor,FROST,0);}
 let game=read_ptr(base+crate::GAME_DATA_RVA);let data=if game!=0{read_ptr(game+8)}else{0};let level=if data!=0{read_i32(data+0x68).clamp(1,15)as u8}else{1};
 let seed=Seed{serial,level,life:3.,target:0};let point=crate::position(actor,base);
 // Once per flask, not when the persistent storm is replaced for pursuit.
 // The separate native visual has no attack, frost payload or child chain.
 if emit(owner,seed,point,base,false){emit(owner,seed,point,base,true);}
}
pub unsafe fn outgoing(module:usize,source:usize,hit:usize){
 if !ACTIVE.load(Ordering::Acquire)||module==0||source==0||hit==0{return;}let base=BASE.load(Ordering::Acquire);
 if !crate::is_local(source,base)||!has_effect(source,OWNER,base)||read_i32(hit+0x280)<=0{return;}
 if crate::weapon_contact::at_contact(source,hit,base).is_none(){return;}
 let target=read_ptr(module+8);if target==0||target==source||crate::ambulance::party_member(target,base){return;}
 let owner=read_ptr(source+8)as u64;
 if let Ok(mut s)=STATE.lock(){if s.pending.is_none()&&s.latest.get(&owner).and_then(|(_,i,_)|s.flights.get(i)).is_some_and(|f|f.left>0.&&f.seed.target==0){s.pending=Some(read_ptr(target+8)as u64);}}
}
pub unsafe fn update(actor:usize,base:usize,dt:f32){
 let scene=read_ptr(base+0x3c131c8);let owner=read_ptr(actor+8)as u64;
 let pending={let Ok(mut s)=STATE.lock()else{return;};
  if s.scene!=scene{s.scene=scene;s.latest.clear();s.groups.clear();s.flights.clear();s.pending=None;s.gap=0.;s.hits.clear();}
  s.time+=f64::from(dt);let now=s.time;s.hits.retain(|_,t|now-*t<1.);
  s.gap=(s.gap-dt).max(0.);
  if s.actor!=owner||!alive(actor)||!has_effect(actor,OWNER,base)||has_effect(actor,crate::reward_cycle_runtime::RESET,base){s.pending=None;return;}
  s.pending.take().and_then(|target|s.latest.get(&owner).and_then(|(_,i,_)|s.flights.get(i).map(|f|(*i,*f,target))))
 };
 if let Some((ins,f,target))=pending{if f.left>0.&&f.seed.target==0&&read_ptr(ins+8)as u64==f.handle{
  let point=ptr::read_unaligned((ins+0x10)as*const Point);emit(owner,Seed{life:3.,target,..f.seed},point,base,false);
 }}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn start_visual_uses_native_coefficients_without_storm_metadata(){
  let seed=Seed{serial:41,level:15,life:3.,target:0};
  assert_eq!(emission(seed,true),(START_VISUAL,[1.;4]));assert!(Seed::decode(emission(seed,true).1).is_none());
  assert_eq!(emission(seed,false),(BULLET,seed.encode()));
  let chase=Seed{target:0xffff_ffff_1000_1234,..seed};assert_eq!(emission(chase,false),(BULLET,chase.encode()));
 }
 #[test]fn wire_metadata_preserves_handles_level_duration_and_generation(){for serial in [1,2048,4095]{for level in 1..=15{for life in [0.05,3.,3.25,6.]{for target in [0,0xffff_ffff_1000_1234,u64::MAX-1]{let s=Seed{serial,level,life,target};let v=s.encode();assert!(v.iter().all(|v|v.is_finite()));assert_eq!(Seed::decode(v),Some(s));}}}}assert!(Seed::decode([f32::NAN;4]).is_none());assert!(Seed::decode([1.;4]).is_none());}
 #[test]fn phases_cannot_rearm_or_replace_newer_flasks(){let a=Seed{serial:4095,level:1,life:3.,target:0};let b=Seed{target:17,life:5.,..a};assert!(newer(b,a));assert!(!newer(a,b));assert!(!newer(b,b));assert!(newer(Seed{serial:1,..a},b));assert!(!newer(a,Seed{serial:1,..a}));}
 #[test]fn wire_scope_rebuilds_fresh_hits_and_restores_native_coefficients(){unsafe{
  let seed=Seed{serial:99,level:15,life:3.,target:0};let mut payload=[0u8;0x220];let pp=payload.as_mut_ptr()as usize;
  ptr::write_unaligned((pp+4)as*mut i32,BULLET);ptr::write_unaligned((pp+0x124)as*mut[f32;4],seed.encode());
  let previous=ACTIVE.swap(true,Ordering::AcqRel);
  let mut group_mem=[17i32,0];let g=group_mem.as_mut_ptr()as usize;
  {let _guard=factory(pp);assert_eq!(ptr::read_unaligned((pp+0x124)as*const[f32;4]),[1.;4]);group(g);}
  assert_eq!(ptr::read_unaligned((pp+0x124)as*const[f32;4]),seed.encode());
  let mut context=[0usize;0x120/8];let cp=context.as_mut_ptr()as usize;ptr::write_unaligned((cp+8)as*mut i32,ATTACK);ptr::write_unaligned((cp+12)as*mut i32,1);
  let mut hit=[0xabu8;0x300];let hp=hit.as_mut_ptr()as usize;let before=hit;
  {let _guard=initialize(g);constructed(hp,cp);}
  assert_eq!(ptr::read_unaligned(hp as*const[f32;5]),[0.,80.,0.,0.,0.]);assert_eq!(&hit[20..],&before[20..]);
  finish(g);hit=before;{let _guard=initialize(g);constructed(hp,cp);}assert_eq!(hit,before);
  ACTIVE.store(previous,Ordering::Release);
 }}
 #[test]fn tracking_speed_is_bounded_and_does_not_overshoot(){assert_eq!(toward([0.;4],[10.,0.,0.,1.],0.1),[3.,0.,0.,0.]);assert_eq!(toward([0.;4],[0.1,0.,0.,1.],0.1),[1.,0.,0.,0.]);assert_eq!(toward([0.;4],[0.;4],0.1),[0.;4]);assert_eq!(damage(1),40.);assert_eq!(damage(15),80.);}
}
