//! Execute the original art-fee aggregation/rounding and FP setter. Only the
//! actor's inventory/effect-presence lookups and bullet creation are fixtures.
use super::*;
use crate::{ptr,win::*};
use std::sync::atomic::{AtomicI32,AtomicBool};
static OWNS:AtomicBool=AtomicBool::new(true);
static WEAPON:AtomicI32=AtomicI32::new(8010000);
static LEFT_WEAPON:AtomicI32=AtomicI32::new(2000000);
static ARTS_OWNED:AtomicBool=AtomicBool::new(false);
static ART_ROW:std::sync::atomic::AtomicPtr<u8>=std::sync::atomic::AtomicPtr::new(ptr::null_mut());
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn jump(base:usize,rva:usize,f:usize){let mut b=vec![0xff,0x25,0,0,0,0];b.extend_from_slice(&f.to_le_bytes());ptr::copy_nonoverlapping(b.as_ptr(),(base+rva)as*mut u8,b.len());}
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from(if id==OWNER{OWNS.load(Ordering::Relaxed)}else{id==crate::arts::OWNER&&ARTS_OWNED.load(Ordering::Relaxed)})}
unsafe extern "system" fn weapon(_:usize,source:i32)->i32{if source==-2{LEFT_WEAPON.load(Ordering::Relaxed)}else{WEAPON.load(Ordering::Relaxed)}}
unsafe extern "system" fn handle(actor:usize,out:usize,source:i32)->usize{int(out,weapon(actor,source));out}
unsafe extern "system" fn reference(out:usize,input:usize)->usize{int(out,read_i32(input));put(out+8,0);out}
unsafe extern "system" fn art(_:usize,out:usize)->usize{int(out,1041);put(out+8,ART_ROW.load(Ordering::Relaxed)as usize);out}
unsafe extern "system" fn release(_:usize){}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; execute alone"]
fn native_meteor_fee_is_paid_only_for_accepted_root_shots(){unsafe{
 let path=std::path::PathBuf::from(std::env::var_os("NRHEX_NATIVE_EXE").unwrap());crate::verify_executable(&path).unwrap();
 let raw=std::fs::read(path).unwrap();let headers=crate::pe::Headers::parse(&raw,0).unwrap();
 let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for s in &headers.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 jump(base,crate::HAS_EFFECT_RVA,has as*const()as usize);
 put(base+0x3c0f338,1);ptr::write((base+0x3c0f2f8)as*mut u8,0);
 let mut actor=[0usize;0x800/8];let a=actor.as_mut_ptr()as usize;
 let mut stats=[0u32;0x200/4];let st=stats.as_mut_ptr()as usize;let modules=[st];put(a+0x1b8,modules.as_ptr()as usize);
 let mut vt=[0usize;0x260/8];vt[0x250/8]=weapon as*const()as usize;put(a,vt.as_ptr()as usize);
 let mut world=vec![0usize;0x17500/8];world[0x174e8/8]=a;put(base+crate::WORLD_RVA,world.as_ptr()as usize);
 let mut manager=[0usize;4];let m=manager.as_mut_ptr()as usize;put(a+0x190,m);
 let mut row=[0u8;0x400];let rp=row.as_mut_ptr()as usize;
 let mut node=[0usize;0x88/8];let n=node.as_mut_ptr()as usize;put(n,rp);put(m+8,n);
 int(st+0x140,100);int(st+0x154,1000);
 let mut descriptor=[0u8;0x18];descriptor[0x10]=1;let d=descriptor.as_ptr()as usize;let mut error=-1;let e=&mut error as*mut i32 as usize;
 let mut checks=0;
 for (rate,cost)in [(1f32,10),(0.8,8),(0.45,5),(0.,0),(2.,20)]{
  ptr::write_unaligned((rp+0x2e4)as*mut f32,rate);
  // Magic multipliers are intentionally irrelevant to a weapon art.
  ptr::write_unaligned((rp+0x2e8)as*mut f32,100.);
  assert_eq!(meteor_fee(a,base),cost);
  for fp in [0,1,7,8,9,10,11,19,20,50]{
   int(st+0x150,fp);query(a,1,241,1,true,base);
   assert_eq!(channel_available(a,base),Some(fp>=cost));
   let mut calls=0;let result=emit(a,BEHAVIOR,d,-1,e,base,||{calls+=1;1});
   assert_eq!(result,u8::from(fp>=cost));assert_eq!(calls,i32::from(fp>=cost));
   assert_eq!(read_i32(st+0x150),if fp>=cost{fp-cost}else{fp});checks+=1;
  }
 }
 ptr::write_unaligned((rp+0x2e4)as*mut f32,1.);
 int(st+0x150,50);for _ in 0..5{assert_eq!(emit(a,BEHAVIOR,d,-1,e,base,||1),1);}assert_eq!(read_i32(st+0x150),0);
 assert_eq!(emit(a,BEHAVIOR,d,-1,e,base,||panic!("free sixth meteor")),0);
 int(st+0x150,100);assert_eq!(emit(a,BEHAVIOR,d,-1,e,base,||0),0);assert_eq!(read_i32(st+0x150),100);
 // Native setter immunity, inactive cost effects, remote/other behaviors and
 // a non-emitting TAE descriptor must retain their own original behavior.
 ptr::write((st+0x189)as*mut u8,0x10);assert_eq!(emit(a,BEHAVIOR,d,-1,e,base,||1),1);assert_eq!(read_i32(st+0x150),100);ptr::write((st+0x189)as*mut u8,0);
 ptr::write_unaligned((rp+0x2e4)as*mut f32,0.);int(n+0x68,2);assert_eq!(meteor_fee(a,base),10);int(n+0x68,0);
 for behavior in [95005350,95005353,95005354,300805900]{assert_eq!(emit(a,behavior,d,-1,e,base,||7),7);assert_eq!(read_i32(st+0x150),100);}
 assert_eq!(emit(a+8,BEHAVIOR,d,-1,e,base,||7),7);
 descriptor[0x10]=0;assert_eq!(emit(a,BEHAVIOR,descriptor.as_ptr()as usize,-1,e,base,||7),7);descriptor[0x10]=1;
 OWNS.store(false,Ordering::Relaxed);assert_eq!(emit(a,BEHAVIOR,d,-1,e,base,||panic!("lost mastery")),0);assert_eq!(channel_available(a,base),None);OWNS.store(true,Ordering::Relaxed);
 WEAPON.store(2000000,Ordering::Relaxed);assert_eq!(emit(a,BEHAVIOR,d,-1,e,base,||panic!("wrong weapon")),0);WEAPON.store(8010000,Ordering::Relaxed);
 ptr::write((a+0x710)as*mut u8,1);assert_eq!(emit(a,BEHAVIOR,d,-1,e,base,||panic!("incapacitated")),0);
 assert_eq!(read_i32(st+0x150),100);ACTION.with(|v|v.set((0,-1,false)));
 // Install all three actual wrappers, then execute the original hand lookup,
 // FP eligibility, reservation, aggregation and rounding. Inventory references
 // are fixtures; both hands deliberately report the same art ID (1041), so the
 // exception must depend on the source weapon, not the art ID alone.
 ptr::write((a+0x710)as*mut u8,0);put(vt.as_mut_ptr()as usize+0x258,handle as*const()as usize);
 jump(base,0x6a1d70,reference as*const()as usize);jump(base,0x6a2180,art as*const()as usize);
 jump(base,0x6a1d90,release as*const()as usize);jump(base,0xea2370,release as*const()as usize);
 let saved_base=crate::BASE.swap(base,Ordering::AcqRel);let saved_active=crate::ACTIVE.swap(true,Ordering::AcqRel);
 for(rva,detour,original)in [
  (crate::onyx_fp::CHECK_RVA,crate::onyx_fp::check as*const u8,&crate::onyx_fp::CHECK_ORIGINAL),
  (crate::onyx_fp::RESERVE_RVA,crate::onyx_fp::reserve as*const u8,&crate::onyx_fp::RESERVE_ORIGINAL),
  (crate::onyx_fp::RATE_RVA,crate::onyx_fp::rate as*const u8,&crate::onyx_fp::RATE_ORIGINAL)]{
  let hook=crate::hook::prepare((base+rva)as*mut u8,detour,64).unwrap();original.store(hook.trampoline,Ordering::Release);crate::hook::write_code((base+rva)as*mut u8,&hook.patch).unwrap();
 }
 let mut art_row=[0u8;0x30];for offset in [0x10,0x12,0x14,0x16]{art_row[offset..offset+2].copy_from_slice(&27i16.to_le_bytes());}ART_ROW.store(art_row.as_mut_ptr(),Ordering::Relaxed);
 let mut art_module=[0usize;0x30/8];let am=art_module.as_mut_ptr()as usize;put(am+8,a);
 let check:unsafe extern "system" fn(usize,i32,i32)->u8=std::mem::transmute(base+crate::onyx_fp::CHECK_RVA);
 let reserve:unsafe extern "system" fn(usize,i32,i32)=std::mem::transmute(base+crate::onyx_fp::RESERVE_RVA);
 let fee:unsafe extern "system" fn(usize,i32,usize)->i32=std::mem::transmute(base+ARTS_FEE_RVA);
 let mut wild_row=[0u8;0x300];let mut wild_node=[0usize;0x88/8];let wn=wild_node.as_mut_ptr()as usize;
 put(wn,wild_row.as_mut_ptr()as usize);int(wn+8,crate::arts::OWNER);put(wn+0x38,n);put(m+8,wn);
 ARTS_OWNED.store(true,Ordering::Relaxed);
 let reference=[1041usize,art_row.as_ptr()as usize];
 for(rate,start,meteor)in [(1f32,27,10),(0.8,22,8),(0.45,13,5),(0.,0,0),(2.,54,20)]{
  ptr::write_unaligned((rp+0x2e4)as*mut f32,rate);int(st+0x150,100);
  let saved_wild=wild_node;let saved_node=node;let saved_row=wild_row;
  for button in 0..4{assert_eq!(check(am,button,1),1);reserve(am,button,1);assert_eq!(read_i32(am+0x10),start);checks+=1;}
  assert_eq!(meteor_fee(a,base),meteor);assert_eq!(fee(reference.as_ptr()as usize,0,m),0,"scope leaked outside Onyx");
  reserve(am,0,0);assert_eq!(read_i32(am+0x10),0,"other hand with identical art must remain free");
  assert_eq!(wild_node,saved_wild);assert_eq!(node,saved_node);assert_eq!(wild_row,saved_row);
 }
 ptr::write_unaligned((rp+0x2e4)as*mut f32,1.);int(st+0x150,1);
 assert_eq!(check(am,0,1),0);assert_eq!(check(am,0,0),1);
 OWNS.store(false,Ordering::Relaxed);reserve(am,0,1);assert_eq!(read_i32(am+0x10),0);OWNS.store(true,Ordering::Relaxed);
 // Reverse hands: the formerly qualifying right hand must immediately go free.
 WEAPON.store(2000000,Ordering::Relaxed);LEFT_WEAPON.store(8010000,Ordering::Relaxed);
 reserve(am,0,1);assert_eq!(read_i32(am+0x10),0);reserve(am,0,0);assert_eq!(read_i32(am+0x10),27);
 WEAPON.store(8010000,Ordering::Relaxed);LEFT_WEAPON.store(2000000,Ordering::Relaxed);
 int(st+0x150,100);reserve(am,0,1);
 let add:unsafe extern "system" fn(usize,i32)=std::mem::transmute(base+FP_ADD_RVA);add(st,-read_i32(am+0x10));
 for _ in 0..5{assert_eq!(emit(a,BEHAVIOR,d,-1,e,base,||1),1);}assert_eq!(read_i32(st+0x150),23,"Wildcard must not waive the 77-FP total");
 crate::ACTIVE.store(false,Ordering::Release);reserve(am,0,1);assert_eq!(read_i32(am+0x10),0);
 ARTS_OWNED.store(false,Ordering::Relaxed);crate::ACTIVE.store(saved_active,Ordering::Release);crate::BASE.store(saved_base,Ordering::Release);
 assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);println!("PASS: {checks} native affordability/payment cases, five-shot total, failure, immunity and ownership boundaries");
}}
