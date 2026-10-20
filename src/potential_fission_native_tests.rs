//! Private PE execution: real dispatch, copy and native choice linking.
//! Allocation, native lottery, character/effect state and rendering are fixtures.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
use std::cell::RefCell;
use std::sync::atomic::AtomicUsize;
static IMAGE:AtomicUsize=AtomicUsize::new(0);
thread_local!{
 static NODES:RefCell<Vec<Box<[u64;32]>>>=const{RefCell::new(Vec::new())};
 static EFFECTS:RefCell<Vec<i32>>=const{RefCell::new(Vec::new())};
 static FAILURE:Cell<i32>=const{Cell::new(-1)};
 static UID:Cell<u32>=const{Cell::new(1)};
}
unsafe fn jump(base:usize,rva:usize,dest:usize){let mut bytes=vec![0x48,0xb8];bytes.extend_from_slice(&dest.to_le_bytes());bytes.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(bytes.as_ptr(),(base+rva)as*mut u8,bytes.len());}
unsafe extern "system" fn has(_:usize,id:i32)->u8{EFFECTS.with(|c|c.borrow().contains(&id))as u8}
unsafe extern "system" fn apply(_:usize,id:i32,_:u8)->u8{EFFECTS.with(|c|{if !c.borrow().contains(&id){c.borrow_mut().push(id);}});1}
unsafe extern "system" fn remove(_:usize,id:i32)->u8{EFFECTS.with(|c|c.borrow_mut().retain(|&n|n!=id));1}
unsafe extern "system" fn lookup(_:usize,uid:u32)->usize{NODES.with(|n|n.borrow().iter().map(|n|n.as_ptr()as usize).find(|&p|ptr::read_unaligned((p+0x7c)as*const u32)==uid).unwrap_or(0))}
unsafe extern "system" fn allocate(manager:usize,descriptor:usize)->usize{
 let index=ptr::read((descriptor+0xa9)as*const u8)as i32;
 if generated(read_i32(descriptor+0xb8))&&FAILURE.with(Cell::get)==index{return 0;}
 let base=IMAGE.load(Ordering::Acquire);let mut node=Box::new([0u64;32]);let p=node.as_mut_ptr()as usize;
 let copy:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(base+0x570410);copy(p+0x20,descriptor);
 let head=NODES.with(|c|c.borrow().first().map_or(0,|n|n.as_ptr()as usize));
 // Native group algorithm walks the manager's list via node+0 and builds
 // the choice chain via node+10/+18, used by native claiming/removal.
 let group:unsafe extern "system" fn(usize,usize,usize)=std::mem::transmute(base+0x57a840);group(0,head,p);
 NODES.with(|c|{let mut nodes=c.borrow_mut();if let Some(last)=nodes.last_mut(){last[0]=p as u64;}nodes.push(node);});ptr::write_unaligned((manager+0x58)as*mut usize,if head==0{p}else{head});p
}
fn descriptor(lot:i32,slot:u8,index:u8,source:u64)->[u64;24]{unsafe{
 let mut data=[0u64;24];let p=data.as_mut_ptr()as usize;
 ptr::write(p as*mut u8,1);ptr::write_unaligned((p+8)as*mut u32,0x40000000|8000000+index as u32);
 ptr::write_unaligned((p+0x58)as*mut u32,2);
 let uid=UID.with(|c|{let n=c.get();c.set(n+1);n});ptr::write_unaligned((p+0x5c)as*mut u32,uid);
 ptr::write_unaligned((p+0x60)as*mut u64,source);ptr::write_unaligned((p+0x74)as*mut u32,0x3c000000);
 ptr::write_unaligned((p+0x78)as*mut [f32;3],[12.,34.,56.]);ptr::write((p+0xa9)as*mut u8,index);
 ptr::write_unaligned((p+0xaa)as*mut u16,0x4040|((slot as u16)<<7));ptr::write_unaligned((p+0xb8)as*mut i32,lot);data
}}
unsafe extern "system" fn lottery(manager:usize,_:usize,lot:i32,_:u8,_:u8,_:u8,_:u8,_:*const f32)->u8{
 let base=IMAGE.load(Ordering::Acquire);
 let dispatch:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(base+REWARD_CREATE_RVA);
 let count=if crate::snack::bonus_lot(lot){3}else{ROOTS.iter().find(|r|r.1==lot||r.2==lot).unwrap().3};
 for slot in 1..=3{for index in (0..count).rev(){let mut d=descriptor(lot,slot,index,999);let before=d;dispatch(manager,d.as_mut_ptr()as usize);assert_eq!(d,before);}}1
}
unsafe fn emit(base:usize,manager:usize,lot:i32,slot:u8,source:u64,count:u8){
 let dispatch:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(base+REWARD_CREATE_RVA);
 for index in 0..count{let mut d=descriptor(lot,slot,index,source);let before=d;dispatch(manager,d.as_mut_ptr()as usize);assert_eq!(d,before);}
}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; maps private PE and must run alone"]
fn compiled_fission_routes_saved_charges_and_native_independent_groups(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").expect("NRHEX_NATIVE_EXE")).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),EXE_HASH);
 let h=pe::Headers::parse(&raw,0).unwrap();let allocation=VirtualAlloc(ptr::null_mut(),h.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!allocation.is_null());let base=allocation as usize;
 for s in &h.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 IMAGE.store(base,Ordering::Release);
 for(rva,dest)in [(0x582680,lookup as*const()as usize),(HAS_EFFECT_RVA,has as*const()as usize),(APPLY_SELF_RVA,apply as*const()as usize),(REMOVE_EFFECT_RVA,remove as*const()as usize),(SPAWN_RVA,lottery as*const()as usize)]{jump(base,rva,dest);}
 let entry=hook::prepare((base+CREATE_RVA)as*mut u8,create as*const u8,64).unwrap();CREATE_ORIGINAL.store(allocate as*mut u8,Ordering::Release);hook::write_code((base+CREATE_RVA)as*mut u8,&entry.patch).unwrap();
 let link=hook::prepare((base+GROUP_RVA)as*mut u8,group as*const u8,64).unwrap();assert_eq!(link.stolen,7);GROUP_ORIGINAL.store(link.trampoline,Ordering::Release);GROUP_SKIP.store((base+0x57a8e1)as*mut u8,Ordering::Release);hook::write_code((base+GROUP_RVA)as*mut u8,&link.patch).unwrap();
 let dispatch=hook::prepare((base+REWARD_CREATE_RVA)as*mut u8,reward_create_detour as*const u8,64).unwrap();let old_dispatch=REWARD_CREATE_ORIG.swap(dispatch.trampoline,Ordering::AcqRel);hook::write_code((base+REWARD_CREATE_RVA)as*mut u8,&dispatch.patch).unwrap();
 let old_base=BASE.swap(base,Ordering::AcqRel);let old_active=ACTIVE.swap(true,Ordering::AcqRel);
 let mut world=vec![0u64;0x17500/8];let mut player=vec![0u64;0x718/8+1];let player=player.as_mut_ptr()as usize;let mut modules=[0usize;1];let mut stats=vec![0i32;0x148/4];stats[0x140/4]=100;modules[0]=stats.as_mut_ptr()as usize;ptr::write_unaligned((player+0x1b8)as*mut usize,modules.as_mut_ptr()as usize);
 let mut effects=[0u64;8];ptr::write_unaligned((player+0x190)as*mut usize,effects.as_mut_ptr()as usize);
 world[0x174e8/8]=player as u64;ptr::write_unaligned((base+WORLD_RVA)as*mut usize,world.as_mut_ptr()as usize);
 let mut data=[0i32;64];let mut game=[0usize;2];game[1]=data.as_mut_ptr()as usize;ptr::write_unaligned((base+GAME_DATA_RVA)as*mut usize,game.as_mut_ptr()as usize);
 let mut item_manager=vec![0u64;0x600/8];let items=item_manager.as_mut_ptr()as usize;ptr::write_unaligned((base+ITEMS_RVA)as*mut usize,items);
 let old_state={let mut s=STATE.lock().unwrap();std::mem::replace(&mut*s,State::new())};
 let (source,a,b,count)=ROOTS[0];
 for(raw_slot,local_slot)in [(0,1),(1,1),(2,2),(3,3)]{
  data[2]=raw_slot;*STATE.lock().unwrap()=State::new();ptr::write_unaligned((items+0x58)as*mut usize,0);NODES.with(|n|n.borrow_mut().clear());EFFECTS.with(|c|c.borrow_mut().clear());
  // Existing groups and even late option packets remain original on acquiring.
  emit(base,items,source,local_slot as u8,100,count);*STATE.lock().unwrap()=State::new();apply(player,OWNER,0);emit(base,items,source,local_slot as u8,100,count);
  assert_eq!(STATE.lock().unwrap().spent,Some(0));ptr::write_unaligned((items+0x58)as*mut usize,0);NODES.with(|n|n.borrow_mut().clear());
  // Original teammate rewards pass unchanged; only our fresh group is withheld.
  for slot in 1..=3{emit(base,items,source,slot,200,count);}
  assert_eq!(NODES.with(|n|n.borrow().len()),2*count as usize);assert_eq!(STATE.lock().unwrap().queue.len(),1);
  FAILURE.with(|c|c.set(1));update(player,base,0.3);assert_eq!(STATE.lock().unwrap().spent,Some(1));assert!(has(player,OWNER+1)!=0);
  FAILURE.with(|c|c.set(-1));update(player,base,0.3);assert!(STATE.lock().unwrap().queue.is_empty());
  let nodes=NODES.with(|n|n.borrow().iter().map(|n|n.as_ptr()as usize).collect::<Vec<_>>());
  let aa:Vec<_>=nodes.iter().copied().filter(|&p|read_i32(p+0xd8)==a).collect();let bb:Vec<_>=nodes.iter().copied().filter(|&p|read_i32(p+0xd8)==b).collect();assert_eq!(aa.len(),count as usize);assert_eq!(bb.len(),count as usize);
  for ball in [&aa,&bb]{let head=ball.iter().copied().find(|&p|read_ptr(p+0x18)==0).unwrap();let mut seen=Vec::new();let mut p=head;while p!=0{assert!(!seen.contains(&p));seen.push(p);assert_eq!(ptr::read_unaligned((p+0x80)as*const u64),200);p=read_ptr(p+0x10);}assert_eq!(seen.len(),count as usize);assert!(seen.iter().all(|p|ball.contains(p)));}
  // Claim's native linked-choice traversal is confined to one ball.
  for p in &aa{assert!(!bb.contains(&read_ptr(p+0x10)));assert!(!bb.contains(&read_ptr(p+0x18)));}
  for n in [300,400,500]{emit(base,items,source,local_slot as u8,n,count);update(player,base,0.3);}
  assert_eq!(STATE.lock().unwrap().spent,Some(3));assert!(has(player,OWNER+3)!=0);
  // Reconstruction from acknowledged native state resumes at the saved charge.
  *STATE.lock().unwrap()=State::new();emit(base,items,source,local_slot as u8,600,count);assert!(STATE.lock().unwrap().queue.is_empty());
  apply(player,reward_cycle_runtime::RESET,0);update(player,base,0.3);assert_eq!(STATE.lock().unwrap().spent,Some(0));assert!(spent_native(player,base)==0);remove(0,reward_cycle_runtime::RESET);
 }
 for slot in 1..=3{
  data[2]=slot;ptr::write_unaligned((items+0x58)as*mut usize,0);NODES.with(|nodes|nodes.borrow_mut().clear());
  emit(base,items,crate::snack::LOT,slot as u8,999,3);emit(base,items,crate::snack::LOT,slot as u8,999,3);
  assert_eq!(NODES.with(|nodes|nodes.borrow().iter().filter(|node|read_ptr(node.as_ptr()as usize+0x18)==0).count()),1);
  ptr::write_unaligned((items+0x58)as*mut usize,0);NODES.with(|nodes|nodes.borrow_mut().clear());
  let mut balls=Vec::new();
  for lot in [crate::snack::LOT,crate::snack::PREMIUM_LOT,crate::snack::LOT]{
   assert_eq!(crate::spawn_personal_lot(player,base,lot,None),3);
   let nodes=NODES.with(|nodes|nodes.borrow().iter().rev().take(3).map(|node|node.as_ptr()as usize).collect::<Vec<_>>());balls.push(nodes);
  }
  let nodes=NODES.with(|nodes|nodes.borrow().iter().map(|node|node.as_ptr()as usize).collect::<Vec<_>>());
  assert_eq!(nodes.len(),9);
  assert_eq!(nodes.iter().filter(|&&node|read_ptr(node+0x18)==0).count(),3,"Snack rewards must not merge into an existing ball");
  for ball in &balls{
   let mut node=*ball.iter().find(|&&node|read_ptr(node+0x18)==0).unwrap();let mut seen=Vec::new();
   while node!=0{assert!(ball.contains(&node));assert!(!seen.contains(&node));seen.push(node);node=read_ptr(node+0x10);}
   assert_eq!(seen.len(),3);
  }
  emit(base,items,crate::snack::LOT+100,slot as u8,999,3);
  assert_eq!(NODES.with(|nodes|nodes.borrow().iter().filter(|node|read_ptr(node.as_ptr()as usize+0x18)==0).count()),4);
  assert_eq!(crate::spawn_personal_lot(player,base,crate::snack::LOT,None),3);
  assert_eq!(NODES.with(|nodes|nodes.borrow().iter().filter(|node|read_ptr(node.as_ptr()as usize+0x18)==0).count()),5);
  assert_eq!(PERSONAL_REWARD.with(|scope|scope.get().manager),0);
 }
 *STATE.lock().unwrap()=old_state;BASE.store(old_base,Ordering::Release);ACTIVE.store(old_active,Ordering::Release);REWARD_CREATE_ORIG.store(old_dispatch,Ordering::Release);NODES.with(|n|n.borrow_mut().clear());VirtualFree(allocation,0,MEM_RELEASE);
}}
