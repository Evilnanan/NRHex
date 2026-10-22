//! Native pending-contact lookup/removal, compiled FP scopes, and slot setter.
use super::*;use crate::win::*;use sha2::{Digest,Sha256};use std::cell::{Cell,RefCell};
thread_local!{static EFFECTS:RefCell<Vec<i32>>=const{RefCell::new(Vec::new())};static NATIVE_XP:Cell<u32>=const{Cell::new(0)};static ROWS:RefCell<Vec<(i32,usize)>>=const{RefCell::new(Vec::new())};static EVENTS:RefCell<Vec<i32>>=const{RefCell::new(Vec::new())};static INVENTORY:RefCell<Vec<(i32,i32)>>=const{RefCell::new(Vec::new())};}
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn jump(base:usize,rva:usize,target:usize){let mut b=vec![0xff,0x25,0,0,0,0];b.extend_from_slice(&target.to_le_bytes());ptr::copy_nonoverlapping(b.as_ptr(),(base+rva)as*mut u8,b.len());}
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from(EFFECTS.with(|s|s.borrow().contains(&id)))}
unsafe extern "system" fn add(_:usize,id:i32,_:u8)->u8{EVENTS.with(|s|s.borrow_mut().push(id));EFFECTS.with(|s|{if !s.borrow().contains(&id){s.borrow_mut().push(id);}});1}
unsafe extern "system" fn remove(_:usize,id:i32)->u8{EFFECTS.with(|s|s.borrow_mut().retain(|&v|v!=id));1}
unsafe extern "system" fn param_original(out:usize,id:i32)->usize{int(out,id);put(out+8,ROWS.with(|s|s.borrow().iter().find(|r|r.0==id).map_or(0,|r|r.1)));out}
unsafe extern "system" fn legality(_:usize,item:*const i32)->u8{let mut out=[0usize;2];param(out.as_mut_ptr()as usize,*item&0x0fffffff);u8::from(out[1]!=0&&ptr::read_unaligned((out[1]+0x80)as*const i16)==0)}
unsafe extern "system" fn use_item(_:usize,good:i32,_:usize,_:u8,_:u8,_:f32)->u8{let item=good|0x40000000;legality(0,&item)}
unsafe extern "system" fn native_xp(_:usize,kind:u8,points:u16){assert_eq!(kind,9);NATIVE_XP.with(|s|s.set(s.get()+u32::from(points)));}
unsafe extern "system" fn free(_:usize,_:usize){}
unsafe extern "system" fn lookup(_:usize,id:*const i32,_:i32)->i32{INVENTORY.with(|items|items.borrow().iter().find(|item|item.0==(*id&0x0fffffff)).map_or(-1,|item|item.1))}
unsafe extern "system" fn uid(_:usize,out:*mut u32,index:i32)->usize{*out=if index>=0{1000+index as u32}else{0};out as usize}
#[unsafe(naked)]unsafe extern "system" fn award_xp(_:usize,_:u8,_:u16){core::arch::naked_asm!("sub rsp,0x28","jmp qword ptr [rip+{target}]",target=sym XP_ENTRY);}
static XP_ENTRY:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
fn fixture_row(data:&[u8],id:i32)->[u8;0xe0]{let count=u16::from_le_bytes(data[10..12].try_into().unwrap())as usize;for i in 0..count{let at=64+i*24;if i32::from_le_bytes(data[at..at+4].try_into().unwrap())==id{let off=u64::from_le_bytes(data[at+8..at+16].try_into().unwrap())as usize;return data[off..off+0xe0].try_into().unwrap();}}panic!("missing goods {id}")}
#[test]#[ignore="requires current native EXE and installed Wraith goods fixture; run alone"]
fn native_wraith_hit_credit_fp_scopes_and_deferred_slot_upgrade(){unsafe{
 let bytes=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&bytes)),crate::EXE_HASH);
 let fixture=std::fs::read(std::env::var_os("NRHEX_WRAITH_FIXTURE").unwrap()).unwrap();let rows:Vec<_>=ORIGINALS.iter().map(|&id|(id,Box::new(fixture_row(&fixture,id)))).collect();ROWS.with(|s|*s.borrow_mut()=rows.iter().map(|(id,r)|(*id,r.as_ptr()as usize)).collect());
 let headers=crate::pe::Headers::parse(&bytes,0).unwrap();let image=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!image.is_null());let base=image as usize;for s in &headers.sections{ptr::copy_nonoverlapping(bytes[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 for(rva,f)in [(crate::HAS_EFFECT_RVA,has as*const()as usize),(crate::APPLY_SELF_RVA,add as*const()as usize),(crate::REMOVE_EFFECT_RVA,remove as*const()as usize),(0x1f5610,lookup as*const()as usize),(0x1f59e0,uid as*const()as usize)]{jump(base,rva,f);}
 // Execute the original CALL at the actual credit return site; the naked
 // wrapper sees 34E0B5, exactly as in the real contact stack frame.
 jump(base,XP_RVA,xp as*const()as usize);ptr::copy_nonoverlapping([0x48,0x83,0xc4,0x28,0xc3].as_ptr(),(base+0x34e0b5)as*mut u8,5);XP_ENTRY.store((base+0x34e0b0)as*mut u8,Ordering::Release);
 let saved_base=BASE.swap(base,Ordering::AcqRel);let saved_active=ACTIVE.swap(true,Ordering::AcqRel);
 PARAM_ORIGINAL.store(param_original as*mut u8,Ordering::Release);CAN_ORIGINAL.store(legality as*mut u8,Ordering::Release);USE_ORIGINAL.store(use_item as*mut u8,Ordering::Release);TOKEN_ORIGINAL.store((base+TOKEN_RVA)as*mut u8,Ordering::Release);XP_ORIGINAL.store(native_xp as*mut u8,Ordering::Release);
 let mut world=vec![0usize;0x17500/8];let mut actor=vec![0usize;0x800/8];let actor=actor.as_mut_ptr()as usize;put(actor+8,123);world[0x174e8/8]=actor;put(base+crate::WORLD_RVA,world.as_ptr()as usize);
 let mut manager=[0usize;16];put(actor+0x190,manager.as_mut_ptr()as usize);let mut gd=vec![0u8;0x800];let g=gd.as_mut_ptr()as usize;put(actor+0x5a0,g);let mut gm=[0usize;4];gm[1]=g;put(base+crate::GAME_DATA_RVA,gm.as_ptr()as usize);ptr::write((g+0xc7)as*mut u8,1);
 let mut modules=vec![0usize;0x200/8];let mut stats=vec![0u8;0x200];int(stats.as_mut_ptr()as usize+0x140,100);modules[0]=stats.as_mut_ptr()as usize;put(actor+0x1b8,modules.as_ptr()as usize);
 EFFECTS.with(|s|*s.borrow_mut()=vec![OWNER,OWNER+90]);reset();
 INVENTORY.with(|items|*items.borrow_mut()=vec![(3080,5),(3010,7)]);
 for &(good,ref original)in &rows{let item=good|0x40000000;assert_eq!(can_use(actor,&item),1);let ctx=[actor];assert_eq!(used(ctx.as_ptr()as usize,good,0,0,0,0.),1);let mut out=[0usize;2];param(out.as_mut_ptr()as usize,good);assert_eq!(out[1],original.as_ptr()as usize);assert_ne!(ptr::read_unaligned((out[1]+0x80)as*const i16),0);}
 EFFECTS.with(|s|s.borrow_mut().clear());assert_eq!(can_use(actor,&(3080|0x40000000)),0);EFFECTS.with(|s|s.borrow_mut().push(OWNER));assert_eq!(can_use(actor+8,&(3080|0x40000000)),0);
 // A pending use token is consumed exactly once by native contact bookkeeping.
 let mut sentinel=[0usize;3];let mut node=[0usize;3];let sent=sentinel.as_mut_ptr()as usize;let n=node.as_mut_ptr()as usize;put(sent,n);put(sent+8,n);put(n,sent);put(n+8,sent);int(n+0x10,77);put(actor+0x748,sent);put(actor+0x750,1);let mut allocator=[0usize;1];let mut vt=[0usize;14];vt[13]=free as*const()as usize;allocator[0]=vt.as_ptr()as usize;put(actor+0x740,allocator.as_ptr()as usize);
 {let mut s=STATE.lock().unwrap();s.bind(actor,123);s.units=Some(0);s.stage=0;}
 let mut descriptor=[0u8;0x110];let desc=descriptor.as_mut_ptr()as usize;put(desc,999);int(desc+8,-1);int(desc+0x18,3080);let emitted_id=77;
 emitted(desc,&emitted_id as*const i32 as usize,base);assert!(STATE.lock().unwrap().tokens.is_empty());
 put(desc,123);int(desc+8,1);emitted(desc,&emitted_id as*const i32 as usize,base);assert!(STATE.lock().unwrap().tokens.is_empty());
 int(desc+8,-1);emitted(desc,&emitted_id as*const i32 as usize,base);assert_eq!(STATE.lock().unwrap().tokens.get(&77),Some(&3080));
 assert_eq!(STATE.lock().unwrap().units,Some(0));assert_eq!(token(actor,&77),1);let consume:unsafe extern "system" fn(usize,*const i32)=std::mem::transmute(base+0x669a20);consume(actor,&77);award_xp(g,9,10);assert_eq!(STATE.lock().unwrap().units,Some(1));assert_eq!(NATIVE_XP.with(Cell::get),0);assert_eq!(token(actor,&77),0);assert_eq!(STATE.lock().unwrap().units,Some(1));
 // A Scholar forwards native XP as well; non-tool/unscoped XP is unchanged.
 ptr::write((g+0xc7)as*mut u8,9);CREDIT.with(|s|s.set(Some(Credit{actor,good:3080})));award_xp(g,9,10);assert_eq!(NATIVE_XP.with(Cell::get),10);assert_eq!(STATE.lock().unwrap().units,Some(2));credited(g,9,40,base+1);assert_eq!(NATIVE_XP.with(Cell::get),50);
 // Native quickslot setter retains selected slot while upgrading in place.
 let slots=g+0x1d8+0x248;put(slots+0x98,1);for i in 0..18{int(slots+12+i*8,-1);}int(slots+12+3*8,5);int(slots+0xa0,3);
 EFFECTS.with(|s|*s.borrow_mut()=vec![OWNER,OWNER+90]);{let mut s=STATE.lock().unwrap();s.units=Some(20);s.stage=0;s.saw_item=true;s.ready=false;}
 update(actor,base,0.);assert_eq!(crate::read_i32(slots+12+3*8),5);assert!(!EFFECTS.with(|s|s.borrow().contains(&(OWNER+91))));
 for _ in 0..3{query(actor,FRAME,base);update(actor,base,0.);}assert_eq!(crate::read_i32(slots+12+3*8),5);query(actor,FRAME,base);update(actor,base,0.);assert_eq!(crate::read_i32(slots+12+3*8),7);assert_eq!(crate::read_i32(slots+0xa0),3);assert!(EFFECTS.with(|s|s.borrow().contains(&(OWNER+91))));assert!(EVENTS.with(|s|s.borrow().contains(&(OWNER+7))));
 for existing in [false,true]{
  reset();EFFECTS.with(|effects|*effects.borrow_mut()=vec![OWNER]);EVENTS.with(|events|events.borrow_mut().clear());
  INVENTORY.with(|items|*items.borrow_mut()=if existing{vec![(3080,5)]}else{vec![]});
  for _ in 0..3{query(actor,FRAME,base);}update(actor,base,0.);
  assert_eq!(EVENTS.with(|events|events.borrow().contains(&(OWNER+1))),!existing);
  if !existing{assert!(!EFFECTS.with(|effects|effects.borrow().contains(&(OWNER+90))));INVENTORY.with(|items|items.borrow_mut().push((3080,5)));update(actor,base,0.);}
  assert!(EFFECTS.with(|effects|effects.borrow().contains(&(OWNER+90))));
  EVENTS.with(|events|events.borrow_mut().clear());update(actor,base,0.);assert!(EVENTS.with(|events|events.borrow().is_empty()));
 }
 for preowned in [false,true]{
  reset();EFFECTS.with(|effects|*effects.borrow_mut()=vec![OWNER,OWNER+90]);EVENTS.with(|events|events.borrow_mut().clear());
  INVENTORY.with(|items|*items.borrow_mut()=if preowned{vec![(3080,5),(3010,7)]}else{vec![(3080,5)]});
  for slot in 0..18{int(slots+12+slot*8,-1);}int(slots+12+3*8,5);int(slots+12+6*8,7);
  {let mut state=STATE.lock().unwrap();state.units=Some(20);}for _ in 0..3{query(actor,FRAME,base);}update(actor,base,0.);
  assert_eq!(EVENTS.with(|events|events.borrow().contains(&(OWNER+2))),!preowned);
  if !preowned{
   assert_eq!(read_i32(slots+12+3*8),5);assert!(!EVENTS.with(|events|events.borrow().contains(&(OWNER+7))));
   INVENTORY.with(|items|items.borrow_mut().push((3010,7)));query(actor,BUSY,base);update(actor,base,0.);assert_eq!(read_i32(slots+12+3*8),5);
   for _ in 0..4{query(actor,FRAME,base);}update(actor,base,0.);
  }
  assert_eq!(read_i32(slots+12+3*8),7);assert_eq!(read_i32(slots+12+6*8),if preowned{7}else{-1});assert_eq!(read_i32(slots+0xa0),3);
  assert!(EVENTS.with(|events|events.borrow().contains(&(OWNER+7))));assert_eq!(STATE.lock().unwrap().given,Some(1));
 }
 for(good,row)in &rows{assert_eq!(row.as_ref(),&fixture_row(&fixture,*good));}
 reset();BASE.store(saved_base,Ordering::Release);ACTIVE.store(saved_active,Ordering::Release);VirtualFree(image,0,MEM_RELEASE);
}}
