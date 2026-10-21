//! Execute the shared seven-argument constructor wrapper and native impact copy.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
use std::{cell::Cell,sync::atomic::Ordering};
thread_local!{static OWNED:Cell<bool>=const{Cell::new(false)};static MIGHTY:Cell<bool>=const{Cell::new(false)};static COPY:Cell<usize>=const{Cell::new(0)};}
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from((id==OWNER&&OWNED.with(Cell::get))||(id==crate::strength::OWNER&&MIGHTY.with(Cell::get)))}
unsafe extern "system" fn constructor(hit:usize,row:usize,context:usize,flag:u8,value:i32,token:usize,rate:f32)->u64{
 assert_ne!(context,0);assert_eq!((flag,value,token,rate),(173,-249,0x12345678,1.25));
 let copy:unsafe extern "system" fn(usize,usize)=std::mem::transmute(COPY.with(Cell::get));copy(hit,row);
 ptr::write_unaligned((hit+0x280)as*mut u32,0x12345678);0xabcdef0123456789
}
fn row(bytes:&[u8],id:i32)->&[u8]{
 let count=u16::from_le_bytes(bytes[10..12].try_into().unwrap())as usize;
 for i in 0..count{let at=64+i*24;if i32::from_le_bytes(bytes[at..at+4].try_into().unwrap())==id{let offset=u64::from_le_bytes(bytes[at+8..at+16].try_into().unwrap())as usize;return &bytes[offset..offset+0x80];}}
 panic!("Missing attack {id}")
}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE and NRHEX_MARKING_FIXTURES; run alone"]
fn native_marking_constructor_uses_upward_cut_and_retains_copies(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let params=std::fs::read(std::env::var_os("NRHEX_MARKING_FIXTURES").unwrap()).unwrap();let marking=row(&params,ATTACK);let stamp=row(&params,300000290);assert_eq!(marking[0x72],1);assert_eq!(stamp[0x72],IMPACT);assert_eq!(row(&params,600000120)[0x72],0);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 // Run the original E709D4 -> E709D8 -> E709E8 instructions, supplying only
 // the constructor's source-row and output registers in a fixture prologue.
 let entry=0xe70800;let mut code=vec![0x56,0x48,0x89,0xce,0x48,0x89,0xd1,0xe9];code.extend_from_slice(&((0xe709d4i64-(entry+12)as i64)as i32).to_le_bytes());ptr::copy_nonoverlapping(code.as_ptr(),(base+entry)as*mut u8,code.len());ptr::copy_nonoverlapping([0x5e,0xc3].as_ptr(),(base+0xe709eb)as*mut u8,2);COPY.with(|s|s.set(base+entry));
 let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&(has as*const()as usize).to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(jump.as_ptr(),(base+crate::HAS_EFFECT_RVA)as*mut u8,jump.len());
 let saved_base=crate::BASE.swap(base,Ordering::AcqRel);let saved_active=crate::ACTIVE.swap(true,Ordering::AcqRel);let saved_original=crate::HIT_BUILD_ORIG.swap(constructor as*mut u8,Ordering::AcqRel);
 let mut world=vec![0usize;0x17500/8];let mut local=[0usize;0x200/8];let player=local.as_mut_ptr()as usize;local[0x190/8]=1;world[0x174e8/8]=player;ptr::write_unaligned((base+crate::WORLD_RVA)as*mut usize,world.as_ptr()as usize);
 let call:unsafe extern "system" fn(usize,usize,usize,u8,i32,usize,f32)->u64=std::mem::transmute(crate::strength_detour as*const());
 for owned in [false,true]{OWNED.with(|s|s.set(owned));for mighty in [false,true]{MIGHTY.with(|s|s.set(mighty));for local in [false,true]{for table in [0,1]{for attack in [ATTACK,600000120,300000290]{
  let native=row(&params,attack);let mut context=[0usize;0x120/8];context[0]=if local{player}else{player+8};ptr::write_unaligned((context.as_mut_ptr()as usize+8)as*mut i32,attack);ptr::write_unaligned((context.as_mut_ptr()as usize+12)as*mut i32,table);
  let mut hit=[0u8;0x300];let h=hit.as_mut_ptr()as usize;let context=context.as_mut_ptr()as usize;
  assert_eq!(call(h,native.as_ptr()as usize,context,173,-249,0x12345678,1.25),0xabcdef0123456789);
  let expected=if local&&owned&&table==1&&attack==ATTACK{IMPACT}else if local&&mighty{crate::strength::remap(native[0x72])}else{native[0x72]};assert_eq!(hit[0x24],expected);assert_eq!(crate::read_i32(h+0x280),0x12345678);
  // Native cached/network copies keep the constructed input, independently
  // of later ownership changes; no target mapping is overwritten here.
  let copied=hit;OWNED.with(|s|s.set(!owned));assert_eq!(copied[0x24],expected);OWNED.with(|s|s.set(owned));assert_eq!(hit[0x2b6],0);
 }}}}}
 assert_eq!(marking[0x72],1);assert_eq!(stamp[0x72],9);
 crate::HIT_BUILD_ORIG.store(saved_original,Ordering::Release);crate::BASE.store(saved_base,Ordering::Release);crate::ACTIVE.store(saved_active,Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
