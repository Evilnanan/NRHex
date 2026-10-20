//! Native source-map resolution, compiled HKS query/input/impulse routes.
//! Only actor/effect storage, the ordinary env fallback and launch callbacks
//! are fixtures; native map selection and horizontal getter really execute.
use super::*;
use crate::{ptr,read_ptr};
use crate::win::*;
use sha2::{Digest,Sha256};
use std::{cell::Cell,sync::atomic::Ordering};
thread_local!{static HAS:Cell<bool>=const{Cell::new(true)};static INPUT_RESULT:Cell<u8>=const{Cell::new(0)};}
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn patch(base:usize,rva:usize,b:&[u8]){ptr::copy_nonoverlapping(b.as_ptr(),(base+rva)as*mut u8,b.len());}
unsafe fn jump(base:usize,rva:usize,f:usize){let mut b=vec![0x48,0xb8];b.extend_from_slice(&f.to_le_bytes());b.extend_from_slice(&[0xff,0xe0]);patch(base,rva,&b);}
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from(id==5031||(id==crate::cat::OWNER&&HAS.with(Cell::get)))}
unsafe extern "system" fn ordinary_env(context:usize,code:i32,vm:usize)->i32{assert_ne!(context,0);assert_ne!(vm,0);assert_ne!(code,QUERY);77}
unsafe extern "system" fn input(_:usize,mode:i32,action:i32)->u8{assert_eq!((mode,action),(1,6));INPUT_RESULT.with(Cell::get)}
unsafe extern "system" fn launch(velocity:usize,out:*mut f32,_input:*const f32,_dt:usize){
 let row=crate::read_i32(velocity+0x14);let mut data=[0f32;4];data[2]=-1.3;let reference=[row as usize,data.as_ptr()as usize];
 *out=crate::slingshot_runtime::horizontal(reference.as_ptr()as usize);
}
core::arch::global_asm!(r#"
.global cat_native_source_map
cat_native_source_map:
 push rbx
 sub rsp,0x60
 mov rbx,rcx
 mov rax,0xffffffff00000000
 and rax,rcx
 mov [rsp+0x50],rax
 call rdx
 add rsp,0x60
 pop rbx
 ret
"#);
unsafe extern "system"{fn cat_native_source_map(handle:u64,fragment:usize)->u32;}
#[test]fn arena_families_include_dlc_without_blocking_day_boss_or_preparation_maps(){
 for major in 1..=254u32{for minor in [0,10,11,20,99]{
  assert_eq!(outside_nightlord(Some((major<<24)|(minor<<16))),!matches!(major,16|18|19));
 }}
 for map in [None,Some(0),Some(u32::MAX)]{assert!(!outside_nightlord(map));}
}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; run alone"]
fn cat_arena_compiled_query_input_and_impulse_follow_native_current_map(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 // Return the map in ECX exactly before the native constructor stores it.
 patch(base,0x576d6b,&[0x89,0xc8,0xc3]);jump(base,crate::HAS_EFFECT_RVA,has as*const()as usize);
 let saved_base=crate::BASE.swap(base,Ordering::AcqRel);let saved_active=crate::ACTIVE.swap(true,Ordering::AcqRel);
 let old_env=crate::ambulance::ENV_ORIGINAL.swap(ordinary_env as*mut u8,Ordering::AcqRel);
 let old_input=crate::ACTION_REQUEST_ORIG.swap(input as*mut u8,Ordering::AcqRel);
 let old_launch=crate::slingshot_runtime::ORIGINAL.swap(launch as*mut u8,Ordering::AcqRel);
 let old_horizontal=crate::slingshot_runtime::HORIZONTAL_ORIGINAL.swap((base+crate::slingshot_runtime::HORIZONTAL_RVA)as*mut u8,Ordering::AcqRel);
 let mut world=vec![0usize;0x17500/8];let world=world.as_mut_ptr()as usize;put(base+crate::WORLD_RVA,world);
 let mut a=[0usize;0x720/8];let player=a.as_mut_ptr()as usize;put(world+0x174e8,player);put(player+0x190,player);
 let mut stats=[0usize;0x200/8];let stats=stats.as_mut_ptr()as usize;int(stats+0x140,100);
 let mut modules=[0usize;0x100/8];let modules=modules.as_mut_ptr()as usize;put(player+0x1b8,modules);put(modules,stats);
 let mut physics=[0usize;0x300/8];let physics=physics.as_mut_ptr()as usize;put(modules+0x68,physics);ptr::write((physics+0x2c0)as*mut u8,1);
 let mut controls=[0usize;0x40/8];let controls=controls.as_mut_ptr()as usize;put(controls+8,player);put(controls+0x20,64);put(modules+0x80,controls);
 let mut velocity=[0usize;0x30/8];let velocity=velocity.as_mut_ptr()as usize;put(velocity+0x20,player);int(velocity+0x14,510);
 let mut context=[0usize;0x40/8];let map_context=context.as_mut_ptr()as usize;put(base+crate::MAP_CONTEXT_RVA,map_context);let hks_context=[player];let vm=1usize;
 // Fallback maps are per-request reads, not cached at acquisition or launch.
 for fallback in [60u32<<24,18<<24,19<<24,(16<<24)|(11<<16),13<<24,15<<24,49<<24]{
  int(map_context+0x2c,fallback as i32);
  for packed in [0,u32::MAX,60<<24,18<<24,19<<24,16<<24,(16<<24)|(10<<16)]{
   let handle=(u64::from(packed)<<32)|123;put(player+8,handle as usize);
   let native=cat_native_source_map(handle,base+0x576d44);assert_eq!(current_map(player,base),Some(native));
   let allowed=outside_nightlord(Some(native));
   for owned in [false,true]{HAS.with(|s|s.set(owned));
    assert_eq!(crate::ambulance::env(hks_context.as_ptr()as usize,QUERY,vm),i32::from(owned&&allowed));
    // Holding the ability still reads true even inside a forbidden arena.
    assert_eq!(crate::has_effect(player,crate::cat::OWNER,base),owned);
    INPUT_RESULT.with(|s|s.set(0));assert_eq!(crate::action_request_detour(controls,1,6),u8::from(owned&&allowed));
    INPUT_RESULT.with(|s|s.set(1));assert_eq!(crate::action_request_detour(controls,1,6),1);INPUT_RESULT.with(|s|s.set(0));
    let mut out=0.;crate::slingshot_runtime::detour(velocity,&mut out,ptr::null(),0);assert_eq!(out,if owned&&allowed{4.8}else{-1.3});
   }
  }
 }
 HAS.with(|s|s.set(true));put(player+8,u32::MAX as usize|((60u64<<24)<<32)as usize);
 for (hp,state)in [(0,0),(100,1),(100,2)]{int(stats+0x140,hp);ptr::write((player+0x710)as*mut u8,state);assert!(!enabled(player,base));assert_eq!(crate::ambulance::env(hks_context.as_ptr()as usize,QUERY,vm),0);}
 int(stats+0x140,100);ptr::write((player+0x710)as*mut u8,0);
 assert_eq!(crate::ambulance::env(hks_context.as_ptr()as usize,1116,vm),77);
 crate::ACTIVE.store(false,Ordering::Release);assert_eq!(crate::ambulance::env(hks_context.as_ptr()as usize,QUERY,vm),0);crate::ACTIVE.store(true,Ordering::Release);
 assert_eq!(crate::ambulance::env(0,QUERY,vm),0);assert_eq!(crate::ambulance::env(hks_context.as_ptr()as usize,QUERY,0),0);
 put(player+8,u64::MAX as usize);put(base+crate::MAP_CONTEXT_RVA,0);assert_eq!(current_map(player,base),None);assert!(!enabled(player,base));
 put(base+crate::MAP_CONTEXT_RVA,map_context);int(map_context+0x2c,0);assert!(!enabled(player,base));
 put(world+0x174e8,player+8);assert!(!enabled(player,base));
 crate::ambulance::ENV_ORIGINAL.store(old_env,Ordering::Release);crate::ACTION_REQUEST_ORIG.store(old_input,Ordering::Release);crate::slingshot_runtime::ORIGINAL.store(old_launch,Ordering::Release);crate::slingshot_runtime::HORIZONTAL_ORIGINAL.store(old_horizontal,Ordering::Release);crate::BASE.store(saved_base,Ordering::Release);crate::ACTIVE.store(saved_active,Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
