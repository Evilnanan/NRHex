//! Shared attack eligibility; consumers retain their own timing/target rules.
//! Impact must be selected before reaction calculation, whereas a proc can
//! additionally require positive committed HP damage at accepted contact.
use crate::{has_effect,read_ptr,spell_percentage,sword};
use std::ptr;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum Kind{Direct,SwordSpell}
fn classify(direct:bool,melee:bool,native_spell:bool,mapped_spell:bool,sword:bool)->Option<Kind>{
 if mapped_spell||native_spell{return (mapped_spell&&sword).then_some(Kind::SwordSpell);}
 (direct&&melee).then_some(Kind::Direct)
}
pub unsafe fn from_source(actor:usize,hit:usize,direct:bool,base:usize)->Option<Kind>{
 if actor==0||hit==0{return None;}
 let mapped=spell_percentage(hit).is_some();
 classify(direct,crate::jiahao::melee_attack(&ptr::read_unaligned((hit+0x124)as*const[u32;8])),
  matches!(ptr::read((hit+0x122)as*const u8),3|4),mapped,mapped&&has_effect(actor,sword::OWNER,base))
}
pub unsafe fn at_contact(actor:usize,hit:usize,base:usize)->Option<Kind>{
 if actor==0||hit==0{return None;}
 from_source(actor,hit,read_ptr(hit+0x228)==actor,base)
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn spell_exception_requires_sword_and_known_spell_not_a_melee_tag(){
  for direct in [false,true]{for melee in [false,true]{for native in [false,true]{for mapped in [false,true]{for sword in [false,true]{
   let expected=if native||mapped{if mapped&&sword{Some(Kind::SwordSpell)}else{None}}else if direct&&melee{Some(Kind::Direct)}else{None};
   assert_eq!(classify(direct,melee,native,mapped,sword),expected);
  }}}}}
 }
 #[test]fn arrows_skill_missiles_and_untagged_auxiliary_contacts_are_not_weapon_contacts(){
  for sword in [false,true]{assert_eq!(classify(false,true,false,false,sword),None);assert_eq!(classify(false,false,false,false,sword),None);assert_eq!(classify(true,false,false,false,sword),None);assert_eq!(classify(true,true,false,false,sword),Some(Kind::Direct));}
 }
 #[test]#[ignore="requires NRHEX_NATIVE_EXE; initializes global spell catalog, run alone"]
 fn native_category_copy_and_shared_filter_cover_sword_sources(){unsafe{
  use crate::win::*;use sha2::{Digest,Sha256};
  unsafe extern "system" fn has(manager:usize,effect:i32)->u8{u8::from(manager==2&&effect==sword::OWNER)}
  let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
  let headers=crate::pe::Headers::parse(&raw,0).unwrap();let rva=0xe7065eu32;
  let section=headers.sections.iter().find(|s|rva>=s.virtual_address&&rva<s.virtual_address+s.raw_size).unwrap();
  let offset=(section.raw_offset+rva-section.virtual_address)as usize;let bytes=&raw[offset..offset+0x16];
  assert_eq!(bytes,&[0x0f,0x10,0x45,0xf4,0x0f,0x11,0x86,0x24,0x01,0,0,0x0f,0x10,0x4d,0x04,0x0f,0x11,0x8e,0x34,0x01,0,0]);
  let memory=VirtualAlloc(ptr::null_mut(),crate::HAS_EFFECT_RVA+0x1000,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
  let mut code=vec![0x56,0x55,0x48,0x89,0xce,0x48,0x8d,0x6a,0x0c];code.extend_from_slice(bytes);code.extend_from_slice(&[0x5d,0x5e,0xc3]);ptr::copy_nonoverlapping(code.as_ptr(),(base+0x100)as*mut u8,code.len());
  let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&(has as*const()as usize).to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(jump.as_ptr(),(base+crate::HAS_EFFECT_RVA)as*mut u8,jump.len());
  let copy:unsafe extern "system" fn(usize,*const u32)=std::mem::transmute(base+0x100);
  crate::SWORD_BONUSES.get_or_init(crate::sword::Bonuses::defaults);let spell=crate::sword_map::ATTACKS[0].0;
  let mut actor=[0usize;0x200/8];let a=actor.as_mut_ptr()as usize;
  for sword in [false,true]{actor[0x190/8]=if sword{2}else{1};for direct in [false,true]{for (id,kind,mapped)in [(-1,1,false),(-1,3,false),(spell,3,true),(spell,4,true),(spell,1,true)]{for melee in [false,true]{for ranged in [false,true]{
   let mut hit=[0u8;0x2c8];let h=hit.as_mut_ptr()as usize;hit[0x122]=kind;
   ptr::write_unaligned((h+0x40)as*mut i32,id);ptr::write_unaligned((h+0x44)as*mut i32,1);ptr::write_unaligned((h+0x228)as*mut usize,if direct{a}else{a+8});
   let mut mask=[0u32;8];if melee{mask[130/32]|=1<<(130%32);}if ranged{mask[105/32]|=1<<(105%32);}copy(h,mask.as_ptr());
   assert_eq!(ptr::read_unaligned((h+0x124)as*const[u32;8]),mask);let before=hit;
   let expected=if mapped&&sword{Some(Kind::SwordSpell)}else if !mapped&&kind==1&&direct&&melee&&!ranged{Some(Kind::Direct)}else{None};
   assert_eq!(at_contact(a,h,base),expected);assert_eq!(from_source(a,h,direct,base),expected);assert_eq!(hit,before);
  }}}}}
  assert_eq!(VirtualFree(memory,0,MEM_RELEASE),1);
 }}
}
