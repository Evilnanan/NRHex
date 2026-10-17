//! One authoritative roll by the local owner, native spirit placement/network
//! creation on both peers, and handle-only lifecycle records across streaming.
use crate::{demons::{self,Summoning},native_actor::{self,resolve},read_ptr,has_effect,has_state,ACTIVE,BASE,APPLY_SELF_RVA};
use std::{collections::HashMap,ptr,sync::{atomic::{AtomicPtr,AtomicU32,Ordering},Mutex,OnceLock}};
pub const CREATE_RVA:usize=0x3918d0;
pub const REMOVE_RVA:usize=0x51b2b0;
pub const TEAM_RVA:usize=0x3d77c0;
pub const REMOVE_PATTERN:&str="48 85 D2 0F 84 28 02 00 00 48 89 5C 24 08 48 89 74 24 18 57 48 83 EC 20 48 8B DA 48 8B F1 48 8B 02 48 8B CA FF 50 58 4C 8B 86 88 75 01 00 48 8B 8E 80 75 01 00 49 3B C8 0F 84 98 00 00 00 66 90";
pub const TEAM_PATTERN:&str="80 79 74 05 74 06 0F B6 02 88 41 74 C3 90 8B 45 48 89 5C 24 10 57 48 83 EC 20 48 8B 01 48 8B F9 32 DB FF 90 A0 01 00 00 48 85 C0 74 22 48 83 38 00 74 1C 83 7F 70 04 75 04 B3 05 EB 21 48 8D 54";
pub static CREATE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static REMOVE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static STATE:OnceLock<Mutex<Summoning>>=OnceLock::new();
static ACTORS:OnceLock<Mutex<HashMap<u64,u32>>>=OnceLock::new();
static NEXT:AtomicU32=AtomicU32::new(0);
fn state()->&'static Mutex<Summoning>{STATE.get_or_init(Default::default)}
fn actors()->&'static Mutex<HashMap<u64,u32>>{ACTORS.get_or_init(Default::default)}
fn valid_descriptor(params:[i32;3])->Option<u32>{
 let seed=demons::seed_from_chara(params[2])?;let roll=demons::roll(seed)?;
 (params[0]==roll.npc()&&params[1]==0).then_some(seed)
}
fn register(handle:u64,seed:u32){if handle!=0&&handle!=u64::MAX{if let Ok(mut a)=actors().lock(){a.insert(handle,seed);}}}
pub unsafe fn is_actor(actor:usize)->bool{actor!=0&&actors().lock().is_ok_and(|a|a.contains_key(&ptr::read_unaligned((actor+8)as*const u64)))}
type Create=unsafe extern "system" fn(usize,*mut u64,u32,i32,*const i32,u8)->*mut u64;
pub unsafe extern "system" fn create(implementation:usize,out:*mut u64,owner:u32,slot:i32,params:*const i32,flag:u8)->*mut u64{
 let original:Create=std::mem::transmute(CREATE_ORIGINAL.load(Ordering::Acquire));
 let seed=if ACTIVE.load(Ordering::Acquire)&&!params.is_null()&&(0..6).contains(&slot){valid_descriptor(ptr::read_unaligned(params as*const[i32;3]))}else{None};
 if let Some(seed)=seed{if !crate::demons_params::prepare(seed,BASE.load(Ordering::Acquire)){
  if !out.is_null(){ptr::write_unaligned(out,u64::MAX);}crate::nlog_info!("Demon creation deferred: parameters not ready, seed={seed}");return out;
 }}
 let result=original(implementation,out,owner,slot,params,flag);
 if let Some(seed)=seed{if !out.is_null(){let handle=ptr::read_unaligned(out);register(handle,seed);let base=BASE.load(Ordering::Acquire);let actor=resolve(base,handle);if actor!=0{set_team(actor,base);}}}
 result
}
unsafe fn set_team(actor:usize,base:usize){
 if ptr::read((actor+0x74)as*const u8)!=29{let set:unsafe extern "system" fn(usize,*const u8)=std::mem::transmute(base+TEAM_RVA);set(actor,&29);}
}
pub unsafe fn actor_update(actor:usize,base:usize){if is_actor(actor){set_team(actor,base);}}
// Only our registered virtual-character summons are routed. Native Revenant
// spirits and original Libra sinners retain every original passive.
pub unsafe fn route_effect(actor:usize,effect:i32)->Option<i32>{
 if !matches!(effect,705200|297000|705012|296400)||!is_actor(actor){return Some(effect);}
 match effect{705200=>Some(demons::DRAIN),_=>None}
}
pub unsafe fn died(actor:usize){if is_actor(actor){let handle=ptr::read_unaligned((actor+8)as*const u64);if let Ok(mut s)=state().lock(){s.retired(handle);}}}
// This is confirmed native world removal, not an unresolved actor handle.
pub unsafe extern "system" fn remove(world:usize,actor:usize){
 let original:unsafe extern "system" fn(usize,usize)=std::mem::transmute(REMOVE_ORIGINAL.load(Ordering::Acquire));
 let handle=if ACTIVE.load(Ordering::Acquire)&&is_actor(actor){Some(ptr::read_unaligned((actor+8)as*const u64))}else{None};
 original(world,actor);
 if let Some(handle)=handle{if let Ok(mut a)=actors().lock(){a.remove(&handle);}if let Ok(mut s)=state().lock(){s.retired(handle);}}
}
pub unsafe fn reset(player:usize,base:usize){
 if let Ok(mut s)=state().lock(){*s=Summoning::default();}if let Ok(mut a)=actors().lock(){a.clear();}
 if has_effect(player,demons::COOLDOWN,base){let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);remove(read_ptr(player+0x190),demons::COOLDOWN);}
}
pub unsafe fn update(player:usize,base:usize,dt:f32){
 let owned=has_effect(player,demons::OWNER,base);let madness=has_state(player,437,base);
 let stats=native_actor::stats(player);let alive=stats!=0&&!native_actor::incapacitated(player,stats);
 let cooldown=has_effect(player,demons::COOLDOWN,base);
 let live=state().lock().map(|s|s.live).unwrap_or(0);
 if live!=0{
  let actor=resolve(base,live);
  if actor!=0&&crate::spatial_ready(actor){let dead:unsafe extern "system" fn(usize)->u8=std::mem::transmute(base+crate::SPIRIT_DEAD_RVA);if dead(actor)!=0{if let Ok(mut s)=state().lock(){s.retired(live);}}}
 }
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);
 // A single native 60-second effect backs up the game-time state over a PC
 // replacement. Do not refresh an acknowledged cooldown each update.
 let dirty=state().lock().is_ok_and(|s|s.cooldown_dirty);
 if dirty{
  if cooldown{if let Ok(mut s)=state().lock(){s.cooldown_dirty=false;}}
  else if state().lock().is_ok_and(|s|s.cooldown<=0.){if let Ok(mut s)=state().lock(){s.cooldown_dirty=false;}}
  else if stats!=0{apply(player,demons::COOLDOWN,0);}
 }
 let counter=NEXT.fetch_add(1,Ordering::Relaxed);
 let seed=counter.wrapping_mul(63847).wrapping_add((crate::GetTickCount64()as u32)&(demons::SEEDS-1))&(demons::SEEDS-1);
 let request=state().lock().ok().and_then(|mut s|s.observe(owned,madness,alive,cooldown,dt,seed));
 if let Some(seed)=request{
  let roll=demons::roll(seed).unwrap();let soul=crate::spirit::Soul{npc:roll.npc(),think:0,chara_init:demons::chara_id(seed).unwrap()};
  if let Some(handle)=crate::spirit_runtime::summon_descriptor(player,soul,base){
   register(handle,seed);if let Ok(mut s)=state().lock(){s.created(handle);}
   crate::nlog_info!("Demon summoned: seed={} hero={} skin={} right={} left={:?} handle={}",seed,roll.hero,roll.skin,roll.right.source,roll.left.map(|h|h.source),handle);
  }
 }
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn only_matching_virtual_ci_and_private_skin_descriptors_are_registered(){
  for seed in [0,1,42,demons::SEEDS-1]{let r=demons::roll(seed).unwrap();let ci=demons::chara_id(seed).unwrap();assert_eq!(valid_descriptor([r.npc(),0,ci]),Some(seed));assert_eq!(valid_descriptor([r.npc()+1,0,ci]),None);assert_eq!(valid_descriptor([r.npc(),1,ci]),None);assert_eq!(valid_descriptor([r.npc(),0,-1]),None);}
  assert_eq!(valid_descriptor([700035000,0,35000]),None);
 }
 #[test]fn sinner_passives_are_routed_by_handle_and_only_local_death_starts_cooldown(){unsafe{
  let handle=0x12340077;let mut a=[0u64;16];a[1]=handle;let a=a.as_mut_ptr()as usize;
  let old_state={let mut s=state().lock().unwrap();std::mem::take(&mut*s)};
  register(handle,42);assert!(is_actor(a));assert_eq!(route_effect(a,705200),Some(demons::DRAIN));
  for id in [297000,705012,296400]{assert_eq!(route_effect(a,id),None);}
  assert_eq!(route_effect(a,7740),Some(7740));died(a);assert_eq!(state().lock().unwrap().cooldown,0.);
  state().lock().unwrap().created(handle);died(a);assert_eq!(state().lock().unwrap().cooldown,60.);
  state().lock().unwrap().cooldown=20.;died(a);assert_eq!(state().lock().unwrap().cooldown,20.);
  // Actor replacement with the same generation-bearing handle stays scoped.
  let mut replacement=[0u64;16];replacement[1]=handle;let replacement=replacement.as_ptr()as usize;assert_eq!(route_effect(replacement,705200),Some(demons::DRAIN));
  actors().lock().unwrap().remove(&handle);assert_eq!(route_effect(replacement,705200),Some(705200));
  *state().lock().unwrap()=old_state;
 }}
}
