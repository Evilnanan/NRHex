//! Compiled runtime with deferred effect requests and the common worker protocol.
//! Event byte bindings are independently validated by HexBuild; no game event VM.
use super::*;use crate::{ptr,read_i32,BASE,win::*};
use std::{cell::RefCell,sync::atomic::Ordering};
thread_local!{
 static EFFECTS:RefCell<std::collections::BTreeSet<i32>>=const{RefCell::new(std::collections::BTreeSet::new())};
 static QUEUED:RefCell<Vec<i32>>=const{RefCell::new(Vec::new())};
 static POPUPS:RefCell<Vec<i32>>=const{RefCell::new(Vec::new())};
 static WORKERS:RefCell<std::collections::BTreeSet<i32>>=const{RefCell::new(std::collections::BTreeSet::new())};
}
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn jump(base:usize,rva:usize,f:usize){let mut b=vec![0xff,0x25,0,0,0,0];b.extend_from_slice(&f.to_le_bytes());ptr::copy_nonoverlapping(b.as_ptr(),(base+rva)as*mut u8,b.len());}
unsafe extern "system" fn has(_:usize,id:i32)->u8{EFFECTS.with(|s|s.borrow().contains(&id))as u8}
unsafe extern "system" fn add(_:usize,id:i32,_:u8)->u8{QUEUED.with(|s|s.borrow_mut().push(id));1}
unsafe extern "system" fn remove(_:usize,id:i32)->u8{EFFECTS.with(|s|s.borrow_mut().remove(&id));1}
unsafe extern "system" fn popup(menu:usize,id:i32,record_only:u8){assert_eq!(menu,0x123400);assert_eq!(record_only,0);POPUPS.with(|s|s.borrow_mut().push(id));}
fn flush(){let ids=QUEUED.with(|s|std::mem::take(&mut*s.borrow_mut()));EFFECTS.with(|s|s.borrow_mut().extend(ids));}
fn initialize(effects:impl IntoIterator<Item=i32>){reset();EFFECTS.with(|s|*s.borrow_mut()=effects.into_iter().collect());QUEUED.with(|s|s.borrow_mut().clear());POPUPS.with(|s|s.borrow_mut().clear());WORKERS.with(|s|s.borrow_mut().clear());}
// This is an explicit event-protocol fixture, not native event execution.
fn workers(){
 let effects=EFFECTS.with(|s|s.borrow().clone());if !effects.contains(&OWNER)||!effects.contains(&READY)||effects.contains(&RECEIPT)||effects.contains(&crate::reward_cycle_runtime::RESET){return;}
 for (i,&owner)in OWNER_IDS.iter().enumerate(){
  if owner!=OWNER&&effects.contains(&marker(i))&&!effects.contains(&owner)&&WORKERS.with(|s|s.borrow_mut().insert(owner)){
   QUEUED.with(|s|s.borrow_mut().push(owner));
  }
 }
}
#[test]#[ignore="requires current EXE; isolated compiled delivery protocol; run alone"]
fn compiled_double_dip_deferred_selection_grant_and_recovery(){unsafe{
 let path=std::path::PathBuf::from(std::env::var_os("NRHEX_NATIVE_EXE").unwrap());crate::verify_executable(&path).unwrap();let bytes=std::fs::read(path).unwrap();let headers=crate::pe::Headers::parse(&bytes,0).unwrap();
 let image=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!image.is_null());let base=image as usize;for section in &headers.sections{ptr::copy_nonoverlapping(bytes[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 for(rva,f)in [(crate::HAS_EFFECT_RVA,has as*const()as usize),(crate::APPLY_SELF_RVA,add as*const()as usize),(crate::REMOVE_EFFECT_RVA,remove as*const()as usize)]{jump(base,rva,f);}
 jump(base,POPUP_RVA,popup as*const()as usize);put(base+MENU_RVA,0x123400);
 let old_base=BASE.swap(base,Ordering::AcqRel);let mut world=vec![0usize;0x17500/8];let mut actor_mem=vec![0usize;0x800/8];let actor=actor_mem.as_mut_ptr()as usize;put(actor+0x190,1);world[0x174e8/8]=actor;put(base+crate::WORLD_RVA,world.as_ptr()as usize);
 let mut stats=vec![0usize;0x200/8];let stat=stats.as_mut_ptr()as usize;int(stat+0x140,100);let modules=[stat];put(actor+0x1b8,modules.as_ptr()as usize);let mut data=vec![0u8;0x200];let d=data.as_mut_ptr()as usize;let gd=[0usize,d];put(base+crate::GAME_DATA_RVA,gd.as_ptr()as usize);
 for hero in 1..=10{
  ptr::write((d+0xc7)as*mut u8,hero);initialize([OWNER,95000900,95002600]);update(actor,base);let selected=STATE.lock().unwrap().selected.clone().unwrap();assert_eq!(selected.len(),2);assert_ne!(selected[0],selected[1]);assert!(selected.iter().all(|&i|role_allowed(OWNER_IDS[i],Some(hero))&&!matches!(OWNER_IDS[i],95000900|95002600)));
  assert_eq!(QUEUED.with(|s|s.borrow().clone()),selected.iter().map(|&i|marker(i)).collect::<Vec<_>>());
  for _ in 0..40{update(actor,base);workers();}assert_eq!(QUEUED.with(|s|s.borrow().len()),2);assert!(POPUPS.with(|s|s.borrow().is_empty()));
  flush();for _ in 0..10{update(actor,base);}assert_eq!(QUEUED.with(|s|s.borrow().clone()),vec![READY]);workers();assert!(POPUPS.with(|s|s.borrow().is_empty()));
  flush();workers();assert!(POPUPS.with(|s|s.borrow().is_empty()));for _ in 0..10{update(actor,base);}assert!(!QUEUED.with(|s|s.borrow().contains(&RECEIPT)));assert!(POPUPS.with(|s|s.borrow().is_empty()));
  // Character reconstruction preserves the frozen selection and pending grants.
  put(actor+8,999);update(actor,base);assert_eq!(STATE.lock().unwrap().selected,Some(selected.clone()));flush();
  // Both gifts have arrived, but a missing UI must not silently consume them.
  put(base+MENU_RVA,0);for _ in 0..10{update(actor,base);}assert!(POPUPS.with(|s|s.borrow().is_empty()));assert!(QUEUED.with(|s|s.borrow().is_empty()));
  put(base+MENU_RVA,0x123400);update(actor,base);assert_eq!(QUEUED.with(|s|s.borrow().clone()),vec![RECEIPT]);assert_eq!(POPUPS.with(|s|s.borrow().clone()),selected.iter().map(|&i|OWNER_IDS[i]).collect::<Vec<_>>());
  for _ in 0..10{update(actor,base);}assert_eq!(QUEUED.with(|s|s.borrow().len()),1);flush();update(actor,base);assert_eq!(has(actor,READY),0);for &i in &selected{assert_eq!(has(actor,marker(i)),0);assert_eq!(has(actor,OWNER_IDS[i]),1);}
  reset();for _ in 0..30{update(actor,base);workers();}assert!(QUEUED.with(|s|s.borrow().is_empty()));assert_eq!(POPUPS.with(|s|s.borrow().len()),2);
 }
 let first=OWNER_IDS.iter().position(|&id|id==95004500).unwrap();ptr::write((d+0xc7)as*mut u8,3);
 initialize([OWNER,marker(first)]);update(actor,base);let selected=STATE.lock().unwrap().selected.clone().unwrap();assert_eq!(selected.len(),2);assert!(selected.contains(&first));assert_eq!(QUEUED.with(|s|s.borrow().len()),1);
 // Resume after READY and one committed target, without another random draw.
 let other=selected.iter().copied().find(|&i|i!=first).unwrap();initialize([OWNER,READY,marker(first),marker(other),OWNER_IDS[first]]);update(actor,base);assert_eq!(STATE.lock().unwrap().selected.as_ref().unwrap().iter().copied().collect::<HexSet>(),selected.iter().copied().collect::<HexSet>());assert!(QUEUED.with(|s|s.borrow().is_empty()));assert_eq!(POPUPS.with(|s|s.borrow().clone()),vec![OWNER_IDS[first]]);workers();flush();update(actor,base);flush();update(actor,base);assert_eq!(POPUPS.with(|s|s.borrow().clone()),vec![OWNER_IDS[first],OWNER_IDS[other]]);
 // A saved completion/reset receipt must block workers before the DLL gets
 // its cleanup frame, even if READY/selection survived but a target is absent.
 for terminal in [RECEIPT,crate::reward_cycle_runtime::RESET]{
  initialize([OWNER,READY,marker(first),terminal]);workers();assert!(QUEUED.with(|s|s.borrow().is_empty()));assert!(POPUPS.with(|s|s.borrow().is_empty()));update(actor,base);assert!(QUEUED.with(|s|s.borrow().is_empty()));
 }
 // Missing ownership, unknown hero, down/dead, reset and foreign actors cannot
 // begin or reroll a delivery. Ordinary mutual exclusions are separate tests.
 for mode in 0..6{
  initialize(if mode==0{vec![]}else if mode==4{vec![OWNER,crate::reward_cycle_runtime::RESET]}else{vec![OWNER]});ptr::write((d+0xc7)as*mut u8,if mode==1{0}else{3});int(stat+0x140,if mode==2{0}else{100});ptr::write((actor+0x710)as*mut u8,if mode==3{1}else{0});if mode==5{world[0x174e8/8]=0;}
  update(actor,base);assert!(QUEUED.with(|s|s.borrow().is_empty()));assert!(STATE.lock().unwrap().selected.is_none());world[0x174e8/8]=actor;
 }
 assert_eq!(read_i32(stat+0x140),100);reset();BASE.store(old_base,Ordering::Release);VirtualFree(image,0,MEM_RELEASE);
}}
