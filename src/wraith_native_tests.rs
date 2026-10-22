//! Native pending-contact lookup/removal, compiled FP scopes, and slot setter.
use super::*;use crate::win::*;use sha2::{Digest,Sha256};use std::cell::{Cell,RefCell};
thread_local!{static EFFECTS:RefCell<Vec<i32>>=const{RefCell::new(Vec::new())};static NATIVE_XP:Cell<u32>=const{Cell::new(0)};static ROWS:RefCell<Vec<(i32,usize)>>=const{RefCell::new(Vec::new())};static EVENTS:RefCell<Vec<i32>>=const{RefCell::new(Vec::new())};static INVENTORY:RefCell<Vec<(i32,i32)>>=const{RefCell::new(Vec::new())};static ITEM_INTERFACE:Cell<usize>=const{Cell::new(0)};static OTHER_LEGAL:Cell<bool>=const{Cell::new(true)};static LAST_CAN:Cell<(i32,usize,usize,i32,i32,i32,u8)>=const{Cell::new((0,0,0,0,0,0,0))};}
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn jump(base:usize,rva:usize,target:usize){let mut b=vec![0xff,0x25,0,0,0,0];b.extend_from_slice(&target.to_le_bytes());ptr::copy_nonoverlapping(b.as_ptr(),(base+rva)as*mut u8,b.len());}
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from(EFFECTS.with(|s|s.borrow().contains(&id)))}
unsafe extern "system" fn add(_:usize,id:i32,_:u8)->u8{EVENTS.with(|s|s.borrow_mut().push(id));EFFECTS.with(|s|{if !s.borrow().contains(&id){s.borrow_mut().push(id);}});1}
unsafe extern "system" fn remove(_:usize,id:i32)->u8{EFFECTS.with(|s|s.borrow_mut().retain(|&v|v!=id));1}
unsafe extern "system" fn param_original(out:usize,id:i32)->usize{int(out,id);put(out+8,ROWS.with(|s|s.borrow().iter().find(|r|r.0==id).map_or(0,|r|r.1)));out}
// Execute the real native FP read/cost read and final insufficient-FP branch.
// Other legality conditions are an explicit fixture, rather than silently
// replacing the entire usability result with consumeMP == 0.
core::arch::global_asm!(r#"
.global wraith_native_fp
wraith_native_fp:
 push rbp
 push r12
 sub rsp,0xc8
 lea rbp,[rsp+0x80]
 mov r12,rcx
 mov rcx,rdx
 mov qword ptr [rbp-0x70],0
 mov [rsp+0x90],r9
 call r8
 call qword ptr [rsp+0x90]
 add rsp,0xc8
 pop r12
 pop rbp
 ret
"#);
unsafe extern "system"{fn wraith_native_fp(actor:usize,row:usize,read:usize,compare:usize)->u8;}
unsafe extern "system" fn legality(good:i32,actor:usize,manager:usize,kind:i32,right:i32,left:i32,flag:u8)->u8{
 LAST_CAN.with(|s|s.set((good,actor,manager,kind,right,left,flag)));
 if actor==0||!OTHER_LEGAL.with(Cell::get){return 0;}
 let mut out=[0usize;2];param(out.as_mut_ptr()as usize,good);if out[1]==0{return 0;}
 let base=BASE.load(Ordering::Acquire);wraith_native_fp(actor,out[1],base+0x6d10f3,base+0x6d124a)
}
unsafe extern "system" fn use_item(context:usize,good:i32,_:usize,_:u8,_:u8,_:f32)->u8{legality(good,read_ptr(context),0,0,-1,-2,0)}
unsafe extern "system" fn weapon(_:usize,_:i32)->i32{-1}
unsafe extern "system" fn item_interface(_:usize)->usize{ITEM_INTERFACE.with(Cell::get)}
unsafe extern "system" fn interface_enabled(_:usize)->u8{1}
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
 // Stop after native cost calculation, then retain the actual native CMP/JL.
 // HP, state and inventory gates are represented by OTHER_LEGAL below.
 ptr::write((base+0x6d116c)as*mut u8,0xc3);
 ptr::copy_nonoverlapping([0xb0,1,0xc3].as_ptr(),(base+0x6d1253)as*mut u8,3);
 ptr::copy_nonoverlapping([0x30,0xc0,0xc3].as_ptr(),(base+0x6d1260)as*mut u8,3);
 let shared=crate::hook::prepare((base+CAN_RVA)as*mut u8,can_use as*const u8,64).unwrap();
 crate::hook::write_code((base+CAN_RVA)as*mut u8,&shared.patch).unwrap();
 jump(base,PARAM_RVA,param as*const()as usize);
 let selected:unsafe extern "system" fn(usize)->i32=std::mem::transmute(base+0x3cfdb0);
 let common:CanUse=std::mem::transmute(base+CAN_RVA);
 let can=|actor:usize,good:i32|common(good,actor,0x12345678,-33,-44,-55,0x86);
 let saved_base=BASE.swap(base,Ordering::AcqRel);let saved_active=ACTIVE.swap(true,Ordering::AcqRel);
 PARAM_ORIGINAL.store(param_original as*mut u8,Ordering::Release);CAN_ORIGINAL.store(legality as*mut u8,Ordering::Release);USE_ORIGINAL.store(use_item as*mut u8,Ordering::Release);TOKEN_ORIGINAL.store((base+TOKEN_RVA)as*mut u8,Ordering::Release);XP_ORIGINAL.store(native_xp as*mut u8,Ordering::Release);
 let mut world=vec![0usize;0x17500/8];let mut actor=vec![0usize;0x800/8];let actor=actor.as_mut_ptr()as usize;put(actor+8,123);world[0x174e8/8]=actor;put(base+crate::WORLD_RVA,world.as_ptr()as usize);
 let mut manager=[0usize;16];put(actor+0x190,manager.as_mut_ptr()as usize);let mut gd=vec![0u8;0x800];let g=gd.as_mut_ptr()as usize;put(actor+0x5a0,g);let mut gm=[0usize;4];gm[1]=g;put(base+crate::GAME_DATA_RVA,gm.as_ptr()as usize);ptr::write((g+0xc7)as*mut u8,1);
 let mut modules=vec![0usize;0x200/8];let mut stats=vec![0u8;0x200];int(stats.as_mut_ptr()as usize+0x140,100);modules[0]=stats.as_mut_ptr()as usize;put(actor+0x1b8,modules.as_ptr()as usize);
 let mut actor_vtable=vec![0usize;0x610/8];actor_vtable[0x250/8]=weapon as*const()as usize;actor_vtable[0x608/8]=item_interface as*const()as usize;put(actor,actor_vtable.as_ptr()as usize);
 let mut interface_vtable=vec![0usize;0x110/8];interface_vtable[0x108/8]=interface_enabled as*const()as usize;let interface=[interface_vtable.as_ptr()as usize];ITEM_INTERFACE.with(|s|s.set(interface.as_ptr()as usize));
 EFFECTS.with(|s|*s.borrow_mut()=vec![OWNER,OWNER+90]);reset();
 // Reproduce the missed path: the original selected-item getter calls the
 // common gate directly, bypassing the old 3D49A0 wrapper entirely.
 int(actor+0x174,3080|0x40000000);jump(base,CAN_RVA,legality as*const()as usize);assert_eq!(selected(actor),255);
 crate::hook::write_code((base+CAN_RVA)as*mut u8,&shared.patch).unwrap();

 INVENTORY.with(|items|*items.borrow_mut()=vec![(3080,5),(3010,7)]);
 for &(good,ref original)in &rows{
  let cost=i16::from_le_bytes(original[0x80..0x82].try_into().unwrap())as i32;assert!(cost>0);
  for fp in [0,1,cost-1,cost,cost+1]{
   int(stats.as_mut_ptr()as usize+0x150,fp);assert_eq!(can(actor,good),1,"owner good {good}, FP {fp}");
   assert_eq!(LAST_CAN.with(Cell::get),(good,actor,0x12345678,-33,-44,-55,0x86));
   assert!(!FREE.with(Cell::get));assert_eq!(read_i32(stats.as_ptr()as usize+0x150),fp);
   int(actor+0x174,good|0x40000000);assert_eq!(selected(actor),i32::from(original[0x42]),"native selected item no longer reports unusable");
  }
  int(stats.as_mut_ptr()as usize+0x150,0);let ctx=[actor];assert_eq!(used(ctx.as_ptr()as usize,good,0,0,0,0.),1);
  let mut out=[0usize;2];param(out.as_mut_ptr()as usize,good);assert_eq!(out[1],original.as_ptr()as usize);assert_ne!(ptr::read_unaligned((out[1]+0x80)as*const i16),0);
  EFFECTS.with(|s|s.borrow_mut().clear());
  for fp in [0,cost-1,cost,cost+1]{int(stats.as_mut_ptr()as usize+0x150,fp);assert_eq!(can(actor,good),u8::from(fp>=cost));assert_eq!(selected(actor),if fp>=cost{i32::from(original[0x42])}else{255});}
  EFFECTS.with(|s|s.borrow_mut().push(OWNER));
 }
 int(stats.as_mut_ptr()as usize+0x150,0);OTHER_LEGAL.with(|s|s.set(false));assert_eq!(can(actor,3080),0);OTHER_LEGAL.with(|s|s.set(true));
 let mut remote=vec![0usize;0x800/8];let remote=remote.as_mut_ptr()as usize;put(remote+0x1b8,modules.as_ptr()as usize);assert_eq!(can(remote,3080),0);
 ACTIVE.store(false,Ordering::Release);assert_eq!(can(actor,3080),0);ACTIVE.store(true,Ordering::Release);
 EFFECTS.with(|s|s.borrow_mut().push(crate::reward_cycle_runtime::RESET));assert_eq!(can(actor,3080),0);EFFECTS.with(|s|s.borrow_mut().retain(|&id|id!=crate::reward_cycle_runtime::RESET));
 let other=Box::new(*rows[0].1);ROWS.with(|s|s.borrow_mut().push((3050,other.as_ptr()as usize)));assert_eq!(can(actor,3050),0);assert_eq!(can(actor,-1),0);assert_eq!(can(0,3080),0);
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
