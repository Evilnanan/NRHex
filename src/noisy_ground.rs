//! Place only the local Poltergeist root on a nearby support surface.
//! Keep the native stationary preparation/charge/eruption and network chain.
use crate::{has_effect,is_local,noisy,ptr,read_i32,read_ptr,ACTIVE,BASE};
use crate::spirit_placement::{Sweep,Vector};
use std::{cell::Cell,sync::atomic::Ordering};
const BEHAVIOR:i32=950015000;
const LIFT:f32=0.5;
const DROP:f32=0.75;
const RADIUS:f32=0.05;
const SURFACE_LIFT:f32=0.02;
pub(crate) type Emit=unsafe extern "system" fn(usize,usize,usize,usize)->usize;
#[repr(C,align(16))]struct Descriptor([u8;0x110]);
#[derive(Clone,Copy)]struct Placement{handle:u64,pose:[[f32;4];4],consumed:bool}
thread_local!{static PLACEMENT:Cell<Option<Placement>>=const{Cell::new(None)};}
struct Guard(Option<Placement>);
impl Guard{fn enter(value:Option<Placement>)->Self{Self(PLACEMENT.with(|s|s.replace(value)))}}
impl Drop for Guard{fn drop(&mut self){PLACEMENT.with(|s|s.set(self.0));}}

fn root(behavior:i32,bullet:i32)->bool{
 bullet==noisy::BULLET_BASE||bullet==-1&&behavior==BEHAVIOR
}
fn surface(origin:[f32;4],contact:[f32;4],normal:[f32;4],fraction:f32)->Option<[f32;4]>{
 if !origin[..3].iter().chain(&contact[..3]).chain(&normal[..3]).all(|v|v.is_finite())||
  !fraction.is_finite()||fraction<=0.||fraction>1.||normal[1]<0.5||normal[1]>1.01{return None;}
 // DB04A0 +09DF returns the swept sphere CENTER, not the surface point.
 // Remove its radius along the contact normal, then project onto exact X/Z.
 let y=contact[1]-(normal[0]*(origin[0]-contact[0])+normal[2]*(origin[2]-contact[2])+RADIUS)/normal[1];
 if !y.is_finite()||y<origin[1]-DROP||y>origin[1]+0.25||
  (contact[0]-origin[0]).hypot(contact[2]-origin[2])>RADIUS+0.02{return None;}
 Some([origin[0],y+SURFACE_LIFT,origin[2],1.])
}
unsafe fn probe(sweep:Sweep,geometry:usize,origin:[f32;4])->Option<[f32;4]>{
 if geometry==0||!origin[..3].iter().all(|v|v.is_finite()){return None;}
 let start=Vector([origin[0],origin[1]+LIFT,origin[2],1.]);
 let motion=Vector([0.,-(LIFT+DROP),0.,0.]);
 let mut hit=u32::MAX;let mut flags=0u32;let mut fraction=f32::NAN;
 let mut contact=Vector([f32::NAN;4]);let mut normal=Vector([f32::NAN;4]);
 // Same geometry manager and terrain/asset mask as native family placement.
 let result=sweep(geometry,&mut hit,0x79,start.0.as_ptr(),motion.0.as_ptr(),RADIUS,0,
  &mut flags,contact.0.as_mut_ptr(),normal.0.as_mut_ptr(),&mut fraction,ptr::null_mut());
 if result!=&mut hit as*mut u32||hit&0xffffff==0xffffff{return None;}
 surface(origin,contact.0,normal.0,fraction)
}
pub(crate) unsafe fn ground_at(origin:[f32;4],base:usize)->Option<[f32;4]>{
 let manager=read_ptr(base+0x3c1e028);if manager==0{return None;}
 probe(std::mem::transmute(base+crate::SPIRIT_SWEEP_RVA),read_ptr(manager+0x98),origin)
}
unsafe fn locate(player:usize,base:usize)->Option<[f32;4]>{
 if !crate::spatial_ready(player){return None;}
 let manager=read_ptr(base+0x3c1e028);if manager==0{return None;}
 let geometry=read_ptr(manager+0x98);if geometry==0{return None;}
 let mut origin=Vector([f32::NAN;4]);
 let position:unsafe extern "system" fn(usize,*mut f32)->*const f32=std::mem::transmute(base+crate::CHR_POSITION_RVA);
 position(player,origin.0.as_mut_ptr());
 probe(std::mem::transmute(base+crate::SPIRIT_SWEEP_RVA),geometry,origin.0)
}
fn pose(position:[f32;4])->[[f32;4];4]{
 [[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],position]
}
unsafe fn dispatch(original:Emit,manager:usize,out:usize,input:usize,error:usize,ground:Option<[f32;4]>)->usize{
 let Some(ground)=ground else{
  // Native 35A460/35DA70 failure contract: output -1, error 3, return out.
  if out!=0{ptr::write_unaligned(out as*mut i32,-1);}
  if error!=0{ptr::write_unaligned(error as*mut i32,3);}
  return out;
 };
 let mut shadow=Descriptor(ptr::read_unaligned(input as*const[u8;0x110]));
 // The probe replaces the travelling seed. Its child is stationary and
 // retains the original .5 s preparation, followed by the 4 s charge.
 shadow.0[0x14..0x18].copy_from_slice(&(noisy::BULLET_BASE+1).to_le_bytes());
 shadow.0[0x1c..0x20].copy_from_slice(&(-1i32).to_le_bytes()); // no dummy reattachment
 let flags=read_i32(input+0x44)as u32|3; // independent, fixed pose; retain network bit
 shadow.0[0x44..0x48].copy_from_slice(&flags.to_le_bytes());
 let pose=pose(ground);
 ptr::copy_nonoverlapping(pose.as_ptr()as*const u8,shadow.0.as_mut_ptr().add(0x50),0x40);
 // Borrowed, synchronous descriptor only: no retain/release of its references.
 // A stationary world area has no parent or entity/auto-lock target.
 shadow.0[0x90..0x98].fill(0);
 shadow.0[0xb0..0x100].fill(0);
 shadow.0[0xb8..0xc0].fill(0xff);shadow.0[0xe0..0xe4].fill(0xff);shadow.0[0xf0..0xf4].fill(0xff);
 let _guard=Guard::enter(Some(Placement{handle:ptr::read_unaligned(input as*const u64),pose,consumed:false}));
 original(manager,out,shadow.0.as_ptr()as usize,error)
}
pub(crate) unsafe fn emit(original:Emit,manager:usize,out:usize,input:usize,error:usize)->usize{
 // A nested unrelated root must not inherit the outer placement scope.
 let _guard=Guard::enter(None);
 if !ACTIVE.load(Ordering::Acquire)||input==0||
  !root(read_i32(input+8),read_i32(input+0x14)){
  return original(manager,out,input,error);
 }
 let base=BASE.load(Ordering::Acquire);
 let resolve:unsafe extern "system" fn(u64)->usize=std::mem::transmute(base+crate::CHR_HANDLE_RVA);
 let player=resolve(ptr::read_unaligned(input as*const u64));
 if player==0||!is_local(player,base)||!has_effect(player,noisy::OWNER,base){return original(manager,out,input,error);}
 let result=dispatch(original,manager,out,input,error,locate(player,base));
 if out==0||read_i32(out)<0{
  if let Some(state)=crate::NOISY.get(){if let Ok(mut lava)=state.lock(){lava.emitted(false);}}
 }
 result
}
pub(crate) unsafe fn factory(payload:usize){
 if payload==0{return;}
 PLACEMENT.with(|s|{
  let Some(mut p)=s.get()else{return;};
  if p.consumed||read_i32(payload+4)!=noisy::BULLET_BASE+1||ptr::read_unaligned((payload+0x10)as*const u64)!=p.handle{return;}
  p.consumed=true;s.set(Some(p));
  // Native parsing may rebuild the pose. Restore it before allocation and
  // preserve fixed-pose selection in later native 345ED0 and network replay.
  ptr::copy_nonoverlapping(p.pose.as_ptr()as*const u8,(payload+0x190)as*mut u8,0x40);
  ptr::write_unaligned((payload+8)as*mut i32,-1);
  ptr::write((payload+0x169)as*mut u8,1);
 });
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn root_selection_excludes_native_chain_children_and_other_behaviors(){
  assert!(root(BEHAVIOR,-1));assert!(root(-1,noisy::BULLET_BASE));
  for id in noisy::BULLET_BASE+1..=noisy::BULLET_BASE+7{assert!(!root(BEHAVIOR,id));}
  assert!(!root(863050000,-1));assert!(!root(-1,12482050));
 }
 #[test]fn floor_and_slope_keep_exact_player_xz_and_reject_lower_floors_walls_and_misses(){
  let origin=[10.,20.,30.,1.];
  assert_eq!(surface(origin,[10.,20.05,30.,0.],[0.,1.,0.,0.],0.4),Some([10.,20.02,30.,1.]));
  let slope=surface(origin,[10.,20.0625,30.,0.],[0.6,0.8,0.,0.],0.4).unwrap();
  assert_eq!((slope[0],slope[2]),(10.,30.));assert!((slope[1]-20.02).abs()<0.00001);
  for y in [19.,12.,20.5,f32::NAN]{assert!(surface(origin,[10.,y,30.,0.],[0.,1.,0.,0.],0.4).is_none());}
  for normal in [[1.,0.,0.,0.],[0.,-1.,0.,0.],[0.,f32::NAN,0.,0.]]{assert!(surface(origin,origin,normal,0.4).is_none());}
  for fraction in [-1.,0.,1.01,f32::NAN]{assert!(surface(origin,origin,[0.,1.,0.,0.],fraction).is_none());}
  assert!(surface(origin,[11.,20.,30.,0.],[0.,1.,0.,0.],0.4).is_none());
 }
 unsafe extern "system" fn capture(manager:usize,out:usize,input:usize,error:usize)->usize{
  assert_eq!(manager,17);assert_eq!(input%16,0);
  assert_eq!(read_i32(input+0x14),noisy::BULLET_BASE+1);assert_eq!(read_i32(input+0x1c),-1);
  assert_eq!(read_i32(input+0x44),0x8b);assert_eq!(read_ptr(input+0x98),0x1234);assert_eq!(read_ptr(input+0xa0),0x5678);
  assert_eq!(read_ptr(input+0xc0),0);assert_eq!(ptr::read_unaligned((input+0xb8)as*const u64),u64::MAX);
  let expected=pose([10.,20.02,30.,1.]);assert_eq!(ptr::read_unaligned((input+0x50)as*const[[f32;4];4]),expected);
  let mut payload=[0u8;0x220];let p=payload.as_mut_ptr()as usize;
  ptr::write_unaligned((p+4)as*mut i32,noisy::BULLET_BASE+1);ptr::write_unaligned((p+0x10)as*mut u64,42);
  {let _nested=Guard::enter(None);factory(p);assert_eq!(payload[0x169],0);}
  factory(p);assert_eq!(payload[0x169],1);assert_eq!(read_i32(p+8),-1);
  assert_eq!(ptr::read_unaligned((p+0x190)as*const[[f32;4];4]),expected);
  payload[0x190..].fill(0);factory(p);assert!(payload[0x190..].iter().all(|b|*b==0));
  ptr::write_unaligned(out as*mut i32,123);ptr::write_unaligned(error as*mut i32,0);out
 }
 #[test]fn descriptor_is_borrowed_and_original_bytes_outputs_and_nested_scope_are_preserved(){unsafe{
  let mut data=Descriptor([0x55;0x110]);let p=data.0.as_mut_ptr()as usize;
  ptr::write_unaligned(p as*mut u64,42);ptr::write_unaligned((p+0x44)as*mut i32,0x88);
  ptr::write_unaligned((p+0x98)as*mut usize,0x1234);ptr::write_unaligned((p+0xa0)as*mut usize,0x5678);
  let before=data.0;let mut out=-1;let mut error=-1;let o=&mut out as*mut i32 as usize;let e=&mut error as*mut i32 as usize;
  assert_eq!(dispatch(capture,17,o,p,e,Some([10.,20.02,30.,1.])),o);assert_eq!((out,error),(123,0));assert_eq!(data.0,before);
  assert!(PLACEMENT.with(Cell::get).is_none());
  assert_eq!(dispatch(capture,17,o,p,e,None),o);assert_eq!((out,error),(-1,3));assert_eq!(data.0,before);
 }}
}
#[cfg(test)]#[path="noisy_ground_native_tests.rs"]mod native_tests;
