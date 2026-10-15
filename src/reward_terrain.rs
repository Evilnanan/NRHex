//! Read the native current-terrain flag when choosing rewards, on either peer.
//! No player SpEffect or cached terrain survives between lottery requests.
use crate::{read_i32,read_ptr};
pub const MANAGER_RVA:usize=0x3c115a8;
pub const GET_RVA:usize=0x60ce40;
pub const GET_PATTERN:&str="44 8B 41 1C 44 8B DA 33 D2 41 8B C3 41 F7 F0 4C 8B D1 45 33 C9";
pub const GREAT_HOLLOW:u32=7604;

pub unsafe fn great_hollow(base:usize)->Option<bool>{
 let manager=read_ptr(base+MANAGER_RVA);
 // The native reader divides by group size and traverses the registered tree.
 // During loading, leave Cage unavailable rather than calling an unready tree.
 if manager==0||read_i32(manager+0x1c)!=1000||read_ptr(manager+0x38)==0{return None;}
 let get:unsafe extern "system" fn(usize,u32)->u32=std::mem::transmute(base+GET_RVA);
 Some(get(manager,GREAT_HOLLOW)!=0)
}

pub fn exclude_cage(terrain:Option<bool>)->bool{terrain.unwrap_or(true)}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn only_confirmed_non_hollow_terrain_allows_cage(){
  use crate::{hex_pool::{OWNER_IDS,ORDERS},reward_choices};
  let cage=OWNER_IDS.iter().position(|&id|id==95001600).unwrap();
  for terrain in [None,Some(true),Some(false)]{
   let excluded=reward_choices::terrain_exclusions(0,exclude_cage(terrain));
   assert_eq!(reward_choices::candidates(excluded).iter().any(|&i|ORDERS[i].contains(&cage)),terrain==Some(false));
   assert_eq!(ORDERS[reward_choices::preferred([cage,0,1],excluded).unwrap()].contains(&cage),terrain==Some(false));
  }
 }
 #[test]fn draw_reads_live_native_flag_without_player_marker_or_role_dependency(){unsafe{
  use crate::win::*;use core::ptr;use std::cell::Cell;
  thread_local!{static FLAG:Cell<u32>=const{Cell::new(0)};static CALLS:Cell<usize>=const{Cell::new(0)};}
  unsafe extern "system" fn get(_:usize,flag:u32)->u32{assert_eq!(flag,GREAT_HOLLOW);CALLS.with(|c|c.set(c.get()+1));FLAG.with(Cell::get)}
  let image=VirtualAlloc(ptr::null_mut(),0x3d00000,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!image.is_null());let base=image as usize;
  let mut jump=[0u8;12];jump[..2].copy_from_slice(&[0x48,0xb8]);jump[2..10].copy_from_slice(&(get as *const () as usize).to_le_bytes());jump[10..].copy_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(jump.as_ptr(),(base+GET_RVA)as *mut u8,12);
  let mut manager=[0usize;0x40/8];let m=manager.as_mut_ptr()as usize;
  assert_eq!(great_hollow(base),None);
  ptr::write_unaligned((base+MANAGER_RVA)as *mut usize,m);assert_eq!(great_hollow(base),None);
  ptr::write_unaligned((m+0x1c)as *mut i32,1000);assert_eq!(great_hollow(base),None);manager[0x38/8]=m;
  for flag in [0,1,1,0,1,0]{FLAG.with(|c|c.set(flag));assert_eq!(great_hollow(base),Some(flag!=0));}
  assert_eq!(CALLS.with(Cell::get),6);assert_eq!(VirtualFree(image,0,MEM_RELEASE),1);
 }}
}
