//! Silence only the local managed Ram's water surface one-shot.
//! All other sound events and every native floor VFX keep their original path.
use crate::{ptr,read_ptr,read_i32};
use std::sync::atomic::{AtomicPtr,Ordering};
pub const SOUND_RVA:usize=0x40f060;
pub const SOUND_PATTERN:&str="48 89 5C 24 10 55 56 57 41 54 41 55 41 56 41 57 48 8D 6C 24 D9 48 81 EC B0 00 00 00 4C 8B F2 4C 8B E9 48 8B 79 18";
pub const MATERIAL_RVA:usize=0x459a10;
pub const MATERIAL_PATTERN:&str="48 89 5C 24 08 57 48 83 EC 20 0F B7 79 10 48 8B D9";
pub static SOUND_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
type Native=unsafe extern "system" fn(usize,usize);
const SURFACE:i32=100003000;
fn water_sound(kind:i32,id:i32,slot:i32,mode:u8,material:i32)->bool{
 kind==15&&id==SURFACE&&slot==-1&&mode==0&&material==21
}
unsafe fn material(actor:usize)->Option<i32>{
 let modules=read_ptr(actor+0x1b8);if modules==0{return None;}
 let module=read_ptr(modules+0xa0);let base=crate::BASE.load(Ordering::Acquire);
 if module==0||base==0{return None;}
 // Same getter as native 4C4020, including the actor's material override.
 let get:unsafe extern "system" fn(usize)->i32=std::mem::transmute(base+MATERIAL_RVA);
 Some(get(module))
}
pub unsafe extern "system" fn sound(owner:usize,event:usize){
 let original:Native=std::mem::transmute(SOUND_ORIGINAL.load(Ordering::Acquire));
 if owner!=0&&event!=0{
  let p=read_ptr(event+8);
  if p!=0&&water_sound(read_i32(p),read_i32(p+4),read_i32(p+12),ptr::read((p+0x12)as*const u8),21){
   let actor=read_ptr(owner+0x18);
   if actor!=0&&crate::lightning_ram::ground_feedback(actor).is_some()&&material(actor)==Some(21){return;}
  }
 }
 // Preserve every start/end/update of slotted, looping and unrelated sounds.
 original(owner,event);
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn only_water_surface_one_shot_is_muted(){
  assert!(water_sound(15,SURFACE,-1,0,21));
  for material in [-1,0,1,2,5,11,20,22,30]{assert!(!water_sound(15,SURFACE,-1,0,material));}
  for(kind,id,slot,mode)in[(5,440462,-1,1),(5,440463,-1,0),(15,200,-1,0),(15,6200,-1,0),(15,0,-1,0),(15,SURFACE,0,0),(15,SURFACE,-1,1)]{assert!(!water_sound(kind,id,slot,mode,21));}
 }
}
