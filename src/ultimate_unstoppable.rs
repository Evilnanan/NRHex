//! Fifteen seconds of native Endure reactions/VFX without its damage reduction.
//! Category1001 is required by native reaction selection and cannot use the
//! category20 refresh path. Replace only our private effect on activation.
use crate::{has_effect,is_local,native_actor,read_ptr};
pub const OWNER:i32=95003900;
pub const BUFF:i32=OWNER+1;
pub unsafe fn activated(player:usize,base:usize){
 if player==0||!is_local(player,base)||!has_effect(player,OWNER,base)||has_effect(player,crate::reward_cycle_runtime::RESET,base){return;}
 let stats=native_actor::stats(player);
 if stats==0||native_actor::incapacitated(player,stats){return;}
 // Expired nodes can stop qualifying for has_effect before native cleanup.
 // Remove by ID even then; native removal returns 0 only when none exists.
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);
 remove(read_ptr(player+0x190),BUFF);
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);
 apply(player,BUFF,0);
}
#[cfg(test)]#[path="ultimate_unstoppable_native_tests.rs"]mod native_tests;
