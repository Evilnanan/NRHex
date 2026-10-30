//! Fifteen seconds of native Endure reactions/VFX without its damage reduction.
//! Category1001 is required by native reaction selection and cannot use the
//! category20 refresh path. Replace only our private effect on activation.
use crate::{has_effect,is_local,native_actor,read_ptr};
pub const OWNER:i32=95003900;
pub const BUFF:i32=OWNER+1;
// Native full-rest meter reset (ChrIns 3D89B0): bypasses buildup immunity,
// preserving proc counts, active statuses and resistance thresholds.
pub const CLEAR_RVA:usize=0x42dc40;
pub const CLEAR_PATTERN:&str="8B 51 2C 45 33 C0 4C 8B C9 85 D2 41 8B C0 0F 49 C2 89 41 10 41 8B C0 8B 51 30 85 D2 0F 49 C2 89 41";
unsafe fn clear_buildup(player:usize,base:usize){
 let modules=read_ptr(player+0x1b8);if modules==0{return;}
 let resist=read_ptr(modules+0x20);
 if resist==0||read_ptr(resist)!=base+0x2c113c8||read_ptr(resist+8)!=player{return;}
 let clear:unsafe extern "system" fn(usize)=std::mem::transmute(base+CLEAR_RVA);
 clear(resist);
}
pub unsafe fn activated(player:usize,base:usize){
 if player==0||!is_local(player,base)||!has_effect(player,OWNER,base)||has_effect(player,crate::reward_cycle_runtime::RESET,base){return;}
 let stats=native_actor::stats(player);
 if stats==0||native_actor::incapacitated(player,stats){return;}
 clear_buildup(player,base);
 // Expired nodes can stop qualifying for has_effect before native cleanup.
 // Remove by ID even then; native removal returns 0 only when none exists.
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);
 remove(read_ptr(player+0x190),BUFF);
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);
 apply(player,BUFF,0);
}
#[cfg(test)]#[path="ultimate_unstoppable_native_tests.rs"]mod native_tests;
