//! Execute the game's charged-hit selector/delivery gate and state275 emitter
//! against installed PARAM rows; only actor/effect storage and final shot API
//! are fixture services. No game process or network is accessed.
use super::*;
use crate::{ptr,read_i32,read_ptr};
use crate::win::*;
use sha2::{Digest,Sha256};
use std::{cell::{Cell,RefCell},collections::BTreeSet,sync::atomic::Ordering};
thread_local!{
 static EFFECTS:RefCell<BTreeSet<i32>>=RefCell::new(BTreeSet::new());
 static MISTS:Cell<usize>=const{Cell::new(0)};
 static SHOTS:Cell<usize>=const{Cell::new(0)};
 static FAIL:Cell<bool>=const{Cell::new(false)};
 static DEFER:Cell<bool>=const{Cell::new(false)};
 static REENTER:Cell<bool>=const{Cell::new(false)};
 static IMAGE:Cell<usize>=const{Cell::new(0)};
}
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn patch(base:usize,rva:usize,bytes:&[u8]){ptr::copy_nonoverlapping(bytes.as_ptr(),(base+rva)as*mut u8,bytes.len());}
unsafe fn jump(base:usize,rva:usize,target:usize){let mut b=vec![0x48,0xb8];b.extend_from_slice(&target.to_le_bytes());b.extend_from_slice(&[0xff,0xe0]);patch(base,rva,&b);}
unsafe extern "system" fn has(_:usize,id:i32)->u8{EFFECTS.with(|s|u8::from(s.borrow().contains(&id)))}
unsafe extern "system" fn remove(_:usize,id:i32)->u8{EFFECTS.with(|s|s.borrow_mut().remove(&id));1}
unsafe extern "system" fn apply(player:usize,id:i32,flag:u8)->u8{
 assert_eq!(flag,0);
 if id==MIST{
  if FAIL.with(Cell::get){return 0;}
  MISTS.with(|s|s.set(s.get()+1));
  if REENTER.with(Cell::get){assert!(pulse(player,PULSE,IMAGE.with(Cell::get)));}
 }else{assert!(matches!(id,COOLDOWN|READY));if DEFER.with(Cell::get){return 1;}}
 EFFECTS.with(|s|s.borrow_mut().insert(id));1
}
unsafe extern "system" fn owner_reference(_:usize,out:usize)->usize{out}
unsafe extern "system" fn suppressed(_:usize)->u8{0}
unsafe extern "system" fn shot(module:usize,behavior:i32,event:i32,descriptor:usize,source:i32,original_source:i32,flag0:u8,flag1:u8,kind:u8){
 assert_ne!(module,0);assert_eq!((behavior,event),(863030000,-1));
 assert_eq!(read_i32(descriptor),80);assert_eq!(ptr::read((descriptor+4)as*const u8),2);
 assert_eq!((source,original_source,flag0,flag1,kind),(16,16,1,0,255));SHOTS.with(|s|s.set(s.get()+1));
}
fn row(bytes:&[u8],id:i32)->[u8;1024]{
 assert_eq!(&bytes[0x2c..0x30],&[0,0x85,7,0]);let count=u16::from_le_bytes(bytes[10..12].try_into().unwrap())as usize;
 for i in 0..count{let at=64+i*24;if i32::from_le_bytes(bytes[at..at+4].try_into().unwrap())==id{let off=u64::from_le_bytes(bytes[at+8..at+16].try_into().unwrap())as usize;return bytes[off..off+1024].try_into().unwrap();}}
 panic!("Missing installed effect {id}")
}
core::arch::global_asm!(r#"
.global heartstealer_delivery_gate
heartstealer_delivery_gate:
 push r15
 sub rsp,0x20
 mov rax,rcx
 mov r15,rdx
 call r8
 add rsp,0x20
 pop r15
 ret
"#);
unsafe extern "system"{fn heartstealer_delivery_gate(row:*const u8,player:usize,fragment:usize)->u8;}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE and NRHEX_HEARTSTEALER_FIXTURES; run alone"]
fn native_heartstealer_charge_selector_and_mist_emit_use_original_chain(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let params=std::fs::read(std::env::var_os("NRHEX_HEARTSTEALER_FIXTURES").unwrap()).unwrap();
 let owner=row(&params,OWNER);let reference=row(&params,8320400);let pulse_row=row(&params,PULSE);let mist=row(&params,MIST);let original_mist=row(&params,8630302);
 assert_eq!(&owner[0x100..0x104],&reference[0x100..0x104]);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 patch(base,0x26909a0,&[0xc3]);patch(base,0x43c9f8,&[0xb0,1,0xc3]);patch(base,0x43ca12,&[0x30,0xc0,0xc3]);
 for(rva,target)in [(crate::HAS_EFFECT_RVA,has as*const()as usize),(crate::APPLY_SELF_RVA,apply as*const()as usize),(crate::REMOVE_EFFECT_RVA,remove as*const()as usize),(0x3ced90,owner_reference as*const()as usize),(0x4d1b20,suppressed as*const()as usize),(0x397570,shot as*const()as usize)]{jump(base,rva,target);}
 let saved_base=crate::BASE.swap(base,Ordering::AcqRel);let saved_active=crate::ACTIVE.swap(true,Ordering::AcqRel);IMAGE.with(|s|s.set(base));
 let saved_gate={let mut g=GATE.lock().unwrap();std::mem::replace(&mut*g,Gate::new())};
 let mut actor=[0usize;0x720/8];let player=actor.as_mut_ptr()as usize;let mut modules=[0usize;4];let modules=modules.as_mut_ptr()as usize;put(player+0x1b8,modules);
 let mut stats=[0usize;0x150/8];let stats=stats.as_mut_ptr()as usize;put(modules,stats);int(stats+0x140,1000);
 let mut world=vec![0usize;0x17500/8];let world=world.as_mut_ptr()as usize;put(base+crate::WORLD_RVA,world);put(world+0x174e8,player);
 let mut manager=[0usize;0x40/8];let manager=manager.as_mut_ptr()as usize;put(player+0x190,manager);put(manager+0x10,player);
 let mut node=[0usize;0x70/8];let node=node.as_mut_ptr()as usize;put(node,owner.as_ptr()as usize);put(manager+8,node);
 assert_eq!(heartstealer_delivery_gate(pulse_row.as_ptr(),player,base+0x43c9dc),1);let mut broken=pulse_row;broken[0x160]&=!2;assert_eq!(heartstealer_delivery_gate(broken.as_ptr(),player,base+0x43c9dc),0);
 let mut hit=[0u8;0x2c8];let h=hit.as_mut_ptr()as usize;int(h+0x280,100);int(h+0x40,10000);int(h+0x44,1);
 let mut ids=[0i32;8];let mut out=[0usize;4];let outp=out.as_mut_ptr()as usize;
 let select:unsafe extern "system" fn(usize,usize,usize)=std::mem::transmute(base+0x505e90);
 let mut notify=|kind:u8,tag:usize|{
  hit[0x122]=kind;hit[0x124..0x144].fill(0);if tag>0{hit[0x124+tag/8]|=1<<(tag%8);}
  put(outp+8,ids.as_mut_ptr()as usize);put(outp+0x10,ids.as_mut_ptr()as usize);put(outp+0x18,ids.as_mut_ptr()as usize+32);select(manager,outp,h);
  let count=(read_ptr(outp+0x10)-read_ptr(outp+8))/4;for &id in &ids[..count]{assert_eq!(id,PULSE);assert_eq!(crate::effect_detour(player,id,player,0,0,0,0,0,0,0,0),1);}count
 };
 EFFECTS.with(|s|s.borrow_mut().insert(OWNER));
 assert!(ready(player,base));update(player,base,1./60.);assert!(has_effect(player,READY,base));
 // The actual native selector rejects empty category/ordinary/charged spells.
 for(kind,tag)in [(1,0),(1,102),(3,110),(4,110),(3,100),(4,100)]{assert_eq!(notify(kind,tag),0);assert_eq!(MISTS.with(Cell::get),0);}
 REENTER.with(|s|s.set(true));DEFER.with(|s|s.set(true));
 assert_eq!(notify(1,100),1);assert_eq!(MISTS.with(Cell::get),1);for _ in 0..100{assert_eq!(notify(1,100),1);}assert_eq!(MISTS.with(Cell::get),1);
 assert!(!ready(player,base));assert!(!has_effect(player,READY,base));
 // A missing/deferred native effect cannot reopen the 30-second reservation.
 for _ in 0..29{update(player,base,1.);notify(1,100);}assert_eq!(MISTS.with(Cell::get),1);
 for dt in [0.,-1.,f32::NAN,f32::INFINITY,1.1]{update(player,base,dt);notify(1,100);}assert_eq!(MISTS.with(Cell::get),1);
 update(player,base,1.);assert!(ready(player,base));DEFER.with(|s|s.set(false));notify(1,100);assert_eq!(MISTS.with(Cell::get),2);assert!(has_effect(player,COOLDOWN,base));
 for _ in 0..30{update(player,base,1.);}notify(1,100);assert_eq!(MISTS.with(Cell::get),2);EFFECTS.with(|s|{s.borrow_mut().remove(&COOLDOWN);});notify(1,100);assert_eq!(MISTS.with(Cell::get),3);
 // Restored native cooldown blocks even when this DLL's reservation is empty.
 *GATE.lock().unwrap()=Gate::new();notify(1,100);assert_eq!(MISTS.with(Cell::get),3);
 assert!(!ready(player,base));
 EFFECTS.with(|s|{s.borrow_mut().remove(&COOLDOWN);});
 for(mode,state,hp,owned)in [(0,0,0,true),(1,1,100,true),(2,2,100,true),(3,0,100,false)]{
  *GATE.lock().unwrap()=Gate::new();ptr::write((player+0x710)as*mut u8,state);int(stats+0x140,hp);EFFECTS.with(|s|{if owned{s.borrow_mut().insert(OWNER);}else{s.borrow_mut().remove(&OWNER);}});
  assert!(pulse(player,PULSE,base));assert_eq!(MISTS.with(Cell::get),3,"Invalid local actor mode {mode}");
  assert!(!ready(player,base));update(player,base,1./60.);assert!(!has_effect(player,READY,base));
 }
 ptr::write((player+0x710)as*mut u8,0);int(stats+0x140,100);EFFECTS.with(|s|{s.borrow_mut().insert(OWNER);});assert!(pulse(player+8,PULSE,base));assert_eq!(MISTS.with(Cell::get),3);
 FAIL.with(|s|s.set(true));assert!(pulse(player,PULSE,base));assert_eq!(GATE.lock().unwrap().remaining,0.);FAIL.with(|s|s.set(false));pulse(player,PULSE,base);assert_eq!(MISTS.with(Cell::get),4);
 // Native 50AA20 consumes only the first-frame emission flag, dispatches the
 // original behavior at dummy80, and ignores ordinary/expired effect nodes.
 let emit:unsafe extern "system" fn(usize)=std::mem::transmute(base+0x50aa20);int(manager+0x28,1<<24);
 for r in [&mist,&original_mist]{put(node,r.as_ptr()as usize);int(node+0x68,4);let before=SHOTS.with(Cell::get);emit(manager);assert_eq!(SHOTS.with(Cell::get),before+1);for flags in [0,8,0x40004,0x80004,0x80000004u32 as i32]{int(node+0x68,flags);emit(manager);}assert_eq!(SHOTS.with(Cell::get),before+1);}
 put(node,owner.as_ptr()as usize);int(node+0x68,4);let before=SHOTS.with(Cell::get);emit(manager);assert_eq!(SHOTS.with(Cell::get),before);
 reset(player,base);assert_eq!(GATE.lock().unwrap().remaining,0.);assert!(!has_effect(player,COOLDOWN,base)&&!has_effect(player,MIST,base));
 assert!(!has_effect(player,READY,base));assert!(ready(player,base));update(player,base,1./60.);assert!(has_effect(player,READY,base));
 EFFECTS.with(|effects|{effects.borrow_mut().insert(crate::reward_cycle_runtime::RESET);});
 update(player,base,1./60.);assert!(!ready(player,base));assert!(!has_effect(player,READY,base));
 *GATE.lock().unwrap()=saved_gate;crate::BASE.store(saved_base,Ordering::Release);crate::ACTIVE.store(saved_active,Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
