//! Execute the real sweep ABI adapter and fixed-pose getter. World collision
//! is a controlled fixture; this does not claim castle/gameplay verification.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
thread_local!{static FLOORS:std::cell::RefCell<Vec<f32>>=const{std::cell::RefCell::new(Vec::new())};}
unsafe extern "system" fn collision(geometry:usize,hit:*mut u32,mask:u32,start:*const f32,motion:*const f32,radius:f32,extra:usize,
 flags:*mut u32,contact:*mut f32,normal:*mut f32,fraction:*mut f32,last:*mut f32)->*mut u32{
 assert_eq!((geometry,mask,extra),(123,0x79,0));assert_eq!(radius,RADIUS);assert!(!last.is_null()); // DB0B80 supplies its own temporary contact buffer.
 assert_eq!(start as usize%16,0);assert_eq!(motion as usize%16,0);
 let start=ptr::read_unaligned(start as*const[f32;4]);let motion=ptr::read_unaligned(motion as*const[f32;4]);
 assert_eq!(start,[10.,20.5,30.,1.]);assert_eq!(motion,[0.,-1.25,0.,0.]);
 let ground=FLOORS.with(|s|s.borrow().iter().copied().filter(|&y|y<=start[1]-radius&&y>=start[1]+motion[1]-radius).max_by(f32::total_cmp));
 if let Some(y)=ground{
  ptr::write(hit,42);ptr::write(flags,0);ptr::write(fraction,(start[1]-radius-y)/-motion[1]);
  ptr::copy_nonoverlapping([start[0],y+radius,start[2],1.].as_ptr(),contact,4);
  ptr::copy_nonoverlapping([0.,1.,0.,0.].as_ptr(),normal,4);
  ptr::copy_nonoverlapping([start[0],y,start[2],1.].as_ptr(),last,4);
 }else{ptr::write(hit,u32::MAX);}hit
}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; run alone"]
fn noisy_ground_real_sweep_adapter_and_native_fixed_pose(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();
 assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();
 let memory=VirtualAlloc(ptr::null_mut(),0xdc0000,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for (start,end)in[(0xdb0b80usize,0xdb0c91usize),(0x345ed0,0x34602e)]{
  let section=headers.sections.iter().find(|s|start>=s.virtual_address as usize&&end<=s.virtual_address as usize+s.raw_size as usize).unwrap();
  let offset=section.raw_offset as usize+start-section.virtual_address as usize;
  assert_eq!(raw[offset+end-start-1],0xc3,"native slice must include RET");
  ptr::copy_nonoverlapping(raw[offset..].as_ptr(),(base+start)as*mut u8,end-start);
 }
 let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&(collision as*const()as usize).to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);
 ptr::copy_nonoverlapping(jump.as_ptr(),(base+0xdb04a0)as*mut u8,jump.len());
 let sweep:Sweep=std::mem::transmute(base+0xdb0b80);
 let origin=[10.,20.,30.,1.];
 for floors in [vec![12.,20.],vec![20.,12.],vec![19.8,12.]]{
  let y=floors.iter().copied().max_by(f32::total_cmp).unwrap();FLOORS.with(|s|*s.borrow_mut()=floors);
  assert_eq!(probe(sweep,123,origin),Some([10.,y+SURFACE_LIFT,30.,1.]));
 }
 for floors in [vec![12.],vec![],vec![19.2],vec![20.4,12.]]{
  FLOORS.with(|s|*s.borrow_mut()=floors);assert_eq!(probe(sweep,123,origin),None);
 }
 #[repr(align(16))]struct Group([u8;0x5a0]);
 let mut group=Group([0;0x5a0]);let g=group.0.as_mut_ptr()as usize;
 let expected=pose([10.,20.02,30.,1.]);
 ptr::copy_nonoverlapping(expected.as_ptr()as*const u8,(g+0x4c0)as*mut u8,0x40);
 ptr::write_unaligned((g+0x500)as*mut i32,-1);ptr::write((g+0x499)as*mut u8,1);
 #[repr(align(16))]struct Matrix([[f32;4];4]);let mut result=Matrix([[f32::NAN;4];4]);
 let getter:unsafe extern "system" fn(usize,*mut f32)->*mut f32=std::mem::transmute(base+0x345ed0);
 assert_eq!(getter(g,result.0.as_mut_ptr()as*mut f32),result.0.as_mut_ptr()as*mut f32);
 assert_eq!(result.0,expected);
 assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
