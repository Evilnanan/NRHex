use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
thread_local!{static ACTOR:Cell<usize>=const{Cell::new(0)};static RELATION:Cell<usize>=const{Cell::new(0)};}
unsafe extern "system" fn object_kind(object:usize)->i32{read_i32(object+0x10)}
unsafe extern "system" fn resolve(handle:u64)->usize{if handle==0x10000001{ACTOR.with(Cell::get)}else{0}}
unsafe extern "system" fn resolve_world(_:usize,handle:usize)->usize{resolve(ptr::read_unaligned(handle as*const u64))}
unsafe extern "system" fn effective_team(manager:usize,out:usize)->usize{ptr::write(out as*mut u8,ptr::read((manager-0x338+0x75)as*const u8));out}
unsafe extern "system" fn collision(hit:usize,target:usize,_:usize)->u8{
 let source=ACTOR.with(Cell::get);let relation:unsafe extern "system" fn(usize,usize,usize)->i32=std::mem::transmute(RELATION.with(Cell::get));
 relation(source,target,read_ptr(hit+0x20))as u8
}
unsafe fn jump(base:usize,rva:usize,target:usize){
 let mut bytes=vec![0x48,0xb8];bytes.extend_from_slice(&target.to_le_bytes());bytes.extend_from_slice(&[0xff,0xe0]);
 ptr::copy_nonoverlapping(bytes.as_ptr(),(base+rva)as*mut u8,bytes.len());
}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; isolated native relationship table and hooks must run alone"]
fn compiled_balloon_hooks_execute_native_relation_masks_without_changing_the_actor(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").expect("NRHEX_NATIVE_EXE")).unwrap();
 assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();
 let allocation=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!allocation.is_null());let base=allocation as usize;
 for section in &headers.sections{
  ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);
 }
 let mut actor=vec![0u64;0x350/8];let actor=actor.as_mut_ptr()as usize;
 let mut target=vec![0u64;0x350/8];let target=target.as_mut_ptr()as usize;
 let mut bullet=vec![0u64;0xc0/8];let bullet=bullet.as_mut_ptr()as usize;
 let mut vtable=vec![0usize;3];vtable[2]=object_kind as*const()as usize;
 for (object,kind) in [(actor,1i32),(target,1),(bullet,3)]{
  ptr::write_unaligned(object as*mut usize,vtable.as_ptr()as usize);ptr::write_unaligned((object+0x10)as*mut i32,kind);
 }
 ptr::write((actor+0x74)as*mut u8,2);ptr::write((actor+0x75)as*mut u8,77);
 ptr::write((target+0x74)as*mut u8,6);ptr::write((target+0x75)as*mut u8,6);
 ptr::write_unaligned((bullet+0xa4)as*mut i32,balloon::BULLET);ptr::write_unaligned((bullet+0xb0)as*mut u64,0x10000001);
 let mut hit=vec![0u64;0x240/8];let hit=hit.as_mut_ptr()as usize;
 ptr::write_unaligned((hit+0x40)as*mut i32,balloon::ATTACK);ptr::write_unaligned((hit+0x44)as*mut i32,1);
 ptr::write_unaligned((hit+0x228)as*mut usize,bullet);ptr::write_unaligned((hit+0x230)as*mut usize,target);
 let mut row=[0u8;0x90];row[0x81]=1;let mut reference=[0u64;3];reference[2]=row.as_ptr()as u64;
 ptr::write_unaligned((hit+0x20)as*mut usize,reference.as_ptr()as usize);
 let methods=[base+0x52c900,base+0x52c910,base+0x52c920];
 let objects=[methods.as_ptr()as usize,methods.as_ptr().add(1)as usize,methods.as_ptr().add(2)as usize];
 for (source,destination,index) in [(77,6,0),(77,7,0),(77,2,0),(2,6,2),(2,7,2),(2,2,1)]{
  ptr::write_unaligned((base+0x398d410+(source*81+destination)*8)as*mut usize,objects.as_ptr().add(index)as usize);
 }
 ACTOR.with(|value|value.set(actor));RELATION.with(|value|value.set(base+0x52cb90));
 jump(base,CHR_HANDLE_RVA,resolve as*const()as usize);jump(base,0x518f30,resolve_world as*const()as usize);
 jump(base,0x4b4100,effective_team as*const()as usize);jump(base,0x33c120,collision as*const()as usize);
 ptr::copy_nonoverlapping([0xb0u8,1,0xc3].as_ptr(),(base+0x4b41a0)as*mut u8,3);
 ptr::copy_nonoverlapping([0x31u8,0xc0,0xc3].as_ptr(),(base+crate::HAS_STATE_RVA)as*mut u8,3);
 ptr::write_unaligned((base+crate::WORLD_RVA)as*mut usize,1);
 let saved_base=BASE.swap(base,Ordering::AcqRel);let saved_active=ACTIVE.swap(true,Ordering::AcqRel);
 let saved=[TEAM_ORIGINAL.load(Ordering::Acquire),HIT_ORIGINAL.load(Ordering::Acquire),BULLET_ORIGINAL.load(Ordering::Acquire)];
 for (rva,detour,original) in [(TEAM_RVA,team_detour as*const u8,&TEAM_ORIGINAL),(HIT_RVA,hit_detour as*const u8,&HIT_ORIGINAL),(BULLET_RVA,bullet_detour as*const u8,&BULLET_ORIGINAL)]{
  let prepared=crate::hook::prepare((base+rva)as*mut u8,detour,64).unwrap();original.store(prepared.trampoline,Ordering::Release);
  crate::hook::write_code((base+rva)as*mut u8,&prepared.patch).unwrap();
 }
 let native_hit:unsafe extern "system" fn(usize)->u8=std::mem::transmute(base+HIT_RVA);
 let native_bullet:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(base+BULLET_RVA);
 let native_team:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(base+TEAM_RVA);
 let mut output=0u8;let out=&mut output as*mut u8 as usize;
 assert_eq!(native_team(actor,out),out);assert_eq!(output,77);
 assert_eq!(native_bullet(bullet,out),out);assert_eq!(output,2);
 ACTIVE.store(false,Ordering::Release);assert_eq!(native_hit(hit),0);assert_eq!(native_bullet(bullet,out),out);assert_eq!(output,77);
 ACTIVE.store(true,Ordering::Release);assert_eq!(native_hit(hit),1);
 ptr::write((target+0x75)as*mut u8,7);assert_eq!(native_hit(hit),1);
 ptr::write((target+0x75)as*mut u8,2);assert_eq!(native_hit(hit),0);
 ptr::write_unaligned((hit+0x230)as*mut usize,actor);assert_eq!(native_hit(hit),0);
 ptr::write_unaligned((hit+0x230)as*mut usize,target);ptr::write((target+0x75)as*mut u8,6);
 ptr::write_unaligned((hit+0x40)as*mut i32,12064105);assert_eq!(native_hit(hit),0);
 ptr::write_unaligned((hit+0x40)as*mut i32,balloon::ATTACK);ptr::write_unaligned((hit+0x44)as*mut i32,0);assert_eq!(native_hit(hit),0);
 ptr::write_unaligned((hit+0x44)as*mut i32,1);ptr::write_unaligned((bullet+0xa4)as*mut i32,12641050);assert_eq!(native_hit(hit),0);
 assert_eq!(native_bullet(bullet,out),out);assert_eq!(output,77);
 assert_eq!(ptr::read((actor+0x74)as*const u8),2);assert_eq!(ptr::read((actor+0x75)as*const u8),77);assert_eq!(SOURCE.with(Cell::get),0);
 TEAM_ORIGINAL.store(saved[0],Ordering::Release);HIT_ORIGINAL.store(saved[1],Ordering::Release);BULLET_ORIGINAL.store(saved[2],Ordering::Release);
 BASE.store(saved_base,Ordering::Release);ACTIVE.store(saved_active,Ordering::Release);VirtualFree(allocation,0,MEM_RELEASE);
}}
