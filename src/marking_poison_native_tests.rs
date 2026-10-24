//! Actual native state152 collection and applicability, shared compiled apply
//! and HKS query hooks; actor storage and final effect submission are fixtures.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
use std::{cell::{Cell,RefCell},sync::atomic::Ordering};
thread_local!{static OWNED:Cell<bool>=const{Cell::new(true)};static RESET:Cell<bool>=const{Cell::new(false)};static MODE:Cell<f32>=const{Cell::new(1.)};static CALLS:RefCell<Vec<(usize,i32,usize)>>=const{RefCell::new(Vec::new())};}
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn patch(b:usize,r:usize,v:&[u8]){ptr::copy_nonoverlapping(v.as_ptr(),(b+r)as*mut u8,v.len());}
unsafe fn jump(b:usize,r:usize,f:usize){let mut v=vec![0x48,0xb8];v.extend_from_slice(&f.to_le_bytes());v.extend_from_slice(&[0xff,0xe0]);patch(b,r,&v);}
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from(id==OWNER&&OWNED.with(Cell::get)||id==crate::reward_cycle_runtime::RESET&&RESET.with(Cell::get))}
unsafe extern "system" fn kind(_:usize)->i32{1}
unsafe extern "system" fn number(_:usize,i:i32)->f32{assert_eq!(i,2);MODE.with(Cell::get)}
unsafe extern "system" fn received(t:usize,e:i32,s:usize,a:u8,b:usize,c:usize,d:u8,f:u8,g:u8,h:u8,i:u8,j:u8,k:i32)->u8{
 assert_eq!((a,b,d,f,g,h,i,j,k),(0x81,0x12345678,0x82,0x83,0x84,0x85,0x86,0x87,-99));assert_ne!(c,0);CALLS.with(|v|v.borrow_mut().push((t,e,s)));0x91
}
fn effect(bytes:&[u8],id:i32)->Vec<u8>{
 let n=u16::from_le_bytes(bytes[10..12].try_into().unwrap())as usize;
 for i in 0..n{let at=64+24*i;if i32::from_le_bytes(bytes[at..at+4].try_into().unwrap())==id{let off=u64::from_le_bytes(bytes[at+8..at+16].try_into().unwrap())as usize;return bytes[off..off+0x400].to_vec();}}panic!("Missing effect {id}")
}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE and NRHEX_MARKING_POISON_FIXTURES; run alone"]
fn native_marking_followup_targets_and_original_collector_are_scoped(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let params=std::fs::read(std::env::var_os("NRHEX_MARKING_POISON_FIXTURES").unwrap()).unwrap();let owner=effect(&params,OWNER);let burst=effect(&params,BURST);let native=effect(&params,7500202);
 // The byte offset comes from this installed parameter's def. Resource audit
 // separately proves every non-condition field is an exact native clone.
 assert_ne!(burst,native);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let mem=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!mem.is_null());let base=mem as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 patch(base,0x26909a0,&[0xc3]);jump(base,crate::HAS_EFFECT_RVA,has as*const()as usize);jump(base,crate::ambulance::NUMBER_RVA,number as*const()as usize);
 let saved_base=crate::BASE.swap(base,Ordering::AcqRel);let saved_active=crate::ACTIVE.swap(true,Ordering::AcqRel);let saved_apply=crate::erosion::ORIGINAL.swap(received as*mut u8,Ordering::AcqRel);
 let mut world=vec![0usize;0x17500/8];put(base+crate::WORLD_RVA,world.as_mut_ptr()as usize);
 let mut a=[0usize;0x720/8];let player=a.as_mut_ptr()as usize;world[0x174e8/8]=player;put(player+8,42);
 let mut modules=[0usize;0x200/8];let mut stats=[0usize;0x200/8];put(player+0x1b8,modules.as_mut_ptr()as usize);modules[0]=stats.as_mut_ptr()as usize;int(modules[0]+0x140,100);
 let mut target=[0usize;0x720/8];let target=target.as_mut_ptr()as usize;let mut other=[0usize;0x720/8];let other=other.as_mut_ptr()as usize;
 let mut vt=[0usize;4];vt[2]=kind as*const()as usize;let mut tm=[0usize;8];let mut ts=[0usize;0x200/8];tm[0]=ts.as_mut_ptr()as usize;int(tm[0]+0x140,1000);
 for(t,h)in[(target,51),(other,52)]{put(t,vt.as_ptr()as usize);put(t+8,h);put(t+0x1b8,tm.as_ptr()as usize);}
 let mut node=[0usize;0x80/8];node[0]=owner.as_ptr()as usize;let mut manager=[0usize;8];manager[1]=node.as_ptr()as usize;put(player+0x190,manager.as_mut_ptr()as usize);
 let filter=crate::hook::prepare((base+crate::EFFECT_FILTER_RVA)as*mut u8,crate::sword_filter_detour as*const u8,64).unwrap();let saved_filter=crate::EFFECT_FILTER_ORIG.swap(filter.trampoline,Ordering::AcqRel);crate::hook::write_code((base+crate::EFFECT_FILTER_RVA)as*mut u8,&filter.patch).unwrap();
 let saved_collect=crate::erosion::COLLECT_ORIGINAL.swap((base+crate::erosion::COLLECT_RVA)as*mut u8,Ordering::AcqRel);
 let mut desc=[0u8;0x48];let d=desc.as_mut_ptr()as usize;desc[0]=1;int(d+0x38,1);
 let mut values=[0i32;16];let start=values.as_mut_ptr()as usize;let mut vector=[0usize,start,start,start+64];
 let collect=|attack:i32,backstep:bool,values:&mut[i32;16],vector:&mut[usize;4]|{int(d+0x34,attack);ptr::write((d+20)as*mut u8,u8::from(backstep)*2);values.fill(0);values[0]=12345;vector[2]=start+4;crate::erosion::collect(manager.as_ptr()as usize,vector.as_mut_ptr()as usize,d);};
 collect(ATTACK,false,&mut values,&mut vector);assert_eq!(vector[2],start+8);assert_eq!(&values[..2],&[12345,MARK_HIT]);
 collect(10000,false,&mut values,&mut vector);assert_eq!(vector[2],start+4);
 collect(10000,true,&mut values,&mut vector);assert_eq!(vector[2],start+8);assert_eq!(values[1],CARRIER);
 desc[0]=2;desc[4+105/8]|=1<<(105%8);collect(10000,true,&mut values,&mut vector);assert_eq!(values[1],CARRIER);desc[0]=1;desc[4+105/8]=0;
 for attack in [304,600000120]{collect(attack,false,&mut values,&mut vector);assert_eq!(vector[2],start+4);}
 let apply=|t,e|crate::erosion::detour(t,e,player,0x81,0x12345678,d,0x82,0x83,0x84,0x85,0x86,0x87,-99);
 let context=[player];let query=|mode:i32|{MODE.with(|v|v.set(mode as f32));assert_eq!(crate::ambulance::env(context.as_ptr()as usize,QUERY,1),0);};
 // Execute the real HKS query dispatcher for both start variants. Generic
 // transitions (including landing) and the TAE pulse do not change origin.
 for mode in [2,12,1,11,2]{
  MODE.with(|v|v.set(mode as f32));assert_eq!(crate::ambulance::env(context.as_ptr()as usize,ORIGIN_QUERY,1),0);
  assert_eq!(air_started(player),matches!(mode,2|12));
  query(0);pulse(player,BEGIN,base);assert_eq!(air_started(player),matches!(mode,2|12));
  MODE.with(|v|v.set(0.));crate::ambulance::env(context.as_ptr()as usize,ORIGIN_QUERY,1);assert_eq!(air_started(player),matches!(mode,2|12));
 }
 for mode in 1..=4{
  reset();assert!(pulse(player,BEGIN,base));assert_eq!(apply(target,MARK_HIT),0);query(mode);query(mode+10);
  CALLS.with(|v|v.borrow_mut().clear());assert_eq!(apply(target,CARRIER),0x91);assert_eq!(apply(target,CARRIER),0);CALLS.with(|v|assert_eq!(&*v.borrow(),&[(target,BURST,player)]));
  assert!(pulse(player,BEGIN,base));apply(target,MARK_HIT);query(mode);CALLS.with(|v|v.borrow_mut().clear());assert_eq!(apply(other,CARRIER),0);assert_eq!(apply(target,CARRIER),0);CALLS.with(|v|assert!(v.borrow().is_empty()));
  assert!(pulse(player,BEGIN,base));apply(target,MARK_HIT);query(mode);query(0);query(mode);assert_eq!(apply(target,CARRIER),0);
 }
 // New slide misses; old targets are not eligible. No poison state is installed.
 pulse(player,BEGIN,base);apply(target,MARK_HIT);pulse(player,BEGIN,base);query(1);assert_eq!(apply(target,CARRIER),0);
 pulse(player,BEGIN,base);apply(target,MARK_HIT);query(1);put(target+8,999);assert_eq!(apply(target,CARRIER),0);put(target+8,51);
 for(owned,resetting,hp,down)in[(false,false,100,0),(true,true,100,0),(true,false,0,0),(true,false,100,1),(true,false,100,2)]{
  OWNED.with(|v|v.set(true));RESET.with(|v|v.set(false));int(modules[0]+0x140,100);ptr::write((player+0x710)as*mut u8,0);pulse(player,BEGIN,base);apply(target,MARK_HIT);query(1);
  OWNED.with(|v|v.set(owned));RESET.with(|v|v.set(resetting));int(modules[0]+0x140,hp);ptr::write((player+0x710)as*mut u8,down);assert_eq!(apply(target,CARRIER),0);update(player,base);
 }
 OWNED.with(|v|v.set(true));RESET.with(|v|v.set(false));int(modules[0]+0x140,100);ptr::write((player+0x710)as*mut u8,0);
 // Non-private effects and inactive runtime retain all thirteen arguments.
 assert_eq!(apply(target,7500202),0x91);crate::ACTIVE.store(false,Ordering::Release);assert_eq!(apply(target,CARRIER),0x91);
 reset();crate::EFFECT_FILTER_ORIG.store(saved_filter,Ordering::Release);crate::erosion::COLLECT_ORIGINAL.store(saved_collect,Ordering::Release);crate::erosion::ORIGINAL.store(saved_apply,Ordering::Release);crate::BASE.store(saved_base,Ordering::Release);crate::ACTIVE.store(saved_active,Ordering::Release);assert_ne!(VirtualFree(mem,0,MEM_RELEASE),0);
}}
