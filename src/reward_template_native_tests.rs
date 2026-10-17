//! The compiled descriptor hook, original dispatch/copy and native item reads.
//! Allocation, grouping/rendering and UID lookup use explicit fixtures.
use crate::*;
use crate::win::*;
use sha2::{Digest,Sha256};
use std::cell::RefCell;
use std::sync::atomic::AtomicUsize;
static IMAGE:AtomicUsize=AtomicUsize::new(0);
thread_local!{
 static NODES:RefCell<Vec<Box<[u64;32]>>>=const{RefCell::new(Vec::new())};
 static FAIL:Cell<i32>=const{Cell::new(-1)};
}
unsafe fn patch(base:usize,rva:usize,bytes:&[u8]){ptr::copy_nonoverlapping(bytes.as_ptr(),(base+rva)as*mut u8,bytes.len());}
unsafe fn jump(base:usize,rva:usize,dest:usize){let mut b=vec![0x48,0xb8];b.extend_from_slice(&dest.to_le_bytes());b.extend_from_slice(&[0xff,0xe0]);patch(base,rva,&b);}
unsafe extern "system" fn create_fixture(_:usize,descriptor:usize)->usize{
 let index=ptr::read((descriptor+0xa9)as*const u8)as i32;
 if FAIL.with(|c|c.get()==index||c.get()==-2){return 0;}
 let mut node=Box::new([0u64;32]);let address=node.as_mut_ptr()as usize;
 let copy:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(IMAGE.load(Ordering::Acquire)+0x570410);
 assert_eq!(copy(address+0x20,descriptor),address+0x20);
 NODES.with(|n|n.borrow_mut().push(node));address
}
unsafe extern "system" fn lookup_fixture(_:usize,uid:u32)->usize{
 NODES.with(|n|n.borrow().iter().map(|n|n.as_ptr()as usize).find(|&p|ptr::read_unaligned((p+0x78)as*const u32)==uid).unwrap_or(0))
}
fn scope(slot:u8)->personal_reward::Scope{
 personal_reward::Scope{manager:42,lot:hex_pool::TEMPLATE_LOT,slot,choices:Some([95003100,95000700,95002600]),..Default::default()}
}
fn descriptor(slot:u8,index:u8)->[u64;24]{unsafe{
 let mut data=[0u64;24];let p=data.as_mut_ptr()as usize;
 ptr::write(p as*mut u8,1);ptr::write_unaligned((p+8)as*mut u32,0x40000000|hex_pool::OWNER_IDS[index.min(2)as usize]as u32);
 // Native source header: network bit4; UID, group and ownership remain native.
 ptr::write_unaligned((p+0x58)as*mut u32,0x2012+index as u32*0x20);
 ptr::write_unaligned((p+0x5c)as*mut u32,0x48001234);
 ptr::write_unaligned((p+0x74)as*mut u32,0x3c000000);
 ptr::write_unaligned((p+0x78)as*mut [f32;3],[12.,34.,56.]);
 ptr::write((p+0xa9)as*mut u8,index);ptr::write_unaligned((p+0xaa)as*mut u16,0x843|((slot as u16)<<7));
 ptr::write_unaligned((p+0xb8)as*mut i32,hex_pool::TEMPLATE_LOT);data
}}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; maps a private copy and must run alone"]
fn compiled_runtime_choices_and_native_dispatch_copy_and_read_execute_together(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").expect("NRHEX_NATIVE_EXE")).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),EXE_HASH);
 let h=pe::Headers::parse(&raw,0).unwrap();let allocation=VirtualAlloc(ptr::null_mut(),h.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!allocation.is_null());let base=allocation as usize;
 for s in &h.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 IMAGE.store(base,Ordering::Release);
 // Original dispatch and 0x6A1BA0 network-bit check execute unchanged. The local
 // creator fixture runs the exact 0x570410 descriptor-to-node copy routine.
 jump(base,0x57d0a0,create_fixture as*const()as usize);jump(base,0x582680,lookup_fixture as*const()as usize);
 let prepared=hook::prepare((base+REWARD_CREATE_RVA)as*mut u8,reward_create_detour as*const u8,64).unwrap();
 let previous_original=REWARD_CREATE_ORIG.swap(prepared.trampoline,Ordering::AcqRel);
 hook::write_code((base+REWARD_CREATE_RVA)as*mut u8,&prepared.patch).unwrap();
 let dispatch:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(base+REWARD_CREATE_RVA);
 let read_item:unsafe extern "system" fn(usize,usize,u32)->u8=std::mem::transmute(base+0x570fc0);
 let previous_scope=PERSONAL_REWARD.with(|c|c.replace(personal_reward::Scope::default()));
 for local in 1..=3{
  NODES.with(|n|n.borrow_mut().clear());PERSONAL_REWARD.with(|c|c.set(scope(local)));
  for owner in 1..=3{for index in [2,0,1]{
   let mut data=descriptor(owner,index);let before=data;let p=data.as_mut_ptr()as usize;let node=dispatch(42,p);assert_eq!(data,before);
   if owner!=local{assert_eq!(node,0);continue;}assert_ne!(node,0);
   let mut expected=before;let e=expected.as_mut_ptr()as usize;
   ptr::write_unaligned((e+8)as*mut u32,0x40000000|scope(local).choices.unwrap()[index as usize]as u32);
   ptr::write_unaligned((e+0x58)as*mut u32,0x2002+index as u32*0x20);
   assert_eq!(ptr::read_unaligned((node+0x20)as*const [u64;24]),expected); // UID/group/position/owner/quality unchanged
   let mut out=[0u64;12];let uid=ptr::read_unaligned((node+0x78)as*const u32);
   assert_eq!(read_item(42,out.as_mut_ptr()as usize,uid),1);
   assert_eq!(ptr::read_unaligned(out.as_ptr()as*const u32),0x40000000|scope(local).choices.unwrap()[index as usize]as u32);
   // The same callback cannot submit a second node for an already attempted slot.
   assert_eq!(dispatch(42,p),0);assert_eq!(data,before);
  }}
  let saved=PERSONAL_REWARD.with(Cell::get);assert_eq!((saved.created,saved.attempted,saved.created_mask),(3,7,7));
  assert_eq!(NODES.with(|n|n.borrow().len()),3);
 }
 // Wrong managers/lots, a native weapon, or an absent scope retain item data.
 for kind in 0..4{
  PERSONAL_REWARD.with(|c|c.set(if kind==3{personal_reward::Scope::default()}else{scope(1)}));
  let mut data=descriptor(1,0);let p=data.as_mut_ptr()as usize;ptr::write_unaligned((p+0x58)as*mut u32,0x2002);
  if kind==1{ptr::write_unaligned((p+0xb8)as*mut i32,6100002);}if kind==2{ptr::write_unaligned((p+0x58)as*mut u32,0x2001);ptr::write_unaligned((p+8)as*mut u32,100);}
  let before=data;let node=dispatch(if kind==0{43}else{42},p);assert_ne!(node,0);assert_eq!(data,before);assert_eq!(ptr::read_unaligned((node+0x28)as*const u32),ptr::read_unaligned((p+8)as*const u32));
  assert_eq!(PERSONAL_REWARD.with(Cell::get).created,0);
 }
 // Invalid ordinals / unexpected prototype categories create no node.
 for index in [3,255]{PERSONAL_REWARD.with(|c|c.set(scope(1)));let mut d=descriptor(1,index);let before=d;assert_eq!(dispatch(42,d.as_mut_ptr()as usize),0);assert_eq!(d,before);}
 PERSONAL_REWARD.with(|c|c.set(scope(1)));let mut d=descriptor(1,0);ptr::write_unaligned((d.as_mut_ptr()as usize+8)as*mut u32,100);assert_eq!(dispatch(42,d.as_mut_ptr()as usize),0);
 // Failure marks only real successful creations; a repeated failed callback
 // cannot duplicate/re-enter. A fresh zero-creation attempt can be retried.
 for failure in [1,-2]{
  FAIL.with(|c|c.set(failure));PERSONAL_REWARD.with(|c|c.set(scope(1)));
  for index in 0..3{let mut d=descriptor(1,index);let before=d;let result=dispatch(42,d.as_mut_ptr()as usize);assert_eq!(result==0,failure==-2||index==failure as u8);assert_eq!(d,before);}
  let state=PERSONAL_REWARD.with(Cell::get);assert_eq!(state.attempted,7);assert_eq!(state.created,if failure==-2{0}else{2});assert_eq!(state.created_mask,if failure==-2{0}else{5});
 }
 FAIL.with(|c|c.set(-1));PERSONAL_REWARD.with(|c|c.set(scope(1)));let mut d=descriptor(1,0);assert_ne!(dispatch(42,d.as_mut_ptr()as usize),0);
 PERSONAL_REWARD.with(|c|c.set(previous_scope));REWARD_CREATE_ORIG.store(previous_original,Ordering::Release);IMAGE.store(0,Ordering::Release);NODES.with(|n|n.borrow_mut().clear());VirtualFree(allocation,0,MEM_RELEASE);
}}
