//! Run the original cooldown product and compiled gates/damage against fixtures.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
use std::{cell::{Cell,RefCell},sync::atomic::Ordering};
thread_local!{static OWNED:Cell<bool>=const{Cell::new(false)};static RESET:Cell<bool>=const{Cell::new(false)};static QUERY:Cell<i32>=const{Cell::new(0)};static EFFECTS:RefCell<Vec<(usize,i32)>>=const{RefCell::new(Vec::new())};static APPLIED:RefCell<Vec<(usize,i32)>>=const{RefCell::new(Vec::new())};}
unsafe extern "system" fn has(manager:usize,id:i32)->u8{u8::from(if id==OWNER{OWNED.with(Cell::get)}else if id==crate::reward_cycle_runtime::RESET{RESET.with(Cell::get)}else{EFFECTS.with(|s|s.borrow().contains(&(manager,id)))})}
unsafe extern "system" fn applied(actor:usize,id:i32,source:usize,a4:usize,a5:usize,a6:u8,a7:u8,a8:u8,a9:u8,a10:u8,a11:i32)->u8{
 assert_eq!((source,a4,a5,a6,a7,a8,a9,a10,a11),(actor,1,2,3,4,5,6,7,-8));
 APPLIED.with(|s|s.borrow_mut().push((actor,id)));
 // Native category-201 allocation/replacement is a fixture; parameter
 // verification separately proves active/private/native all retain category201/priority51.
 EFFECTS.with(|s|{let mut s=s.borrow_mut();s.retain(|&(p,_)|p!=actor);s.push((actor,id));});1
}
unsafe extern "system" fn apply(actor:usize,id:i32,flag:u8)->u8{assert_eq!(flag,0);crate::effect_detour(actor,id,actor,1,2,3,4,5,6,7,-8)}
unsafe extern "system" fn env(_:usize,code:i32,vm:usize)->i32{assert_eq!(vm,17);QUERY.with(|s|s.set(code));1}
#[unsafe(naked)]unsafe extern "system" fn restage_fixture(_:usize,_:f32)->i32{
 core::arch::naked_asm!("push rdi","push r14","sub rsp,0x28","mov rdi,rcx","xor r14d,r14d","jmp {detour}",detour=sym restage_detour);
}
fn row(bytes:&[u8],id:i32)->&[u8]{let count=u16::from_le_bytes(bytes[10..12].try_into().unwrap())as usize;for i in 0..count{let at=64+i*24;if i32::from_le_bytes(bytes[at..at+4].try_into().unwrap())==id{let start=u64::from_le_bytes(bytes[at+8..at+16].try_into().unwrap())as usize;return &bytes[start..];}}panic!("missing {id}")}
#[test]#[ignore="requires original EXE and installed Basics parameter fixture; run alone"]
fn native_basics_cooldown_product_seal_and_damage(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let params=std::fs::read(std::env::var_os("NRHEX_BASICS_FIXTURE").unwrap()).unwrap();let headers=crate::pe::Headers::parse(&raw,0).unwrap();
 let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&(has as*const()as usize).to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(jump.as_ptr(),(base+crate::HAS_EFFECT_RVA)as*mut u8,jump.len());
 let original:unsafe extern "system" fn(usize)->f32=std::mem::transmute(base+0x508480);
 let mut manager=[0usize;4];let mut first=[0usize;0x70/8];let mut second=[0usize;0x70/8];manager[1]=first.as_mut_ptr()as usize;first[0x38/8]=second.as_mut_ptr()as usize;
 for (id,expected)in [(OWNER+1,0.7),(OWNER+2,0.8),(OWNER+3,0.5),(OWNER+7,0.6)]{
  first[0]=row(&params,id).as_ptr()as usize;second[0]=row(&params,7000802).as_ptr()as usize;
  assert!((original(manager.as_ptr()as usize)-expected*0.9).abs()<1e-6);
  first[0x68/8]=1;assert!((original(manager.as_ptr()as usize)-0.9).abs()<1e-6);first[0x68/8]=0;
 }
 let mut world=vec![0usize;0x17500/8];let mut actor=vec![0usize;0x200/8];actor[0x190/8]=1;let actor=actor.as_mut_ptr()as usize;world[0x174e8/8]=actor;ptr::write_unaligned((base+crate::WORLD_RVA)as*mut usize,world.as_ptr()as usize);
 let mut game=[0usize;2];let mut data=[0u8;0xd0];game[1]=data.as_mut_ptr()as usize;ptr::write_unaligned((base+crate::GAME_DATA_RVA)as*mut usize,game.as_ptr()as usize);
 let saved_base=crate::BASE.swap(base,Ordering::AcqRel);let saved_active=crate::ACTIVE.swap(true,Ordering::AcqRel);let saved_env=crate::ambulance::ENV_ORIGINAL.swap(env as*mut u8,Ordering::AcqRel);let context=[actor];
 // Retain original MULSS, native summation and truncation instructions. Only
 // replace the continuation with our fixture epilogue, preserving RDI/R14.
 let prepared=crate::hook::prepare((base+RESTAGE_RVA)as*mut u8,restage_detour as*const u8,64).unwrap();assert_eq!(prepared.stolen,5);
 let saved_rest=RESTAGE_ORIGINAL.swap(prepared.trampoline,Ordering::AcqRel);
 let epilogue=[0x44,0x89,0xf0,0x48,0x83,0xc4,0x28,0x41,0x5e,0x5f,0xc3];ptr::copy_nonoverlapping(epilogue.as_ptr(),(base+0x449add)as*mut u8,epilogue.len());
 let mut replay=[0usize;0x40/8];replay[1]=actor;
 for owned in [false,true]{for reset in [false,true]{for hero in [1,4,6]{
  OWNED.with(|s|s.set(owned));RESET.with(|s|s.set(reset));data[0xc7]=hero;
  for rate in [0.5f32,0.6]{ptr::write_unaligned((replay.as_mut_ptr()as usize+0x3c)as*mut f32,rate);let factor=if owned&&!reset&&hero==4{1.2}else{1.};assert_eq!(restage_fixture(replay.as_ptr()as usize,101.),(101f32*factor*rate)as i32);}
 }}}
 for owned in [false,true]{for reset in [false,true]{OWNED.with(|s|s.set(owned));RESET.with(|s|s.set(reset));
  for (query,native)in [(ULTIMATE_QUERY,1106),(READY_QUERY,370)]{QUERY.with(|s|s.set(0));let result=crate::ambulance::env(context.as_ptr()as usize,query,17);assert_eq!(result,i32::from(!owned||reset));assert_eq!(QUERY.with(Cell::get),if owned&&!reset{0}else{native});}
  for (hero,attack,expected)in [(1u8,600000000i32,150i32),(2,600000050,150),(3,600000110,150),(4,600000150,100),(5,600000210,250),(7,82310,175),(8,960200,115),(8,600000350,150),(9,600000400,100),(10,600000000,100)]{
   data[0xc7]=hero;let mut hit=[0u8;0x300];hit[0x40..0x44].copy_from_slice(&attack.to_le_bytes());hit[0x44..0x48].copy_from_slice(&1i32.to_le_bytes());hit[0x280..0x284].copy_from_slice(&100i32.to_le_bytes());let before=hit;
   calculated(actor,actor+8,actor,hit.as_mut_ptr()as usize,base);let expected=if owned&&!reset{expected}else{100};assert_eq!(read_i32(hit.as_ptr()as usize+0x280),expected);
   for i in 0..hit.len(){if !(0x280..0x284).contains(&i){assert_eq!(hit[i],before[i]);}}
  }
 }}
 data[0xc7]=6;OWNED.with(|s|s.set(true));RESET.with(|s|s.set(false));
 let mut modules=vec![0usize;0x168/8];let mut wrapper=[0usize;3];let mut family=vec![0u64;0x140/8];let mut summon=[0u64;2];
 ptr::write_unaligned((actor+0x1b8)as*mut usize,modules.as_mut_ptr()as usize);modules[0x160/8]=wrapper.as_mut_ptr()as usize;wrapper[2]=family.as_mut_ptr()as usize;
 let handles=[0x701effffffff,0x701fffffffff,0x7020ffffffff];
 let mut npcs=[vec![0usize;0x200/8],vec![0usize;0x200/8],vec![0usize;0x200/8]];let actors:[usize;3]=std::array::from_fn(|slot|npcs[slot].as_mut_ptr()as usize);
 let mut session=vec![0u32;0x128/4];session[0x120/4]=12;world[0x9910/8]=session.as_mut_ptr()as usize;
 let mut entries=vec![0usize;60*2];let mut collection=[0usize;4];let table=[base+0x4ac780,base+0x4ac710,base+0x4ac770];
 collection[0]=table.as_ptr()as usize;collection[2]=60;collection[3]=entries.as_mut_ptr()as usize;world[(0x170b8+14*8)/8]=collection.as_mut_ptr()as usize;
 for slot in 0..3{family[(0x10+slot*0x60)/8]=handles[slot];npcs[slot][1]=0xffffffff10e0001e+slot;npcs[slot][0x190/8]=actors[slot];entries[(30+slot)*2]=actors[slot];assert_ne!(npcs[slot][1]as u64,handles[slot]);assert_eq!(native_actor::resolve(base,handles[slot]),actors[slot]);assert_eq!(family_slot(actor,actors[slot],base),Some(slot));}
 for source in actors.into_iter().chain([summon.as_ptr()as usize,0]){let mut hit=[0u8;0x300];hit[0x280..0x284].copy_from_slice(&100i32.to_le_bytes());calculated(actor,actor+8,source,hit.as_mut_ptr()as usize,base);assert_eq!(read_i32(hit.as_ptr()as usize+0x280),if actors.contains(&source){300}else{100});}
 for copied in [handles[0],npcs[0][1]as u64]{summon[1]=copied;assert!(!family_source(actor,summon.as_ptr()as usize,base));}
 entries[30*2]=0;assert!(!family_source(actor,actors[0],base));entries[30*2]=actors[0];
 let saved_handle=family[0x10/8];for invalid in [0,u64::MAX]{family[0x10/8]=invalid;assert!(!family_source(actor,actors[0],base));}family[0x10/8]=saved_handle;
 // Preserve every native regeneration field apart from the one-third interval.
 for i in 0..3{let native=row(&params,FAMILY_IDLE[i]);let private=row(&params,FAMILY_RELOAD[i]);let interval=|b:&[u8]|f32::from_le_bytes(b[12..16].try_into().unwrap());assert_eq!(interval(private),interval(native)/3.);}
 let saved_apply=crate::APPLY_ORIG.swap(applied as*mut u8,Ordering::AcqRel);
 for (address,target)in [(crate::APPLY_SELF_RVA,apply as*const()as usize)]{let mut j=vec![0x48,0xb8];j.extend_from_slice(&target.to_le_bytes());j.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(j.as_ptr(),(base+address)as*mut u8,j.len());}
 for hero in [1,6]{data[0xc7]=hero;for owned in [false,true]{for reset in [false,true]{OWNED.with(|s|s.set(owned));RESET.with(|s|s.set(reset));for i in 0..3{for j in 0..3{
  assert_eq!(route_effect(actors[i],FAMILY_IDLE[j],base),if i==j&&hero==6&&owned&&!reset{FAMILY_RELOAD[j]}else{FAMILY_IDLE[j]});
  assert_eq!(route_effect(actors[i],FAMILY_RELOAD[j],base),if i==j&&!(hero==6&&owned&&!reset){FAMILY_IDLE[j]}else{FAMILY_RELOAD[j]});
  assert_eq!(route_effect(actors[i],FAMILY_IDLE[j]-1,base),FAMILY_IDLE[j]-1);
 }}}}}
 data[0xc7]=6;OWNED.with(|s|s.set(true));RESET.with(|s|s.set(false));
 EFFECTS.with(|s|*s.borrow_mut()=(0..3).map(|i|(actors[i],FAMILY_IDLE[i])).collect());APPLIED.with(|s|s.borrow_mut().clear());
 update_family(actor,base);assert_eq!(APPLIED.with(|s|s.borrow().clone()),(0..3).map(|i|(actors[i],FAMILY_RELOAD[i])).collect::<Vec<_>>());
 for _ in 0..100{update_family(actor,base);}assert_eq!(APPLIED.with(|s|s.borrow().len()),3,"native healing tick must never be refreshed every frame");
 // Native summon/withdraw transitions switch category normally. A deployed
 // family member stays on the unmodified active effect.
 apply(actors[0],705060,0);update_family(actor,base);assert!(EFFECTS.with(|s|s.borrow().contains(&(actors[0],705060))));
 OWNED.with(|s|s.set(false));update_family(actor,base);for i in 1..3{assert!(EFFECTS.with(|s|s.borrow().contains(&(actors[i],FAMILY_IDLE[i]))));}
 OWNED.with(|s|s.set(true));apply(actors[0],705061,0);assert!(EFFECTS.with(|s|s.borrow().contains(&(actors[0],FAMILY_RELOAD[0]))));
 RESET.with(|s|s.set(true));update_family(actor,base);assert!(EFFECTS.with(|s|s.borrow().contains(&(actors[0],705061))));RESET.with(|s|s.set(false));
 crate::APPLY_ORIG.store(saved_apply,Ordering::Release);
 // A remote context always retains the original native availability result.
 OWNED.with(|s|s.set(true));RESET.with(|s|s.set(false));let remote=[actor+8];assert_eq!(crate::ambulance::env(remote.as_ptr()as usize,ULTIMATE_QUERY,17),1);
 RESTAGE_ORIGINAL.store(saved_rest,Ordering::Release);crate::ambulance::ENV_ORIGINAL.store(saved_env,Ordering::Release);crate::BASE.store(saved_base,Ordering::Release);crate::ACTIVE.store(saved_active,Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
