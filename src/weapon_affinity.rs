//! The same effective discovery category used by native weapon rewards.
//! HeroParam supplies a default; the native SpEffect query applies relic overrides.
use crate::{ptr,read_ptr};
pub const QUERY_RVA:usize=0x56acf0;
pub const QUERY_PATTERN:&str="48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 30 49 8B F0 48 8B F9 BB FF FF FF FF";
pub const OVERRIDE_RVA:usize=0x5072d0;
pub const OVERRIDE_PATTERN:&str="48 8B 41 08 48 85 C0 74 21 BA 4A 08 00 00 66 90 48 8B 08 48 85 C9 74 09 66 39 91 56 01 00 00";

pub fn is_caster(category:i32)->bool{matches!(category,57|61|1600|1700)}

pub unsafe fn current(player:usize,base:usize,hero:Option<u8>)->Option<i32>{
 let hero=hero.filter(|h|(1..=10).contains(h))?;
 if base==0||player==0||read_ptr(player+0x190)==0{return None;}
 // Native 56ACF0 writes only discovery at +10 in the 0x20-byte reward context.
 // Keep the full native context footprint; no native references escape this call.
 let mut context=[0u64;4];
 let query:unsafe extern "system" fn(usize,u8,usize)=std::mem::transmute(base+QUERY_RVA);
 query(context.as_mut_ptr()as usize,hero,player);
 let value=ptr::read_unaligned((context.as_ptr()as usize+0x10)as*const i32);
 (value>=0).then_some(value)
}

#[cfg(test)]
#[path="weapon_affinity_native_tests.rs"]mod native_tests;
