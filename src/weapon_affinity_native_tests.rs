//! Execute native reward-context selection and SpEffect override traversal.
//! Only HeroParam lookup/release are fixtures; this never attaches to a game.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
use std::cell::Cell;
thread_local!{static HERO:Cell<usize>=const{Cell::new(0)};static LOOKUPS:Cell<usize>=const{Cell::new(0)};}
unsafe fn put(p:usize,value:usize){ptr::write_unaligned(p as*mut usize,value);}
unsafe fn int(p:usize,value:i32){ptr::write_unaligned(p as*mut i32,value);}
unsafe extern "system" fn lookup(out:usize,hero:u8)->usize{
 LOOKUPS.with(|n|n.set(n.get()+1));let rows=HERO.with(Cell::get);
 put(out+8,if rows==0||!(1..=10).contains(&hero){0}else{rows+usize::from(hero)*0xc0});out
}
unsafe fn jump(base:usize,rva:usize,target:usize){
 let mut code=vec![0x48,0xb8];code.extend_from_slice(&target.to_le_bytes());code.extend_from_slice(&[0xff,0xe0]);
 ptr::copy_nonoverlapping(code.as_ptr(),(base+rva)as*mut u8,code.len());
}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; run alone"]
fn native_weapon_affinity_follows_relic_overrides_and_hero_defaults(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let allocation=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!allocation.is_null());let base=allocation as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 jump(base,0xe785b0,lookup as*const()as usize);ptr::write((base+0xe782e0)as*mut u8,0xc3);
 let mut defaults=vec![0u8;11*0xc0];let dp=defaults.as_mut_ptr()as usize;
 for hero in 1..=10{int(dp+hero*0xc0+0x98,1000+hero as i32*100);}
 HERO.with(|h|h.set(dp));LOOKUPS.with(|n|n.set(0));
 let mut actor=[0usize;0x200/8];let player=actor.as_mut_ptr()as usize;
 let mut manager=[0usize;4];put(player+0x190,manager.as_mut_ptr()as usize);
 let mut nodes=[[0usize;0x70/8];4];let mut params=[[0u64;0x400/8];4];
 for i in 0..4{nodes[i][0]=params[i].as_mut_ptr()as usize;if i<3{nodes[i][0x38/8]=nodes[i+1].as_mut_ptr()as usize;}}
 let node=nodes[0].as_mut_ptr()as usize;
 let addresses:Vec<_>=params.iter_mut().map(|p|p.as_mut_ptr()as usize).collect();
 let set=|i:usize,state:u16,value:i32|{let p=addresses[i];ptr::write_unaligned((p+0x156)as*mut u16,state);int(p+0x3e4,value);};
 for hero in 1..=10{assert_eq!(current(player,base,Some(hero)),Some(1000+i32::from(hero)*100));}
 assert!(!is_caster(current(player,base,Some(4)).unwrap())); // Duchess defaults to daggers.
 assert!(is_caster(current(player,base,Some(6)).unwrap()));assert!(is_caster(current(player,base,Some(7)).unwrap()));
 // The first native state 2122 wins, regardless of the underlying hero.
 manager[1]=node;set(0,2123,57);set(1,2122,57);set(2,2122,61);set(3,0,-1);
 for hero in 1..=10{assert_eq!(current(player,base,Some(hero)),Some(57));}
 set(1,2122,61);assert_eq!(current(player,base,Some(1)),Some(61));
 for category in [1,3,5,7,9,11,13,14,15,16,17,19,21,23,24,25,28,29,31,35,37,39,41,51,53,55,56,57,61,65,67,69,87]{
  set(1,2122,category);assert_eq!(current(player,base,Some(7)),Some(category));
  assert_eq!(is_caster(category),matches!(category,57|61));
 }
 // Null rows are skipped. A first override of -1 uses the hero fallback,
 // rather than trying a later conflicting override (exact native behavior).
 nodes[0][0]=0;set(1,2122,-1);assert_eq!(current(player,base,Some(7)),Some(1700));
 set(1,0,61);set(2,2122,61);assert_eq!(current(player,base,Some(1)),Some(61));
 HERO.with(|h|h.set(0));assert_eq!(current(player,base,Some(1)),Some(61));
 manager[1]=0;assert_eq!(current(player,base,Some(1)),None);HERO.with(|h|h.set(dp));
 // Query writes only context+10, and never modifies actor/effect state.
 let actor_before=actor;let manager_before=manager;
 let mut context=[0xabu8;0x30];let query:unsafe extern "system" fn(usize,u8,usize)=std::mem::transmute(base+QUERY_RVA);
 query(context.as_mut_ptr()as usize,6,player);assert_eq!(i32::from_le_bytes(context[0x10..0x14].try_into().unwrap()),1600);
 assert!(context[..0x10].iter().chain(context[0x14..].iter()).all(|&v|v==0xab));assert_eq!(actor,actor_before);assert_eq!(manager,manager_before);
 let before=LOOKUPS.with(Cell::get);
 for hero in [None,Some(0),Some(11),Some(255)]{assert_eq!(current(player,base,hero),None);}
 assert_eq!(current(0,base,Some(1)),None);assert_eq!(current(player,0,Some(1)),None);
 put(player+0x190,0);assert_eq!(current(player,base,Some(1)),None);assert_eq!(LOOKUPS.with(Cell::get),before);
 assert_ne!(VirtualFree(allocation,0,MEM_RELEASE),0);
}}
