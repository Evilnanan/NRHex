//! Execute the shipping mid-hook, relocated instructions and actual capsule
//! velocity setter. Scene contacts/ownership are controlled fixtures.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from(id==OWNER)}
unsafe extern "system" fn rotation(_:usize,out:*mut f32)->*const f32{ptr::copy_nonoverlapping([0.;4].as_ptr(),out,4);out}
unsafe extern "system" fn set_rotation(_:usize,p:*const f32){assert!(ptr::read(p.add(1)).is_finite());}
unsafe extern "system" fn weapon(_:usize,source:i32)->i32{assert!(matches!(source,-1|-2));110000}
unsafe extern "system" fn cost(reference:usize,mode:i32,_:usize)->i32{assert_eq!(mode,0);i32::from(ptr::read_unaligned((read_ptr(reference+8)+0x14)as*const i16))}
unsafe extern "system" fn pay(stats:usize,amount:i32){let p=(stats+0x150)as*mut i32;ptr::write(p,(ptr::read(p)+amount).max(0));}
unsafe extern "system" fn collision(_:usize,hit:*mut u32,_:u32,_:*const f32,motion:*const f32,_:f32,_:usize,_:*mut u32,_:*mut f32,normal:*mut f32,fraction:*mut f32,_:*mut f32)->*mut u32{
 if ptr::read(motion.add(1))< -0.2{ptr::write(hit,42);ptr::write(fraction,0.5);ptr::copy_nonoverlapping([0.,1.,0.,0.].as_ptr(),normal,4);}else{ptr::write(hit,u32::MAX);}hit
}
#[unsafe(naked)]unsafe extern "system" fn fixture(_:usize,_:*mut f32,_:*const f32,_:usize){
 core::arch::naked_asm!("push rdi","push rbp","push r14","sub rsp,0x40","mov rdi,rcx","lea rbp,[rdx+0x80]","mov r14,r8","jmp r9");
}
#[test]#[ignore="requires NRHEX_NATIVE_EXE; run alone"]
fn native_ram_velocity_submission_and_local_owner_boundary(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 for(rva,f)in[(crate::HAS_EFFECT_RVA,has as*const()as usize),(crate::CHR_ROTATION_RVA,rotation as*const()as usize),(crate::parry_turn::SET_ROTATION_RVA,set_rotation as*const()as usize),(0xdb04a0,collision as*const()as usize),(crate::onyx_mastery::ARTS_FEE_RVA,cost as*const()as usize),(crate::onyx_mastery::FP_ADD_RVA,pay as*const()as usize)]{
  let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&f.to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(jump.as_ptr(),(base+rva)as*mut u8,jump.len());
 }
 let mut world=vec![0usize;0x17500/8];let mut actor=vec![0usize;0x780/8];let mut modules=vec![0usize;0x100/8];let mut stats=vec![0i32;0x200/4];let mut physics=vec![0usize;0x180/8];let mut controller=vec![0usize;0xb0/8];let mut geometry=vec![0usize;0xa0/8];
 #[repr(align(16))]struct Vec4([f32;4]);let mut desired=Vec4([0.,0.,-4.,0.]);let mut submitted=Vec4([f32::NAN;4]);
 let mut vt=vec![0usize;0x258/8];vt[0x250/8]=weapon as*const()as usize;actor[0]=vt.as_mut_ptr()as usize;
 let a=actor.as_mut_ptr()as usize;actor[1]=123;actor[0x190/8]=a;actor[0x1b8/8]=modules.as_mut_ptr()as usize;modules[0]=stats.as_mut_ptr()as usize;modules[0x68/8]=physics.as_mut_ptr()as usize;stats[0x140/4]=100;physics[1]=a;
 physics[0x98/8]=controller.as_mut_ptr()as usize;controller[0xa0/8]=submitted.0.as_mut_ptr()as usize;world[0x174e8/8]=a;geometry[0x98/8]=123;
 ptr::write_unaligned((base+crate::WORLD_RVA)as*mut usize,world.as_ptr()as usize);ptr::write_unaligned((base+0x3c1e028)as*mut usize,geometry.as_ptr()as usize);ptr::write_unaligned((base+0x3c1d9a8)as*mut usize,1);
 let prepared=crate::hook::prepare((base+MOTION_RVA)as*mut u8,motion_detour as*const u8,64).unwrap();assert_eq!(prepared.stolen,11);
 let old_orig=MOTION_ORIGINAL.swap(prepared.trampoline,Ordering::AcqRel);
 crate::hook::write_code((base+MOTION_RVA)as*mut u8,&prepared.patch).unwrap();
 // End after native CALL D977F0. The setter itself is unmodified game code.
 let epilogue=[0x48,0x83,0xc4,0x40,0x41,0x5e,0x5d,0x5f,0xc3];ptr::copy_nonoverlapping(epilogue.as_ptr(),(base+MOTION_RVA+16)as*mut u8,epilogue.len());
 let old_base=BASE.swap(base,Ordering::AcqRel);let old_active=ACTIVE.swap(true,Ordering::AcqRel);let time=[0.,0.,1./60.,0.];
 for enabled in [false,true]{
  *STATE.lock().unwrap()=State{actor:a,handle:123,active:enabled,started:true,drive:true,wish:V(0.,0.,-1.),motion:Motion{velocity:V(0.,0.,-12.),..Default::default()},..Default::default()};
  desired.0=[0.,0.,-4.,0.];fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);
  assert_eq!(submitted.0,desired.0);assert_eq!(desired.0[0],0.);assert_eq!(desired.0[1],0.);
  if enabled{assert!(desired.0[2]< -12.&&desired.0[2]> -13.);}else{assert_eq!(desired.0[2],-4.);}
 }
 // Identical action state must never override another player's capsule.
 world[0x174e8/8]=a+8;desired.0=[1.,2.,3.,0.];fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);assert_eq!(submitted.0,[1.,2.,3.,0.]);
 assert!(art(a,1,base),"remote owner must retain Ram presentation");assert_eq!(query(a,113,1,true,false,0.,base),0,"remote input must not drive local simulation");
 world[0x174e8/8]=a;reset();stats[0x150/4]=100;stats[0x15c/4]=100;
 assert_eq!(query(a,113,1,true,false,0.,base),0);assert_eq!(stats[0x150/4],95);
 for _ in 0..5{assert_eq!(query(a,113,1,true,false,0.,base),0);}assert_eq!(stats[0x150/4],95,"startup paid once");
 assert_eq!(query(a,113,1,true,true,0.,base),2);assert_eq!(stats[0x150/4],90);
 assert_eq!(query(a,113,1,true,true,0.,base),0);assert_eq!(stats[0x150/4],90,"one continuation per window");
 {let mut s=STATE.lock().unwrap();s.started=true;s.motion.locked=true;}
 assert_eq!(query(a,113,1,false,false,0.,base),1);assert_eq!(stats[0x150/4],90,"release preserves inertia without paying");
 assert_eq!(query(a,113,1,false,true,0.,base),2);assert_eq!(stats[0x150/4],90);
 {let mut s=STATE.lock().unwrap();s.motion.locked=false;}
 assert_eq!(query(a,113,1,false,false,0.,base),0);assert!(!STATE.lock().unwrap().active);
 reset();stats[0x150/4]=0;assert_eq!(query(a,113,1,true,false,0.,base),0);assert!(!STATE.lock().unwrap().active);
 reset();MOTION_ORIGINAL.store(old_orig,Ordering::Release);BASE.store(old_base,Ordering::Release);ACTIVE.store(old_active,Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
