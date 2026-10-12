//! Narrow exception to the wall-jump animation's 0.9 second input mask.
//! Keep the first takeoff: original TAE marker 5031 starts at 0.366 seconds.
pub fn needs_raw_jump(original:u8,mode:i32,action:i32,raw:u64,local_owner:bool,alive:bool,airborne:bool,contact:bool,takeoff_complete:bool)->bool {
 original==0&&mode==1&&action==6&&raw&64!=0&&local_owner&&alive&&airborne&&contact&&takeoff_complete
}
#[cfg(test)] mod tests {
 use super::*;
 #[test] fn only_blocked_owner_jump_after_takeoff_can_use_raw_request(){
  assert!(needs_raw_jump(0,1,6,64,true,true,true,true,true));
  for original in [1,255]{assert!(!needs_raw_jump(original,1,6,64,true,true,true,true,true));}
  for mode in [0,2]{assert!(!needs_raw_jump(0,mode,6,64,true,true,true,true,true));}
  for action in [0,2,7,17]{assert!(!needs_raw_jump(0,1,action,64,true,true,true,true,true));}
  for raw in [0,1,32,128]{assert!(!needs_raw_jump(0,1,6,raw,true,true,true,true,true));}
  for absent in 0..5 {
   let mut gates=[true;5];gates[absent]=false;
   assert!(!needs_raw_jump(0,1,6,64,gates[0],gates[1],gates[2],gates[3],gates[4]));
  }
 }
}
