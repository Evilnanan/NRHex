//! Commit the rotation only when the changed hand's weapon allows enchantment.
use crate::read_ptr;
pub const ELIGIBILITY_QUERY:i32=990050;
#[repr(C)]
struct WeaponRef{id:i32,padding:u32,row:usize,base_id:i32,base_padding:u32,reinforce_id:i32,reinforce_padding:u32,reinforce_row:usize,version:u32,version_padding:u32}

pub unsafe fn can_enchant(actor:usize,hand:f32,base:usize)->bool{
 // RELIC_WEAPONCHANGE_ENCHANT: 0 right, 1 left; native weapon sources
 // are -1 right, -2 left (381220). Never fall back to the other hand.
 let source=if hand==0.{-1}else if hand==1.{-2}else{return false;};
 if actor==0||base==0{return false;}
 let vtable=read_ptr(actor);if vtable==0{return false;}
 let address=read_ptr(vtable+0x250);if address==0{return false;}
 let weapon:unsafe extern "system" fn(usize,i32)->i32=std::mem::transmute(address);
 let id=weapon(actor,source);if id<0{return false;}
 let mut reference=WeaponRef{id:-1,padding:0,row:0,base_id:-1,base_padding:0,reinforce_id:-1,reinforce_padding:0,reinforce_row:0,version:0,version_padding:0};
 // Existing signature-guarded resolver handles reinforcement and MMV rows.
 let lookup:unsafe extern "system" fn(*mut WeaponRef,i32)->usize=std::mem::transmute(base+crate::parry::WEAPON_PARAM_RVA);
 lookup(&mut reference,id);
 // EquipParamWeapon.isEnhance, also tested by native 2BAE20 / 51123F.
 reference.row!=0&&std::ptr::read((reference.row+0x106)as*const u8)&0x80!=0
}

#[cfg(test)]mod tests{
 use super::*;
 thread_local!{static ROW:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};}
 unsafe extern "system" fn weapon(actor:usize,source:i32)->i32{
  // Native 381220 maps -1 to the selected odd (right) slot, -2 to
  // the selected even (left) slot. Keep this independent of HKS hand IDs.
  assert!(matches!(source,-2|-1));crate::read_i32(actor+if source==-1{8}else{12})
 }
 unsafe extern "system" fn lookup(out:*mut WeaponRef,id:i32)->usize{
  assert!(matches!(id,1000|33000000|34000000|1000025|99000025));
  if id==1000025||id==99000025{(*out).row=ROW.with(|r|r.get());}0
 }
 #[test]fn changed_hand_live_parameter_and_missing_weapon(){unsafe{
  let mut row=[0u8;0x108];row[0x106]=0x80;ROW.with(|r|r.set(row.as_ptr()as usize));
  let mut vt=[0usize;0x258/8];vt[0x250/8]=weapon as *const ()as usize;
  let mut actor=[0usize;2];actor[0]=vt.as_ptr()as usize;
  let ptr=actor.as_mut_ptr()as usize;
  let set=|right:i32,left:i32|{std::ptr::write_unaligned((ptr+8)as*mut i32,right);std::ptr::write_unaligned((ptr+12)as*mut i32,left);};
  let base=lookup as *const ()as usize-crate::parry::WEAPON_PARAM_RVA;
  for id in [-1,1000,33000000,34000000]{
   set(id,1000025);assert!(!can_enchant(ptr,0.,base));assert!(can_enchant(ptr,1.,base));
   set(99000025,id);assert!(can_enchant(ptr,0.,base));assert!(!can_enchant(ptr,1.,base));
  }
  set(1000025,99000025);
  for flags in [0x7f,0xff,0,0x80]{row[0x106]=flags;assert_eq!(can_enchant(ptr,0.,base),flags&0x80!=0);assert_eq!(can_enchant(ptr,1.,base),flags&0x80!=0);}
  for hand in [-1.,0.5,2.,f32::NAN,f32::INFINITY]{assert!(!can_enchant(ptr,hand,base));}
  ROW.with(|r|r.set(0));assert!(!can_enchant(ptr,0.,base));
  assert!(!can_enchant(0,0.,base));assert!(!can_enchant(ptr,0.,0));
  vt[0x250/8]=0;assert!(!can_enchant(ptr,0.,base));
  actor[0]=0;assert!(!can_enchant(ptr,0.,base));
 }}
}
