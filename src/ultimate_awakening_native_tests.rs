//! Compiled successful-ultimate routing plus original native matching,
//! lifetime refresh and FP multiplier aggregation over installed effect rows.
use super::*;
use crate::ptr;use crate::win::*;
use sha2::{Digest,Sha256};
use std::{cell::Cell,sync::atomic::Ordering};
thread_local!{
 static OWNED:Cell<bool>=const{Cell::new(true)};static RESET:Cell<bool>=const{Cell::new(false)};
 static ACCEPT:Cell<u8>=const{Cell::new(1)};static FAIL:Cell<bool>=const{Cell::new(false)};
 static IMAGE:Cell<usize>=const{Cell::new(0)};static NODE:Cell<usize>=const{Cell::new(0)};
 static APPLICATIONS:Cell<usize>=const{Cell::new(0)};static FORWARDS:Cell<usize>=const{Cell::new(0)};
}
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn float(p:usize,v:f32){ptr::write_unaligned(p as*mut f32,v);}
unsafe fn value(p:usize)->f32{ptr::read_unaligned(p as*const f32)}
unsafe fn patch(base:usize,rva:usize,b:&[u8]){ptr::copy_nonoverlapping(b.as_ptr(),(base+rva)as*mut u8,b.len());}
unsafe fn jump(base:usize,rva:usize,f:usize){let mut b=vec![0x48,0xb8];b.extend_from_slice(&f.to_le_bytes());b.extend_from_slice(&[0xff,0xe0]);patch(base,rva,&b);}
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from((id==OWNER&&OWNED.with(Cell::get))||(id==crate::reward_cycle_runtime::RESET&&RESET.with(Cell::get)))}
unsafe extern "system" fn original(_:usize,id:i32,_:usize,_:usize,_:usize,_:u8,_:u8,_:u8,_:u8,_:u8,_:i32)->u8{FORWARDS.with(|s|s.set(s.get()+1));if id==crate::runtime::ULTIMATE_TRIGGER{ACCEPT.with(Cell::get)}else{1}}
unsafe extern "system" fn apply(player:usize,id:i32,flag:u8)->u8{
 assert_eq!((id,flag),(BUFF,0));APPLICATIONS.with(|s|s.set(s.get()+1));if FAIL.with(Cell::get){return 0;}
 // Exercise nested dispatch: applying our own buff must not activate again.
 assert_eq!(crate::effect_detour(player,id,player,0,0,0,0,0,0,0,0),1);
 let base=IMAGE.with(Cell::get);let node=NODE.with(Cell::get);
 let matching:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+0x510b20);
 let category:unsafe extern "system" fn(usize,u16,u8,i32)->u8=std::mem::transmute(base+0x511450);
 assert_eq!(matching(node,BUFF),1);assert_eq!(category(node,20,0,BUFF),1);
 let refresh:unsafe extern "system" fn(usize,i32,f32,u8)->u8=std::mem::transmute(base+0x511700);
 refresh(node,BUFF,1.,0)
}
fn row(bytes:&[u8],id:i32)->[u8;1024]{
 let count=u16::from_le_bytes(bytes[10..12].try_into().unwrap())as usize;
 for i in 0..count{let at=64+i*24;if i32::from_le_bytes(bytes[at..at+4].try_into().unwrap())==id{let off=u64::from_le_bytes(bytes[at+8..at+16].try_into().unwrap())as usize;return bytes[off..off+1024].try_into().unwrap();}}
 panic!("Missing installed effect {id}")
}
#[test]fn awakening_is_available_for_all_heroes_affinities_and_party_sizes(){
 let index=crate::hex_pool::OWNER_IDS.iter().position(|&id|id==OWNER).unwrap();
 for hero in std::iter::once(None).chain((0..=10).map(Some)){for affinity in [None,Some(1100),Some(1600),Some(1700),Some(57),Some(61)]{for party in [None,Some(1),Some(2),Some(3)]{
  let filtered=crate::reward_choices::pool_exclusions(crate::reward_choices::party_exclusions(crate::reward_choices::weapon_exclusions(crate::reward_choices::role_exclusions(Default::default(),hero),affinity),party));
  assert!(!filtered.contains(&index));let round=crate::reward_cycle::Round::new(&filtered,&Default::default());assert!(round.candidates.contains(&index));
  assert!(round.preferred([index,0,1]).unwrap().contains(&index));assert!(crate::reward_cycle::Single::new(&filtered,&Default::default(),&crate::reward_choices::mask([0,1])).candidates.contains(&index));
 }}}
 assert_eq!(crate::hex_pool::HISTORY_BITS[index],35);
}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE and NRHEX_AWAKENING_FIXTURES; run alone"]
fn native_ultimate_awakening_success_refresh_and_fp_aggregation(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let params=std::fs::read(std::env::var_os("NRHEX_AWAKENING_FIXTURES").unwrap()).unwrap();let buff=row(&params,BUFF);let tear=row(&params,511060);let echo=row(&params,crate::echo_cast::OWNER);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 // Row lookup is a fixture: 511700's actual lifetime/flags branch executes.
 patch(base,0xea17e0,&[0xc3]);jump(base,crate::HAS_EFFECT_RVA,has as*const()as usize);jump(base,crate::APPLY_SELF_RVA,apply as*const()as usize);
 let saved_base=crate::BASE.swap(base,Ordering::AcqRel);let saved_active=crate::ACTIVE.swap(true,Ordering::AcqRel);let saved_original=crate::APPLY_ORIG.swap(original as*mut u8,Ordering::AcqRel);IMAGE.with(|s|s.set(base));
 let mut world=vec![0usize;0x17500/8];let world=world.as_mut_ptr()as usize;put(base+crate::WORLD_RVA,world);
 let mut a=[0usize;0x720/8];let player=a.as_mut_ptr()as usize;put(world+0x174e8,player);
 let mut modules=[0usize;0x180/8];let modules=modules.as_mut_ptr()as usize;put(player+0x1b8,modules);let mut stats=[0usize;0x180/8];let stats=stats.as_mut_ptr()as usize;put(modules,stats);int(stats+0x140,100);int(stats+0x150,42);
 let mut manager=[0usize;8];let manager=manager.as_mut_ptr()as usize;put(player+0x190,manager);
 let mut n=[0usize;0x88/8];let node=n.as_mut_ptr()as usize;put(node,buff.as_ptr()as usize);int(node+8,BUFF);put(manager+8,node);NODE.with(|s|s.set(node));
 let mut en=[0usize;0x88/8];let echo_node=en.as_mut_ptr()as usize;put(echo_node,echo.as_ptr()as usize);int(echo_node+8,crate::echo_cast::OWNER);put(node+0x38,echo_node);
 let mut tn=[0usize;0x88/8];let tear_node=tn.as_mut_ptr()as usize;put(tear_node,tear.as_ptr()as usize);int(tear_node+8,511060);float(tear_node+0x4c,7.);
 let cast=|id,actor|crate::effect_detour(actor,id,actor,0,0,0,0,0,0,0,0);
 float(node+0x4c,0.);assert_eq!(cast(12345,player),1);cast(crate::runtime::ULTIMATE_ACTION,player);assert_eq!(APPLICATIONS.with(Cell::get),0);
 ACCEPT.with(|s|s.set(0));assert_eq!(cast(crate::runtime::ULTIMATE_TRIGGER,player),0);assert_eq!(APPLICATIONS.with(Cell::get),0);ACCEPT.with(|s|s.set(1));
 assert_eq!(cast(crate::runtime::ULTIMATE_TRIGGER,player),1);assert_eq!(APPLICATIONS.with(Cell::get),1);assert_eq!(value(node+0x4c),10.);assert_eq!(crate::read_i32(stats+0x150),42);
 for remaining in [9.5,1.,0.001]{float(node+0x4c,remaining);cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(value(node+0x4c),10.);}
 // Installed native cost rows, including Echo Casting's x2, aggregate through
 // the original functions. Expiry is represented by native inactive-node flags.
 for(rva,with_echo)in [(0x5061b0,1.),(0x507310,2.),(0x5081e0,1.),(0x507410,2.)]{
  let aggregate:unsafe extern "system" fn(usize)->f32=std::mem::transmute(base+rva);
  int(node+0x68,0);assert_eq!(aggregate(manager),0.);int(node+0x68,2);assert_eq!(aggregate(manager),with_echo);
  put(echo_node+0x38,tear_node);assert_eq!(aggregate(manager),0.);put(echo_node+0x38,0);assert_eq!(value(tear_node+0x4c),7.);
 }
 let matching:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+0x510b20);int(node+0x68,0);assert_eq!(matching(node,511060),0);assert_eq!(matching(tear_node,BUFF),0);
 for(owned,reset,hp,down)in [(false,false,100,0),(true,true,100,0),(true,false,0,0),(true,false,100,1),(true,false,100,2)]{
  OWNED.with(|s|s.set(owned));RESET.with(|s|s.set(reset));int(stats+0x140,hp);ptr::write((player+0x710)as*mut u8,down);let count=APPLICATIONS.with(Cell::get);cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(APPLICATIONS.with(Cell::get),count);
 }
 OWNED.with(|s|s.set(true));RESET.with(|s|s.set(false));int(stats+0x140,100);ptr::write((player+0x710)as*mut u8,0);
 let count=APPLICATIONS.with(Cell::get);cast(crate::runtime::ULTIMATE_TRIGGER,player+8);assert_eq!(APPLICATIONS.with(Cell::get),count);
 crate::ACTIVE.store(false,Ordering::Release);cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(APPLICATIONS.with(Cell::get),count);crate::ACTIVE.store(true,Ordering::Release);
 FAIL.with(|s|s.set(true));float(node+0x4c,2.);cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(value(node+0x4c),2.);FAIL.with(|s|s.set(false));cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(value(node+0x4c),10.);
 // Missing stats/manager fail safely, without inventing a buff or FP refill.
 put(modules,0);let count=APPLICATIONS.with(Cell::get);cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(APPLICATIONS.with(Cell::get),count);put(modules,stats);
 put(player+0x190,0);cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(APPLICATIONS.with(Cell::get),count);
 crate::BASE.store(saved_base,Ordering::Release);crate::ACTIVE.store(saved_active,Ordering::Release);crate::APPLY_ORIG.store(saved_original,Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
