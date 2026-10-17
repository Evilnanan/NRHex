use crate::native_actor::stats;
use crate::{balloon,has_effect,is_local,read_i32,read_ptr,ArtsBulletFn,ARTS_BULLET_RVA,BALLOON,REMOVE_EFFECT_RVA,APPLY_SELF_RVA,ACTIVE,BASE};
use std::{ptr,sync::atomic::{AtomicPtr,Ordering}};

pub const STATE_RVA:usize=0x66f3b0;
pub const STATE_PATTERN:&str="48 89 5C 24 08 57 48 83 EC 50 0F B6 81 10 07 00 00 48 8B F9 0F 29 74 24 40 0F 28 F2";
pub static STATE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());

unsafe fn life(player:usize,attributes:usize)->balloon::Life{
 let coordinate=read_ptr(player+0x60);
 let collision=if coordinate!=0{read_ptr(coordinate+0xc8)}else{0};
 if collision!=0&&ptr::read_unaligned((collision+0x24)as*const u32)&1!=0{return balloon::Life::Dead;}
 match ptr::read((player+0x710)as*const u8){
  1=>balloon::Life::Down,
  2=>balloon::Life::Reviving,
  0 if collision!=0&&read_i32(attributes+0x140)>0=>balloon::Life::Alive,
  _=>balloon::Life::Uncommitted
 }
}
pub(crate) unsafe fn prepare(player:usize,base:usize){
 if !is_local(player,base){return;}
 let attributes=stats(player);if attributes==0{return;}
 let state=life(player,attributes);let owned=has_effect(player,balloon::OWNER,base);
 if let Some(balloon)=BALLOON.get(){if let Ok(mut balloon)=balloon.lock(){
  balloon.synchronize_life(player,attributes,owned,state);
  if owned&&!state.committed()&&state!=balloon::Life::Reviving{balloon.remember_max(read_i32(attributes+0x144));}
 }}
}
pub(crate) unsafe fn transition(player:usize,base:usize){
 transition_as(player,base,None);
}
pub(crate) unsafe fn action_started(player:usize,base:usize){
 transition_as(player,base,Some(balloon::Life::Starting));
}
unsafe fn transition_as(player:usize,base:usize,notification:Option<balloon::Life>){
 if !is_local(player,base){return;}
 let attributes=stats(player);if attributes==0{return;}
 let state=notification.unwrap_or_else(||life(player,attributes));let owned=has_effect(player,balloon::OWNER,base);
 let damage=BALLOON.get().and_then(|balloon|balloon.lock().ok().and_then(|mut balloon|{
  balloon.synchronize_life(player,attributes,owned,state);
  if owned&&state==balloon::Life::Alive{balloon.remember_max(read_i32(attributes+0x144));}
  let maximum=if balloon.maximum()>0{balloon.maximum()}else{read_i32(attributes+0x144)};
  balloon.commit(state,maximum);balloon.take_for(player,attributes)
 }));
 if let Some(damage)=damage{emit_balloon(player,damage,base);}
}
pub unsafe extern "system" fn state_detour(player:usize,state:u8,time:f32){
 let original:unsafe extern "system" fn(usize,u8,f32)=std::mem::transmute(STATE_ORIGINAL.load(Ordering::Acquire));
 let base=BASE.load(Ordering::Acquire);let local=ACTIVE.load(Ordering::Acquire)&&is_local(player,base);
 if local{prepare(player,base);}
 original(player,state,time);
 if local{transition(player,base);}
}
unsafe fn emit_balloon(player:usize,damage:i32,base:usize){
 debug_assert!(damage>0);
 let emit:ArtsBulletFn=std::mem::transmute(base+ARTS_BULLET_RVA);
 emit_balloon_behavior(emit,player);
}
unsafe fn emit_balloon_behavior(emit:ArtsBulletFn,player:usize)->bool{
 let descriptor=[85u32,0x0000ff02,0,0xbf800000,0x00000101];let mut error=-1i32;
 emit(player+0x100,balloon::BEHAVIOR,-1,descriptor.as_ptr()as usize,0x10,0x10,1,0,0xff,&mut error as*mut i32 as usize)!=0
}
pub(crate) unsafe fn update(player:usize,base:usize,_attributes:usize){
 let owned=has_effect(player,balloon::OWNER,base);let marked=has_effect(player,balloon::MARKER,base);
 if owned&&!marked{
  let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);
  apply(player,balloon::MARKER,0);
 }else if !owned&&marked{
  let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);
  remove(read_ptr(player+0x190),balloon::MARKER);
 }
 transition(player,base);
}

#[cfg(test)]mod tests{
 use super::*;
 use std::cell::Cell;
 thread_local!{
  static OWNED:Cell<bool>=const{Cell::new(true)};
  static EMITTED:Cell<usize>=const{Cell::new(0)};
  static FORWARDED:Cell<(usize,u8,u32)>=const{Cell::new((0,0,0))};
 }
 unsafe extern "system" fn has(_:usize,effect:i32)->u8{u8::from(effect==balloon::OWNER&&OWNED.with(Cell::get))}
 unsafe extern "system" fn emit(module:usize,behavior:i32,event:i32,descriptor:usize,source:i32,original_source:i32,flag0:u8,flag1:u8,kind:u8,error:usize)->u8{
  assert!(module!=0);assert_eq!((behavior,event),(950017000,-1));
  assert_eq!(ptr::read_unaligned(descriptor as*const[u32;5]),[85,0xff02,0,0xbf800000,0x101]);
  assert_eq!((source,original_source,flag0,flag1,kind),(16,16,1,0,255));
  ptr::write(error as*mut i32,0);EMITTED.with(|count|count.set(count.get()+1));1
 }
 unsafe extern "system" fn state_original(player:usize,state:u8,time:f32){FORWARDED.with(|value|value.set((player,state,time.to_bits())));}
 #[test]fn death_explosion_calls_the_native_emitter_without_a_living_effect_update(){unsafe{
  assert!(emit_balloon_behavior(emit,0x1000));
 }}
 #[test]fn native_state_wrapper_preserves_actor_byte_state_and_float_time(){unsafe{
  let previous=ACTIVE.swap(false,Ordering::AcqRel);
  let saved=STATE_ORIGINAL.swap(state_original as*mut u8,Ordering::AcqRel);
  for (player,state,time) in [(1usize,0u8,0f32),(123,1,-1.),(456,2,3.25),(789,255,f32::NAN)]{
   state_detour(player,state,time);assert_eq!(FORWARDED.with(Cell::get),(player,state,time.to_bits()));
  }
  STATE_ORIGINAL.store(saved,Ordering::Release);ACTIVE.store(previous,Ordering::Release);
 }}
 #[test]fn native_commit_gates_balloon_and_positive_hp_does_not_fake_revival(){unsafe{
  let memory=crate::win::VirtualAlloc(ptr::null_mut(),0x4e00000,crate::win::MEM_COMMIT|crate::win::MEM_RESERVE,crate::win::PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());
  let base=memory as usize;
  for (offset,target) in [(crate::HAS_EFFECT_RVA,has as*const()as usize),(ARTS_BULLET_RVA,emit as*const()as usize)]{
   let mut bytes=vec![0x48,0xb8];bytes.extend_from_slice(&target.to_le_bytes());bytes.extend_from_slice(&[0xff,0xe0]);
   ptr::copy_nonoverlapping(bytes.as_ptr(),(base+offset)as*mut u8,bytes.len());
  }
  let mut actor=vec![0u64;0x740/8];let actor=actor.as_mut_ptr()as usize;
  ptr::write_unaligned((actor+0x190)as*mut usize,actor);
  let mut attributes=vec![0u64;0x200/8];let attributes=attributes.as_mut_ptr()as usize;
  let mut modules=vec![attributes];let mut world=vec![0u64;0x17500/8];
  let mut coordinate=vec![0u64;0xd0/8];let mut collision=vec![0u64;0x30/8];
  let collision_flags=(collision.as_mut_ptr()as usize+0x24)as*mut u32;
  ptr::write_unaligned((actor+0x1b8)as*mut usize,modules.as_mut_ptr()as usize);
  ptr::write_unaligned((actor+0x60)as*mut usize,coordinate.as_mut_ptr()as usize);
  coordinate[0xc8/8]=collision.as_mut_ptr()as u64;
  world[0x174e8/8]=actor as u64;ptr::write_unaligned((base+crate::WORLD_RVA)as*mut usize,world.as_ptr()as usize);
  ptr::write_unaligned((attributes+0x140)as*mut i32,1000);ptr::write_unaligned((attributes+0x144)as*mut i32,1500);
  let state=BALLOON.get_or_init(||std::sync::Mutex::new(balloon::Balloon::default()));
  let previous=std::mem::take(&mut*state.lock().unwrap());OWNED.with(|owned|owned.set(true));EMITTED.with(|count|count.set(0));
  prepare(actor,base);
  for _ in 0..100{
   ptr::write_unaligned((attributes+0x140)as*mut i32,0);assert_eq!(life(actor,attributes),balloon::Life::Uncommitted);transition(actor,base);
   ptr::write_unaligned((attributes+0x140)as*mut i32,446);transition(actor,base);
  }
  assert_eq!(EMITTED.with(Cell::get),0);
  action_started(actor,base);assert_eq!(EMITTED.with(Cell::get),1);
  for _ in 0..100{transition(actor,base);action_started(actor,base);}assert_eq!(EMITTED.with(Cell::get),1);
  ptr::write(collision_flags,1);transition(actor,base);assert_eq!(EMITTED.with(Cell::get),1);
  ptr::write(collision_flags,0);transition(actor,base);
  EMITTED.with(|count|count.set(0));
  OWNED.with(|owned|owned.set(false));ptr::write_unaligned((attributes+0x144)as*mut i32,200);ptr::write((actor+0x710)as*mut u8,1);
  transition(actor,base);assert_eq!(EMITTED.with(Cell::get),1);assert_eq!(state.lock().unwrap().base_hp_for(actor,attributes),Some(1500));
  for _ in 0..100{transition(actor,base);}assert_eq!(EMITTED.with(Cell::get),1);
  ptr::write((actor+0x710)as*mut u8,2);assert_eq!(life(actor,attributes),balloon::Life::Reviving);transition(actor,base);assert_eq!(EMITTED.with(Cell::get),1);
  OWNED.with(|owned|owned.set(true));ptr::write_unaligned((attributes+0x144)as*mut i32,1500);ptr::write((actor+0x710)as*mut u8,0);transition(actor,base);
  ptr::write((actor+0x710)as*mut u8,1);transition(actor,base);assert_eq!(EMITTED.with(Cell::get),2);
  ptr::write((actor+0x710)as*mut u8,0);ptr::write(collision_flags,1);transition(actor,base);assert_eq!(EMITTED.with(Cell::get),2);
  ptr::write_unaligned((attributes+0x140)as*mut i32,700);transition(actor,base);assert_eq!(life(actor,attributes),balloon::Life::Dead);assert_eq!(EMITTED.with(Cell::get),2);
  ptr::write_unaligned((attributes+0x140)as*mut i32,0);ptr::write(collision_flags,0);transition(actor,base);ptr::write(collision_flags,1);transition(actor,base);assert_eq!(EMITTED.with(Cell::get),2);
  ptr::write(collision_flags,0);ptr::write_unaligned((attributes+0x140)as*mut i32,1000);transition(actor,base);
  ptr::write(collision_flags,1);transition(actor,base);assert_eq!(EMITTED.with(Cell::get),3);
  *state.lock().unwrap()=previous;crate::win::VirtualFree(memory,0,crate::win::MEM_RELEASE);
 }}
}
