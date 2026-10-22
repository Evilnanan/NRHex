//! Execute native item loads, settled price*quantity and statistics via the
//! compiled callback. Actor queries and private reward creation are fixtures.
use super::*;use crate::win::*;use sha2::{Digest,Sha256};
use std::{cell::RefCell,collections::BTreeSet};
#[derive(Default)]struct Fixture{ids:BTreeSet<i32>,created:u32,spawns:u32,slot:u8,rarity:u8,row:Vec<u8>,lots:Vec<i32>}
thread_local!{static FIXTURE:RefCell<Fixture>=RefCell::new(Fixture::default());}
unsafe fn put(p:usize,n:usize){ptr::write_unaligned(p as*mut usize,n);}
unsafe fn int(p:usize,n:i32){ptr::write_unaligned(p as*mut i32,n);}
unsafe fn jump(base:usize,rva:usize,target:usize){let mut b=vec![0x48,0xb8];b.extend_from_slice(&target.to_le_bytes());b.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(b.as_ptr(),(base+rva)as*mut u8,b.len());}
unsafe extern "system" fn has(_:usize,id:i32)->u8{FIXTURE.with(|f|u8::from(f.borrow().ids.contains(&id)))}
unsafe extern "system" fn spawn(manager:usize,_:usize,lot:i32,a:u8,b:u8,c:u8,d:u8,pos:*const f32)->u8{
 assert!([LOT,PREMIUM_LOT].contains(&lot));assert_eq!((a,b,c,d),(0,1,1,0));assert_eq!(ptr::read_unaligned(pos as*const [f32;4]),[0.;4]);
 FIXTURE.with(|f|{let mut f=f.borrow_mut();f.spawns+=1;f.lots.push(lot);crate::PERSONAL_REWARD.with(|s|{let mut scope=s.get();assert_eq!(scope.manager,manager);assert_eq!(scope.lot,lot);assert_eq!(scope.slot,f.slot);assert!(scope.choices.is_none());scope.created=f.created;s.set(scope);});});1
}
unsafe extern "system" fn weapon_param(out:usize,id:i32)->usize{
 FIXTURE.with(|state|{let mut state=state.borrow_mut();state.row[0x240]=state.rarity;int(out,id);int(out+0x10,id-id%100);put(out+8,state.row.as_ptr()as usize);});out
}
unsafe extern "system" fn price_object(out:usize,kind:u8,value:i32,_materials:usize)->usize{
 // Native cost-object allocation/copy only; arithmetic runs in 8C24B1.
 ptr::write_bytes(out as*mut u8,0,0x38);ptr::write((out+8)as*mut u8,kind);int(out+0xc,value);out
}
unsafe extern "system" fn vector_copy(_:usize,_:usize,_:usize,_:u8){}
unsafe fn verify_set_meal_quotes(base:usize,data:usize){
 let prepared=crate::hook::prepare((base+PRICE_RVA)as*mut u8,price as*const u8,64).unwrap();
 PRICE_ORIGINAL.store(prepared.trampoline,Ordering::Release);crate::hook::write_code((base+PRICE_RVA)as*mut u8,&prepared.patch).unwrap();
 jump(base,0x8eefe0,price_object as*const()as usize);jump(base,0x8aade0,vector_copy as*const()as usize);
 ptr::write((base+0x1f4d80)as*mut u8,0xc3);
 // The redirected RBP exposes the descriptor, not the original 0x640-byte
 // C++ frame. Its destructor-bookkeeping slot at RBP-50 is outside that
 // descriptor: omit only this unused bookkeeping write with the cleanup stub.
 ptr::write_bytes((base+0x8c2520)as*mut u8,0x90,4);
 // Aligned fixture frame, with RBP-10 pointing at the real quote descriptor.
 // Run BOTH original branches: no discount and float multiply/truncate.
 let start=0x8c2400;
 let mut code=vec![0x55,0x56,0x48,0x81,0xec,0xa8,0,0,0,
  0x0f,0x29,0xb4,0x24,0x80,0,0,0,0x0f,0x29,0xbc,0x24,0x90,0,0,0,
  0x48,0x8d,0x69,0x10,0x31,0xf6,0x0f,0x28,0xf1,
  0xb8,0,0,0x80,0x3f,0x66,0x0f,0x6e,0xf8,0xe9];
 code.extend_from_slice(&((0x8c24a6i64-(start+code.len()+4)as i64)as i32).to_le_bytes());
 ptr::copy_nonoverlapping(code.as_ptr(),(base+start)as*mut u8,code.len());
 let tail=[0x8b,0x45,0x14,
  0x0f,0x28,0xb4,0x24,0x80,0,0,0,0x0f,0x28,0xbc,0x24,0x90,0,0,0,
  0x48,0x81,0xc4,0xa8,0,0,0,0x5e,0x5d,0xc3];
 ptr::copy_nonoverlapping(tail.as_ptr(),(base+0x8c2555)as*mut u8,tail.len());
 let quote_row:unsafe extern "system" fn(usize,f32)->i32=std::mem::transmute(base+start);
 let affordable:unsafe extern "system" fn(usize)->i32=std::mem::transmute(base+0x7e9ac0);
 put(base+0x3c149d8,1); // Native GameData manager existence guard.
 let mut row=[0usize;0x110/8];let record=row.as_mut_ptr()as usize;
 for code in [1_000_000,33_000_000,40_000_000,0x4000_0000|3500,0x2000_0000|100,0,0x0fff_ffff,u32::MAX]{
  for kind in [0u8,1,5]{for value in [0,1,3000,8000,20_000]{for discount in [1.,0.8,0.5]{
   ptr::write_bytes(record as*mut u8,0,0x110);int(record+0x54,code as i32);int(record+0x24,value);ptr::write((record+0x20)as*mut u8,kind);
   let expected=(value as f32*discount)as i32+if weapon(code)&&kind==0{7000}else{0};
   assert_eq!(quote_row(record,discount),expected,"item={code:x}, kind={kind}, price={value}, discount={discount}");
   if kind==0&&expected>0{
    for balance in [expected-1,expected,expected*3]{int(data+0x6c,balance);assert_eq!(affordable(record+0x18),(balance/expected).min(999));}
   }
  }}}
 }
 // Fresh menu rebuilds start from native prices, and cannot compound the fee.
 for _ in 0..3{int(record+0x54,1_000_000);ptr::write((record+0x20)as*mut u8,0);int(record+0x24,8000);assert_eq!(quote_row(record,1.),15000);}
 for id in [OWNER,crate::level_rewards::EXPEDITION]{
  FIXTURE.with(|f|f.borrow_mut().ids.remove(&id));int(record+0x24,8000);assert_eq!(quote_row(record,0.5),4000);FIXTURE.with(|f|f.borrow_mut().ids.insert(id));
 }
 FIXTURE.with(|f|f.borrow_mut().ids.insert(crate::reward_cycle_runtime::RESET));int(record+0x24,8000);assert_eq!(quote_row(record,1.),8000);FIXTURE.with(|f|f.borrow_mut().ids.remove(&crate::reward_cycle_runtime::RESET));
 crate::ACTIVE.store(false,Ordering::Release);int(record+0x24,8000);assert_eq!(quote_row(record,1.),8000);crate::ACTIVE.store(true,Ordering::Release);
 for value in [-1,i32::MAX-100]{int(record+0x24,value);assert_eq!(quote_row(record,1.),if value<0{value}else{i32::MAX});}
}
fn set_pending(quantity:u32){let mut state=PENDING.lock().unwrap();*state=Pending::new();state.bought(quantity,false);}
fn balance()->u32{PENDING.lock().unwrap().count}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; run alone"]
fn native_extra_practice_rewards_merchant_weapon_purchases(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 let saved_base=crate::BASE.swap(base,Ordering::AcqRel);let saved_active=crate::ACTIVE.swap(true,Ordering::AcqRel);let saved_pending={let mut s=PENDING.lock().unwrap();std::mem::replace(&mut *s,Pending::new())};
 let mut game=[0usize;0x200/8];let game=game.as_mut_ptr()as usize;put(base+crate::GAME_DATA_RVA,game);let mut data=[0usize;0x80/8];let data=data.as_mut_ptr()as usize;put(game+8,data);int(data+8,1);int(game+0x190,999000);
 let mut world=vec![0usize;0x17500/8];let world=world.as_mut_ptr()as usize;put(base+crate::WORLD_RVA,world);
 let mut actor=[0usize;0x720/8];let player=actor.as_mut_ptr()as usize;put(world+0x174e8,player);put(player+0x60,1);
 let mut manager=[0usize;4];let manager=manager.as_mut_ptr()as usize;put(player+0x190,manager);
 let mut modules=[0usize;0x80/8];let modules=modules.as_mut_ptr()as usize;put(player+0x1b8,modules);put(modules+0x68,1);
 let mut stats=[0usize;0x150/8];let stats=stats.as_mut_ptr()as usize;put(modules,stats);int(stats+0x140,100);
 let mut items=[0usize;0x60/8];let items=items.as_mut_ptr()as usize;put(base+crate::ITEMS_RVA,items);
 FIXTURE.with(|f|{*f.borrow_mut()=Fixture{created:3,slot:1,row:vec![0;0x280],..Default::default()};f.borrow_mut().ids.extend([OWNER,crate::level_rewards::EXPEDITION]);});
 jump(base,0xea7fc0,weapon_param as*const()as usize);
 for rva in [0xea9a60,0x26909a0]{ptr::write((base+rva)as*mut u8,0xc3);}
 ptr::copy_nonoverlapping([0xb0,1,0xc3].as_ptr(),(base+0x6c12f0)as*mut u8,3);
 jump(base,crate::HAS_EFFECT_RVA,has as*const()as usize);jump(base,crate::SPAWN_RVA,spawn as*const()as usize);
 let prepared=crate::hook::prepare((base+SPEND_RVA)as*mut u8,spent as*const u8,64).unwrap();SPEND_ORIGINAL.store(prepared.trampoline,Ordering::Release);crate::hook::write_code((base+SPEND_RVA)as*mut u8,&prepared.patch).unwrap();
 verify_set_meal_quotes(base,data);
 // Fixture prologue preserves the real RBX item record. Execute each original
 // item load and price*quantity/CALL; skip unrelated UI and item-grant services.
 for(start,item,calc,ret)in [(0xa7cbb0,ITEM_A_RVA,BUY_A_RVA,0xa7cbd4),(0xa7cfb0,ITEM_B_RVA,BUY_B_RVA,0xa7cfd4)]{
  let mut code=vec![0x53,0x48,0x83,0xec,0x40,0x49,0x89,0xca,0x48,0x89,0xd3,0x48,0x89,0xd9,0xe9];code.extend_from_slice(&((item as i64-(start+19)as i64)as i32).to_le_bytes());ptr::copy_nonoverlapping(code.as_ptr(),(base+start)as*mut u8,code.len());
  let mut tail=vec![0x4c,0x89,0xd0,0xe9];tail.extend_from_slice(&((calc as i64-(item+15)as i64)as i32).to_le_bytes());ptr::copy_nonoverlapping(tail.as_ptr(),(base+item+7)as*mut u8,tail.len());
  ptr::copy_nonoverlapping([0x48,0x83,0xc4,0x40,0x5b,0xc3].as_ptr(),(base+ret)as*mut u8,6);
 }
 let buy_a:unsafe extern "system" fn(*const i32,usize)->u32=std::mem::transmute(base+0xa7cbb0);let buy_b:unsafe extern "system" fn(*const i32,usize)->u32=std::mem::transmute(base+0xa7cfb0);
 let mut capture=[0usize;0x120/8];let capture=capture.as_mut_ptr()as usize;
 let set=|code:u32,quantity|{int(capture+0x64,code as i32);int(capture+0x5c,code as i32);int(capture+0xc0,code as i32);int(capture+0xb8,code as i32);int(capture+0x118,quantity);int(capture+0x110,quantity);};
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
 set_pending(2);let count=FIXTURE.with(|f|f.borrow().spawns);update(player,base,1./60.);assert_eq!(balance(),0);assert_eq!(FIXTURE.with(|f|f.borrow().spawns),count+2);update(player,base,1./60.);assert_eq!(FIXTURE.with(|f|f.borrow().spawns),count+2);
 for slot in 1..=3{
  int(data+8,slot);FIXTURE.with(|f|f.borrow_mut().slot=slot as u8);set_pending(2);
  FIXTURE.with(|f|f.borrow_mut().created=0);update(player,base,1.);assert_eq!(balance(),2);
  FIXTURE.with(|f|f.borrow_mut().created=1);update(player,base,1.);assert_eq!(balance(),1);
  FIXTURE.with(|f|f.borrow_mut().created=3);update(player,base,1.);assert_eq!(balance(),0);
  let count=FIXTURE.with(|f|f.borrow().spawns);set(1_000_000,3);buy_a(&8000,capture);assert_eq!(balance(),3);update(player,base,1./60.);assert_eq!(balance(),0);assert_eq!(FIXTURE.with(|f|f.borrow().spawns),count+3);update(player,base,1.);assert_eq!(FIXTURE.with(|f|f.borrow().spawns),count+3);
 }
 set_pending(2);FIXTURE.with(|f|f.borrow_mut().ids.insert(crate::reward_cycle_runtime::RESET));buy_a(&8000,capture);assert_eq!(balance(),2);update(player,base,1.);assert_eq!(balance(),0);FIXTURE.with(|f|f.borrow_mut().ids.remove(&crate::reward_cycle_runtime::RESET));update(player,base,1.);assert_eq!(balance(),0);
 for buy in [buy_a,buy_b]{
  let mut expected=Vec::new();FIXTURE.with(|state|state.borrow_mut().lots.clear());
  for (rarity,cursed) in [(0,false),(1,false),(2,false),(3,false),(0,true),(2,true),(4,false),(255,false)]{
   set(1_000_000,2);FIXTURE.with(|state|state.borrow_mut().rarity=rarity);
   ptr::write((capture+0xde)as*mut u8,u8::from(cursed));ptr::write((capture+0xd6)as*mut u8,u8::from(cursed));
   buy(&4000,capture);expected.extend([if cursed||[2,3].contains(&rarity){PREMIUM_LOT}else{LOT};2]);
  }
  FIXTURE.with(|state|state.borrow_mut().rarity=0);ptr::write((capture+0xde)as*mut u8,0);ptr::write((capture+0xd6)as*mut u8,0);
  update(player,base,1.);assert_eq!(balance(),0);assert_eq!(FIXTURE.with(|state|state.borrow().lots.clone()),expected);
 }
 // Feed the freshly quoted descriptor directly to BOTH real purchase layouts.
 // Their original price reads/multiplications/statistic calls must see the
 // same unit price, with exactly one reward per item, including bulk purchases.
 let quote_row:unsafe extern "system" fn(usize,f32)->i32=std::mem::transmute(base+0x8c2400);
 for (buy,offset) in [(buy_a,0x10),(buy_b,8)]{for discount in [1.,0.8,0.5]{for quantity in [1,3]{
  set(1_000_000,quantity);let record=capture+offset;ptr::write((record+0x20)as*mut u8,0);int(record+0x24,8000);
  let quoted=quote_row(record,discount);assert_eq!(quoted,(8000.*discount)as i32+7000);
  let before=balance();int(game+0x190,0);assert_eq!(buy((record+0x24)as*const i32,capture),(quoted*quantity)as u32);assert_eq!(balance(),before+quantity as u32);
 }}}
 *PENDING.lock().unwrap()=saved_pending;crate::BASE.store(saved_base,Ordering::Release);crate::ACTIVE.store(saved_active,Ordering::Release);SPEND_ORIGINAL.store(ptr::null_mut(),Ordering::Release);PRICE_ORIGINAL.store(ptr::null_mut(),Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
