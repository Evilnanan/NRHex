//! Real native item-deduction branch and compiled local scope/gift transaction.
use super::*;
use crate::win::*;
use std::cell::RefCell;
thread_local!{
 static EFFECTS:RefCell<Vec<i32>>=const{RefCell::new(Vec::new())};
 static ROWS:RefCell<Vec<(i32,usize)>>=const{RefCell::new(Vec::new())};
 static ITEMS:RefCell<Vec<(i32,i32,u32,u32)>>=const{RefCell::new(Vec::new())}; // good,index,serial,quantity
 static EVENTS:RefCell<Vec<i32>>=const{RefCell::new(Vec::new())};
 static GROUNDS:Cell<u32>=const{Cell::new(0)};
 static GROUND_FAIL:Cell<bool>=const{Cell::new(false)};
 static DEDUCT:Cell<u32>=const{Cell::new(0)};
 static CALLBACKS:Cell<u32>=const{Cell::new(0)};
}
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn jump(base:usize,rva:usize,f:usize){let mut b=vec![0xff,0x25,0,0,0,0];b.extend_from_slice(&f.to_le_bytes());ptr::copy_nonoverlapping(b.as_ptr(),(base+rva)as*mut u8,b.len());}
unsafe extern "system" fn has(_:usize,id:i32)->u8{EFFECTS.with(|s|s.borrow().contains(&id))as u8}
unsafe extern "system" fn add(_:usize,id:i32,_:u8)->u8{EVENTS.with(|s|s.borrow_mut().push(id));EFFECTS.with(|s|{if !s.borrow().contains(&id){s.borrow_mut().push(id);}});1}
unsafe extern "system" fn remove(_:usize,id:i32)->u8{EFFECTS.with(|s|s.borrow_mut().retain(|&v|v!=id));1}
unsafe extern "system" fn row(out:usize,id:i32)->usize{int(out,id);put(out+8,ROWS.with(|s|s.borrow().iter().find(|r|r.0==id).map_or(0,|r|r.1)));out}
unsafe extern "system" fn lookup(_:usize,id:*const i32,start:i32)->i32{ITEMS.with(|s|s.borrow().iter().filter(|r|r.0==(*id&0x0fffffff)&&r.1>=start).map(|r|r.1).min().unwrap_or(-1))}
unsafe extern "system" fn serial(_:usize,index:i32)->u32{ITEMS.with(|s|s.borrow().iter().find(|r|r.1==index).map_or(0,|r|r.2))}
unsafe extern "system" fn quantity(_:usize,index:i32)->u32{ITEMS.with(|s|s.borrow().iter().find(|r|r.1==index).map_or(0,|r|r.3))}
unsafe extern "system" fn capacity(_:usize)->i32{4}
unsafe extern "system" fn selected(_:usize,out:usize,index:i32)->usize{int(out,ITEMS.with(|s|s.borrow().iter().find(|r|r.1==index).map_or(-1,|r|r.0|0x40000000)));out}
unsafe extern "system" fn deduct(_:usize,index:i32,delta:i32,_:u8)->i32{assert_eq!(delta,-1);DEDUCT.with(|s|s.set(s.get()+1));ITEMS.with(|s|{let mut s=s.borrow_mut();let r=s.iter_mut().find(|r|r.1==index).unwrap();r.3=r.3.saturating_sub(1);r.3 as i32})}
unsafe extern "system" fn succeeded(actor:usize,_:*const i32,index:i32){
 CALLBACKS.with(|s|s.set(s.get()+1));if actor==0{return;}
 let base=BASE.load(Ordering::Acquire);let native:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+0x1f12b0);
 native(read_ptr(actor+0x5a0)+0x1d8,index,0);
}
unsafe extern "system" fn ground(_:usize,_:usize,lot:i32,kind:u8,_:u8,_:u8,_:u8,_:*const f32)->u8{
 assert_eq!((lot,kind),(GROUND_LOT,1));if !GROUND_FAIL.with(Cell::get){GROUNDS.with(|s|s.set(s.get()+1));crate::PERSONAL_REWARD.with(|s|{let mut v=s.get();v.created=1;s.set(v);});}1
}
fn fixture_row(data:&[u8],id:i32)->[u8;0xe0]{let n=u16::from_le_bytes(data[10..12].try_into().unwrap())as usize;for i in 0..n{let at=64+i*24;if i32::from_le_bytes(data[at..at+4].try_into().unwrap())==id{let off=u64::from_le_bytes(data[at+8..at+16].try_into().unwrap())as usize;return data[off..off+0xe0].try_into().unwrap();}}panic!("missing goods {id}")}
#[test]#[ignore="requires original EXE and installed native Ironjar fixture; run alone"]
fn native_metal_consumption_scope_and_gift_transactions(){unsafe{
 let bytes=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();crate::verify_executable(std::path::Path::new(&std::env::var_os("NRHEX_NATIVE_EXE").unwrap())).unwrap();
 let fixture=std::fs::read(std::env::var_os("NRHEX_METAL_FIXTURE").unwrap()).unwrap();let rows:Vec<_>=[GOOD,3500,3550].into_iter().map(|id|(id,Box::new(fixture_row(&fixture,id)))).collect();ROWS.with(|s|*s.borrow_mut()=rows.iter().map(|(id,row)|(*id,row.as_ptr()as usize)).collect());
 let headers=crate::pe::Headers::parse(&bytes,0).unwrap();let image=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!image.is_null());let base=image as usize;for section in &headers.sections{ptr::copy_nonoverlapping(bytes[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 // Native TAE65 ->410220 ->successful use ->PlayerIns vtable+2E8 ->669AE0.
 assert_eq!(ptr::read_unaligned((base+0x2c307f8+0x2e8)as*const u64),0x140000000+CONSUME_RVA as u64);
 let branch=ptr::read((base+0x415560+65)as*const u8)as usize;assert_eq!(ptr::read_unaligned((base+0x415504+branch*4)as*const u32),0x414aad);
 assert_eq!(std::slice::from_raw_parts((base+0x4103d6)as*const u8,6),&[0xff,0x90,0xe8,2,0,0]);
 // Read real quantities from native inventory entries before replacing the
 // inventory accessors for asynchronous transaction scenarios.
 let mut root=[0usize;16];let root_ptr=root.as_mut_ptr()as usize;let mut entries=[0u32;40];put(root_ptr+0x40,entries.as_mut_ptr()as usize);int(root_ptr+0x1c,8);int(root_ptr+0x70,7);
 entries[5*5]=0xb0000dc0;entries[5*5+1]=(0x40000000|GOOD)as u32;entries[5*5+2]=1;entries[5*5+3]=101;
 let native_quantity:unsafe extern "system" fn(usize,i32)->u32=std::mem::transmute(base+0x1f5ca0);assert_eq!(native_quantity(root_ptr,5),1);entries[5*5+2]=2;assert_eq!(native_quantity(root_ptr,5),2);
 for (rva,f) in [(crate::HAS_EFFECT_RVA,has as*const()as usize),(crate::APPLY_SELF_RVA,add as*const()as usize),(crate::REMOVE_EFFECT_RVA,remove as*const()as usize),(wraith::PARAM_RVA,wraith::param as*const()as usize),(0x1f5a20,selected as*const()as usize),(0x1f06b0,deduct as*const()as usize),(0x1f5610,lookup as*const()as usize),(0x1f5700,serial as*const()as usize),(0x1f5ca0,quantity as*const()as usize),(0x1f2370,capacity as*const()as usize),(crate::SPAWN_RVA,ground as*const()as usize)]{jump(base,rva,f);}
 let old_base=BASE.swap(base,Ordering::AcqRel);let old_active=ACTIVE.swap(true,Ordering::AcqRel);let old_param=wraith::PARAM_ORIGINAL.swap(row as*mut u8,Ordering::AcqRel);let old_consume=CONSUME_ORIGINAL.swap(succeeded as*mut u8,Ordering::AcqRel);
 let mut world=vec![0usize;0x17500/8];let mut actor_mem=vec![0usize;0x800/8];let actor=actor_mem.as_mut_ptr()as usize;let mut gd=vec![0usize;0x1000/8];let g=gd.as_mut_ptr()as usize;put(actor+0x5a0,g);put(actor+8,123);put(actor+0x190,1);
 let mut stats=vec![0usize;0x300/8];int(stats.as_mut_ptr()as usize+0x140,100);let mut modules=vec![0usize;0x200/8];modules[0]=stats.as_mut_ptr()as usize;put(actor+0x1b8,modules.as_ptr()as usize);
 let mut physics=vec![0usize;0x200/8];put(modules.as_mut_ptr()as usize+0x68,physics.as_mut_ptr()as usize);let coord=[0usize;8];put(actor+0x60,coord.as_ptr()as usize);
 let mut item_manager=vec![0usize;0x600/8];put(base+crate::ITEMS_RVA,item_manager.as_mut_ptr()as usize);let mut gm=[0usize;4];gm[1]=g;put(base+crate::GAME_DATA_RVA,gm.as_ptr()as usize);int(g+8,1);
 world[0x174e8/8]=actor;put(base+crate::WORLD_RVA,world.as_ptr()as usize);let slots=g+0x1d8+0x248;
 // Native quantity read/branch, not a fake isConsume check. Other actors and
 // unrelated goods still deduct; the same shared row stays byte-identical.
 for owner in [false,true]{for id in [GOOD,3500,3550]{
  EFFECTS.with(|s|*s.borrow_mut()=if owner{vec![OWNER]}else{vec![]});ITEMS.with(|s|*s.borrow_mut()=vec![(id,5,10,2)]);DEDUCT.with(|s|s.set(0));
  let before=*rows.iter().find(|r|r.0==id).unwrap().1;consumed(actor,&(0x40000000|id),5);
  assert_eq!(DEDUCT.with(Cell::get),u32::from(!(owner&&id==GOOD)));assert_eq!(*rows.iter().find(|r|r.0==id).unwrap().1,before);
  let mut out=[0usize;2];wraith::param(out.as_mut_ptr()as usize,id);assert_eq!(out[1],rows.iter().find(|r|r.0==id).unwrap().1.as_ptr()as usize);
 }}
 EFFECTS.with(|s|*s.borrow_mut()=vec![OWNER]);let mut remote=vec![0usize;0x800/8];let remote=remote.as_mut_ptr()as usize;put(remote+0x5a0,g);
 for (target,active,resetting) in [(remote,true,false),(actor,false,false),(actor,true,true)]{
  ACTIVE.store(active,Ordering::Release);if resetting{EFFECTS.with(|s|s.borrow_mut().push(crate::reward_cycle_runtime::RESET));}ITEMS.with(|s|*s.borrow_mut()=vec![(GOOD,5,10,2)]);DEDUCT.with(|s|s.set(0));consumed(target,&(0x40000000|GOOD),5);assert_eq!(DEDUCT.with(Cell::get),1);
 }
 ACTIVE.store(true,Ordering::Release);EFFECTS.with(|s|*s.borrow_mut()=vec![OWNER]);
 {let _outer=Guard::enter(true);{let _inner=Guard::enter(false);assert!(!FREE.with(Cell::get));}assert!(FREE.with(Cell::get));}assert!(!FREE.with(Cell::get));
 // Gift: delayed native event application cannot duplicate; either a new
 // serial or growth of the original stack acknowledges receipt.
 for existing in [false,true]{
  reset();EFFECTS.with(|s|*s.borrow_mut()=vec![OWNER]);EVENTS.with(|s|s.borrow_mut().clear());ITEMS.with(|s|*s.borrow_mut()=if existing{vec![(GOOD,5,10,1)]}else{vec![]});for slot in 0..18{int(slots+12+slot*8,-1);}
  observe(actor,wraith::BUSY,base);observe(actor,wraith::FRAME,base);update(actor,base);assert!(EVENTS.with(|s|s.borrow().is_empty()));
  for _ in 0..3{wraith::query(actor,wraith::FRAME,base);}update(actor,base);assert!(STATE.lock().unwrap().requested.is_some());assert!(!STATE.lock().unwrap().given);
  remove(0,REQUEST);for _ in 0..300{update(actor,base);}assert_eq!(EVENTS.with(|s|s.borrow().iter().filter(|&&id|id==REQUEST).count()),1);
  put(actor+8,456);for _ in 0..3{observe(actor,wraith::FRAME,base);}update(actor,base);assert!(STATE.lock().unwrap().requested.is_some());
  ITEMS.with(|s|if existing{s.borrow_mut()[0].3=2;}else{s.borrow_mut().push((GOOD,5,11,1));});update(actor,base);assert!(STATE.lock().unwrap().given);assert_ne!(has(actor,RECEIPT),0);
  ITEMS.with(|s|s.borrow_mut().clear());for _ in 0..30{update(actor,base);}assert_eq!(EVENTS.with(|s|s.borrow().iter().filter(|&&id|id==REQUEST).count()),1);
  reset();for _ in 0..3{observe(actor,wraith::FRAME,base);}update(actor,base);assert!(STATE.lock().unwrap().given);
 }
 // Full slots and full stacks create one ordinary ground item; failed creation
 // can retry, unclaimed items do not repeat even after reconstruction.
 for full_stack in [false,true]{
  reset();EFFECTS.with(|s|*s.borrow_mut()=vec![OWNER]);EVENTS.with(|s|s.borrow_mut().clear());GROUNDS.with(|s|s.set(0));ITEMS.with(|s|*s.borrow_mut()=if full_stack{vec![(GOOD,5,10,2)]}else{vec![]});
  for slot in 0..18{int(slots+12+slot*8,if full_stack{-1}else{slot as i32});}
  for _ in 0..3{observe(actor,wraith::FRAME,base);}GROUND_FAIL.with(|s|s.set(true));update(actor,base);assert!(!STATE.lock().unwrap().given);GROUND_FAIL.with(|s|s.set(false));update(actor,base);assert!(STATE.lock().unwrap().given);
  for _ in 0..100{update(actor,base);}reset();for _ in 0..3{observe(actor,wraith::FRAME,base);}update(actor,base);assert_eq!(GROUNDS.with(Cell::get),1);assert!(!EVENTS.with(|s|s.borrow().contains(&REQUEST)));
 }
 reset();BASE.store(old_base,Ordering::Release);ACTIVE.store(old_active,Ordering::Release);wraith::PARAM_ORIGINAL.store(old_param,Ordering::Release);CONSUME_ORIGINAL.store(old_consume,Ordering::Release);VirtualFree(image,0,MEM_RELEASE);
}}
