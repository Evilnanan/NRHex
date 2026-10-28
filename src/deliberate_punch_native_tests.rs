//! Real input multiplication/combiner code plus the compiled mid-function hooks.
//! Actor identity, effects and accepted-impact dispatch are controlled fixtures.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
use std::cell::Cell;
thread_local!{static BASIC:Cell<bool>=const{Cell::new(false)};}
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from(matches!(id,OWNER|704000)||id==crate::basics::OWNER&&BASIC.with(Cell::get))}
#[unsafe(naked)]unsafe extern "system" fn object_fixture(_:usize,_:f32){core::arch::naked_asm!("push rbp","sub rsp,0x20","lea rbp,[rcx+0x78]","movaps xmm0,xmm1","jmp {detour}",detour=sym object_detour);}
#[unsafe(naked)]unsafe extern "system" fn scale_fixture(_:usize,_:f32){core::arch::naked_asm!("push rbp","sub rsp,0x20","lea rbp,[rcx+0x78]","movaps xmm2,xmm1","shufps xmm2,xmm2,0","jmp {detour}",detour=sym scale_detour);}
#[unsafe(naked)]unsafe extern "system" fn commit_fixture(_:usize,_:usize)->i32{core::arch::naked_asm!("push r12","push r14","sub rsp,0x28","mov r12,rcx","mov r14,rdx","jmp {detour}",detour=sym commit_detour);}
#[test]#[ignore="requires NRHEX_NATIVE_EXE; run alone"]
fn native_punch_shadow_commit_and_additive_bonus(){unsafe{
 let bytes=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&bytes)),crate::EXE_HASH);
 let headers=crate::pe::Headers::parse(&bytes,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for s in &headers.sections{ptr::copy_nonoverlapping(bytes[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&(has as*const()as usize).to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(jump.as_ptr(),(base+crate::HAS_EFFECT_RVA)as*mut u8,jump.len());
 let mut saved=Vec::new();for(rva,detour,original)in [(OBJECT_RVA,object_detour as*const u8,&OBJECT_ORIGINAL),(SCALE_RVA,scale_detour as*const u8,&SCALE_ORIGINAL),(COMMIT_RVA,commit_detour as*const u8,&COMMIT_ORIGINAL)]{
  let hook=crate::hook::prepare((base+rva)as*mut u8,detour,64).unwrap();saved.push((original,original.swap(hook.trampoline,Ordering::AcqRel)));
 }
 // Preserve the actual vector multiplications and the actual post-damage load.
 for(rva,epilogue)in [(0x43a423,&[0x48,0x83,0xc4,0x20,0x5d,0xc3][..]),(0x43a48f,&[0x48,0x83,0xc4,0x20,0x5d,0xc3][..]),(0x43bfb8,&[0x89,0xd0,0x48,0x83,0xc4,0x28,0x41,0x5e,0x41,0x5c,0xc3][..])]{ptr::copy_nonoverlapping(epilogue.as_ptr(),(base+rva)as*mut u8,epilogue.len());}
 let old_calc=crate::glass_cannon::ORIGINAL.swap((base+crate::glass_cannon::RVA)as*mut u8,Ordering::AcqRel);
 let mut actor=vec![0usize;0x720/8];let mut stats=vec![0i32;0x150/4];let mut modules=[0usize;1];modules[0]=stats.as_mut_ptr()as usize;
 actor[0x1b8/8]=modules.as_ptr()as usize;actor[0x190/8]=1;actor[1]=123;let actor=actor.as_ptr()as usize;stats[0x140/4]=1;stats[0x144/4]=1500;
 let module=[0usize,actor];let module=module.as_ptr()as usize;
 let mut world=vec![0usize;0x17500/8];world[0x174e8/8]=actor;ptr::write_unaligned((base+crate::WORLD_RVA)as*mut usize,world.as_ptr()as usize);
 let mut data=[0u8;0xd0];data[0xc7]=5;let game=[0usize,data.as_ptr()as usize];ptr::write_unaligned((base+crate::GAME_DATA_RVA)as*mut usize,game.as_ptr()as usize);
 let old_base=BASE.swap(base,Ordering::AcqRel);let old_active=ACTIVE.swap(true,Ordering::AcqRel);
 let mut hit=[0u8;0x300];let hit=hit.as_mut_ptr()as usize;
 for object in [0.,0.25,1.]{for cut in [0.,0.25,2.5]{
  assert!(identity(actor,base).is_some());event(actor,1,base);let serial=STATE.lock().unwrap().serial;
  IMPACTS.with(|s|s.borrow_mut().push(Impact{module,hit,serial:Some(serial),before:None}));enter(module,hit);
  let mut attack=[100.,20.,30.,40.,10.];object_fixture(attack.as_mut_ptr()as usize,object);scale_fixture(attack.as_mut_ptr()as usize,2.);attack[0]+=50.;
  let one=[1.;5];let cuts=[cut;5];let mut out=[0.;5];
  let post=crate::glass_cannon::detour(out.as_mut_ptr(),attack.as_ptr(),one.as_ptr(),cuts.as_ptr(),one.as_ptr(),one.as_ptr(),one.as_ptr(),one.as_ptr(),1.,1.,1.,1.,1.,1.,1.);
  assert!((post-(400.*object+50.)*cut).abs()<0.01);leave();ptr::write_unaligned((hit+0x280)as*mut i32,post as i32);
  assert_eq!(commit_fixture(module,hit),post as i32);assert_eq!(commit_fixture(module,hit),post as i32);
  let s=STATE.lock().unwrap();assert!((s.before-450.).abs()<0.01,"object={object} cut={cut} before={} after={} cap={} phase={}",s.before,s.after,s.cap,s.phase);assert_eq!(s.after,post as i32 as f64);drop(s);
  IMPACTS.with(|s|{s.borrow_mut().pop();});
 }}
 // No dispatch candidate: previews and cached replays do not add either sum.
 event(actor,1,base);enter(module,hit);assert!(raw_attack([100.;5].as_ptr(),1.).is_none());leave();commit_fixture(module,hit);assert_eq!(STATE.lock().unwrap().before,0.);
 let serial=STATE.lock().unwrap().serial;IMPACTS.with(|s|s.borrow_mut().push(Impact{module,hit,serial:Some(serial),before:Some(300.)}));
 ptr::write((hit+0x2c1)as*mut u8,8);enter(module,hit);assert!(raw_attack([100.;5].as_ptr(),1.).is_none());leave();ptr::write((hit+0x2c1)as*mut u8,0);
 ptr::write((hit+0x2c3)as*mut u8,2);commit_fixture(module,hit);assert_eq!(STATE.lock().unwrap().before,0.);ptr::write((hit+0x2c3)as*mut u8,0);
 ptr::write_unaligned((hit+0x280)as*mut i32,75);commit_fixture(module,hit);IMPACTS.with(|s|{s.borrow_mut().pop();});event(actor,3,base);
 ptr::write_unaligned((hit+0x40)as*mut i32,600000210);ptr::write_unaligned((hit+0x44)as*mut i32,1);constructed(actor,hit,base);
 for basic in [false,true]{BASIC.with(|s|s.set(basic));ptr::write_unaligned((hit+0x280)as*mut i32,100);crate::basics::calculated(actor,actor+8,actor,hit,base);assert_eq!(read_i32(hit+0x280),if basic{262}else{112});}
 event(actor,0,base);assert_eq!(bonus(actor,hit,base),0.);
 STATE.lock().unwrap().clear();BASE.store(old_base,Ordering::Release);ACTIVE.store(old_active,Ordering::Release);crate::glass_cannon::ORIGINAL.store(old_calc,Ordering::Release);for(p,old)in saved{p.store(old,Ordering::Release);}VirtualFree(memory,0,MEM_RELEASE);
}}
