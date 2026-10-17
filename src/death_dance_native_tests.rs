//! Opt-in execution of the retained game's code with this compiled detour.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
unsafe fn patch(base:usize,rva:usize,bytes:&[u8]){ptr::copy_nonoverlapping(bytes.as_ptr(),(base+rva)as*mut u8,bytes.len());}
unsafe fn integer(address:usize,value:i32){ptr::write_unaligned(address as*mut i32,value);}
unsafe fn pointer(address:usize,value:usize){ptr::write_unaligned(address as*mut usize,value);}
static FINISHED:Mutex<Vec<(usize,i32,usize,usize,i32,u8)>>=Mutex::new(Vec::new());
unsafe extern "system" fn finish_fixture(module:usize,loss:i32,a:usize,b:usize,before:i32,flag:u8){FINISHED.lock().unwrap().push((module,loss,a,b,before,flag));}
unsafe fn float(address:usize)->f32{ptr::read_unaligned(address as*const f32)}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE pointing to the supported original game EXE; run alone"]
fn compiled_detour_and_original_native_pool_execute_together(){unsafe{
 let path=std::env::var_os("NRHEX_NATIVE_EXE").expect("NRHEX_NATIVE_EXE");
 let raw=std::fs::read(path).unwrap();
 assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();
 let allocation=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);
 assert!(!allocation.is_null());let base=allocation as usize;
 for section in &headers.sections{
  let start=section.raw_offset as usize;let count=section.raw_size as usize;
  ptr::copy_nonoverlapping(raw[start..start+count].as_ptr(),(base+section.virtual_address as usize)as*mut u8,count);
 }
 // Stubs stand in for network authority, actor/effect lookup and ownership.
 // The HP setter, real branch, compiled mid-hook, queue and settlement execute.
 for(rva,bytes)in [(0x6ada30,&[0x31,0xc0,0xc3][..]),(0x42c120,&[0x48,0x8b,0xc1,0xc3]),(0x42c130,&[0x48,0x8b,0xc1,0xc3]),(0x3d4180,&[0xb0,1,0xc3]),(crate::HAS_STATE_RVA,&[0xb0,1,0xc3]),(crate::HAS_EFFECT_RVA,&[0xb0,1,0xc3])]{patch(base,rva,bytes);}
 let mut player=vec![0u64;0x720/8];let player=player.as_mut_ptr()as usize;
 let mut modules=vec![0u64;0x200/8];let modules=modules.as_mut_ptr()as usize;
 let mut stats=vec![0u64;0x200/8];let stats=stats.as_mut_ptr()as usize;
 let mut world=vec![0u64;0x17500/8];let world=world.as_mut_ptr()as usize;
 pointer(player+0x1b8,modules);pointer(player+0x190,player);pointer(modules,stats);
 pointer(world+0x174e8,player);pointer(base+crate::WORLD_RVA,world);
 let prepared=crate::hook::prepare((base+RVA)as*mut u8,detour as*const u8,64).unwrap();
 assert_eq!(prepared.stolen,6);
 ORIGINAL.store(prepared.trampoline,Ordering::Release);crate::hook::write_code((base+RVA)as*mut u8,&prepared.patch).unwrap();
 let protection=crate::hook::prepare((base+PROTECT_RVA)as*mut u8,protect_detour as*const u8,64).unwrap();
 assert_eq!(protection.stolen,11);
 PROTECT_ORIGINAL.store(protection.trampoline,Ordering::Release);
 PROTECT_RESUME.store((base+PROTECT_RESUME_RVA)as*mut u8,Ordering::Release);
 crate::hook::write_code((base+PROTECT_RVA)as*mut u8,&protection.patch).unwrap();
 let previous_base=BASE.swap(base,Ordering::AcqRel);let previous_active=ACTIVE.swap(true,Ordering::AcqRel);
 let setter:unsafe extern "system" fn(usize,i32,u8,f32,f32,u8)=std::mem::transmute(base+SET_RVA);
 let queue:unsafe extern "system" fn(usize,f32)=std::mem::transmute(base+0x4275a0);
 let settle:unsafe extern "system" fn(usize,f32)=std::mem::transmute(base+0x4289c0);
 let reset=||{
  ptr::write_bytes(stats as*mut u8,0,0x200);integer(stats+0x140,1000);integer(stats+0x144,1000);
  pointer(modules,stats);ptr::write((player+0x710)as*mut u8,0);
  *SCHEDULE.lock().unwrap()=Schedule::default();patch(base,crate::HAS_STATE_RVA,&[0xb0,1,0xc3]);patch(base,crate::HAS_EFFECT_RVA,&[0xb0,1,0xc3]);
 };
 let pay=|dt:f32|{
  let payments=SCHEDULE.lock().unwrap().tick(dt);SETTLING.with(|s|s.set(true));
  for(amount,t)in payments{setter(stats,read_i32(stats+0x140)-amount,0,t.rate,t.window,t.refresh);}
  SETTLING.with(|s|s.set(false));
 };
 // The same compiled gate must preserve the unmodified non-owner baseline.
 reset();ACTIVE.store(false,Ordering::Release);
 setter(stats,800,1,1.,1.,1);settle(stats,1.);setter(stats,700,1,1.,1.,1);
 assert_eq!(float(stats+0x168),300.);assert_eq!(float(stats+0x16c),100.);
 settle(stats,0.2);assert_eq!(float(stats+0x168),100.);ACTIVE.store(true,Ordering::Release);
 // Regression: real new-hit resets (not reset=0 artificial debits) retain old
 // protected quota/time. Native cancellation of pending recovery still runs.
 reset();setter(stats,800,1,1.,1.,1);pay(1.);
 assert_eq!(read_i32(stats+0x140),900);assert_eq!(float(stats+0x16c),100.);
 queue(stats,25.);assert_eq!(float(stats+0x174),25.);
 setter(stats,800,1,1.,1.,1);
 assert_eq!(read_i32(stats+0x140),900);assert_eq!(float(stats+0x168),100.);
 assert_eq!(float(stats+0x16c),100.);assert_eq!(float(stats+0x174),0.);
 settle(stats,0.2);assert_eq!(float(stats+0x168),100.);
 pay(1.);assert_eq!(read_i32(stats+0x140),750);assert_eq!(float(stats+0x168),250.);assert_eq!(float(stats+0x16c),250.);
 assert_eq!(SCHEDULE.lock().unwrap().hits.len(),1);
 pay(1.);assert_eq!(read_i32(stats+0x140),700);assert_eq!(float(stats+0x168),300.);assert_eq!(float(stats+0x16c),300.);
 settle(stats,3.);assert_eq!(float(stats+0x168),300.);queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),1000);assert_eq!(float(stats+0x168),0.);
 queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),1000);
 // Healing still clamps both quotas to missing HP, even after a reset hit.
 reset();setter(stats,800,1,1.,1.,1);pay(2.);setter(stats,700,1,1.,1.,1);pay(2.);
 SETTLING.with(|s|s.set(true));setter(stats,950,0,0.,1.,1);SETTLING.with(|s|s.set(false));
 assert_eq!(float(stats+0x168),50.);assert_eq!(float(stats+0x16c),50.);
 queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),1000);
 // Shared protection is naturally refreshed by actual payments and expires.
 reset();setter(stats,800,1,1.,1.,1);pay(2.);settle(stats,3.);
 assert_eq!(float(stats+0x170),1.);setter(stats,700,1,1.,1.,1);
 assert_eq!(float(stats+0x170),1.);pay(2.);assert_eq!(float(stats+0x170),4.);
 settle(stats,3.99);assert_eq!(float(stats+0x168),300.);settle(stats,0.02);
 assert_eq!(float(stats+0x16c),0.);assert!((float(stats+0x168)-280.).abs()<0.01);
 settle(stats,0.3);assert_eq!(float(stats+0x168),0.);queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),700);
 // Already expired portions must never be re-protected by a later hit.
 reset();setter(stats,800,1,1.,1.,1);pay(2.);settle(stats,3.99);settle(stats,0.02);
 assert_eq!(float(stats+0x16c),0.);setter(stats,700,1,1.,1.,1);pay(2.);
 assert_eq!(float(stats+0x16c),100.);assert!((float(stats+0x168)-280.).abs()<0.01);
 settle(stats,0.2);assert_eq!(float(stats+0x168),100.);
 // Nonrecoverable damage neither enters the slot nor renews its timer. Only
 // existing protection survives; immediate damage and native expiry stay intact.
 reset();setter(stats,800,1,1.,1.,1);pay(2.);settle(stats,3.);
 setter(stats,700,1,0.,1.,1);assert_eq!(read_i32(stats+0x140),700);
 assert_eq!(float(stats+0x168),200.);assert_eq!(float(stats+0x16c),200.);assert_eq!(float(stats+0x170),1.);
 settle(stats,0.2);assert_eq!(float(stats+0x168),200.);assert!(SCHEDULE.lock().unwrap().hits.is_empty());
 // Non-owners, rejected native states, non-local stats, and near-death all
 // retain the original reset, while keeping the same machine-code trampoline.
 for kind in 0..4{
  reset();SETTLING.with(|s|s.set(true));setter(stats,800,1,1.,1.,1);SETTLING.with(|s|s.set(false));
  match kind{0=>patch(base,crate::HAS_EFFECT_RVA,&[0x31,0xc0,0xc3]),1=>patch(base,crate::HAS_STATE_RVA,&[0x31,0xc0,0xc3]),2=>pointer(modules,player),_=>ptr::write((player+0x710)as*mut u8,1)}
  setter(stats,700,1,1.,1.,1);assert_eq!(read_i32(stats+0x140),700);assert!(SCHEDULE.lock().unwrap().hits.is_empty());
  assert_eq!(float(stats+0x16c),if kind==1{0.}else{100.});
 }
 // No immediate debit or free recovery; paid parts enter the native pool once.
 reset();setter(stats,800,0,1.,1.,1);assert_eq!(read_i32(stats+0x140),1000);assert_eq!(float(stats+0x168),0.);
 queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),1000);
 pay(0.5);assert_eq!(read_i32(stats+0x140),950);assert_eq!(float(stats+0x168),50.);
 queue(stats,30.);queue(stats,30.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),1000);assert_eq!(float(stats+0x168),0.);
 pay(1.5);assert_eq!(read_i32(stats+0x140),850);assert_eq!(float(stats+0x168),150.);
 queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),1000);assert_eq!(float(stats+0x168),0.);assert!(SCHEDULE.lock().unwrap().hits.is_empty());
 // Overlapping hits retain separate starts rather than renewing one timer.
 reset();setter(stats,800,0,1.,1.,1);pay(0.5);setter(stats,550,1,1.,1.,1);pay(1.5);
 assert_eq!(read_i32(stats+0x140),500);assert_eq!(SCHEDULE.lock().unwrap().hits.len(),1);pay(0.5);assert_eq!(read_i32(stats+0x140),400);
 // Zero-recovery sources such as rain/ailment debits remain immediate.
 reset();setter(stats,800,0,0.,1.,1);assert_eq!(read_i32(stats+0x140),800);assert!(SCHEDULE.lock().unwrap().hits.is_empty());assert_eq!(float(stats+0x168),0.);
 // Native rejection and non-owners cannot be intercepted.
 for(address,bytes)in [(crate::HAS_STATE_RVA,&[0x31,0xc0,0xc3][..]),(crate::HAS_EFFECT_RVA,&[0x31,0xc0,0xc3])]{reset();patch(base,address,bytes);setter(stats,800,0,1.,1.,1);assert_eq!(read_i32(stats+0x140),800);assert!(SCHEDULE.lock().unwrap().hits.is_empty());}
 // Existing historical loss is not promoted to recoverable HP by the delay.
 reset();integer(stats+0x140,700);setter(stats,500,0,1.,1.,1);pay(2.);queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),700);
 // Native healing can continue while debt remains; it consumes no new budget.
 reset();setter(stats,600,0,1.,1.,1);pay(1.);SETTLING.with(|s|s.set(true));setter(stats,950,0,0.,1.,1);SETTLING.with(|s|s.set(false));pay(1.);assert_eq!(read_i32(stats+0x140),750);queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),1000);
 // Execute the actual runtime update and its native completion ABI as well.
 reset();let mut damage=vec![0u64;0x90/8];let damage=damage.as_mut_ptr()as usize;
 pointer(modules+crate::DAMAGE_MODULE_SLOT,damage);pointer(damage,base+crate::ominous_pact::PLAYER_DAMAGE_VTABLE_RVA);pointer(damage+8,player);
 let mut has=vec![0x81,0xea];has.extend_from_slice(&OWNER.to_le_bytes());has.extend_from_slice(&[0x83,0xfa,1,0x0f,0x96,0xc0,0xc3]);patch(base,crate::HAS_EFFECT_RVA,&has);
 patch(base,crate::APPLY_SELF_RVA,&[0xb0,1,0xc3]);
 let mut complete=vec![0x48,0xb8];complete.extend_from_slice(&(finish_fixture as*const()as usize).to_le_bytes());complete.extend_from_slice(&[0xff,0xe0]);patch(base,FINISH_RVA,&complete);
 setter(stats,800,0,1.,1.,1);update(player,base,0.5);assert_eq!(read_i32(stats+0x140),950);assert_eq!(float(stats+0x168),50.);assert_eq!(*FINISHED.lock().unwrap(),vec![(damage,50,0,0,1000,0)]);
 // Existing per-source recovery ratios are retained, rather than inflated.
 reset();setter(stats,800,0,0.5,1.,1);pay(2.);assert_eq!(float(stats+0x168),100.);queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),900);
 // Lethal payment reaches zero; native settlement cannot revive it.
 reset();setter(stats,-5000,0,1.,1.,1);assert_eq!(read_i32(stats+0x140),1000);pay(2.);assert_eq!(read_i32(stats+0x140),0);queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),0);
 ACTIVE.store(previous_active,Ordering::Release);BASE.store(previous_base,Ordering::Release);*SCHEDULE.lock().unwrap()=Schedule::default();
 VirtualFree(allocation,0,MEM_RELEASE);
}}
