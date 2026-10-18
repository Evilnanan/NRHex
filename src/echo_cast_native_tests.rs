//! Real BulletMan root dispatch, native target copy/atomics/release and compiled
//! hooks; engine projectile allocation and transport consumers are fixtures.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
use std::sync::atomic::{AtomicBool,AtomicUsize};
static HAS:AtomicBool=AtomicBool::new(true);
static TARGET_ALIVE:AtomicBool=AtomicBool::new(true);
static TARGET:AtomicUsize=AtomicUsize::new(0);
static TARGET_DESC:AtomicUsize=AtomicUsize::new(0);
static TARGET_COUNT:AtomicUsize=AtomicUsize::new(1);
static HANDLE:AtomicUsize=AtomicUsize::new(0);
static FAIL:AtomicBool=AtomicBool::new(false);
static NESTED:AtomicBool=AtomicBool::new(false);
static CREATE_CALLS:AtomicUsize=AtomicUsize::new(0);
static FAIL_ON_CALL:AtomicUsize=AtomicUsize::new(0);
static DENSITY_SAMPLES:AtomicUsize=AtomicUsize::new(0);
static NESTED_EMIT:AtomicBool=AtomicBool::new(false);
static SHOTS:Mutex<Vec<[u8;0x110]>>=Mutex::new(Vec::new());
struct FactoryShot{bullet:i32,pose:[u8;0x40],targets:Vec<[u8;0x50]>}
static FACTORY_SHOTS:Mutex<Vec<FactoryShot>>=Mutex::new(Vec::new());
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn write(base:usize,rva:usize,v:&[u8]){ptr::copy_nonoverlapping(v.as_ptr(),(base+rva)as*mut u8,v.len());}
unsafe fn jump(base:usize,rva:usize,f:usize){let mut b=vec![0x48,0xb8];b.extend_from_slice(&f.to_le_bytes());b.extend_from_slice(&[0xff,0xe0]);write(base,rva,&b);}
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from(id==OWNER&&HAS.load(Ordering::Relaxed))}
unsafe extern "system" fn kind(_:usize)->i32{1}
unsafe extern "system" fn yes(_:usize)->u8{1}
unsafe extern "system" fn resolve(id:u64)->usize{if id==22&&TARGET_ALIVE.load(Ordering::Relaxed){TARGET.load(Ordering::Relaxed)}else{0}}
unsafe extern "system" fn parse(out:usize,input:usize,_ctx:usize)->usize{
 SHOTS.lock().unwrap().push(ptr::read_unaligned(input as*const[u8;0x110]));
 ptr::write_bytes(out as*mut u8,0,0x220);put(out+0x10,read_ptr(input));int(out+4,read_i32(input+0x14));
 ptr::copy_nonoverlapping((input+0x50)as*const u8,(out+0x190)as*mut u8,0x40);
 // Deliberately simulate native recalculation using a changed current pose.
 // The compiled factory hook must restore the original resolved matrix.
 if REPLAY.with(Cell::get).ticket!=0{int(out+0x1c0,999f32.to_bits()as i32);}
 // A deterministic stand-in for independent native random generation. The
 // density path must call calculation twice and leave both samples untouched.
 if DENSITY.with(Cell::get){let sample=DENSITY_SAMPLES.fetch_add(1,Ordering::Relaxed);int(out+0x1c0,(sample as f32).to_bits()as i32);}
 let target=TARGET_DESC.load(Ordering::Relaxed);let count=TARGET_COUNT.load(Ordering::Relaxed);
 if count!=0{
  let mut rows=vec![[0u64;10];count].into_boxed_slice();let copy:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(BASE.load(Ordering::Relaxed)+TARGET_COPY_RVA);
  for row in &mut rows{copy(row.as_mut_ptr()as usize,target);}
  let start=Box::into_raw(rows)as*mut[u64;10]as usize;put(out+0x148,start);put(out+0x150,start+count*0x50);
 }out
}
unsafe extern "system" fn release_payload(payload:usize){
 let start=read_ptr(payload+0x148);let count=read_ptr(payload+0x150).wrapping_sub(start)/0x50;
 if start!=0{let mut rows=Box::from_raw(ptr::slice_from_raw_parts_mut(start as*mut[u64;10],count));
  let release:unsafe extern "system" fn(usize)=std::mem::transmute(BASE.load(Ordering::Relaxed)+0x3482b0);
  for row in &mut rows{release(row.as_mut_ptr()as usize);}
 }
}
unsafe extern "system" fn create(manager:usize,payload:usize,error:usize)->usize{
 let call=CREATE_CALLS.fetch_add(1,Ordering::Relaxed)+1;
 let start=read_ptr(payload+0x148);let count=read_ptr(payload+0x150).wrapping_sub(start)/0x50;
 FACTORY_SHOTS.lock().unwrap().push(FactoryShot{bullet:read_i32(payload+4),pose:ptr::read_unaligned((payload+0x190)as*const[u8;0x40]),targets:(0..count).map(|i|ptr::read_unaligned((start+i*0x50)as*const[u8;0x50])).collect()});
 if NESTED.swap(false,Ordering::Relaxed){let nested:Factory=std::mem::transmute(BASE.load(Ordering::Relaxed)+FACTORY_RVA);nested(manager,payload,error);}
 if NESTED_EMIT.swap(false,Ordering::Relaxed){
  let input=SHOTS.lock().unwrap().last().copied().unwrap();let nested:Emit=std::mem::transmute(BASE.load(Ordering::Relaxed)+EMIT_RVA);let mut out=-1i32;let mut err=-1i32;
  nested(manager,&mut out as*mut i32 as usize,input.as_ptr()as usize,&mut err as*mut i32 as usize);
 }
 if FAIL.load(Ordering::Relaxed)||FAIL_ON_CALL.load(Ordering::Relaxed)==call{int(error,3);0}else{int(error,0);HANDLE.load(Ordering::Relaxed)+(call-1)*4}
}
unsafe extern "system" fn packet(out:usize,_input:usize,_payload:usize,_handle:i32,_ctx:usize)->usize{ptr::write_bytes(out as*mut u8,0x57,0xc0);out}
unsafe extern "system" fn packet_copy(out:usize,input:usize)->usize{ptr::copy_nonoverlapping(input as*const u8,out as*mut u8,0xc0);out}
unsafe fn install(base:usize,rva:usize,f:*const u8,original:&AtomicPtr<u8>){let p=crate::hook::prepare((base+rva)as*mut u8,f,64).unwrap();original.store(p.trampoline,Ordering::Release);crate::hook::write_code((base+rva)as*mut u8,&p.patch).unwrap();}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; run alone"]
fn echo_compiled_hooks_replay_native_dispatch_and_release_references(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let h=crate::pe::Headers::parse(&raw,0).unwrap();let allocation=VirtualAlloc(ptr::null_mut(),h.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!allocation.is_null());let base=allocation as usize;
 for s in &h.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 let mut world=vec![0usize;0x17500/8];let mut actor=vec![0usize;0x780/8];let mut modules=vec![0usize;0x200/8];let mut stats=vec![0usize;0x200/8];let p=actor.as_mut_ptr()as usize;let st=stats.as_mut_ptr()as usize;
 put(base+crate::WORLD_RVA,world.as_mut_ptr()as usize);world[0x174e8/8]=p;put(p+8,11);put(p+0x190,p);put(p+0x1b8,modules.as_mut_ptr()as usize);modules[0]=st;int(st+0x140,100);put(base+0x3c131c8,55);
 let mut target_actor=[0usize;4];let mut vt=[0usize;4];vt[2]=kind as*const()as usize;target_actor[0]=vt.as_mut_ptr()as usize;TARGET.store(target_actor.as_mut_ptr()as usize,Ordering::Relaxed);
 let mut target_ref=vec![0u32;0x90/4];target_ref[0x78/4]=2;target_ref[0x80/4]=u32::MAX;
 let mut target=[0u64;0x50/8];let tp=target.as_mut_ptr()as usize;ptr::write(tp as*mut u8,1);put(tp+8,22);put(tp+0x10,target_ref.as_mut_ptr()as usize);TARGET_DESC.store(tp,Ordering::Relaxed);
 let mut first=[0u32;16];first[6]=0xc0000005;let mut second=[0u32;16];second[10]=0x80000006;
 let mut manager=vec![0usize;0x200/8];let mp=manager.as_mut_ptr()as usize;let mut network=vec![0u8;0xc0*1024];manager[0x90/8]=network.as_mut_ptr()as usize;manager[0x98/8]=network.as_ptr()as usize+network.len();put(base+0x3c0ae30,mp);
 let handles:Vec<i32>=(17..4113).collect();HANDLE.store(handles.as_ptr()as usize,Ordering::Relaxed);
 for(rva,f)in [(crate::HAS_EFFECT_RVA,has as*const()as usize),(0x63f840,resolve as*const()as usize),(0x3d2f30,yes as*const()as usize),(0x33d010,parse as*const()as usize),(FACTORY_RVA,create as*const()as usize),(0x33e140,packet as*const()as usize),(0x357e00,packet_copy as*const()as usize)]{jump(base,rva,f);}
 jump(base,0x33e570,release_payload as*const()as usize);
 for rva in [0x362940,0x2163f10,0xf56160,0x4e78c0]{write(base,rva,&[0xc3]);}
 install(base,EMIT_RVA,emit as*const u8,&EMIT_ORIGINAL);install(base,FACTORY_RVA,factory as*const u8,&FACTORY_ORIGINAL);
 let old_base=BASE.swap(base,Ordering::AcqRel);let old_active=ACTIVE.swap(true,Ordering::AcqRel);SHOTS.lock().unwrap().clear();QUEUE.lock().unwrap().owner=Owner::default();
 let mut d=Box::new(Descriptor([0;0x110]));let dp=d.0.as_mut_ptr()as usize;put(dp,11);int(dp+8,-1);int(dp+0x14,10402100);int(dp+0x18,4021);int(dp+0x44,8);put(dp+0x98,first.as_mut_ptr()as usize);put(dp+0xa0,second.as_mut_ptr()as usize);put(dp+0xb8,usize::MAX);
 for(i,v)in [1f32,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,2.,3.,4.,1.].iter().enumerate(){int(dp+0x50+i*4,v.to_bits()as i32);}
 let call:Emit=std::mem::transmute(base+EMIT_RVA);let mut out=-1i32;let mut error=-1i32;
 let mut fire=||call(mp,&mut out as*mut i32 as usize,dp,&mut error as*mut i32 as usize);
 NESTED.store(true,Ordering::Relaxed);fire();assert_eq!(QUEUE.lock().unwrap().pending.len(),1);assert_eq!(target_ref[0x78/4],3);assert_eq!(first[6],0xc0000006);assert_eq!(second[10],0x80000007);
 int(dp+0x14,10402150);int(dp+0x80,99f32.to_bits()as i32); // original input buffer reused and caster moved
 update(p,base,0.);update(p,base,0.25);update(p,base,0.24);assert_eq!(SHOTS.lock().unwrap().len(),1);update(p,base,0.01);
 let shots=SHOTS.lock().unwrap();assert_eq!(shots.len(),2);assert_eq!(i32::from_le_bytes(shots[1][0x14..0x18].try_into().unwrap()),10402100);assert_eq!(f32::from_le_bytes(shots[1][0x80..0x84].try_into().unwrap()),2.);assert_eq!(u64::from_le_bytes(shots[1][0xb8..0xc0].try_into().unwrap()),22);drop(shots);
 assert!(QUEUE.lock().unwrap().pending.is_empty());assert_eq!(target_ref[0x78/4],2);assert_eq!(first[6],0xc0000005);assert_eq!(second[10],0x80000006);assert_eq!(manager[0x90/8],network.as_ptr()as usize+0xc0*2); // native network queue retained
 fire();TARGET_ALIVE.store(false,Ordering::Relaxed);let count=SHOTS.lock().unwrap().len();update(p,base,0.5);assert_eq!(SHOTS.lock().unwrap().len(),count);assert_eq!(target_ref[0x78/4],2);TARGET_ALIVE.store(true,Ordering::Relaxed);
 fire();HAS.store(false,Ordering::Relaxed);update(p,base,0.5);assert!(QUEUE.lock().unwrap().pending.is_empty());assert_eq!(target_ref[0x78/4],2);HAS.store(true,Ordering::Relaxed);
 FAIL.store(true,Ordering::Relaxed);fire();assert!(QUEUE.lock().unwrap().pending.is_empty());assert_eq!(target_ref[0x78/4],2);FAIL.store(false,Ordering::Relaxed);
 int(dp+8,1234);fire();assert!(QUEUE.lock().unwrap().pending.is_empty());int(dp+8,-1);
 fire();put(base+0x3c131c8,56);update(p,base,0.5);assert!(QUEUE.lock().unwrap().pending.is_empty());assert_eq!(target_ref[0x78/4],2);
 // Offensive roots still reject multiple resolved targets; support areas do
 // not depend on the original enemy lock or even the existence of targets.
 TARGET_COUNT.store(2,Ordering::Relaxed);fire();assert!(QUEUE.lock().unwrap().pending.is_empty());
 TARGET_ALIVE.store(false,Ordering::Relaxed);put(dp+0xc0,target_ref.as_mut_ptr()as usize);
 for(i,&(magic,root,replay))in crate::echo_cast_data::AREAS.iter().enumerate(){
  TARGET_COUNT.store(if i%2==0{0}else{2},Ordering::Relaxed);
  int(dp+0x18,magic);int(dp+0x14,root);int(dp+0x80,7f32.to_bits()as i32);
  let before=SHOTS.lock().unwrap().len();fire();assert_eq!(QUEUE.lock().unwrap().pending.len(),1);
  assert_eq!(target_ref[0x78/4],2);assert_eq!(first[6],0xc0000006);
  int(dp+0x80,99f32.to_bits()as i32);update(p,base,0.49);assert_eq!(SHOTS.lock().unwrap().len(),before+1);update(p,base,0.01);
  let shots=SHOTS.lock().unwrap();assert_eq!(shots.len(),before+2);
  let shot=&shots[before+1];assert_eq!(i32::from_le_bytes(shot[0x14..0x18].try_into().unwrap()),replay);
  assert_eq!(i32::from_le_bytes(shot[0x18..0x1c].try_into().unwrap()),magic);
  assert_eq!(f32::from_le_bytes(shot[0x80..0x84].try_into().unwrap()),7.);
  assert_eq!(u64::from_le_bytes(shot[0xb8..0xc0].try_into().unwrap()),u64::MAX);
  assert_eq!(u64::from_le_bytes(shot[0xc0..0xc8].try_into().unwrap()),0);drop(shots);
  assert!(QUEUE.lock().unwrap().pending.is_empty());assert_eq!(target_ref[0x78/4],2);assert_eq!(first[6],0xc0000005);assert_eq!(second[10],0x80000006);
 }
 // Native personal heal and the generated echo IDs cannot schedule an echo.
 TARGET_COUNT.store(1,Ordering::Relaxed);int(dp+0x18,6420);int(dp+0x14,10642000);fire();assert!(QUEUE.lock().unwrap().pending.is_empty());
 int(dp+0x18,6421);int(dp+0x14,95003512);fire();assert!(QUEUE.lock().unwrap().pending.is_empty());
 TARGET_ALIVE.store(true,Ordering::Relaxed);
 // No entity target: forward-only, fixed point, and a point at local origin.
 // Native validity rejects the forward sentinel; it must still be replayed.
 let valid:unsafe extern "system" fn(usize)->u8=std::mem::transmute(base+TARGET_VALID_RVA);
 let mut no_lock=[0u64;10];let np=no_lock.as_mut_ptr()as usize;put(np+8,usize::MAX);
 int(np+0x30,-1);int(np+0x40,-1);int(dp+0x18,4021);int(dp+0x14,10402100);
 for(kind,point)in [(6,[0f32;3]),(4,[10.,20.,30.]),(4,[0.;3])]{
  ptr::write(np as*mut u8,kind);for(i,v)in point.iter().enumerate(){int(np+0x34+i*4,v.to_bits()as i32);}
  if kind==6{assert_eq!(valid(np),0);}
  TARGET_DESC.store(np,Ordering::Relaxed);int(dp+0x80,7f32.to_bits()as i32);
  let before=SHOTS.lock().unwrap().len();fire();assert_eq!(QUEUE.lock().unwrap().pending.len(),1);
  let expected_pose=FACTORY_SHOTS.lock().unwrap().last().unwrap().pose;
  TARGET_DESC.store(tp,Ordering::Relaxed);int(dp+0x80,99f32.to_bits()as i32);update(p,base,0.5);
  assert_eq!(SHOTS.lock().unwrap().len(),before+2);assert!(QUEUE.lock().unwrap().pending.is_empty());
  let shots=FACTORY_SHOTS.lock().unwrap();let replay=shots.last().unwrap();assert_eq!(replay.pose,expected_pose);
  assert_eq!(replay.targets[0],ptr::read_unaligned(np as*const[u8;0x50]));drop(shots);
  assert_eq!(target_ref[0x78/4],2);assert_eq!(first[6],0xc0000005);assert_eq!(second[10],0x80000006);
 }
 // Replay must be rejected if recalculation unexpectedly changes the number
 // of independent targets, instead of selecting just the first new target.
 TARGET_DESC.store(np,Ordering::Relaxed);fire();let before=FACTORY_SHOTS.lock().unwrap().len();TARGET_COUNT.store(2,Ordering::Relaxed);TARGET_DESC.store(tp,Ordering::Relaxed);update(p,base,0.5);
 assert_eq!(FACTORY_SHOTS.lock().unwrap().len(),before);assert_eq!(target_ref[0x78/4],2);TARGET_COUNT.store(1,Ordering::Relaxed);
 // Each actual normal/charged/side-volley root gets one ticket. Descendants
 // and parented emissions cannot multiply the full native projectile chain.
 TARGET_DESC.store(np,Ordering::Relaxed);
 for magic in [4040,4050,4090,4220,4500,4510,6820,6830]{
  let roots=crate::echo_cast_data::ROOTS.iter().find(|r|r.0==magic).unwrap().1;
  int(dp+0x18,magic);let before=FACTORY_SHOTS.lock().unwrap().len();
  for &root in roots{int(dp+0x14,root);fire();}
  assert_eq!(QUEUE.lock().unwrap().pending.len(),roots.len());
  let expected:Vec<_>=FACTORY_SHOTS.lock().unwrap()[before..].iter().map(|s|(s.bullet,s.pose)).collect();
  put(dp+0x90,1);fire();put(dp+0x90,0);assert_eq!(QUEUE.lock().unwrap().pending.len(),roots.len());
  if magic==4220{int(dp+0x14,10422015);fire();assert_eq!(QUEUE.lock().unwrap().pending.len(),roots.len());}
  let before=FACTORY_SHOTS.lock().unwrap().len();update(p,base,0.49);assert_eq!(FACTORY_SHOTS.lock().unwrap().len(),before);update(p,base,0.01);
  let shots=FACTORY_SHOTS.lock().unwrap();assert_eq!(shots.len(),before+roots.len());
  assert_eq!(shots[before..].iter().map(|s|(s.bullet,s.pose)).collect::<Vec<_>>(),expected);drop(shots);
  assert!(QUEUE.lock().unwrap().pending.is_empty());assert_eq!(first[6],0xc0000005);assert_eq!(second[10],0x80000006);
 }
 // Meteors: every actual initial/held/side generation root runs twice now,
 // with no delayed ticket, retained references, or copied random sample.
 for &(magic,roots)in crate::echo_cast_data::DENSITY{
  int(dp+0x18,magic);
  for &root in roots{
   int(dp+0x14,root);let source=ptr::read_unaligned(dp as*const[u8;0x110]);
   let count=SHOTS.lock().unwrap().len();let records=FACTORY_SHOTS.lock().unwrap().len();let packets=manager[0x90/8];let next=CREATE_CALLS.load(Ordering::Relaxed);
   let mut public_out=-1i32;let mut public_error=-1i32;
   let result=call(mp,&mut public_out as*mut i32 as usize,dp,&mut public_error as*mut i32 as usize);
   assert_eq!(result,&mut public_out as*mut i32 as usize);assert_eq!(public_out,handles[next]);assert_eq!(public_error,0);
   assert_eq!(SHOTS.lock().unwrap().len(),count+2);assert_eq!(manager[0x90/8],packets+0xc0*2);
   assert_eq!(ptr::read_unaligned(dp as*const[u8;0x110]),source);
   let shots=FACTORY_SHOTS.lock().unwrap();assert_eq!(shots.len(),records+2);
   assert_eq!(shots[records].bullet,root);assert_eq!(shots[records+1].bullet,root);assert_ne!(shots[records].pose,shots[records+1].pose);drop(shots);
   assert!(QUEUE.lock().unwrap().pending.is_empty());update(p,base,0.5);assert_eq!(SHOTS.lock().unwrap().len(),count+2);
   assert_eq!(first[6],0xc0000005);assert_eq!(second[10],0x80000006);assert_eq!(target_ref[0x78/4],2);
  }
 }
 for count in [0,2]{
  TARGET_COUNT.store(count,Ordering::Relaxed);TARGET_DESC.store(tp,Ordering::Relaxed);
  let before=SHOTS.lock().unwrap().len();fire();assert_eq!(SHOTS.lock().unwrap().len(),before+2);
  assert!(QUEUE.lock().unwrap().pending.is_empty());assert_eq!(target_ref[0x78/4],2);
 }
 TARGET_COUNT.store(1,Ordering::Relaxed);TARGET_DESC.store(np,Ordering::Relaxed);
 // First failure creates no extra attempt; an extra failure cannot overwrite
 // the original successful handle/error or cause a retry on a later frame.
 int(dp+0x18,4700);int(dp+0x14,10470010);
 let before=SHOTS.lock().unwrap().len();FAIL.store(true,Ordering::Relaxed);fire();FAIL.store(false,Ordering::Relaxed);assert_eq!(SHOTS.lock().unwrap().len(),before+1);
 let next=CREATE_CALLS.load(Ordering::Relaxed);let packets=manager[0x90/8];FAIL_ON_CALL.store(next+2,Ordering::Relaxed);
 let mut public_out=-1i32;let mut public_error=-1i32;call(mp,&mut public_out as*mut i32 as usize,dp,&mut public_error as*mut i32 as usize);
 assert_eq!(public_out,handles[next]);assert_eq!(public_error,0);assert_eq!(manager[0x90/8],packets+0xc0);FAIL_ON_CALL.store(0,Ordering::Relaxed);
 // Re-entrant emissions during density generation do not double again.
 let before=SHOTS.lock().unwrap().len();NESTED_EMIT.store(true,Ordering::Relaxed);fire();assert_eq!(SHOTS.lock().unwrap().len(),before+3);
 for magic in [4700,4701]{
  int(dp+0x18,magic);
  for bullet in if magic==4700{[10470001,10470002,10470003]}else{[10470102,10470105,10470106]}{
   int(dp+0x14,bullet);let before=SHOTS.lock().unwrap().len();fire();assert_eq!(SHOTS.lock().unwrap().len(),before+1);
  }
 }
 int(dp+0x18,4701);int(dp+0x14,10470100);
 for reason in 0..6{
  match reason{0=>HAS.store(false,Ordering::Relaxed),1=>{put(dp,77);},2=>{int(dp+8,123);},3=>{put(dp+0x90,1);},4=>{int(st+0x140,0);},_=>ACTIVE.store(false,Ordering::Relaxed)}
  let before=SHOTS.lock().unwrap().len();fire();assert_eq!(SHOTS.lock().unwrap().len(),before+1);
  HAS.store(true,Ordering::Relaxed);put(dp,11);int(dp+8,-1);put(dp+0x90,0);int(st+0x140,100);ACTIVE.store(true,Ordering::Relaxed);
 }
 assert!(QUEUE.lock().unwrap().pending.is_empty());assert!(!DENSITY.with(Cell::get));assert_eq!(first[6],0xc0000005);assert_eq!(second[10],0x80000006);assert_eq!(target_ref[0x78/4],2);
 BASE.store(old_base,Ordering::Release);ACTIVE.store(old_active,Ordering::Release);assert_ne!(VirtualFree(allocation,0,MEM_RELEASE),0);
}}
