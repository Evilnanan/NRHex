//! Keep Restage and Executor deflect audio in the native scene-owned bank set.
use crate::{ptr,ACTIVE};
use std::sync::atomic::{AtomicPtr,Ordering};

pub const RVA:usize=0xeed260;
pub const PATTERN:&str="48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 57 48 83 EC 50 0F B6 FA 84 D2 0F 84 08 01 00 00 4C 8B 09";
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
type Insert=unsafe extern "system" fn(usize,u8)->usize;
pub unsafe extern "system" fn detour(set:usize,hero:u8)->usize{
 let original:Insert=std::mem::transmute(ORIGINAL.load(Ordering::Acquire));
 insert(original,ACTIVE.load(Ordering::Acquire),set,hero)
}
unsafe fn insert(original:Insert,active:bool,set:usize,hero:u8)->usize{
 let result=original(set,hero);
 // EED260 has four native call sites, all in EF8B20's hero audio selection.
 // Reuse its unique-set insertion; the scene owns loading, readiness and release.
 // Do this during scene setup, before Escape Plan or Parry can be activated.
 if active&&set!=0&&(1..=10).contains(&hero){for required in [4,8]{if hero!=required{original(set,required);}}}
 result
}

#[cfg(test)]mod tests{
 use super::*;
 use std::collections::BTreeSet;
 struct Fixture{heroes:BTreeSet<u8>,calls:Vec<u8>}
 unsafe extern "system" fn native(set:usize,hero:u8)->usize{
  let f=&mut *(set as *mut Fixture);f.calls.push(hero);
  if hero!=0&&f.heroes.insert(hero){0x123456789abcdef1}else{0}
 }
 #[test]fn solo_and_team_bank_sets_include_duchess_and_executor_once_preserving_return(){unsafe{
  let mut f=Fixture{heroes:BTreeSet::new(),calls:vec![]};let set=&mut f as *mut _ as usize;
  assert_eq!(insert(native,true,set,1),0x123456789abcdef1);
  assert_eq!(f.heroes,BTreeSet::from([1,4,8]));assert_eq!(f.calls,[1,4,8]);
  assert_eq!(insert(native,true,set,1),0);assert_eq!(insert(native,true,set,6),0x123456789abcdef1);
  assert_eq!(insert(native,true,set,4),0);assert_eq!(f.heroes,BTreeSet::from([1,4,6,8]));
  assert_eq!(insert(native,true,set,8),0);assert_eq!(&f.calls[f.calls.len()-2..],&[8,4]);
 }}
 #[test]fn disabled_empty_and_invalid_requests_preserve_original_behavior(){unsafe{
  for (active,hero) in [(false,1),(true,0),(true,255)]{
   let mut f=Fixture{heroes:BTreeSet::new(),calls:vec![]};let set=&mut f as *mut _ as usize;
   insert(native,active,set,hero);assert_eq!(f.calls,[hero]);
  }
 }}
}
