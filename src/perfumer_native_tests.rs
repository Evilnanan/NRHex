//! Compiled native-use routing plus the original 32-class targeting query and
//! the real negative-inventory-index guard. Full item/AI/network services are
//! fixtures; these checks are not an end-to-end gameplay test.
use super::*;use crate::win::*;use sha2::{Digest,Sha256};use std::cell::{Cell,RefCell};
thread_local!{static EFFECTS:RefCell<Vec<i32>>=const{RefCell::new(Vec::new())};static ACCEPT:Cell<u8>=const{Cell::new(1)};static CALLS:Cell<u32>=const{Cell::new(0)};static IMAGE:Cell<usize>=const{Cell::new(0)};}
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn jump(base:usize,rva:usize,target:usize){let mut b=vec![0x48,0xb8];b.extend_from_slice(&target.to_le_bytes());b.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(b.as_ptr(),(base+rva)as*mut u8,b.len());}
unsafe extern "system" fn has(_:usize,id:i32)->u8{EFFECTS.with(|s|u8::from(s.borrow().contains(&id)))}
unsafe extern "system" fn find(_:usize,_:*const i32,_:i32)->i32{-1}
unsafe extern "system" fn check(player:usize,item:*const i32)->u8{
 let base=IMAGE.with(Cell::get);assert_eq!(read_i32(player+0x174),ptr::read_unaligned(item));assert_eq!(read_i32(player+0x178),1);assert_eq!(read_i32(player+0x18c),-1);
 assert_eq!(inventory_lookup(0,item,-1,base+0x6d06b9),0);assert_eq!(inventory_lookup(0,item,-1,base+0x6d06b8),-1);assert_eq!(inventory_lookup(0,&3500,-1,base+0x6d06b9),-1);ACCEPT.with(Cell::get)
}
unsafe extern "system" fn use_good(context:usize,good:i32,data:usize,flag:u8,entry:u8,elapsed:f32)->u8{
 assert_ne!(read_ptr(context),0);assert!(GOODS.contains(&good));assert_eq!((data,flag,entry,elapsed.to_bits()),(1234,7,1,0.375f32.to_bits()));CALLS.with(|c|c.set(c.get()+1));ACCEPT.with(Cell::get)
}
#[test]fn all_heroes_affinities_and_parties_can_draw_perfumer(){let index=crate::hex_pool::OWNER_IDS.iter().position(|&g|g==OWNER).unwrap();assert_eq!(crate::hex_pool::HISTORY_BITS[index],38);for hero in std::iter::once(None).chain((0..=10).map(Some)){for affinity in [None,Some(1100),Some(1600),Some(1700),Some(57),Some(61)]{for party in [None,Some(1),Some(2),Some(3)]{let excluded=crate::reward_choices::pool_exclusions(crate::reward_choices::party_exclusions(crate::reward_choices::weapon_exclusions(crate::reward_choices::role_exclusions(Default::default(),hero),affinity),party));assert!(!excluded.contains(&index));}}}}
#[test]#[ignore="requires native EXE; run alone"]fn native_perfumer_native_use_and_target_summary(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;for s in &headers.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 IMAGE.with(|s|s.set(base));let old_base=BASE.swap(base,Ordering::AcqRel);let old_active=ACTIVE.swap(true,Ordering::AcqRel);
 jump(base,INVENTORY_RVA,inventory as*const()as usize);
 let start=base+0x6d06ac;ptr::copy_nonoverlapping([0x48u8,0x83,0xec,0x28,0x90,0x90,0x90,0x90].as_ptr(),start as*mut u8,8);
 let at=base+0x6d06b4;ptr::write(at as*mut u8,0xe8);ptr::write_unaligned((at+1)as*mut i32,(INVENTORY_RVA as isize-0x6d06b9)as i32);
 ptr::copy_nonoverlapping([0x48u8,0x83,0xc4,0x28,0xc3].as_ptr(),(base+0x6d06b9)as*mut u8,5);
 let original_find=INVENTORY_ORIGINAL.swap(find as*mut u8,Ordering::AcqRel);let original_can=CAN_USE_ORIGINAL.swap(check as*mut u8,Ordering::AcqRel);let original_use=USE_ORIGINAL.swap(use_good as*mut u8,Ordering::AcqRel);
 jump(base,crate::HAS_EFFECT_RVA,has as*const()as usize);jump(base,CAN_USE_RVA,can_use as*const()as usize);
 let mut world=vec![0usize;0x17500/8];let world=world.as_mut_ptr()as usize;put(base+crate::WORLD_RVA,world);
 let mut actor=[0usize;0x720/8];let player=actor.as_mut_ptr()as usize;put(world+0x174e8,player);put(player+0x190,player);
 let mut modules=[0usize;4];let modules=modules.as_mut_ptr()as usize;put(player+0x1b8,modules);let mut stats=[0usize;0x150/8];let stats=stats.as_mut_ptr()as usize;put(modules,stats);int(stats+0x140,1000);
 let mut target=vec![0usize;0x38c0/8];let target=target.as_mut_ptr()as usize;put(base+TARGET_MANAGER,target);EFFECTS.with(|s|s.borrow_mut().push(OWNER));*CYCLE.lock().unwrap()=Cycle::new();
 int(player+0x174,0x40000001);int(player+0x178,0);int(player+0x18c,55);
 // Execute actual602900/332AF0 query, all32 classes including threat0.
 assert!(unobserved(base));for level in 0..32{for state in [1,3,4,5,7]{int(target+0x3820+level*4,state);assert!(!unobserved(base));int(target+0x3820+level*4,0);}}assert!(unobserved(base));put(base+TARGET_MANAGER,0);assert!(!unobserved(base));put(base+TARGET_MANAGER,target);
 for _ in 0..29{update(player,base,1.);}assert_eq!(query(player,BEGIN,base),0);update(player,base,1.);
 // The native checks run with no inventory ownership; only its exact boolean
 // existence test receives the synthetic result, under a scoped local gate.
 for _ in 0..8{
  let previous=CYCLE.lock().unwrap().last;let kind=query(player,BEGIN,base);assert!((1..=3).contains(&kind));let kind=(kind-1)as usize;if previous==Some(0){assert_ne!(kind,0);}
  assert_eq!(CYCLE.lock().unwrap().elapsed,30.);
  let abi:unsafe extern "system" fn(usize,*const i32,i32)->i32=std::mem::transmute(start);let item=encoded(kind);
  assert_eq!(abi(0,&item,-1),-1);VIRTUAL_PRESENCE.with(|s|s.set(true));assert_eq!(abi(0,&item,-1),0);VIRTUAL_PRESENCE.with(|s|s.set(false));
  assert_eq!(inventory_lookup(0,&encoded(kind),-1,base+0x6d06b9),-1);
  let context=[player];let context=context.as_ptr()as usize;ACCEPT.with(|s|s.set(0));assert_eq!(used(context,GOODS[kind],1234,7,1,0.375),0);assert_eq!(CYCLE.lock().unwrap().elapsed,30.);
  ACCEPT.with(|s|s.set(1));assert_eq!(used(context,GOODS[kind],1234,7,1,0.375),1);assert_eq!(CYCLE.lock().unwrap().elapsed,0.);assert_eq!(CYCLE.lock().unwrap().last,Some(kind));
  update(player,base,1.);used(context,GOODS[kind],1234,7,1,0.375);assert_eq!(CYCLE.lock().unwrap().elapsed,1.,"Duplicate completion cannot refresh cooldown");
  query(player,CLIP_ACTIVE,base);query(player,CLIP_INACTIVE,base);assert_eq!(read_i32(player+0x174),0x40000001);assert_eq!(read_i32(player+0x18c),55);for _ in 0..29{update(player,base,1.);}
 }
 // Guard an actual unowned inventory slot: -1 returns before any pointer
 // dereference or decrement, exactly the argument used by native65 post-use.
 let consume:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+0x1f12b0);assert_eq!(consume(0,-1,0),0);
 let kind=query(player,BEGIN,base);assert_ne!(kind,0);query(player,MANUAL,base);assert!(CYCLE.lock().unwrap().pending.is_none());assert_eq!(CYCLE.lock().unwrap().elapsed,30.);
 ACCEPT.with(|s|s.set(0));assert_eq!(query(player,BEGIN,base),0);assert!(CYCLE.lock().unwrap().pending.is_none());assert_eq!(read_i32(player+0x174),0x40000001);ACCEPT.with(|s|s.set(1));
 for down in [1,2]{ptr::write((player+0x710)as*mut u8,down);assert_eq!(query(player,BEGIN,base),0);}ptr::write((player+0x710)as*mut u8,0);int(stats+0x140,0);assert_eq!(query(player,BEGIN,base),0);int(stats+0x140,1000);
 int(target+0x3820+31*4,7);update(player,base,0.1);assert_eq!(query(player,BEGIN,base),0);int(target+0x3820+31*4,0);update(player,base,0.25);assert_eq!(query(player,BEGIN,base),0);update(player,base,0.25);assert_ne!(query(player,BEGIN,base),0);
 query(player,CLIP_INACTIVE,base);assert!(CYCLE.lock().unwrap().pending.is_some());update(player,base,0.5);query(player,CLIP_INACTIVE,base);assert!(CYCLE.lock().unwrap().pending.is_none());
 assert_eq!(query(player+8,BEGIN,base),0);EFFECTS.with(|s|s.borrow_mut().clear());assert_eq!(query(player,BEGIN,base),0);reset(player,base);assert_eq!(CYCLE.lock().unwrap().elapsed,0.);
 INVENTORY_ORIGINAL.store(original_find,Ordering::Release);CAN_USE_ORIGINAL.store(original_can,Ordering::Release);USE_ORIGINAL.store(original_use,Ordering::Release);BASE.store(old_base,Ordering::Release);ACTIVE.store(old_active,Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
