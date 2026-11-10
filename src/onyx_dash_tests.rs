//! Exercise the ownership/hand gate with the actual ground-state routine.
use super::*;
use crate::{ptr,win::*};
use std::sync::atomic::{AtomicI32,AtomicBool};
static OWNS:AtomicBool=AtomicBool::new(true);
static RIGHT:AtomicI32=AtomicI32::new(8010000);
static LEFT:AtomicI32=AtomicI32::new(8010000);
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from(id==OWNER&&OWNS.load(Ordering::Relaxed))}
unsafe extern "system" fn get(_:usize,source:i32)->i32{match source{-1=>RIGHT.load(Ordering::Relaxed),-2=>LEFT.load(Ordering::Relaxed),_=>-1}}
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; execute alone"]
fn native_dash_gate_rejects_other_hands_weapons_death_and_airborne(){unsafe{
 let path=std::path::PathBuf::from(std::env::var_os("NRHEX_NATIVE_EXE").unwrap());
 let raw=std::fs::read(path).unwrap();use sha2::Digest;assert_eq!(format!("{:x}",sha2::Sha256::digest(&raw)),crate::EXE_HASH);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();
 let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for s in &headers.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 let mut jump=vec![0xff,0x25,0,0,0,0];jump.extend_from_slice(&(has as*const()as usize).to_le_bytes());ptr::copy_nonoverlapping(jump.as_ptr(),(base+crate::HAS_EFFECT_RVA)as*mut u8,jump.len());
 let mut actor=[0usize;0x800/8];let a=actor.as_mut_ptr()as usize;let mut stats=[0u64;0x200/8];let st=stats.as_mut_ptr()as usize;
 let mut action=[0u64;0x300/8];let mut physics=[0u64;0x800/8];let ph=physics.as_mut_ptr()as usize;let mut modules=[0usize;0x200/8];modules[0]=st;modules[1]=action.as_mut_ptr()as usize;modules[0x68/8]=ph;put(a+0x1b8,modules.as_ptr()as usize);
 let mut equipment=1i32;put(a+0x670,&mut equipment as*mut i32 as usize);
 let mut vt=[0usize;0x260/8];vt[0x250/8]=get as*const()as usize;put(a,vt.as_ptr()as usize);
 let mut world=vec![0usize;0x17500/8];world[0x174e8/8]=a;put(base+crate::WORLD_RVA,world.as_ptr()as usize);
 let previous_base=crate::BASE.swap(base,Ordering::AcqRel);let previous_active=crate::ACTIVE.swap(true,Ordering::AcqRel);
 // Native airborne checks need the jump-controller's owner and an idle row.
 put(a+0x190,a);put(ph+8,a);put(ph+0x420,a);int(ph+0x414,-1);int(st+0x140,100);ptr::write((ph+0x92)as*mut u8,1);
 let mut cases=0;
 for style in [0,1,2,3]{for right in [8010000,8010025,2000000,-1]{for owns in [false,true]{for alive in [false,true]{
  int(&mut equipment as*mut i32 as usize,style);RIGHT.store(right,Ordering::Relaxed);OWNS.store(owns,Ordering::Relaxed);int(st+0x140,if alive{100}else{0});
  assert_eq!(dash_ready(a,base),style==1&&weapon(right)&&owns&&alive,"style={style} right={right} owns={owns} alive={alive}");cases+=1;
 }}}}
 int(&mut equipment as*mut i32 as usize,1);RIGHT.store(8010000,Ordering::Relaxed);OWNS.store(true,Ordering::Relaxed);int(st+0x140,100);
 ptr::write((ph+0x92)as*mut u8,0);assert!(!dash_ready(a,base));cases+=1;
 ptr::write((ph+0x92)as*mut u8,1);world[0x174e8/8]=0;assert!(!dash_ready(a,base));cases+=1;
 crate::BASE.store(previous_base,Ordering::Release);crate::ACTIVE.store(previous_active,Ordering::Release);
 assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);println!("PASS: {cases} native-ground/ownership/hand/weapon dash gate cases");
}}
