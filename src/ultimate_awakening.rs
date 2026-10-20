//! Native ten-second Cerulean Hidden Tear clone; native category20 refreshes
//! only this ID. No custom timer, FP writes, or modifications to the item buff.
use crate::{has_effect,is_local,native_actor};
pub const OWNER:i32=95003800;
pub const BUFF:i32=OWNER+1;
pub unsafe fn activated(player:usize,base:usize){
 if player==0||!is_local(player,base)||!has_effect(player,OWNER,base)||has_effect(player,crate::reward_cycle_runtime::RESET,base){return;}
 let stats=native_actor::stats(player);
 if stats==0||native_actor::incapacitated(player,stats){return;}
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);
 apply(player,BUFF,0);
}
#[cfg(test)]#[path="ultimate_awakening_native_tests.rs"]mod native_tests;
