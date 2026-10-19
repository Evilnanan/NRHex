//! Original settled-price multiplication/call sites and native statistics run
//! against fixtures. Actor effects, saving and item creation are isolated stubs.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
use std::{cell::RefCell,collections::BTreeSet};
#[derive(Default)]
struct Fixture{ids:BTreeSet<i32>,order:Vec<i32>,pending:Vec<(bool,i32)>,nodes:Vec<Box<[u64;14]>>,manager:usize,created:u32,spawns:u32,slot:u8}
impl Fixture{
 unsafe fn sync(&mut self){
  for(add,id)in self.pending.drain(..){if add{if self.ids.insert(id){self.order.insert(0,id);}}else{self.ids.remove(&id);}}
  self.order.retain(|id|self.ids.contains(id));
  for &id in &self.ids{if !self.order.contains(&id){self.order.push(id);}}
  self.nodes=self.order.iter().map(|&id|{let mut n=Box::new([0u64;14]);n[1]=id as u64;n}).collect();
  for i in 0..self.nodes.len(){self.nodes[i][0x38/8]=if i+1<self.nodes.len(){self.nodes[i+1].as_ptr()as u64}else{0};}
  put(self.manager+8,self.nodes.first().map_or(0,|n|n.as_ptr()as usize));
 }
}
thread_local!{static FIXTURE:RefCell<Fixture>=RefCell::new(Fixture::default());}
unsafe fn put(p:usize,n:usize){ptr::write_unaligned(p as*mut usize,n);}
unsafe fn int(p:usize,n:i32){ptr::write_unaligned(p as*mut i32,n);}
unsafe fn jump(base:usize,rva:usize,target:usize){let mut b=vec![0x48,0xb8];b.extend_from_slice(&target.to_le_bytes());b.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(b.as_ptr(),(base+rva)as*mut u8,b.len());}
unsafe extern "system" fn has(_:usize,id:i32)->u8{FIXTURE.with(|f|u8::from(f.borrow().ids.contains(&id)))}
unsafe extern "system" fn apply(_:usize,id:i32,flag:u8)->u8{assert_eq!(flag,0);FIXTURE.with(|f|f.borrow_mut().pending.push((true,id)));1}
unsafe extern "system" fn remove(_:usize,id:i32)->u8{FIXTURE.with(|f|f.borrow_mut().pending.push((false,id)));1}
unsafe extern "system" fn spawn(manager:usize,_:usize,lot:i32,a:u8,b:u8,c:u8,d:u8,pos:*const f32)->u8{
 assert_eq!((lot,a,b,c,d),(LOT,0,1,1,0));assert_eq!(ptr::read_unaligned(pos as*const [f32;4]),[0.;4]);
 FIXTURE.with(|f|{let mut f=f.borrow_mut();f.spawns+=1;
  crate::PERSONAL_REWARD.with(|s|{let mut scope=s.get();assert_eq!(scope.manager,manager);assert_eq!(scope.lot,LOT);assert_eq!(scope.slot,f.slot);assert!(scope.choices.is_none());scope.created=f.created;s.set(scope);});
 });1
}
unsafe fn ack(){FIXTURE.with(|f|f.borrow_mut().sync());}
fn balance()->u32{CREDIT.lock().unwrap().amount.unwrap_or(0)}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; run alone"]
fn native_snack_uses_settled_shop_payments_and_private_immediate_rewards(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 let saved_base=crate::BASE.swap(base,Ordering::AcqRel);let saved_active=crate::ACTIVE.swap(true,Ordering::AcqRel);
 let saved_credit={let mut s=CREDIT.lock().unwrap();std::mem::replace(&mut *s,Credit::new())};
 let mut game=[0usize;0x200/8];let game=game.as_mut_ptr()as usize;put(base+crate::GAME_DATA_RVA,game);
 let mut data=[0usize;4];let data=data.as_mut_ptr()as usize;put(game+8,data);int(data+8,1);int(game+0x190,999000);
 let mut world=vec![0usize;0x17500/8];let world=world.as_mut_ptr()as usize;put(base+crate::WORLD_RVA,world);
 let mut actor=[0usize;0x720/8];let player=actor.as_mut_ptr()as usize;put(world+0x174e8,player);put(player+0x60,1);
 let mut manager=[0usize;4];let manager=manager.as_mut_ptr()as usize;put(player+0x190,manager);
 let mut modules=[0usize;0x80/8];let modules=modules.as_mut_ptr()as usize;put(player+0x1b8,modules);put(modules+0x68,1);
 let mut stats=[0usize;0x150/8];let stats=stats.as_mut_ptr()as usize;put(modules,stats);int(stats+0x140,100);
 let mut items=[0usize;0x60/8];let items=items.as_mut_ptr()as usize;put(base+crate::ITEMS_RVA,items);
 FIXTURE.with(|f|{let mut f=f.borrow_mut();*f=Fixture{manager,created:3,slot:1,..Default::default()};f.ids.extend([OWNER,crate::level_rewards::EXPEDITION]);});ack();
 for(rva,target)in [(crate::HAS_EFFECT_RVA,has as*const()as usize),(crate::APPLY_SELF_RVA,apply as*const()as usize),(crate::REMOVE_EFFECT_RVA,remove as*const()as usize),(crate::SPAWN_RVA,spawn as*const()as usize)]{jump(base,rva,target);}
 let prepared=crate::hook::prepare((base+SPEND_RVA)as*mut u8,spent as*const u8,64).unwrap();SPEND_ORIGINAL.store(prepared.trampoline,Ordering::Release);crate::hook::write_code((base+SPEND_RVA)as*mut u8,&prepared.patch).unwrap();
 // Fixture prologues enter the actual price*quantity instructions. Preserve
 // the original native CALL and its exact allowlisted return address.
 for(start,calc,ret)in [(0xa7cbb0,0xa7cbc6,0xa7cbd4),(0xa7cfb0,0xa7cfc6,0xa7cfd4)]{
  let mut code=vec![0x53,0x48,0x83,0xec,0x20,0x48,0x89,0xc8,0x48,0x89,0xd3,0xe9];code.extend_from_slice(&((calc as i64-(start+16)as i64)as i32).to_le_bytes());
  ptr::copy_nonoverlapping(code.as_ptr(),(base+start)as*mut u8,code.len());ptr::copy_nonoverlapping([0x48,0x83,0xc4,0x20,0x5b,0xc3].as_ptr(),(base+ret)as*mut u8,6);
 }
 let buy_a:unsafe extern "system" fn(*const i32,usize)->u32=std::mem::transmute(base+0xa7cbb0);
 let buy_b:unsafe extern "system" fn(*const i32,usize)->u32=std::mem::transmute(base+0xa7cfb0);
 let mut capture=[0usize;0x120/8];let capture=capture.as_mut_ptr()as usize;
 int(capture+0x118,3);int(capture+0x110,2);
 assert_eq!(buy_a(&8000,capture),999999);assert_eq!(balance(),24000);assert_eq!(read_i32(game+0x190),999999);
 // A second transaction before the first saved markers are acknowledged.
 assert_eq!(buy_b(&8000,capture),999999);assert_eq!(balance(),40000);ack();
 assert_eq!(decode(&records(player)),40000);
 for buy in [buy_a,buy_b]{
  for (unit_price,quantity,credited) in [(3499,10,0),(2000,10,0),(3500,1,3500),(3500,6,21000),(3501,2,7002)]{
   int(capture+0x118,quantity);int(capture+0x110,quantity);
   let before=balance();int(game+0x190,0);
   assert_eq!(buy(&unit_price,capture),u32::try_from(unit_price*quantity).unwrap());
   assert_eq!(balance(),before+credited);ack();assert_eq!(decode(&records(player)),before+credited);
  }
 }
 CREDIT.lock().unwrap().amount=Some(40000);persist(player,base,40000);ack();persist(player,base,40000);ack();
 int(capture+0x118,3);int(capture+0x110,2);
 // Actual accounting for other rune spending is forwarded but not rewarded.
 let original:unsafe extern "system" fn(i32)->u32=std::mem::transmute(SPEND_ORIGINAL.load(Ordering::Acquire));
 int(game+0x190,10);assert_eq!(settled(100,base+0xa403ef,1),110);assert_eq!(original(10),120);assert_eq!(balance(),40000);
 FIXTURE.with(|f|{f.borrow_mut().ids.remove(&OWNER);});ack();buy_a(&4000,capture);assert_eq!(balance(),40000);
 FIXTURE.with(|f|{f.borrow_mut().ids.insert(OWNER);});ack();crate::ACTIVE.store(false,Ordering::Release);buy_b(&4000,capture);assert_eq!(balance(),40000);crate::ACTIVE.store(true,Ordering::Release);
 for amount in [0,-100]{settled(amount,base+0xa7cbd4,1);assert_eq!(balance(),40000);}
 int(stats+0x140,0);update(player,base,1.);assert_eq!(balance(),40000);int(stats+0x140,100);
 let mut ball=[0usize;0xe0/8];let ball=ball.as_mut_ptr()as usize;put(items+0x58,ball);int(ball+0x78,2);ptr::write_unaligned((ball+0xca)as*mut u16,0xc0);int(ball+0xd8,LOT+1);
 let before=FIXTURE.with(|f|f.borrow().spawns);
 update(player,base,1./60.);assert_eq!(balance(),0);assert_eq!(FIXTURE.with(|f|f.borrow().spawns),before+2);
 update(player,base,1./60.);assert_eq!(FIXTURE.with(|f|f.borrow().spawns),before+2);
 ack();persist(player,base,0);ack();
 for slot in 1..=3{
  int(data+8,slot);FIXTURE.with(|f|f.borrow_mut().slot=slot as u8);
  CREDIT.lock().unwrap().amount=Some(40000);persist(player,base,40000);ack();persist(player,base,40000);ack();
  FIXTURE.with(|f|f.borrow_mut().created=0);update(player,base,1.);assert_eq!(balance(),40000);ack();
  FIXTURE.with(|f|f.borrow_mut().created=1);update(player,base,1.);assert_eq!(balance(),20000);ack();persist(player,base,20000);ack();
  FIXTURE.with(|f|f.borrow_mut().created=3);update(player,base,1.);assert_eq!(balance(),0);ack();persist(player,base,0);ack();
  let count=FIXTURE.with(|f|f.borrow().spawns);update(player,base,1.);assert_eq!(FIXTURE.with(|f|f.borrow().spawns),count);
  int(capture+0x118,6);buy_a(&8000,capture);assert_eq!(balance(),48000);
  update(player,base,1./60.);assert_eq!(balance(),8000);assert_eq!(FIXTURE.with(|f|f.borrow().spawns),count+2);
  update(player,base,1./60.);assert_eq!(balance(),8000);assert_eq!(FIXTURE.with(|f|f.borrow().spawns),count+2);
  ack();update(player,base,1.);ack();assert_eq!(decode(&records(player)),8000);
 }
 put(items+0x58,0);
 // Explicit reset survives deferred marker removal and ignores old credit.
 CREDIT.lock().unwrap().amount=Some(12345);persist(player,base,12345);ack();reset(player,base);assert_eq!(balance(),0);buy_a(&8000,capture);assert_eq!(balance(),0);ack();update(player,base,1.);ack();update(player,base,1.);assert!(!CREDIT.lock().unwrap().resetting);assert_eq!(decode(&records(player)),0);
 *CREDIT.lock().unwrap()=saved_credit;crate::BASE.store(saved_base,Ordering::Release);crate::ACTIVE.store(saved_active,Ordering::Release);SPEND_ORIGINAL.store(ptr::null_mut(),Ordering::Release);
 assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
