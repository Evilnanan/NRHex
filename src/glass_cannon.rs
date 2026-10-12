//! Source-owned defense bypass inside the native per-recipient calculator.
use crate::{has_effect, read_ptr, ACTIVE, BASE, ptr};
use std::cell::RefCell;
use std::sync::atomic::{AtomicPtr, Ordering};

pub const OWNER:i32=95000300;
pub const RVA:usize=0x6c5fc0;
pub const PATTERN:&str="48 8B C4 48 89 58 08 57 48 81 EC 80 00 00 00 4C 8B 9C 24 B0 00 00 00 49 8B D8 4C 8B 94 24 B8 00 00 00";
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub const OBJECT_RVA:usize=0x506700;
pub const OBJECT_PATTERN:&str="48 8B 41 08 F3 0F 10 15 ? ? ? ? 0F 28 CA 48 85 C0 74 2B F7 40 68 03 00 0C 80 75 19 48 8B 08 48 85 C9 74 0A F3 0F 10 81 18 03 00 00";
pub static OBJECT_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
thread_local!{static SCOPES:RefCell<Vec<(bool,usize)>>=const{RefCell::new(Vec::new())};}

pub unsafe extern "system" fn enter(module:usize,source:usize,hit:usize){
 // 43D4BE supplies a resolved ChrIns or zero, including bullet owners.
 // Never interpret a generic hit source/bullet as a character.
 let active=ACTIVE.load(Ordering::Acquire);
 let target=if active&&module!=0&&source!=0&&hit!=0{read_ptr(module+8)}else{0};
 let bypass=eligible(active,source,target,hit,
  |actor|has_effect(actor,OWNER,BASE.load(Ordering::Acquire)));
 SCOPES.with(|s|s.borrow_mut().push((bypass,if bypass{read_ptr(target+0x190)}else{0})));
}
fn eligible(active:bool,source:usize,target:usize,hit:usize,owns:impl FnOnce(usize)->bool)->bool{
 active&&source!=0&&target!=0&&source!=target&&hit!=0&&owns(source)
}
#[cfg(test)]fn push(enabled:bool){SCOPES.with(|s|s.borrow_mut().push((enabled,0)));}
pub unsafe extern "system" fn leave(){SCOPES.with(|s|{s.borrow_mut().pop();});}
fn enabled()->bool{SCOPES.with(|s|s.borrow().last().map(|v|v.0).unwrap_or(false))}
pub unsafe extern "system" fn object_detour(manager:usize)->f32{
 let original:unsafe extern "system" fn(usize)->f32=std::mem::transmute(OBJECT_ORIGINAL.load(Ordering::Acquire));
 let value=original(manager);
 // 439E80's object-defense branch applies this target coefficient to the
 // base components before the combiner. Restrict to this recipient manager.
 if SCOPES.with(|s|s.borrow().last().is_some_and(|v|v.0&&v.1!=0&&v.1==manager)){1.}else{value}
}

// Verified at all four native call sites: eight pointers, seven stack floats;
// return is XMM0 (float), despite the decompiler's integer return type.
type Calculate=unsafe extern "system" fn(*mut f32,*const f32,*const f32,*const f32,*const f32,
 *const f32,*const f32,*const f32,f32,f32,f32,f32,f32,f32,f32)->f32;
pub unsafe extern "system" fn detour(out:*mut f32,attack:*const f32,rates:*const f32,effects:*const f32,
 armor:*const f32,guard:*const f32,conditional:*const f32,additional:*const f32,
 deflection:f32,repeated:f32,rebound:f32,attack_rate:f32,positional:f32,target_state:f32,balance:f32)->f32{
 let original:Calculate=std::mem::transmute(ORIGINAL.load(Ordering::Acquire));
 calculate(original,enabled(),out,attack,rates,effects,armor,guard,conditional,additional,
  deflection,repeated,rebound,attack_rate,positional,target_state,balance)
}
unsafe fn calculate(original:Calculate,bypass:bool,out:*mut f32,attack:*const f32,rates:*const f32,
 effects:*const f32,armor:*const f32,guard:*const f32,conditional:*const f32,additional:*const f32,
 deflection:f32,repeated:f32,rebound:f32,attack_rate:f32,positional:f32,target_state:f32,balance:f32)->f32{
 if !bypass{return original(out,attack,rates,effects,armor,guard,conditional,additional,
  deflection,repeated,rebound,attack_rate,positional,target_state,balance);}
 // Neutralize all five recipient coefficients for all five damage elements.
 // Keep attack components/rates, hit-specific offensive multipliers and the
 // native balance factor. No shared rows, recipient stats or HP are modified.
 let neutral=[1f32;5];let n=neutral.as_ptr();
 original(out,attack,rates,n,n,n,n,n,1.,repeated,1.,attack_rate,positional,1.,balance)
}

#[cfg(test)]mod tests{
 use super::*;
 unsafe extern "system" fn native(out:*mut f32,a:*const f32,r:*const f32,e:*const f32,
  armor:*const f32,g:*const f32,c:*const f32,x:*const f32,d:f32,repeated:f32,b:f32,
  rate:f32,pos:f32,state:f32,balance:f32)->f32{
  let scalar=d*repeated*b*rate*pos*state*balance;
  let mut sum=0.;for i in 0..5{let value=*a.add(i)* *r.add(i)* *e.add(i)* *armor.add(i)* *g.add(i)* *c.add(i)* *x.add(i)*scalar;*out.add(i)=value;sum+=value;}sum
 }
 #[test]fn bypass_all_defensive_layers_preserves_offense_and_original_inputs(){unsafe{
  let a=[100.,200.,300.,400.,500.];let rates=[1.2,1.5,2.,0.5,0.];
  for cuts in [[0.;5],[0.1,0.2,0.3,0.4,0.5],[2.;5]]{
   let saved=cuts;let mut out=[0.;5];
   let value=calculate(native,true,out.as_mut_ptr(),a.as_ptr(),rates.as_ptr(),cuts.as_ptr(),cuts.as_ptr(),cuts.as_ptr(),cuts.as_ptr(),cuts.as_ptr(),0.25,0.5,0.,2.,1.5,0.,0.75);
   let mut expected=[0.;5];for i in 0..5{expected[i]=a[i]*rates[i]*0.5*2.*1.5*0.75;}
   assert_eq!(out,expected);assert_eq!(value,expected.iter().sum());assert_eq!(cuts,saved);
  }
 }}
 #[test]fn non_holder_keeps_all_fifteen_arguments_and_float_return(){unsafe{
  let mut out=[0.;5];let a=[100.;5];let one=[1.;5];let cut=[0.5;5];
  let got=calculate(native,false,out.as_mut_ptr(),a.as_ptr(),one.as_ptr(),cut.as_ptr(),one.as_ptr(),one.as_ptr(),one.as_ptr(),one.as_ptr(),0.25,2.,0.5,1.5,2.,0.5,0.75);
  assert_eq!(got,70.3125);assert_eq!(out,[14.0625;5]);
 }}
 #[test]fn nested_recipients_cannot_inherit_the_outer_bypass(){unsafe{
  assert!(!enabled());push(true);assert!(enabled());push(false);assert!(!enabled());
  leave();assert!(enabled());leave();assert!(!enabled());
 }}
 #[test]fn eligibility_queries_only_resolved_attacker_and_never_grants_defender_penetration(){
  assert!(eligible(true,12,34,56,|actor|{assert_eq!(actor,12);true}));
  assert!(!eligible(true,12,34,56,|actor|actor==34));
  for (active,source,target,hit) in [(false,12,34,56),(true,0,34,56),(true,12,0,56),(true,12,12,56),(true,12,34,0)]{
   assert!(!eligible(active,source,target,hit,|_|panic!("invalid source queried")));
  }
 }
 #[test]fn pre_combiner_defense_query_changes_only_current_recipient_and_restores_nested_scope(){unsafe{
  unsafe extern "system" fn fixture(_:usize)->f32{0.2}
  let old=OBJECT_ORIGINAL.swap(fixture as *const () as *mut u8,Ordering::AcqRel);
  SCOPES.with(|s|s.borrow_mut().push((true,123)));
  assert_eq!(object_detour(123),1.);assert_eq!(object_detour(456),0.2);
  push(false);assert_eq!(object_detour(123),0.2);leave();assert_eq!(object_detour(123),1.);
  leave();assert_eq!(object_detour(123),0.2);OBJECT_ORIGINAL.store(old,Ordering::Release);
 }}
 #[test]fn actual_detour_forwards_all_pointer_and_stack_arguments(){unsafe{
  unsafe extern "system" fn fixture(out:*mut f32,a:*const f32,r:*const f32,e:*const f32,
   armor:*const f32,g:*const f32,c:*const f32,x:*const f32,d:f32,repeated:f32,b:f32,
   rate:f32,pos:f32,state:f32,balance:f32)->f32{
   assert_eq!([out as usize,a as usize,r as usize,e as usize,armor as usize,g as usize,c as usize,x as usize],[11,22,33,44,55,66,77,88]);
   assert_eq!([d,repeated,b,rate,pos,state,balance],[1.25,2.5,3.75,4.,5.25,6.5,7.75]);
   1234.5
  }
  let old=ORIGINAL.swap(fixture as *const () as *mut u8,Ordering::AcqRel);push(false);
  assert_eq!(detour(11 as *mut f32,22 as *const f32,33 as *const f32,44 as *const f32,
   55 as *const f32,66 as *const f32,77 as *const f32,88 as *const f32,1.25,2.5,3.75,4.,5.25,6.5,7.75),1234.5);
  leave();ORIGINAL.store(old,Ordering::Release);
 }}
}
