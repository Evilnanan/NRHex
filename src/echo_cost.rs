//! Echo's surcharge is computed from the native cached fee at payment time.
//! No second cost is attached to replay bullets, and the cache stays unchanged.
use crate::{has_effect,read_i32,read_ptr};
pub const RVA:usize=0xe926c0;
pub const PATTERN:&str="48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 57 48 83 EC 30 8B FA 48 8B F1 33 DB 85 D2 0F 88 27 01 00 00 48 8B 0D 96 61 D9 02 48 85 C9 75 2E 48 8D 0D AE 1F D7 02 E8 85 0C 2C 01 4C 8B C8 4C 8D";
pub const GETTER_RVA:usize=0xe92250;
pub const GETTER_PATTERN:&str="48 8B 41 08 48 85 C0 74 13 84 D2 75 05 0F BF 40 12 C3 80 FA 01 75 05 0F";
fn adjusted(cost:i32,kind:u8,owned:bool)->i32{
 if owned&&matches!(kind,0|1)&&cost>0{((i64::from(cost)*17+5)/10).min(i64::from(i32::MAX)) as i32}else{cost}
}
pub unsafe fn extra(context:usize,cost:i32,base:usize)->i32{
 let magic=read_i32(context+0xc);
 if cost<=0||magic<0||!has_effect(read_ptr(context),crate::echo_cast::OWNER,base){return 0;}
 let mut reference=[u64::from(u32::MAX),0];
 let lookup:unsafe extern "system" fn(*mut u64,i32)->usize=std::mem::transmute(base+RVA);
 lookup(reference.as_mut_ptr(),magic);
 let row=reference[1]as usize;if row==0{return 0;}
 adjusted(cost,std::ptr::read((row+0x26)as*const u8),true)-cost
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn surcharge_keeps_the_previous_total_rounding_and_saturation(){
  for cost in 0..10000{assert_eq!(adjusted(cost,0,true),((f64::from(cost)*1.7).round())as i32);}
  assert_eq!(adjusted(35,0,true)-35,25);assert_eq!(adjusted(46,1,true)-46,32);
  for kind in [2,3,255]{assert_eq!(adjusted(35,kind,true),35);}
  for cost in [0,-1,i32::MIN,i32::MAX]{assert_eq!(adjusted(cost,0,true),cost);}
  assert_eq!(adjusted(35,0,false),35);
 }
}
