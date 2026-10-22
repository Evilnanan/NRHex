//! Execute native range aggregation and the compiled shared damage callback.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
use std::{cell::Cell,sync::atomic::Ordering};
thread_local!{static OWNED:Cell<bool>=const{Cell::new(false)};static RESETTING:Cell<bool>=const{Cell::new(false)};}
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from(id==OWNER&&OWNED.with(Cell::get)||id==crate::reward_cycle_runtime::RESET&&RESETTING.with(Cell::get))}
fn row(bytes:&[u8],id:i32)->&[u8]{
 let count=u16::from_le_bytes(bytes[10..12].try_into().unwrap())as usize;
 for i in 0..count{let at=64+i*24;if i32::from_le_bytes(bytes[at..at+4].try_into().unwrap())==id{let offset=u64::from_le_bytes(bytes[at+8..at+16].try_into().unwrap())as usize;return &bytes[offset..offset+0x350];}}
 panic!("Missing effect {id}")
}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE and NRHEX_SNIPER_FIXTURE; run alone"]
fn native_sniper_range_and_shared_damage_scope(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let params=std::fs::read(std::env::var_os("NRHEX_SNIPER_FIXTURE").unwrap()).unwrap();let effect=row(&params,OWNER);let native=row(&params,321000);
 assert_eq!(i16::from_le_bytes(effect[0x13c..0x13e].try_into().unwrap()),200);
 for offset in [0x38,0x3c,0x40,0x44,0x1dc]{assert_eq!(f32::from_le_bytes(effect[offset..offset+4].try_into().unwrap()),0.7);}
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 let patch=|rva:usize,bytes:&[u8]|ptr::copy_nonoverlapping(bytes.as_ptr(),(base+rva)as*mut u8,bytes.len());
 patch(0x26909a0,&[0xc3]);
 let mut manager=[0usize;16];let mut first=[0usize;16];let mut second=[0usize;16];manager[1]=first.as_mut_ptr()as usize;first[0]=effect.as_ptr()as usize;second[0]=native.as_ptr()as usize;
 let get:unsafe extern "system" fn(usize)->f32=std::mem::transmute(base+0x5065c0);
 assert_eq!(get(manager.as_ptr()as usize),2.);first[0x38/8]=second.as_mut_ptr()as usize;assert_eq!(get(manager.as_ptr()as usize),2.5);first[0x38/8]=0;
 let mut weapon=[0u64;0x200/8];let mut lookup=vec![0x48,0xb8];lookup.extend_from_slice(&(weapon.as_mut_ptr()as usize).to_le_bytes());lookup.extend_from_slice(&[0x48,0x89,0x41,0x08,0xc3]);patch(0xea7fc0,&lookup);patch(0xea9a60,&[0xc3]);
 let mut bullet=[0u64;0x200/8];let mut wrapper=[0usize;4];wrapper[0]=bullet.as_mut_ptr()as usize;
 let range:unsafe extern "system" fn(usize,i32,usize)->f32=std::mem::transmute(base+0xe875a0);
 for (distance,expected)in [(12f32,36f32),(43.,129.)]{ptr::write_unaligned((wrapper[0]+0x14)as*mut f32,distance);assert_eq!(range(wrapper.as_ptr()as usize,42040000,manager.as_ptr()as usize),expected);}
 let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&(has as*const()as usize).to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);patch(crate::HAS_EFFECT_RVA,&jump);
 let saved_base=crate::BASE.swap(base,Ordering::AcqRel);let saved_active=crate::ACTIVE.swap(true,Ordering::AcqRel);
 let mut world=vec![0usize;0x17500/8];let mut local=[0usize;0x200/8];let player=local.as_mut_ptr()as usize;local[0x190/8]=1;local[0x60/8]=1;world[0x174e8/8]=player;ptr::write_unaligned((base+crate::WORLD_RVA)as*mut usize,world.as_ptr()as usize);ptr::write_unaligned((base+crate::GAME_DATA_RVA)as*mut usize,0);
 let mut modules=[0usize;0x180/8];let mut physics=[0u64;0x90/8];local[0x1b8/8]=modules.as_mut_ptr()as usize;modules[0x68/8]=physics.as_mut_ptr()as usize;
 let mut target=[0usize;0x200/8];let target=target.as_mut_ptr()as usize;let mut module=[0usize;2];module[1]=target;
 for owned in [false,true]{OWNED.with(|v|v.set(owned));for reset in [false,true]{RESETTING.with(|v|v.set(reset));for cached in [false,true]{for attack in [40000,40210,600000110,5100850,5100855,5100880,5100885]{for distance in [0f32,10.,25.,50.,100.]{for source in [0,player,player+8]{
  ptr::write_unaligned((physics.as_mut_ptr()as usize+0x70)as*mut[f32;4],[10.,20.,30.,1.]);
  let mut hit=[0u8;0x300];let h=hit.as_mut_ptr()as usize;ptr::write_unaligned((h+0x40)as*mut i32,attack);ptr::write_unaligned((h+0x44)as*mut i32,1);
  ptr::write_unaligned((h+0x1e0)as*mut[f32;4],[1e6,1e6,1e6,1.]);ptr::write_unaligned((h+0x1f0)as*mut[f32;4],[10.+distance,20.,30.,1.]);
  ptr::write_unaligned((h+0x280)as*mut i32,700);hit[0x2c1]=if cached{8}else{0};let before=hit;
  crate::on_rear_calculated(module.as_ptr()as usize,source,h);
  let expected=if owned&&!reset&&!cached&&source==player{crate::sword::boosted(700,multiplier(Some(distance),attack,1))}else{700};assert_eq!(read_i32(h+0x280),expected);
  let mut comparison=hit;comparison[0x280..0x284].copy_from_slice(&before[0x280..0x284]);assert_eq!(comparison,before,"Only HP damage may change");
 }}}}}}
 // The point and current player position determine distance, not target origin.
 OWNED.with(|v|v.set(true));RESETTING.with(|v|v.set(false));let mut hit=[0u8;0x300];let h=hit.as_mut_ptr()as usize;ptr::write_unaligned((h+0x280)as*mut i32,700);ptr::write_unaligned((h+0x1f0)as*mut[f32;4],[60.,20.,30.,1.]);
 ptr::write_unaligned((physics.as_mut_ptr()as usize+0x70)as*mut[f32;4],[35.,20.,30.,1.]);crate::on_rear_calculated(module.as_ptr()as usize,player,h);assert_eq!(read_i32(h+0x280),1225);
 for damage in [0,-1]{ptr::write_unaligned((h+0x280)as*mut i32,damage);crate::on_rear_calculated(module.as_ptr()as usize,player,h);assert_eq!(read_i32(h+0x280),damage);}
 ptr::write_unaligned((h+0x280)as*mut i32,700);module[1]=player;crate::on_rear_calculated(module.as_ptr()as usize,player,h);assert_eq!(read_i32(h+0x280),700);
 module[1]=target;local[0x60/8]=0;crate::on_rear_calculated(module.as_ptr()as usize,player,h);assert_eq!(read_i32(h+0x280),700);
 crate::BASE.store(saved_base,Ordering::Release);crate::ACTIVE.store(saved_active,Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
