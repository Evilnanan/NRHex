//! Opt-in execution of the retained game's code with this compiled detour.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
#[path="death_dance_flask_tests.rs"]mod flask;
unsafe fn patch(base:usize,rva:usize,bytes:&[u8]){ptr::copy_nonoverlapping(bytes.as_ptr(),(base+rva)as*mut u8,bytes.len());}
unsafe fn integer(address:usize,value:i32){ptr::write_unaligned(address as*mut i32,value);}
unsafe fn pointer(address:usize,value:usize){ptr::write_unaligned(address as*mut usize,value);}
static FINISHED:Mutex<Vec<(usize,i32,usize,usize,i32,u8)>>=Mutex::new(Vec::new());
static EXPLOSIONS:std::sync::atomic::AtomicUsize=std::sync::atomic::AtomicUsize::new(0);
unsafe extern "system" fn balloon_owned_fixture(_:usize,effect:i32)->u8{u8::from(matches!(effect,OWNER|FLASK|crate::balloon::OWNER))}
unsafe extern "system" fn explosion_fixture(_:usize,behavior:i32,_:i32,_:usize,_:i32,_:i32,_:u8,_:u8,_:u8,_:usize)->u8{
 assert_eq!(behavior,crate::balloon::BEHAVIOR);assert!(!FINISHED.lock().unwrap().is_empty());
 EXPLOSIONS.fetch_add(1,Ordering::Relaxed);1
}
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
  let payments=SCHEDULE.lock().unwrap().tick(dt*SECONDS as f32);SETTLING.with(|s|s.set(true));
  for(amount,t)in payments{setter(stats,read_i32(stats+0x140)-amount,0,t.rate,t.window,t.refresh);}
  SETTLING.with(|s|s.set(false));
 };
 // The same compiled gate must preserve the unmodified non-owner baseline.
 reset();ACTIVE.store(false,Ordering::Release);
 setter(stats,800,1,1.,1.,1);settle(stats,1.);setter(stats,700,1,1.,1.,1);
 assert_eq!(float(stats+0x168),300.);assert_eq!(float(stats+0x16c),100.);
 settle(stats,0.2);assert_eq!(float(stats+0x168),100.);ACTIVE.store(true,Ordering::Release);
 reset();setter(stats,800,1,1.,1.,1);pay(0.5);
 assert_eq!(read_i32(stats+0x140),900);assert_eq!(float(stats+0x16c),100.);
 queue(stats,25.);assert_eq!(float(stats+0x174),25.);
 setter(stats,800,1,1.,1.,1);
 assert_eq!(read_i32(stats+0x140),900);assert_eq!(float(stats+0x168),100.);
 assert_eq!(float(stats+0x16c),0.);assert_eq!(float(stats+0x170),0.);assert_eq!(float(stats+0x174),0.);
 settle(stats,0.2);assert_eq!(float(stats+0x168),0.);
 pay(0.5);assert_eq!(read_i32(stats+0x140),750);assert_eq!(float(stats+0x168),150.);assert_eq!(float(stats+0x16c),150.);
 assert_eq!(SCHEDULE.lock().unwrap().hits.len(),1);
 pay(0.5);assert_eq!(read_i32(stats+0x140),700);assert_eq!(float(stats+0x168),200.);assert_eq!(float(stats+0x16c),200.);
 settle(stats,3.);assert_eq!(float(stats+0x168),200.);queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),900);assert_eq!(float(stats+0x168),0.);
 queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),900);
 // Healing still clamps both quotas to missing HP, even after a reset hit.
 reset();setter(stats,800,1,1.,1.,1);pay(1.0);setter(stats,700,1,1.,1.,1);pay(1.0);
 SETTLING.with(|s|s.set(true));setter(stats,950,0,0.,1.,1);SETTLING.with(|s|s.set(false));
 assert_eq!(float(stats+0x168),50.);assert_eq!(float(stats+0x16c),50.);
 queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),1000);
 // Shared protection is naturally refreshed by actual payments and expires.
 reset();setter(stats,800,1,1.,1.,1);pay(1.0);settle(stats,3.);
 assert_eq!(float(stats+0x170),1.);setter(stats,700,1,1.,1.,1);
 assert_eq!(float(stats+0x170),0.);pay(1.0);assert_eq!(float(stats+0x170),4.);
 settle(stats,3.99);assert_eq!(float(stats+0x168),100.);settle(stats,0.02);
 assert_eq!(float(stats+0x16c),0.);assert!((float(stats+0x168)-80.).abs()<0.01);
 settle(stats,0.3);assert_eq!(float(stats+0x168),0.);queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),700);
 // Already expired portions must never be re-protected by a later hit.
 reset();setter(stats,800,1,1.,1.,1);pay(1.0);settle(stats,3.99);settle(stats,0.02);
 assert_eq!(float(stats+0x16c),0.);setter(stats,700,1,1.,1.,1);pay(1.0);
 assert_eq!(float(stats+0x16c),100.);assert!((float(stats+0x168)-280.).abs()<0.01);
 settle(stats,0.2);assert_eq!(float(stats+0x168),100.);
 reset();setter(stats,800,1,1.,1.,1);pay(1.0);settle(stats,3.);
 setter(stats,700,1,0.,1.,1);assert_eq!(read_i32(stats+0x140),700);
 assert_eq!(float(stats+0x168),200.);assert_eq!(float(stats+0x16c),0.);assert_eq!(float(stats+0x170),0.);
 settle(stats,0.2);assert_eq!(float(stats+0x168),0.);assert!(SCHEDULE.lock().unwrap().hits.is_empty());
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
 pay(0.25);assert_eq!(read_i32(stats+0x140),950);assert_eq!(float(stats+0x168),50.);
 queue(stats,30.);queue(stats,30.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),1000);assert_eq!(float(stats+0x168),0.);
 pay(0.75);assert_eq!(read_i32(stats+0x140),850);assert_eq!(float(stats+0x168),150.);
 queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),1000);assert_eq!(float(stats+0x168),0.);assert!(SCHEDULE.lock().unwrap().hits.is_empty());
 // Overlapping hits retain separate starts rather than renewing one timer.
 reset();setter(stats,800,0,1.,1.,1);pay(0.25);setter(stats,550,1,1.,1.,1);pay(0.75);
 assert_eq!(read_i32(stats+0x140),500);assert_eq!(SCHEDULE.lock().unwrap().hits.len(),1);pay(0.25);assert_eq!(read_i32(stats+0x140),400);
 // Zero-recovery sources such as rain/ailment debits remain immediate.
 reset();setter(stats,800,0,0.,1.,1);assert_eq!(read_i32(stats+0x140),800);assert!(SCHEDULE.lock().unwrap().hits.is_empty());assert_eq!(float(stats+0x168),0.);
 // Native rejection and non-owners cannot be intercepted.
 for(address,bytes)in [(crate::HAS_STATE_RVA,&[0x31,0xc0,0xc3][..]),(crate::HAS_EFFECT_RVA,&[0x31,0xc0,0xc3])]{reset();patch(base,address,bytes);setter(stats,800,0,1.,1.,1);assert_eq!(read_i32(stats+0x140),800);assert!(SCHEDULE.lock().unwrap().hits.is_empty());}
 // Existing historical loss is not promoted to recoverable HP by the delay.
 reset();integer(stats+0x140,700);setter(stats,500,0,1.,1.,1);pay(1.0);queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),700);
 // Native healing can continue while debt remains; it consumes no new budget.
 reset();setter(stats,600,0,1.,1.,1);pay(0.5);SETTLING.with(|s|s.set(true));setter(stats,950,0,0.,1.,1);SETTLING.with(|s|s.set(false));pay(0.5);assert_eq!(read_i32(stats+0x140),750);queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),1000);
 // Existing per-source recovery ratios are retained, rather than inflated.
 reset();setter(stats,800,0,0.5,1.,1);pay(1.0);assert_eq!(float(stats+0x168),100.);queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),900);
 // Lethal payment reaches zero; native settlement cannot revive it.
 reset();setter(stats,-5000,0,1.,1.,1);assert_eq!(read_i32(stats+0x140),1000);pay(1.0);assert_eq!(read_i32(stats+0x140),0);queue(stats,999.);settle(stats,0.016);assert_eq!(read_i32(stats+0x140),0);
 // Runtime uses the actual native delta, immunity virtual and floor wrapper,
 // not an always-success replacement for any of these protection gates.
 let mut damage=vec![0u64;0x90/8];let damage=damage.as_mut_ptr()as usize;
 pointer(modules+crate::DAMAGE_MODULE_SLOT,damage);pointer(damage,base+crate::ominous_pact::PLAYER_DAMAGE_VTABLE_RVA);pointer(damage+8,player);
 let mut behavior=vec![0u64;0x60/8];let behavior=behavior.as_mut_ptr()as usize;pointer(modules+8,behavior);
 let mut handle=[0u8;0x10];handle[8]=4;pointer(player+0x18,handle.as_ptr()as usize);
 let mut vtable=vec![0usize;0x218/8];vtable[0x210/8]=base+0x66d410;pointer(player,vtable.as_ptr()as usize);
 // Retain real module->actor and module->effects getters in these fixtures.
 patch(base,0x42c120,&[0x48,0x8b,0x41,8,0xc3]);
 patch(base,0x42c130,&[0x48,0x8b,0x41,8,0x48,0x8b,0x80,0x90,1,0,0,0xc3]);
 // The three native debug-manager presence checks must be initialized. Their
 // global flags remain zero, as in an ordinary non-debug game.
 pointer(base+0x3c0f338,1);
 let mut has=vec![0x81,0xea];has.extend_from_slice(&OWNER.to_le_bytes());has.extend_from_slice(&[0x83,0xfa,1,0x0f,0x96,0xc0,0xc3]);
 patch(base,crate::APPLY_SELF_RVA,&[0xb0,1,0xc3]);
 let mut complete=vec![0x48,0xb8];complete.extend_from_slice(&(finish_fixture as*const()as usize).to_le_bytes());complete.extend_from_slice(&[0xff,0xe0]);patch(base,FINISH_RVA,&complete);
 let runtime_reset=||{
  reset();pointer(stats+8,player);pointer(modules+8,behavior);ptr::write_bytes(behavior as*mut u8,0,0x60);
  ptr::write_bytes((player+0x1f0)as*mut u8,0,0x10);integer(player+0x6e4,0);
  patch(base,crate::HAS_EFFECT_RVA,&has);
  // Native regain state 449 is present, native HP-floor states absent.
  patch(base,crate::HAS_STATE_RVA,&[0x66,0x81,0xfa,0xc1,1,0x0f,0x94,0xc0,0xc3]);
  FINISHED.lock().unwrap().clear();
 };
 runtime_reset();setter(stats,800,0,1.,1.,1);
 update(player,base,1.);assert_eq!(read_i32(stats+0x140),900);assert_eq!(SCHEDULE.lock().unwrap().hits.len(),1);
 update(player,base,1.);assert_eq!(read_i32(stats+0x140),800);assert!(SCHEDULE.lock().unwrap().hits.is_empty());
 let advance_fraction=|player,base,fraction:f32|update(player,base,fraction*SECONDS as f32);
 runtime_reset();setter(stats,800,0,1.,1.,1);advance_fraction(player,base,0.25);
 assert_eq!(read_i32(stats+0x140),950);assert_eq!(float(stats+0x168),50.);assert_eq!(*FINISHED.lock().unwrap(),vec![(damage,50,0,0,1000,0)]);
 advance_fraction(player,base,0.75);assert_eq!(read_i32(stats+0x140),800);assert!(SCHEDULE.lock().unwrap().hits.is_empty());
 runtime_reset();integer(stats+0x140,10);setter(stats,-90,1,1.,1.,1);
 assert_eq!(read_i32(stats+0x140),10);
 assert_eq!(SCHEDULE.lock().unwrap().hits[0].amount,100);
 advance_fraction(player,base,0.05);assert_eq!(read_i32(stats+0x140),5);
 setter(stats,7,0,0.,1.,1);
 advance_fraction(player,base,0.05);assert_eq!(read_i32(stats+0x140),2);
 advance_fraction(player,base,0.02);assert_eq!(read_i32(stats+0x140),0);
 assert_eq!(float(stats+0x168),12.);
 // Preserve smooth, per-frame HP movement; no native poison/rot timer is used.
 runtime_reset();setter(stats,760,0,1.,1.,1);
 for frame in 1..=120{advance_fraction(player,base,1./120.);assert_eq!(read_i32(stats+0x140),1000-2*frame);}
 assert!(SCHEDULE.lock().unwrap().hits.is_empty());assert_eq!(FINISHED.lock().unwrap().len(),120);
 let native_immunity:unsafe extern "system" fn(usize,u8)->u8=std::mem::transmute(base+0x66d410);
 let native_debit:unsafe extern "system" fn(usize,i32,u8,u8,f32,f32,u8)=std::mem::transmute(base+DEBIT_RVA);
 runtime_reset();integer(behavior+0x58,2);
 assert_eq!(native_immunity(player,0),1);assert_eq!(native_immunity(player,1),0);
 SETTLING.with(|scope|scope.set(true));native_debit(stats,-200,1,0,1.,1.,1);SETTLING.with(|scope|scope.set(false));
 assert_eq!(read_i32(stats+0x140),800);assert_eq!(float(stats+0x168),200.);
 runtime_reset();setter(stats,0,0,1.,1.,1);integer(behavior+0x58,2);
 for frame in 1..=120{
  advance_fraction(player,base,1./120.);
  let expected=if frame==120{0}else{1000-(1000f64*f64::from(frame as f32*(1./120.))).floor()as i32};
  assert!((read_i32(stats+0x140)-expected).abs()<=1,"frame {frame}");
 }
 assert_eq!(read_i32(stats+0x140),0);assert!(SCHEDULE.lock().unwrap().hits.is_empty());
 for mechanism in 0..6{
  runtime_reset();setter(stats,800,0,1.,1.,1);
  match mechanism{
   0=>integer(behavior+0x58,1),1=>integer(behavior+0x58,2),
   2=>ptr::write((player+0x1f3)as*mut u8,0x10),
   3=>pointer(behavior+0x58,1usize<<59),
   4=>ptr::write((player+0x1fb)as*mut u8,4),
   _=>ptr::write_unaligned((player+0x6e4)as*mut f32,1.),
  }
  let immune=matches!(mechanism,0|3|5);
  assert_eq!(native_immunity(player,0),1);assert_eq!(native_immunity(player,1),u8::from(matches!(mechanism,0|5)));
  advance_fraction(player,base,0.5);
  assert_eq!(read_i32(stats+0x140),if immune{1000}else{900});
  assert_eq!(float(stats+0x168),if immune{0.}else{100.});
  assert_eq!(FINISHED.lock().unwrap().len(),usize::from(!immune));
  pointer(behavior+0x58,0);ptr::write((player+0x1f3)as*mut u8,0);ptr::write((player+0x1fb)as*mut u8,0);integer(player+0x6e4,0);
  advance_fraction(player,base,0.5);assert_eq!(read_i32(stats+0x140),if immune{900}else{800});
  assert_eq!(float(stats+0x168),if immune{100.}else{200.});
  advance_fraction(player,base,0.5);assert_eq!(read_i32(stats+0x140),if immune{900}else{800});
 }
 runtime_reset();setter(stats,800,0,1.,1.,1);integer(behavior+0x58,3);
 assert_eq!(native_immunity(player,1),1);
 advance_fraction(player,base,0.5);assert_eq!(read_i32(stats+0x140),1000);assert_eq!(float(stats+0x168),0.);
 integer(behavior+0x58,2);advance_fraction(player,base,0.5);
 assert_eq!(read_i32(stats+0x140),900);assert_eq!(float(stats+0x168),100.);
 advance_fraction(player,base,0.5);assert_eq!(read_i32(stats+0x140),900);assert!(SCHEDULE.lock().unwrap().hits.is_empty());
 runtime_reset();setter(stats,-1000,0,1.,1.,1);pointer(behavior+0x58,(1usize<<59)|2);
 for _ in 0..120{advance_fraction(player,base,1./120.);assert_eq!(read_i32(stats+0x140),1000);}
 assert!(SCHEDULE.lock().unwrap().hits.is_empty());assert_eq!(float(stats+0x168),0.);assert!(FINISHED.lock().unwrap().is_empty());
 pointer(behavior+0x58,2);advance_fraction(player,base,1.);assert_eq!(read_i32(stats+0x140),1000);
 runtime_reset();setter(stats,0,0,1.,1.,1);pointer(behavior+0x58,(1usize<<59)|2);
 advance_fraction(player,base,0.25);assert_eq!(read_i32(stats+0x140),1000);assert_eq!(float(stats+0x168),0.);
 pointer(behavior+0x58,2);advance_fraction(player,base,0.75);
 assert_eq!(read_i32(stats+0x140),250);assert_eq!(float(stats+0x168),750.);assert!(SCHEDULE.lock().unwrap().hits.is_empty());
 // Lock HP using the real floor wrapper (behavior bit17, stats flags and
 // native state143/235); only the actual debit can become recoverable.
 for mechanism in 0..5{
  runtime_reset();setter(stats,0,0,1.,1.,1);
  match mechanism{
   0=>integer(behavior+0x58,1<<17),
   1=>ptr::write((stats+0x189)as*mut u8,4),
   2=>ptr::write((stats+0x189)as*mut u8,2),
   _=>{
    let state=if mechanism==3{143u16}else{235};
    let mut code=vec![0x66,0x81,0xfa];code.extend_from_slice(&state.to_le_bytes());code.extend_from_slice(&[0x0f,0x94,0xc0,0x66,0x81,0xfa,0xc1,1,0x0f,0x94,0xc2,0x08,0xd0,0xc3]);patch(base,crate::HAS_STATE_RVA,&code);
   }
  }
  advance_fraction(player,base,1.);assert_eq!(read_i32(stats+0x140),1);assert_eq!(float(stats+0x168),999.);assert_eq!(FINISHED.lock().unwrap()[0].1,999);
  pointer(behavior+0x58,0);ptr::write((stats+0x189)as*mut u8,0);advance_fraction(player,base,1.);assert_eq!(read_i32(stats+0x140),1);
 }
 // Ordinary collision-only conditions do not enter the damage-immunity gate:
 // +1F8 bit3 is read by 3D3DB0, but not by the periodic +210 virtual.
 runtime_reset();setter(stats,800,0,1.,1.,1);ptr::write((player+0x1f8)as*mut u8,8);
 advance_fraction(player,base,1.);assert_eq!(read_i32(stats+0x140),800);assert_eq!(float(stats+0x168),200.);
 // With no protection, a lethal debt still completes native death bookkeeping.
 runtime_reset();setter(stats,0,0,1.,1.,1);advance_fraction(player,base,1.);assert_eq!(read_i32(stats+0x140),0);assert_eq!(FINISHED.lock().unwrap()[0].1,1000);
 let balloon_state=crate::BALLOON.get_or_init(||Mutex::new(crate::balloon::Balloon::default()));
 let previous_balloon=std::mem::take(&mut *balloon_state.lock().unwrap());
 let mut balloon_has=vec![0x48,0xb8];balloon_has.extend_from_slice(&(balloon_owned_fixture as*const()as usize).to_le_bytes());balloon_has.extend_from_slice(&[0xff,0xe0]);
 let mut explode=vec![0x48,0xb8];explode.extend_from_slice(&(explosion_fixture as*const()as usize).to_le_bytes());explode.extend_from_slice(&[0xff,0xe0]);
 patch(base,crate::ARTS_BULLET_RVA,&explode);
 for protection in 0..3{
  runtime_reset();patch(base,crate::HAS_EFFECT_RVA,&balloon_has);
  {let mut balloon=balloon_state.lock().unwrap();*balloon=crate::balloon::Balloon::default();balloon.synchronize(player,stats,true);balloon.remember_max(1000);}
  EXPLOSIONS.store(0,Ordering::Relaxed);
  setter(stats,-1000,1,1.,1.,1);
  if protection==1{integer(behavior+0x58,1<<17);}else if protection==2{integer(behavior+0x58,1);}
  advance_fraction(player,base,0.25);
  assert_eq!(EXPLOSIONS.load(Ordering::Relaxed),0);
  advance_fraction(player,base,0.75);
  assert_eq!(EXPLOSIONS.load(Ordering::Relaxed),0);
  assert_eq!(read_i32(stats+0x140),match protection{0=>0,1=>1,_=>1000});
  if protection==0{
   assert_eq!(balloon_state.lock().unwrap().base_hp_for(player,stats),None);
   ptr::write((player+0x710)as*mut u8,1);
   crate::balloon_runtime::transition(player,base);
   crate::balloon_runtime::transition(player,base);
   assert_eq!(EXPLOSIONS.load(Ordering::Relaxed),1);
   ptr::write((player+0x710)as*mut u8,0);
  }
 }
 *balloon_state.lock().unwrap()=previous_balloon;
 flask::run(base,&raw);
 ACTIVE.store(previous_active,Ordering::Release);BASE.store(previous_base,Ordering::Release);*SCHEDULE.lock().unwrap()=Schedule::default();
 VirtualFree(allocation,0,MEM_RELEASE);
}}
