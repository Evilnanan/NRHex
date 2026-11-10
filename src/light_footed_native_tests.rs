//! Execute the original type-0 TAE handler and ground check with our live gate.
use super::*;
use crate::win::*;
use std::sync::atomic::{AtomicBool,AtomicI32};
static OWNS:AtomicBool=AtomicBool::new(true);
static BLOCK:AtomicI32=AtomicI32::new(-1);
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from((id==OWNER&&OWNS.load(Ordering::Relaxed))||id==BLOCK.load(Ordering::Relaxed))}
unsafe extern "system" fn virtual_context(_:usize)->usize{0}
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; execute alone"]
fn native_tae_limits_are_scoped_to_mobile_owner_and_fresh_state(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();
 use sha2::Digest;assert_eq!(format!("{:x}",sha2::Sha256::digest(&raw)),crate::EXE_HASH);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();
 let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for s in &headers.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 let expected=TAE_PATTERN.split_whitespace().map(|x|u8::from_str_radix(x,16).unwrap()).collect::<Vec<_>>();
 assert_eq!(std::slice::from_raw_parts((base+TAE_RVA)as*const u8,expected.len()),expected);
 let mut jump=vec![0xff,0x25,0,0,0,0];jump.extend_from_slice(&(has as*const()as usize).to_le_bytes());ptr::copy_nonoverlapping(jump.as_ptr(),(base+crate::HAS_EFFECT_RVA)as*mut u8,jump.len());
 let mut actor=[0usize;0x800/8];let a=actor.as_mut_ptr()as usize;let mut stats=[0u64;0x200/8];let st=stats.as_mut_ptr()as usize;
 let mut action=[0u64;0x300/8];let mut physics=[0u64;0x800/8];let ph=physics.as_mut_ptr()as usize;let mut modules=[0usize;0x200/8];modules[0]=st;modules[1]=action.as_mut_ptr()as usize;modules[0x68/8]=ph;put(a+0x1b8,modules.as_ptr()as usize);
 let mut vt=[0usize;0x610/8];vt[0x600/8]=virtual_context as*const()as usize;put(a,vt.as_ptr()as usize);
 let mut world=vec![0usize;0x17500/8];world[0x174e8/8]=a;put(base+crate::WORLD_RVA,world.as_ptr()as usize);
 let mut graph=[0usize;0xd0/8];let mut limits=[0u64;0x100/8];let lm=limits.as_mut_ptr()as usize;graph[0xc8/8]=lm;put(a+0x60,graph.as_ptr()as usize);
 let mut event=[0usize;8];event[3]=a;let ev=event.as_ptr()as usize;
 let mut data=[0u32;8];let dp=data.as_mut_ptr()as usize;let mut descriptor=[0usize;8];descriptor[1]=dp;let desc=descriptor.as_ptr()as usize;
 let prepared=crate::hook::prepare((base+TAE_RVA)as*mut u8,tae as*const u8,64).unwrap();
 let previous_base=BASE.swap(base,Ordering::AcqRel);let previous_active=ACTIVE.swap(true,Ordering::AcqRel);let previous_original=TAE_ORIGINAL.swap(prepared.trampoline,Ordering::AcqRel);
 put(a+0x190,a);put(ph+8,a);put(ph+0x420,a);int(ph+0x414,-1);int(st+0x140,100);int(st+0x15c,100);ptr::write((ph+0x92)as*mut u8,1);
 let mut cases=0;
 for owns in [false,true]{for upper in [0,1,10,11,12,13,104,105]{for lower in [0,1,2,11]{
  OWNS.store(owns,Ordering::Relaxed);let active=owns&&lower==1&&matches!(upper,11|12|104);
  assert_eq!(query(a,upper,lower,base),i32::from(active));
  for(flag,native_limit)in [(89,3),(90,2),(91,1)]{int(dp,flag);int(lm+0x38,0);tae(ev,desc);assert_eq!(read_i32(lm+0x38),if active&&flag!=89{0}else{native_limit});cases+=1;}
 }}}
 OWNS.store(true,Ordering::Relaxed);
 // Restrictions arriving after the last HKS query must still take effect.
 for blocked in [100020,701530,503520,5520,425,426,42312,4101,4100,46328,46329,100200,crate::reward_cycle_runtime::RESET]{
  assert_eq!(query(a,11,1,base),1);BLOCK.store(blocked,Ordering::Relaxed);
  int(dp,90);int(lm+0x38,0);tae(ev,desc);assert_eq!(read_i32(lm+0x38),2);
  assert_eq!(query(a,11,1,base),0);BLOCK.store(-1,Ordering::Relaxed);cases+=1;
 }
 for mode in 0..9{
  assert_eq!(query(a,12,1,base),1);
  match mode{
   0=>{query(a,-1,-1,base);},1=>{update(a,base);update(a,base);},2=>int(st+0x15c,0),
   3=>ptr::write((ph+0x92)as*mut u8,0),4=>ptr::write((a+0x710)as*mut u8,1),
   5=>int(st+0x140,0),6=>{world[0x174e8/8]=0;},7=>put(a+8,99),8=>OWNS.store(false,Ordering::Relaxed),_=>unreachable!()
  }
  int(dp,91);int(lm+0x38,0);tae(ev,desc);assert_eq!(read_i32(lm+0x38),1,"mode {mode}");cases+=1;
  int(st+0x15c,100);ptr::write((ph+0x92)as*mut u8,1);ptr::write((a+0x710)as*mut u8,0);int(st+0x140,100);world[0x174e8/8]=a;put(a+8,0);OWNS.store(true,Ordering::Relaxed);
 }
 reset();BASE.store(previous_base,Ordering::Release);ACTIVE.store(previous_active,Ordering::Release);TAE_ORIGINAL.store(previous_original,Ordering::Release);
 assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);println!("PASS: {cases} real TAE handler / ground / state cases; not gameplay verification");
}}
