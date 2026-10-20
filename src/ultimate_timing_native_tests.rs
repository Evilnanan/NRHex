use crate::{pe,ptr,win::*};
use sha2::{Digest,Sha256};
use std::cell::Cell;

thread_local!{static CALLS:Cell<usize>=const{Cell::new(0)};}
unsafe fn pointer(address:usize,value:usize){ptr::write_unaligned(address as*mut usize,value);}
unsafe extern "system" fn emit(_:usize,effect:i32,mode:u8){
 assert_eq!((effect,mode),(crate::runtime::ULTIMATE_TRIGGER,0));
 CALLS.with(|calls|calls.set(calls.get()+1));
}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; isolated native TAE entry and consumption"]
fn native_ultimate_timing_entry_only_dispatch(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();
 assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let headers=pe::Headers::parse(&raw,0).unwrap();
 let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&(emit as*const()as usize).to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);
 ptr::copy_nonoverlapping(jump.as_ptr(),(base+0x3c8be0)as*mut u8,jump.len());
 let entry:unsafe extern "system" fn(usize,usize,u8)=std::mem::transmute(base+0x40fff0);
 let consume:unsafe extern "system" fn(usize,f32)=std::mem::transmute(base+0x48b590);
 let mut actor=[0usize;0x200/8];let actor=actor.as_mut_ptr()as usize;
 let mut context=[0usize;4];let context=context.as_mut_ptr()as usize;pointer(context+0x18,actor);
 let mut descriptor=[0usize;4];let descriptor=descriptor.as_mut_ptr()as usize;
 let parameters=[crate::runtime::ULTIMATE_TRIGGER,0,0,0];pointer(descriptor+8,parameters.as_ptr()as usize);
 let mut gauge=[0usize;0x48/8];let gauge=gauge.as_mut_ptr()as usize;
 for cost in [70f32,100.]{
  ptr::write_unaligned((gauge+0x18)as*mut f32,100.);ptr::write_unaligned((gauge+0x14)as*mut f32,100.);
  consume(gauge,cost);assert_eq!(ptr::read_unaligned((gauge+0x14)as*const f32),100.-cost);
  let before=CALLS.with(Cell::get);ptr::write((descriptor+0x18)as*mut u8,1);entry(context,descriptor,0);
  assert_eq!(CALLS.with(Cell::get),before+1);
  ptr::write((descriptor+0x18)as*mut u8,0);
  for _ in 0..60{entry(context,descriptor,0);}
  assert_eq!(CALLS.with(Cell::get),before+1);
 }
 assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
