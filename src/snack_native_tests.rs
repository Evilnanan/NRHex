//! Execute native item loads, settled price*quantity and statistics via the
//! compiled callback. Actor queries and private reward creation are fixtures.
use super::*;use crate::win::*;use sha2::{Digest,Sha256};
use std::{cell::RefCell,collections::BTreeSet};
#[derive(Default)]struct Fixture{ids:BTreeSet<i32>,created:u32,spawns:u32,slot:u8}
thread_local!{static FIXTURE:RefCell<Fixture>=RefCell::new(Fixture::default());}
unsafe fn put(p:usize,n:usize){ptr::write_unaligned(p as*mut usize,n);}
unsafe fn int(p:usize,n:i32){ptr::write_unaligned(p as*mut i32,n);}
unsafe fn jump(base:usize,rva:usize,target:usize){let mut b=vec![0x48,0xb8];b.extend_from_slice(&target.to_le_bytes());b.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(b.as_ptr(),(base+rva)as*mut u8,b.len());}
unsafe extern "system" fn has(_:usize,id:i32)->u8{FIXTURE.with(|f|u8::from(f.borrow().ids.contains(&id)))}
unsafe extern "system" fn spawn(manager:usize,_:usize,lot:i32,a:u8,b:u8,c:u8,d:u8,pos:*const f32)->u8{
 assert_eq!((lot,a,b,c,d),(LOT,0,1,1,0));assert_eq!(ptr::read_unaligned(pos as*const [f32;4]),[0.;4]);
 FIXTURE.with(|f|{let mut f=f.borrow_mut();f.spawns+=1;crate::PERSONAL_REWARD.with(|s|{let mut scope=s.get();assert_eq!(scope.manager,manager);assert_eq!(scope.lot,LOT);assert_eq!(scope.slot,f.slot);assert!(scope.choices.is_none());scope.created=f.created;s.set(scope);});});1
}
fn balance()->u32{PENDING.lock().unwrap().count}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; run alone"]
fn native_extra_practice_rewards_merchant_weapon_purchases(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 let saved_base=crate::BASE.swap(base,Ordering::AcqRel);let saved_active=crate::ACTIVE.swap(true,Ordering::AcqRel);let saved_pending={let mut s=PENDING.lock().unwrap();std::mem::replace(&mut *s,Pending::new())};
 let mut game=[0usize;0x200/8];let game=game.as_mut_ptr()as usize;put(base+crate::GAME_DATA_RVA,game);let mut data=[0usize;4];let data=data.as_mut_ptr()as usize;put(game+8,data);int(data+8,1);int(game+0x190,999000);
 let mut world=vec![0usize;0x17500/8];let world=world.as_mut_ptr()as usize;put(base+crate::WORLD_RVA,world);
 let mut actor=[0usize;0x720/8];let player=actor.as_mut_ptr()as usize;put(world+0x174e8,player);put(player+0x60,1);
 let mut manager=[0usize;4];let manager=manager.as_mut_ptr()as usize;put(player+0x190,manager);
 let mut modules=[0usize;0x80/8];let modules=modules.as_mut_ptr()as usize;put(player+0x1b8,modules);put(modules+0x68,1);
 let mut stats=[0usize;0x150/8];let stats=stats.as_mut_ptr()as usize;put(modules,stats);int(stats+0x140,100);
 let mut items=[0usize;0x60/8];let items=items.as_mut_ptr()as usize;put(base+crate::ITEMS_RVA,items);
 FIXTURE.with(|f|{*f.borrow_mut()=Fixture{created:3,slot:1,..Default::default()};f.borrow_mut().ids.extend([OWNER,crate::level_rewards::EXPEDITION]);});
 jump(base,crate::HAS_EFFECT_RVA,has as*const()as usize);jump(base,crate::SPAWN_RVA,spawn as*const()as usize);
 let prepared=crate::hook::prepare((base+SPEND_RVA)as*mut u8,spent as*const u8,64).unwrap();SPEND_ORIGINAL.store(prepared.trampoline,Ordering::Release);crate::hook::write_code((base+SPEND_RVA)as*mut u8,&prepared.patch).unwrap();
 // Fixture prologue preserves the real RBX item record. Execute each original
 // item load and price*quantity/CALL; skip unrelated UI and item-grant services.
 for(start,item,calc,ret)in [(0xa7cbb0,ITEM_A_RVA,BUY_A_RVA,0xa7cbd4),(0xa7cfb0,ITEM_B_RVA,BUY_B_RVA,0xa7cfd4)]{
  let mut code=vec![0x53,0x48,0x83,0xec,0x40,0x49,0x89,0xca,0x48,0x89,0xd3,0x48,0x89,0xd9,0xe9];code.extend_from_slice(&((item as i64-(start+19)as i64)as i32).to_le_bytes());ptr::copy_nonoverlapping(code.as_ptr(),(base+start)as*mut u8,code.len());
  let mut tail=vec![0x4c,0x89,0xd0,0xe9];tail.extend_from_slice(&((calc as i64-(item+15)as i64)as i32).to_le_bytes());ptr::copy_nonoverlapping(tail.as_ptr(),(base+item+7)as*mut u8,tail.len());
  ptr::copy_nonoverlapping([0x48,0x83,0xc4,0x40,0x5b,0xc3].as_ptr(),(base+ret)as*mut u8,6);
 }
 let buy_a:unsafe extern "system" fn(*const i32,usize)->u32=std::mem::transmute(base+0xa7cbb0);let buy_b:unsafe extern "system" fn(*const i32,usize)->u32=std::mem::transmute(base+0xa7cfb0);
 let mut capture=[0usize;0x120/8];let capture=capture.as_mut_ptr()as usize;
 let set=|code:u32,quantity|{int(capture+0x64,code as i32);int(capture+0x5c,code as i32);int(capture+0x118,quantity);int(capture+0x110,quantity);};
 set(1_000_000,3);assert_eq!(buy_a(&8000,capture),999999);assert_eq!(balance(),3);assert_eq!(read_i32(game+0x190),999999);
 set(40_000_000,2);buy_b(&1,capture);assert_eq!(balance(),5);
 for buy in [buy_a,buy_b]{for price in [0,1,3499,3500,20_000]{set(33_000_000,1);let before=balance();int(game+0x190,0);assert_eq!(buy(&price,capture),price as u32);assert_eq!(balance(),before+1);}}
 for buy in [buy_a,buy_b]{for code in [0,0x0fff_ffff,0x1000_0000|100,0x2000_0000|100,0x4000_0000|100,0x6000_0000|100,u32::MAX]{set(code,4);let before=balance();buy(&50_000,capture);assert_eq!(balance(),before);}}
 set(1_000_000,1);let before=balance();int(game+0x190,0);assert_eq!(settled(100,base+0xa403ef,1),100);assert_eq!(balance(),before);
 FIXTURE.with(|f|f.borrow_mut().ids.remove(&OWNER));buy_a(&4000,capture);assert_eq!(balance(),before);FIXTURE.with(|f|f.borrow_mut().ids.insert(OWNER));
 crate::ACTIVE.store(false,Ordering::Release);buy_b(&4000,capture);assert_eq!(balance(),before);crate::ACTIVE.store(true,Ordering::Release);
 for quantity in [0,-1]{set(1_000_000,quantity);buy_a(&4000,capture);assert_eq!(balance(),before);}set(1_000_000,1);settled(-1,base+0xa7cbd4,capture);assert_eq!(balance(),before);
 FIXTURE.with(|f|f.borrow_mut().ids.remove(&crate::level_rewards::EXPEDITION));buy_a(&4000,capture);assert_eq!(balance(),before);FIXTURE.with(|f|f.borrow_mut().ids.insert(crate::level_rewards::EXPEDITION));
 int(stats+0x140,0);update(player,base,1.);assert_eq!(balance(),before);int(stats+0x140,100);ptr::write((player+0x710)as*mut u8,1);update(player,base,1.);assert_eq!(balance(),before);ptr::write((player+0x710)as*mut u8,0);
 // Old pending balls do not block an independent new three-choice reward.
 let mut ball=[0usize;0xe0/8];let ball=ball.as_mut_ptr()as usize;put(items+0x58,ball);int(ball+0x78,2);ptr::write_unaligned((ball+0xca)as*mut u16,0xc0);int(ball+0xd8,LOT+1);
 PENDING.lock().unwrap().count=2;let count=FIXTURE.with(|f|f.borrow().spawns);update(player,base,1./60.);assert_eq!(balance(),0);assert_eq!(FIXTURE.with(|f|f.borrow().spawns),count+2);update(player,base,1./60.);assert_eq!(FIXTURE.with(|f|f.borrow().spawns),count+2);
 for slot in 1..=3{
  int(data+8,slot);FIXTURE.with(|f|f.borrow_mut().slot=slot as u8);PENDING.lock().unwrap().count=2;
  FIXTURE.with(|f|f.borrow_mut().created=0);update(player,base,1.);assert_eq!(balance(),2);
  FIXTURE.with(|f|f.borrow_mut().created=1);update(player,base,1.);assert_eq!(balance(),1);
  FIXTURE.with(|f|f.borrow_mut().created=3);update(player,base,1.);assert_eq!(balance(),0);
  let count=FIXTURE.with(|f|f.borrow().spawns);set(1_000_000,3);buy_a(&8000,capture);assert_eq!(balance(),3);update(player,base,1./60.);assert_eq!(balance(),0);assert_eq!(FIXTURE.with(|f|f.borrow().spawns),count+3);update(player,base,1.);assert_eq!(FIXTURE.with(|f|f.borrow().spawns),count+3);
 }
 PENDING.lock().unwrap().count=2;FIXTURE.with(|f|f.borrow_mut().ids.insert(crate::reward_cycle_runtime::RESET));buy_a(&8000,capture);assert_eq!(balance(),2);update(player,base,1.);assert_eq!(balance(),0);FIXTURE.with(|f|f.borrow_mut().ids.remove(&crate::reward_cycle_runtime::RESET));update(player,base,1.);assert_eq!(balance(),0);
 *PENDING.lock().unwrap()=saved_pending;crate::BASE.store(saved_base,Ordering::Release);crate::ACTIVE.store(saved_active,Ordering::Release);SPEND_ORIGINAL.store(ptr::null_mut(),Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
