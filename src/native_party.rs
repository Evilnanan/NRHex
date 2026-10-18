//! Original difficulty headcount, including the native retained boss count.
//! Query on demand; this is not a saved expedition mode or a living-player count.
use crate::read_ptr;
pub const COUNT_RVA:usize=0x519660;
pub const COUNT_PATTERN:&str="48 83 EC 28 48 83 3D CC 5C 6F 03 00 75 27";
const MANAGERS:[usize;3]=[0x3c0f338,0x3c21ec8,0x3c13258];

pub unsafe fn difficulty_count(base:usize)->Option<i32>{
 // The original getter asserts on missing managers, then reads the session
 // roster and retained count. Defer during loading rather than entering it.
 if base==0||MANAGERS.iter().any(|&rva|read_ptr(base+rva)==0){return None;}
 let count:unsafe extern "system" fn()->i32=std::mem::transmute(base+COUNT_RVA);
 Some(count())
}

#[cfg(test)]mod tests{
 use super::*;
 use crate::win::*;
 use std::{cell::Cell,ptr};
 thread_local!{static COUNT:Cell<i32>=const{Cell::new(1)};static CALLS:Cell<usize>=const{Cell::new(0)};}
 unsafe extern "system" fn count()->i32{CALLS.with(|c|c.set(c.get()+1));COUNT.with(Cell::get)}
 #[test]fn queries_live_native_count_only_after_all_managers_are_ready(){unsafe{
  assert_eq!(difficulty_count(0),None);
  let image=VirtualAlloc(ptr::null_mut(),0x3d00000,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!image.is_null());let base=image as usize;
  let mut jump=[0u8;12];jump[..2].copy_from_slice(&[0x48,0xb8]);jump[2..10].copy_from_slice(&(count as *const () as usize).to_le_bytes());jump[10..].copy_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(jump.as_ptr(),(base+COUNT_RVA)as *mut u8,12);
  CALLS.with(|c|c.set(0));
  for rva in MANAGERS{assert_eq!(difficulty_count(base),None);ptr::write_unaligned((base+rva)as *mut usize,base);}
  assert_eq!(CALLS.with(Cell::get),0);
  for value in [1,3,2,1]{COUNT.with(|c|c.set(value));assert_eq!(difficulty_count(base),Some(value));}
  assert_eq!(CALLS.with(Cell::get),4);
  ptr::write_unaligned((base+MANAGERS[1])as *mut usize,0);assert_eq!(difficulty_count(base),None);assert_eq!(CALLS.with(Cell::get),4);
  assert_eq!(VirtualFree(image,0,MEM_RELEASE),1);
 }}
}
