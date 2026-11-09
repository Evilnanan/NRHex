//! One shockwave at the primary hit of a critical, using accumulated final damage.
use crate::{has_effect,ptr,read_ptr,read_i32,ACTIVE,BASE};
use std::{cell::RefCell,sync::{Mutex,atomic::Ordering}};
mod data {include!(concat!(env!("OUT_DIR"),"/soul_critical_data.rs"));}
pub const OWNER:i32=95005200;
pub const BULLET:i32=OWNER+50;
pub const PRELUDE:i32=OWNER+51;
pub const ATTACK:i32=OWNER+60;
pub const TARGET_RVA:usize=0x4fcb40;
pub const TARGET_PATTERN:&str="40 53 48 83 EC 20 48 8B D9 48 8B 0D 58 25 71 03 48 85 C9";
#[derive(Clone,Copy,Debug,PartialEq,Eq)]struct Entry{owner:u64,animation:i32,throw:usize}
#[derive(Default)]struct Prelude{current:Option<Entry>,played:bool}
impl Prelude{
 fn observe(&mut self,current:Option<Entry>)->bool{
  if self.current!=current{self.current=current;self.played=false;}current.is_some()&&!self.played
 }
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]struct Identity{owner:u64,target:u64,animation:i32,throw:usize}
#[derive(Default)]struct Critical{identity:Option<Identity>,sum:f64,fired:bool}
impl Critical{
 fn credit(&mut self,id:Identity,damage:i32,main:bool)->Option<f32>{
  if damage<=0{return None;}
  if self.identity.is_some_and(|old|old.owner==id.owner&&old.animation==id.animation&&old.throw==id.throw&&old.target!=id.target){return None;}
  if self.identity!=Some(id){*self=Self{identity:Some(id),..Self::default()};}
  if self.fired{return None;}self.sum=(self.sum+f64::from(damage)).min(f64::from(f32::MAX));
  if !main{return None;}self.fired=true;Some((400.+self.sum*0.5).min(f64::from(f32::MAX))as f32)
 }
}
#[derive(Clone,Copy)]struct Wave{owner:u64,target:u64,damage:f32,point:[f32;4]}
struct State{critical:Critical,prelude:Prelude,pending:Vec<Wave>,scene:usize}
static STATE:Mutex<State>=Mutex::new(State{critical:Critical{identity:None,sum:0.,fired:false},prelude:Prelude{current:None,played:false},pending:Vec::new(),scene:0});
pub fn reset(){if let Ok(mut s)=STATE.lock(){s.critical=Critical::default();s.prelude=Prelude::default();s.pending.clear();}}
pub fn components(total:f32)->[f32;5]{[total*0.625,total*0.375,0.,0.,0.]}
thread_local!{static DISPATCH:RefCell<Vec<(usize,usize,bool)>>=const{RefCell::new(Vec::new())};}
pub struct Guard;
impl Drop for Guard{fn drop(&mut self){DISPATCH.with(|s|{s.borrow_mut().pop();});}}
pub fn scope(module:usize,hit:usize)->Guard{DISPATCH.with(|s|s.borrow_mut().push((module,hit,false)));Guard}
fn first(module:usize,hit:usize)->bool{DISPATCH.with(|s|{let mut s=s.borrow_mut();let Some(v)=s.last_mut().filter(|v|v.0==module&&v.1==hit)else{return false;};if v.2{return false;}v.2=true;true})}
unsafe fn current(actor:usize,base:usize)->Option<(i32,usize)>{
 let modules=read_ptr(actor+0x1b8);if modules==0{return None;}let module=read_ptr(modules+0x88);if module==0{return None;}
 let get:unsafe extern "system" fn(usize)->i32=std::mem::transmute(base+crate::THROW_ANIM_RVA);let anim=get(module);
 if !(31700..31800).contains(&anim){return None;}let throw=read_ptr(module+0x10);(throw!=0).then_some((anim,throw))
}
pub unsafe fn outgoing(module:usize,source:usize,hit:usize){
 if !ACTIVE.load(Ordering::Acquire)||module==0||source==0||hit==0{return;}
 let base=BASE.load(Ordering::Acquire);
 if !crate::is_local(source,base)||!has_effect(source,OWNER,base)||read_i32(hit+0x44)!=1{return;}
 let target=read_ptr(module+8);if target==0||target==source{return;}
 let Some((animation,throw))=current(source,base)else{return;};
 let attack=read_i32(hit+0x40);
 let Ok(index)=data::HITS.binary_search_by_key(&(attack,animation),|r|(r.0,r.1))else{return;};
 if !first(module,hit){return;}
 let damage=read_i32(hit+0x280);if damage<=0{return;}
 let point=if crate::spatial_ready(target){crate::position(target,base)}else{ptr::read_unaligned((hit+0x1f0)as*const[f32;4])};
 if !point[..3].iter().all(|v|v.is_finite()){return;}
 let id=Identity{owner:read_ptr(source+8)as u64,target:read_ptr(target+8)as u64,animation,throw};
 let wave=STATE.lock().ok().and_then(|mut s|s.critical.credit(id,damage,data::HITS[index].2)).map(|damage|Wave{owner:id.owner,target:id.target,damage,point});
 // Create the real wave in the accepted main-hit callback. The earlier visual
 // has no child and cannot schedule damage when a critical is interrupted.
 if let Some(wave)=wave{if !crate::static_charge::emit_soul(wave.owner,wave.target,wave.damage,wave.point,base){if let Ok(mut s)=STATE.lock(){s.pending.push(wave);}}}
}
pub unsafe fn update(actor:usize,base:usize){
 let current=current(actor,base);let owner=read_ptr(actor+8)as u64;let scene=read_ptr(base+0x3c131c8);
 let owned=has_effect(actor,OWNER,base)&&!has_effect(actor,crate::reward_cycle_runtime::RESET,base);
 let (pending,begin)=if let Ok(mut s)=STATE.lock(){
  if s.scene!=scene{s.scene=scene;s.critical=Critical::default();s.prelude=Prelude::default();s.pending.clear();}
  if !owned{s.critical=Critical::default();s.prelude=Prelude::default();s.pending.clear();return;}
  if s.critical.identity.is_some_and(|id|id.owner!=owner||current!=Some((id.animation,id.throw))){s.critical=Critical::default();}
  let entry=current.map(|(animation,throw)|Entry{owner,animation,throw});
  let begin=s.prelude.observe(entry)&&!s.critical.fired;
  (std::mem::take(&mut s.pending),if begin{entry}else{None})
 }else{return;};
 if let Some(entry)=begin{
  // Native throw target getter resolves ChrThrowMan's second actor (+18),
  // independently of camera lock-on, before the first damage notification.
  let get:unsafe extern "system" fn(usize)->usize=std::mem::transmute(base+TARGET_RVA);
  let target=get(entry.throw);
  if target!=0&&target!=actor&&crate::spatial_ready(target){
   let point=crate::position(target,base);
   if point[..3].iter().all(|v|v.is_finite())&&crate::static_charge::emit_soul_prelude(owner,point,base){
    if let Ok(mut s)=STATE.lock(){if s.prelude.current==Some(entry){s.prelude.played=true;}}
   }
  }
 }
 for wave in pending{
  if wave.owner!=owner{continue;}
  // The struck character's feet are the center; no long downward projectile
  // that could select a basement or a floor below the critical target.
  if !crate::static_charge::emit_soul(wave.owner,wave.target,wave.damage,wave.point,base){if let Ok(mut s)=STATE.lock(){s.pending.push(wave);}}
 }
}
#[cfg(test)]mod tests{
 use super::*;
 fn id(target:u64)->Identity{Identity{owner:1,target,animation:31700,throw:9}}
 #[test]fn prelude_plays_once_on_entry_and_rearms_after_exit_or_actor_change(){
  let mut p=Prelude::default();let a=Entry{owner:1,animation:31700,throw:9};
  assert!(!p.observe(None));assert!(p.observe(Some(a)));assert!(p.observe(Some(a)));p.played=true;
  assert!(!p.observe(Some(a)));assert!(!p.observe(None));assert!(p.observe(Some(a)));p.played=true;
  assert!(p.observe(Some(Entry{owner:2,..a})));
 }
 #[test]fn final_damage_accumulates_through_main_once_without_time_gate(){let mut c=Critical::default();assert_eq!(c.credit(id(2),100,false),None);assert_eq!(c.credit(id(2),50,false),None);assert_eq!(c.credit(id(2),1000,true),Some(975.));assert_eq!(c.credit(id(2),1000,true),None);assert_eq!(c.credit(id(2),500,false),None);assert_eq!(c.credit(id(3),400,true),None);c=Critical::default();assert_eq!(c.credit(id(3),400,true),Some(600.));}
 #[test]fn interruption_and_new_critical_clear_the_budget(){let mut c=Critical::default();c.credit(id(2),100,false);c=Critical::default();assert_eq!(c.credit(id(2),400,true),Some(600.));c=Critical::default();assert_eq!(c.credit(id(2),400,true),Some(600.));}
 #[test]fn only_accepted_positive_hits_and_exact_split(){let mut c=Critical::default();assert_eq!(c.credit(id(2),0,true),None);assert_eq!(c.credit(id(2),-10,true),None);assert_eq!(c.credit(id(2),1000,true),Some(900.));assert_eq!(components(900.),[562.5,337.5,0.,0.,0.]);}
 #[test]fn duplicate_notifications_and_nested_dispatches_are_independent(){assert!(!first(1,2));let _a=scope(1,2);assert!(first(1,2));assert!(!first(1,2));{let _b=scope(3,4);assert!(first(3,4));assert!(!first(1,2));}assert!(!first(1,2));}
 #[test]fn generated_map_is_sorted_and_does_not_classify_our_wave(){assert!(!data::HITS.is_empty());assert!(data::HITS.windows(2).all(|w|(w[0].0,w[0].1)<(w[1].0,w[1].1)));assert!(data::HITS.iter().all(|r|r.0!=ATTACK));for (attack,main) in [(2100510,false),(2100511,true)]{assert!(data::HITS.contains(&(attack,31700,main)));}}
}

// Called by the isolated EXE test: executes the real current-throw getter,
// then the actual accepted-hit callback, without a running game or live actors.
#[cfg(test)]pub unsafe fn native_fixture(base:usize,source:usize,target:usize,hit:usize){
 let mut throw=[0usize;0x100/8];let tp=throw.as_mut_ptr()as usize;
 let mut row=[0u8;0x80];ptr::write_unaligned((row.as_mut_ptr()as usize+0x24)as*mut i32,31700);
 throw[0x58/8]=row.as_ptr()as usize;ptr::write_unaligned((tp+0x68)as*mut i32,1);
 let throw_module=[0usize,0,tp];let mut modules=[0usize;0x200/8];modules[0x88/8]=throw_module.as_ptr()as usize;
 let previous=read_ptr(source+0x1b8);ptr::write_unaligned((source+0x1b8)as*mut usize,modules.as_ptr()as usize);
 assert_eq!(current(source,base),Some((31700,tp)));
 // Execute the native target getter; only its registry lookup and the outer
 // bullet factory are fixtures. This verifies the real +18 target handle.
 thread_local!{static RESOLVED:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};static EMITTED:RefCell<Vec<(i32,f32)>>=const{RefCell::new(Vec::new())};}
 unsafe extern "system" fn resolve(_:usize,handle:usize)->usize{assert_eq!(read_ptr(handle),456);RESOLVED.with(std::cell::Cell::get)}
 unsafe extern "system" fn emit(_:usize,out:usize,d:usize,_:usize)->usize{EMITTED.with(|v|v.borrow_mut().push((read_i32(d+0x14),ptr::read_unaligned((d+0x30)as*const f32))));ptr::write_unaligned(out as*mut i32,1);out}
 let resolver=base+0x518f30;let original_resolver=std::slice::from_raw_parts(resolver as*const u8,12).to_vec();
 let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&(resolve as*const()as u64).to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(jump.as_ptr(),resolver as*mut u8,12);
 RESOLVED.with(|v|v.set(target));throw[0x18/8]=456;
 let get:unsafe extern "system" fn(usize)->usize=std::mem::transmute(base+TARGET_RVA);assert_eq!(get(tp),target);
 let old_coord=read_ptr(target+0x60);let old_modules=read_ptr(target+0x1b8);
 let mut physics=[0u8;0x90];ptr::write_unaligned((physics.as_mut_ptr()as usize+0x70)as*mut[f32;4],[2.,3.,4.,1.]);
 let mut target_modules=[0usize;0x70/8];target_modules[0x68/8]=physics.as_ptr()as usize;
 ptr::write_unaligned((target+0x60)as*mut usize,target);ptr::write_unaligned((target+0x1b8)as*mut usize,target_modules.as_ptr()as usize);
 let old_emit=crate::echo_cast::EMIT_ORIGINAL.swap(emit as*const()as*mut u8,Ordering::AcqRel);let old_manager=read_ptr(base+0x3c0ae30);ptr::write_unaligned((base+0x3c0ae30)as*mut usize,1);
 reset();update(source,base);update(source,base);EMITTED.with(|v|assert_eq!(&*v.borrow(),&[(PRELUDE,0.)]));
 // Interrupt before any hit: the visual cannot arrange an explosion.
 ptr::write_unaligned((tp+0x68)as*mut i32,0);update(source,base);assert!(STATE.lock().unwrap().pending.is_empty());
 ptr::write_unaligned((tp+0x68)as*mut i32,1);update(source,base);EMITTED.with(|v|assert_eq!(&*v.borrow(),&[(PRELUDE,0.),(PRELUDE,0.)]));

 let damage_module=[0usize,target];let mp=damage_module.as_ptr()as usize;
 reset();ptr::write_unaligned((hit+0x40)as*mut i32,2100510);ptr::write_unaligned((hit+0x44)as*mut i32,1);ptr::write_unaligned((hit+0x280)as*mut i32,100);
 {let _scope=scope(mp,hit);outgoing(mp,source,hit);outgoing(mp,source,hit);}
 {let s=STATE.lock().unwrap();assert_eq!(s.critical.sum,100.);assert!(s.pending.is_empty());}
 ptr::write_unaligned((hit+0x40)as*mut i32,ATTACK);{let _scope=scope(mp,hit);outgoing(mp,source,hit);}assert_eq!(STATE.lock().unwrap().critical.sum,100.);
 ptr::write_unaligned((hit+0x40)as*mut i32,2100511);ptr::write_unaligned((hit+0x280)as*mut i32,1000);
 for _ in 0..2{let _scope=scope(mp,hit);outgoing(mp,source,hit);}
 {let s=STATE.lock().unwrap();assert_eq!(s.critical.sum,1100.);assert!(s.pending.is_empty());} EMITTED.with(|v|assert_eq!(v.borrow().last(),Some(&(BULLET,950.))));EMITTED.with(|v|assert_eq!(v.borrow().len(),3));
 // Native inactive throw state returns -1; the per-frame path clears a spent
 // action, even if the next critical reuses the same ChrThrowMan and target.
 ptr::write_unaligned((tp+0x68)as*mut i32,0);assert_eq!(current(source,base),None);
 {let mut s=STATE.lock().unwrap();s.pending.clear();s.scene=read_ptr(base+0x3c131c8);}
 update(source,base);assert!(STATE.lock().unwrap().critical.identity.is_none());
 ptr::write_unaligned((tp+0x68)as*mut i32,1);
 {let _scope=scope(mp,hit);outgoing(mp,source,hit);}assert!(STATE.lock().unwrap().pending.is_empty());EMITTED.with(|v|assert_eq!(v.borrow().last(),Some(&(BULLET,900.))));
 crate::echo_cast::EMIT_ORIGINAL.store(old_emit,Ordering::Release);ptr::write_unaligned((base+0x3c0ae30)as*mut usize,old_manager);
 ptr::copy_nonoverlapping(original_resolver.as_ptr(),resolver as*mut u8,12);
 ptr::write_unaligned((target+0x60)as*mut usize,old_coord);ptr::write_unaligned((target+0x1b8)as*mut usize,old_modules);
 reset();ptr::write_unaligned((source+0x1b8)as*mut usize,previous);
}
