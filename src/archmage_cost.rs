//! Gate native spell FP costs by the actual catalyst slot selected by act(2026).
use crate::{ptr,read_ptr,read_i32,ACTIVE,BASE,archmage_runtime};
use std::sync::{Mutex,atomic::{AtomicPtr,Ordering}};
pub const SELECT_RVA:usize=0x66ab90;
pub const SELECT_PATTERN:&str="48 89 5C 24 10 55 56 57 41 56 41 57 48 81 EC 80 00 00 00 41 0F B6 F9 41 8B E8 44 8B FA";
pub const COST_RVA:usize=0x6c5ec0;
pub const COST_PATTERN:&str="48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 20 48 8B F2 48 8B F9 41 0F B6 D0 E8 ? ? ? ? 8B D8";
pub static SELECT_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static COST_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
#[derive(Clone,Copy,Default)]
struct Selection {player:usize,module:usize,context:usize,spell:i32,secondary:bool}
impl Selection {
 fn matches(self,player:usize,module:usize,context:usize,spell:i32)->bool{
  self.secondary&&player!=0&&module!=0&&self.player==player&&self.module==module&&self.context==context&&self.spell>=0&&self.spell==spell
 }
}
static SELECTED:Mutex<Selection>=Mutex::new(Selection{player:0,module:0,context:0,spell:-1,secondary:false});
pub fn clear(){if let Ok(mut s)=SELECTED.lock(){*s=Selection::default();}}
fn secondary(hand:u32,slot:u32,skill:u8)->bool{hand<=1&&slot==1&&skill==0}
// Cast eligibility, TAE spell debit, and sustained-bullet debit. UI queries keep
// their native result; they do not describe the currently selected cast slot.
fn gameplay_cost(caller:usize)->bool{matches!(caller,0x6d15b6|0x38499e|0x6c5eab)}
type Select=unsafe extern "system" fn(usize,u32,u32,u8);
type Cost=unsafe extern "system" fn(usize,usize,u8)->u64;
pub unsafe extern "system" fn select(player:usize,hand:u32,slot:u32,skill:u8){
 let original:Select=std::mem::transmute(SELECT_ORIGINAL.load(Ordering::Acquire));
 let base=BASE.load(Ordering::Acquire);
 let local=ACTIVE.load(Ordering::Acquire)&&crate::is_local(player,base);
 // A reentrant check during selection must never inherit the previous slot.
 if local{clear();}
 original(player,hand,slot,skill);
 if !local{return;}
 let modules=read_ptr(player+0x1b8);
 let module=if modules==0{0}else{read_ptr(modules+0x60)};
 let s=Selection{player,module,context:read_ptr(base+0x3c131c8),spell:crate::active_magic(player),secondary:secondary(hand,slot,skill)};
 if let Ok(mut current)=SELECTED.lock(){*current=s;}
}
#[unsafe(naked)]
pub unsafe extern "system" fn wrapper(_reference:usize,_effects:usize,_charge:u8)->u64{
 core::arch::naked_asm!("mov r9, [rsp]","jmp {handler}",handler=sym cost);
}
unsafe extern "system" fn cost(reference:usize,effects:usize,charge:u8,caller:usize)->u64{
 let original:Cost=std::mem::transmute(COST_ORIGINAL.load(Ordering::Acquire));
 let result=original(reference,effects,charge);
 if !ACTIVE.load(Ordering::Acquire)||reference==0||effects==0{return result;}
 let base=BASE.load(Ordering::Acquire);
 if !gameplay_cost(caller.wrapping_sub(base)){return result;}
 let Some(player)=archmage_runtime::local_owner(base) else{return result;};
 if read_ptr(player+0x190)!=effects{return result;}
 let s=match SELECTED.lock(){Ok(s)=>*s,Err(_)=>return result};
 if !s.secondary||s.player!=player{return result;}
 let modules=read_ptr(player+0x1b8);if modules==0{return result;}
 let stats=read_ptr(modules);let module=read_ptr(modules+0x60);
 if stats==0||module==0||module!=s.module||read_i32(stats+0x140)<=0{return result;}
 if !s.matches(player,module,read_ptr(base+0x3c131c8),read_i32(reference))||crate::active_magic(player)!=s.spell{return result;}
 0
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn only_secondary_catalyst_requests_are_free(){
  for hand in [0,1,2,u32::MAX]{for slot in [0,1,2,u32::MAX]{for skill in [0,1,255]{
   assert_eq!(secondary(hand,slot,skill),hand<=1&&slot==1&&skill==0);
  }}}
  for caller in [0x6d15b6,0x38499e,0x6c5eab]{assert!(gameplay_cost(caller));}
  for caller in [0,0x7ccc1b,0x8d5fea,0x384999]{assert!(!gameplay_cost(caller));}
 }
 #[test]fn switching_to_primary_with_same_spell_removes_the_discount(){
  let secondary=Selection{player:1,module:2,context:3,spell:4000,secondary:true};
  assert!(secondary.matches(1,2,3,4000));
  assert!(!Selection{secondary:false,..secondary}.matches(1,2,3,4000));
  for (p,m,c,id) in [(9,2,3,4000),(1,9,3,4000),(1,2,9,4000),(1,2,3,4001),(0,2,3,4000),(1,0,3,4000)]{assert!(!secondary.matches(p,m,c,id));}
  assert!(!Selection::default().matches(1,2,3,4000));
 }
 #[test]fn wrapper_preserves_arguments_and_full_native_return(){unsafe{
  unsafe extern "system" fn native(reference:usize,effects:usize,charge:u8)->u64{
   assert_eq!(reference,0x123456789abcdef0);assert_eq!(effects,0x23456789abcdef01);assert_eq!(charge,255);0xfedcba9876543210
  }
  COST_ORIGINAL.store(native as *const () as *mut u8,Ordering::Release);
  assert_eq!(wrapper(0x123456789abcdef0,0x23456789abcdef01,255),0xfedcba9876543210);
 }}
}
