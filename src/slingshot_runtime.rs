//! Use the native launch calculation, including its history and collision path.
use crate::{slingshot as s,ptr,read_ptr,read_i32,has_effect,ACTIVE,BASE};
use std::sync::{Mutex,OnceLock,atomic::{AtomicPtr,Ordering}};
use std::cell::Cell;
pub const RVA:usize=0x4739f0;
pub const PATTERN:&str="40 55 53 56 57 41 57 48 8D 6C 24 A0 48 81 EC 60 01 00 00";
pub const HORIZONTAL_RVA:usize=0xe737b0;
pub const HORIZONTAL_PATTERN:&str="48 8B 41 08 48 85 C0 74 06 F3 0F 10 40 08 C3 0F 57 C0 C3";
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static HORIZONTAL_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static STATE:OnceLock<Mutex<s::State>>=OnceLock::new();
thread_local!{static IMPULSE:Cell<(i32,f32)>=const{Cell::new((-1,1.))};}
fn state()->&'static Mutex<s::State>{STATE.get_or_init(||Mutex::new(s::State::default()))}
unsafe fn context(player:usize,base:usize)->Option<(usize,bool,bool)> {
 if !crate::is_local(player,base){return None;}
 let modules=read_ptr(player+0x1b8);if modules==0{return None;}
 let stats=read_ptr(modules);let physics=read_ptr(modules+0x68);if stats==0||physics==0{return None;}
 let alive=read_i32(stats+0x140)>0&&!has_effect(player,102100,base)&&!has_effect(player,102150,base);
 // Native collision work copies the previous +92 ground sample into +93.
 // Treat either grounded sample as contact; reject only confirmed airborne.
 Some((stats,alive,s::ground_contact(ptr::read((physics+0x92) as *const u8)!=0,ptr::read((physics+0x93) as *const u8)!=0)))
}
pub unsafe fn pulse(player:usize,token:i32,base:usize)->bool {
 if !matches!(token,s::START|s::JUMP|s::BUSY|s::CANCEL|s::INPUT_DOWN|s::INPUT_UP|s::REQUEST){return false;}
 if let Some((stats,alive,grounded))=context(player,base){
  let owned=has_effect(player,s::OWNER,base);
  if let Ok(mut state)=state().lock(){state.synchronize(player,stats,owned,alive);if owned&&alive{state.notify(token,grounded);}}
 }
 true // private notifications never enter the native effect list
}
pub unsafe fn update(player:usize,base:usize,dt:f32){
 let Some((stats,alive,grounded))=context(player,base) else{return;};
 let owned=has_effect(player,s::OWNER,base);
 let buff={let Ok(mut state)=state().lock() else{return;};state.synchronize(player,stats,owned,alive);if owned&&alive{state.tick(dt,grounded);}state.buff()};
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);
 for id in [s::MARKER,s::BUFFS[0],s::BUFFS[1],s::BUFFS[2]]{
  let wanted=if id==s::MARKER{owned&&alive}else{Some(id)==buff};
  let present=has_effect(player,id,base);
  if wanted&&!present{apply(player,id,0);}else if !wanted&&present{remove(read_ptr(player+0x190),id);}
 }
}
type Launch=unsafe extern "system" fn(usize,*mut f32,*const f32,usize);
pub unsafe extern "system" fn detour(velocity:usize,out:*mut f32,input:*const f32,dt:usize){
 let original:Launch=std::mem::transmute(ORIGINAL.load(Ordering::Acquire));
 let mut scope=(-1,1.);
 if ACTIVE.load(Ordering::Acquire)&&velocity!=0{
  let base=BASE.load(Ordering::Acquire);let player=read_ptr(velocity+0x20);let row=read_i32(velocity+0x14);
  if let Some((stats,alive,_))=context(player,base){
   let owned=has_effect(player,s::OWNER,base);
   if let Ok(mut state)=state().lock(){state.synchronize(player,stats,owned,alive);if owned&&alive{scope=(row,state.impulse(row));}}
  }
 }
 let old=IMPULSE.with(|v|v.replace(scope));
 original(velocity,out,input,dt);
 IMPULSE.with(|v|v.set(old));
}
pub unsafe extern "system" fn horizontal(reference:usize)->f32 {
 let original:unsafe extern "system" fn(usize)->f32=std::mem::transmute(HORIZONTAL_ORIGINAL.load(Ordering::Acquire));
 let value=original(reference);let (row,rate)=IMPULSE.with(Cell::get);
 if rate!=1.&&reference!=0&&read_i32(reference)==row{value*rate}else{value}
}
