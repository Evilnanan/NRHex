//! Actor-owned one-shot recovery: multiple recipients, delayed/reentrant delivery,
//! independent actors, failed native submission and eligibility boundaries.
use super::*;
#[derive(Default)]struct Effects{ids:Vec<i32>,remove_blocked:bool}
unsafe extern "system" fn has(m:usize,id:i32)->u8{u8::from((*(m as *const Effects)).ids.contains(&id))}
unsafe extern "system" fn add(p:usize,id:i32,_:u8)->u8{let e=&mut *(read_ptr(p+0x190) as *mut Effects);if !e.ids.contains(&id){e.ids.push(id);}1}
unsafe extern "system" fn remove(m:usize,id:i32)->u8{
 let e=&mut *(m as *mut Effects);if e.remove_blocked{return 0;}let before=e.ids.len();e.ids.retain(|i|*i!=id);u8::from(e.ids.len()!=before)
}
#[test]fn recovery_is_once_per_opportunity_even_after_pulse_expiry_and_reentry(){unsafe{
 let base=crate::win::VirtualAlloc(ptr::null_mut(),0xeb0000,0x3000,0x40) as usize;assert_ne!(base,0);
 for(rva,address)in[(crate::HAS_EFFECT_RVA,has as *const () as usize),(crate::APPLY_SELF_RVA,add as *const () as usize),(crate::REMOVE_EFFECT_RVA,remove as *const () as usize)]{
  let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&address.to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(jump.as_ptr(),(base+rva) as *mut u8,jump.len());
 }
 TEST_CONTEXT.with(|s|s.set(Some((true,base))));
 let mut vt=vec![0usize;0x120/8];vt[0x118/8]=base+0x4d2f00;
 let mut stats=vec![0u8;0x168];stats[0x140..0x144].copy_from_slice(&100i32.to_le_bytes());
 let mut action=[0u8;0x260];let modules=[stats.as_mut_ptr() as usize,action.as_mut_ptr() as usize];
 let mut actors=[vec![0usize;0x720/8],vec![0usize;0x720/8]];
 let mut effects=[Effects::default(),Effects::default()];
 for i in 0..2{
  actors[i][0]=vt.as_ptr() as usize;actors[i][0x1b8/8]=modules.as_ptr() as usize;actors[i][0x190/8]=&mut effects[i] as *mut Effects as usize;
  effects[i].ids=vec![OWNER,COUNTER,RECOVERY_READY];
 }
 let a=actors[0].as_ptr() as usize;let b=actors[1].as_ptr() as usize;
 let calls=std::cell::Cell::new(0);
 assert_eq!(recover_once(a,base,||{
  calls.set(calls.get()+1);
  assert_eq!(recover_once(a,base,||panic!("reentrant healing")),0);1
 }),1);
 // Clearing the immediate pulse does not rearm the opportunity. This also
 // covers delayed hits long after the native same-ID rejection window closes.
 for _ in 0..100{
  effects[0].ids.retain(|id|*id!=RECOVERY);
  assert_eq!(recover_once(a,base,||panic!("duplicate healing")),0);
 }
 assert_eq!(calls.get(),1);assert!(effects[0].ids.contains(&COUNTER));
 assert_eq!(recover_once(b,base,||1),1); // another actor has its own opportunity
 add(a,RECOVERY_READY,0);
 assert_eq!(recover_once(a,base,||0),0);assert!(effects[0].ids.contains(&RECOVERY_READY));
 assert_eq!(recover_once(a,base,||1),1);assert!(!effects[0].ids.contains(&RECOVERY_READY));
 for mode in 0..6{
  effects[0].ids=vec![OWNER,COUNTER,RECOVERY_READY];
  match mode{
   0=>effects[0].ids.retain(|id|*id!=OWNER),
   1=>effects[0].ids.retain(|id|*id!=COUNTER),
   2=>effects[0].ids.push(crate::reward_cycle_runtime::RESET),
   3=>ptr::write_unaligned(stats.as_mut_ptr().add(0x140) as *mut i32,0),
   4=>ptr::write((a+0x710) as *mut u8,1),
   _=>effects[0].remove_blocked=true,
  }
  assert_eq!(recover_once(a,base,||panic!("ineligible healing")),0);
  assert!(effects[0].ids.contains(&RECOVERY_READY));
  ptr::write_unaligned(stats.as_mut_ptr().add(0x140) as *mut i32,100);
  ptr::write((a+0x710) as *mut u8,0);effects[0].remove_blocked=false;
 }
 assert_eq!(recover_once(0,base,||panic!("null actor")),0);
 TEST_CONTEXT.with(|s|s.set(None));assert_ne!(crate::win::VirtualFree(base as *mut _,0,0x8000),0);
}}
