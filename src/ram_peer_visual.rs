//! Observer-only Ram animation speed; never simulate a peer's movement or hits.
use crate::{has_effect,native_actor,read_ptr,ram_motion::V};
use std::{collections::VecDeque,sync::Mutex,time::Instant};

const WINDOW:f32=0.20;
const NATIVE_SPEED:f32=3.7/(22./30.);
const STALE:f32=0.25;
pub(super) const ROLL:&[u8]=b"a713_040010_hkx_AutoSet_00\0";
pub(super) const ROLL_ALT:&[u8]=b"a713_040010_hkx_AutoSet_01\0";
const START:&[u8]=b"a713_040000_hkx_AutoSet_00\0";
const START_ALT:&[u8]=b"a713_040000_hkx_AutoSet_01\0";
const ACTION:&[u8]=b"SwordArtsOneshotComboEnd_CMSG\0";
fn phase(mut active:impl FnMut(&[u8])->bool)->bool{active(ACTION)&&(active(ROLL)||active(ROLL_ALT))&&!active(START)&&!active(START_ALT)}
fn allowed(owned:bool,fast:bool,reset:bool,alive:bool)->bool{owned&&fast&&!reset&&alive}

#[derive(Default)]struct Travel{samples:VecDeque<(f64,V)>,time:f64,idle:f32,speed:f32}
impl Travel{
 fn clear(&mut self){*self=Self::default();}
 fn seed(&mut self,p:V){self.clear();self.samples.push_back((0.,p));self.speed=NATIVE_SPEED;}
 fn step(&mut self,p:V,dt:f32)->f32{
  if !p.finite()||!dt.is_finite()||dt<=0.||dt>STALE{self.clear();return 1.;}
  let Some(&(_,previous))=self.samples.back()else{self.seed(p);return 1.;};
  self.idle+=dt;
  let distance=p.sub(previous).len();
  // Permit sparse network updates, but never turn a teleport into spin speed.
  if distance>crate::ram_motion::SAFETY_SPEED*self.idle+1.{self.seed(p);return 1.;}
  if distance>0.001{self.idle=0.;}
  self.time+=f64::from(dt);self.samples.push_back((self.time,p));
  while self.samples.len()>2&&self.time-self.samples[1].0>=f64::from(WINDOW){self.samples.pop_front();}
  if self.samples.len()>128{self.seed(p);return 1.;}
  let (then,origin)=self.samples[0];let elapsed=(self.time-then)as f32;
  if elapsed<WINDOW{return 1.;}
  let measured=p.sub(origin).len()/elapsed;
  if !measured.is_finite()||measured>crate::ram_motion::SAFETY_SPEED*1.5{self.seed(p);return 1.;}
  self.speed+=(measured.min(crate::ram_motion::SAFETY_SPEED)-self.speed)*(1.-(-dt/0.10).exp());
  // FAST is the sender's existing >=6m/s marker. Below that the native
  // animation remains intact, including recovery; never slow a peer's stand-up.
  (self.speed/NATIVE_SPEED).clamp(1.,8.)
 }
 fn rebase(&mut self,delta:V){if delta.finite(){for (_,p)in &mut self.samples{*p=p.add(delta);}}else{self.clear();}}
}
#[derive(Clone,Copy,Debug,PartialEq)]struct Identity{actor:usize,handle:u64,context:usize,graph:usize}
struct Track{id:Identity,travel:Travel,rate:f32,seen:Instant}
struct State{scene:usize,tracks:Vec<Track>}
static STATE:Mutex<State>=Mutex::new(State{scene:0,tracks:Vec::new()});
impl State{
 fn observe(&mut self,scene:usize,snapshots:Vec<(Identity,V)>,dt:f32,now:Instant){
  if self.scene!=scene{self.scene=scene;self.tracks.clear();}
  self.tracks.retain(|t|snapshots.iter().any(|(id,_)|*id==t.id));
  for (id,position)in snapshots{
   if let Some(t)=self.tracks.iter_mut().find(|t|t.id==id){t.rate=t.travel.step(position,dt);t.seen=now;}
   else{let mut travel=Travel::default();let rate=travel.step(position,dt);self.tracks.push(Track{id,travel,rate,seen:now});}
  }
 }
 fn rate(&self,scene:usize,context:usize,now:Instant)->Option<(Identity,f32)>{
  if self.scene!=scene{return None;}
  self.tracks.iter().find(|t|t.id.context==context&&now.saturating_duration_since(t.seen).as_secs_f32()<=STALE).map(|t|(t.id,t.rate))
 }
}

unsafe fn owner_ready(actor:usize,base:usize)->bool{
 let stats=native_actor::stats(actor);
 stats!=0&&allowed(has_effect(actor,crate::lightning_ram::OWNER,base),has_effect(actor,crate::lightning_ram::FAST,base),
  has_effect(actor,crate::reward_cycle_runtime::RESET,base),!native_actor::incapacitated(actor,stats))
}
pub(super) unsafe fn rolling(graph:usize,base:usize)->bool{
 let query:unsafe extern "system" fn(usize,*const u8)->u8=std::mem::transmute(base+crate::ram_stagger::NODE_RVA);
 phase(|name|query(graph,name.as_ptr())!=0)
}
pub unsafe fn update(local:usize,base:usize,dt:f32){
 let world=read_ptr(base+crate::WORLD_RVA);let scene=read_ptr(base+0x3c131c8);
 let mut snapshots=Vec::with_capacity(2);
 if world!=0&&local!=0&&dt.is_finite()&&dt>0.&&dt<=STALE{
  let get:unsafe extern "system" fn(usize,i32)->usize=std::mem::transmute(base+0x5197e0);
  for slot in 1..=3{
   let actor=get(world,slot);if actor==0||actor==local||!owner_ready(actor,base)||!crate::spatial_ready(actor){continue;}
   let Some(graph)=crate::ram_stagger::graph(actor,base)else{continue;};
   if !rolling(graph,base){continue;}
   let animation=read_ptr(read_ptr(actor+0x1b8)+0x28);
   let id=Identity{actor,handle:read_ptr(actor+8)as u64,context:animation+0x1800,graph};
   if snapshots.iter().any(|(other,_)|*other==id){continue;}
   snapshots.push((id,V::from(crate::position(actor,base))));
  }
 }
 // Collect live native data before taking the lock. Saved identities are
 // comparison keys only; no saved actor pointer is dereferenced on update.
 if let Ok(mut s)=STATE.lock(){s.observe(scene,snapshots,dt,Instant::now());}
}
pub unsafe fn speed(context:usize,base:usize)->Option<f32>{
 if context==0{return None;}let world=read_ptr(base+crate::WORLD_RVA);if world==0{return None;}
 let scene=read_ptr(base+0x3c131c8);
 let (id,rate)=STATE.lock().ok()?.rate(scene,context,Instant::now())?;
 // Only inspect a live controller already matched by the game-thread roster.
 let actor=read_ptr(context+8);if actor==0||actor!=id.actor||actor==read_ptr(world+0x174e8)||read_ptr(actor+8)as u64!=id.handle{return None;}
 // The graph can change after our update but before this getter. Check it
 // again so damage, windup, recovery, ownership loss never use a cached rate.
 if !owner_ready(actor,base)||crate::ram_stagger::graph(actor,base)!=Some(id.graph)||!rolling(id.graph,base){return None;}
 Some(rate)
}
pub fn rebase(actor:usize,handle:u64,delta:V){if let Ok(mut s)=STATE.lock(){for t in &mut s.tracks{if t.id.actor==actor&&t.id.handle==handle{t.travel.rebase(delta);}}}}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn only_fast_owned_roll_phase_not_start_damage_or_recovery(){
  for owned in [false,true]{for fast in [false,true]{for reset in [false,true]{for alive in [false,true]{assert_eq!(allowed(owned,fast,reset,alive),owned&&fast&&!reset&&alive);}}}}
  for clip in [ROLL, ROLL_ALT] {
   assert!(phase(|n|[ACTION,clip].contains(&n)));
   for start in [START, START_ALT] { assert!(!phase(|n|[ACTION,clip,start].contains(&n))); }
  }
  for nodes in [vec![START],vec![ROLL],vec![ACTION],vec![ACTION,ROLL,START],vec![]]{assert!(!phase(|n|nodes.contains(&n)));}
 }
 #[test]fn smooth_stream_and_packet_bursts_converge_without_sawtooth(){
  for hz in [30.,60.,144.]{for packet in [0.,0.05,0.1]{
   let mut t=Travel::default();let mut rates=Vec::new();
   for i in 0..(hz*3.)as usize{let time=i as f32/hz;let sample=if packet>0.{(time/packet).floor()*packet}else{time};let r=t.step(V(sample*16.,0.,0.),1./hz);if time>2.{rates.push(r);}}
   let average=rates.iter().sum::<f32>()/rates.len()as f32;assert!((average-16./NATIVE_SPEED).abs()<0.12,"{hz} {packet} {average}");
   let spread=rates.iter().copied().fold(f32::NEG_INFINITY,f32::max)-rates.iter().copied().fold(f32::INFINITY,f32::min);assert!(spread<1.,"{hz} {packet} {spread}");
  }}
 }
 #[test]fn teleport_bad_delta_and_long_gap_reset_without_spike(){
  let mut t=Travel::default();for i in 0..120{t.step(V(i as f32/60.*16.,0.,0.),1./60.);}
  assert_eq!(t.step(V(1000.,0.,0.),1./60.),1.);
  for dt in [0.,-1.,f32::NAN,1.]{assert_eq!(t.step(V(1000.,0.,0.),dt),1.);assert!(t.samples.is_empty());}
  assert_eq!(t.step(V(f32::NAN,0.,0.),1./60.),1.);
 }
 #[test]fn rebase_preserves_speed_and_history_in_same_coordinate_frame(){
  let mut a=Travel::default();let mut b=Travel::default();
  for i in 0..120{let p=V(i as f32/60.*16.,0.,0.);a.step(p,1./60.);b.step(p,1./60.);}
  let delta=V(100.,-30.,200.);b.rebase(delta);
  for i in 120..180{let p=V(i as f32/60.*16.,0.,0.);assert!((a.step(p,1./60.)-b.step(p.add(delta),1./60.)).abs()<0.001);}
 }
 #[test]fn stopping_restores_native_rate_and_retains_bounded_history(){
  let mut t=Travel::default();for i in 0..120{t.step(V(i as f32/60.*16.,0.,0.),1./60.);}
  for _ in 0..120{t.step(V(32.,0.,0.),1./60.);}assert_eq!(t.step(V(32.,0.,0.),1./60.),1.);assert!(t.samples.len()<=(WINDOW*60.).ceil()as usize+2);
 }
 #[test]fn leaving_recovery_scene_change_and_reused_actor_discard_old_speed(){
  let mut s=State{scene:0,tracks:Vec::new()};let now=Instant::now();let a=Identity{actor:1,handle:2,context:3,graph:4};
  for i in 0..120{s.observe(1,vec![(a,V(i as f32/60.*16.,0.,0.))],1./60.,now);}
  assert!(s.rate(1,3,now).unwrap().1>3.);assert!(s.rate(1,9,now).is_none());assert!(s.rate(2,3,now).is_none());
  assert!(s.rate(1,3,now+std::time::Duration::from_secs(1)).is_none());
  for b in [Identity{handle:5,..a},Identity{graph:5,..a},Identity{actor:5,..a}]{s.observe(1,vec![(b,V(0.,0.,0.))],1./60.,now);assert_eq!(s.rate(1,3,now).unwrap(),(b,1.));}
  s.observe(1,vec![],1./60.,now);assert!(s.rate(1,3,now).is_none());assert!(s.tracks.is_empty());
  s.observe(2,vec![(a,V(0.,0.,0.))],1./60.,now);assert_eq!(s.rate(2,3,now).unwrap().1,1.);
 }
}
