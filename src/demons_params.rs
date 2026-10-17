//! Immutable virtual rows. Never modify the shared native parameter table and
//! never free a materialized row while native weapon/character refs can exist.
use crate::{demons,demons_data as data,ACTIVE};
use std::{collections::HashMap,ptr,sync::{atomic::{AtomicPtr,Ordering},Mutex,OnceLock}};
pub const CI_RVA:usize=0xe88090;
pub const WEAPON_RVA:usize=0xea7fc0;
pub const CI_PATTERN:&str="48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 57 48 83 EC 30 8B FA 48 8B F1 33 DB 85 D2 0F 88 20 01 00 00 48 8B 0D C6 07 DA 02 48 85 C9 75 2E 48 8D 0D DE C5 D7 02 E8 B5 B2 2C 01 4C 8B C8 4C 8D";
pub const WEAPON_PATTERN:&str="48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 48 89 7C 24 20 41 54 41 56 41 57 48 83 EC 30 44 8B F2 48 8B F9 33 DB 8D 6B FF 85 D2 0F 88 67 01 00 00 B8 1F 85 EB 51 41 F7 E6 C1 EA 05 44 6B FA 64";
pub static CI_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static WEAPON_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
type Lookup=unsafe extern "system" fn(usize,i32)->usize;
#[derive(Clone,Copy)]
#[repr(C,align(16))]
struct Block([u8;16]);
struct Row(Box<[Block]>);
impl Row{
 unsafe fn copy(source:usize,size:usize)->Self{let mut row=Self(vec![Block([0;16]);size.div_ceil(16)].into_boxed_slice());ptr::copy_nonoverlapping(source as*const u8,row.0.as_mut_ptr()as*mut u8,size);row}
 fn pointer(&self)->usize{self.0.as_ptr()as usize}
 unsafe fn set(&mut self,offset:usize,value:i32){ptr::write_unaligned((self.pointer()+offset)as*mut i32,value);}
}
static ROWS:OnceLock<Mutex<HashMap<i32,Row>>>=OnceLock::new();
fn cached(id:i32)->Option<usize>{ROWS.get_or_init(Default::default).lock().ok()?.get(&id).map(Row::pointer)}
fn publish(id:i32,row:Row)->usize{let Ok(mut rows)=ROWS.get_or_init(Default::default).lock()else{return 0;};rows.entry(id).or_insert(row).pointer()}
// Prime on the same game callback that creates the summon, before resource
// loading can request weapons on a worker. Subsequent lookups only read rows.
pub unsafe fn prepare(seed:u32,base:usize)->bool{
 let Some(roll)=demons::roll(seed)else{return false;};let Some(id)=demons::chara_id(seed)else{return false;};
 let mut reference=[0u64;6];let p=reference.as_mut_ptr()as usize;let ci:Lookup=std::mem::transmute(base+CI_RVA);ci(p,id);
 if ptr::read_unaligned((p+8)as*const usize)==0{return false;}
 let weapon:Lookup=std::mem::transmute(base+WEAPON_RVA);
 for left in std::iter::once(false).chain(roll.left.map(|_|true)){
  weapon(p,demons::weapon_id(seed,left).unwrap());if ptr::read_unaligned((p+8)as*const usize)==0{return false;}
 }
 true
}
unsafe fn chara_row(id:i32,source:usize)->usize{
 if let Some(p)=cached(id){return p;}
 if source==0{return 0;}
 let Some(seed)=demons::seed_from_chara(id)else{return 0;};let Some(roll)=demons::roll(seed)else{return 0;};
 let mut row=Row::copy(source,data::CI_SIZE);
 for &offset in &data::CI_WEAPONS{row.set(offset,-1);}
 row.set(data::CI_WEAPONS[0],demons::weapon_id(seed,false).unwrap());
 if roll.left.is_some(){row.set(data::CI_WEAPONS[3],demons::weapon_id(seed,true).unwrap());}
 publish(id,row)
}
unsafe fn weapon_row(id:i32,source:usize,hand:demons::Hand)->usize{
 if let Some(p)=cached(id){return p;}
 if source==0{return 0;}
 let mut row=Row::copy(source,data::WEAPON_SIZE);row.set(data::WEAPON_ART,hand.art);row.set(data::WEAPON_SPELL,hand.spell);publish(id,row)
}
pub unsafe extern "system" fn chara(out:usize,id:i32)->usize{
 let original:Lookup=std::mem::transmute(CI_ORIGINAL.load(Ordering::Acquire));
 chara_call(original,out,id,ACTIVE.load(Ordering::Acquire))
}
unsafe fn chara_call(original:Lookup,out:usize,id:i32,active:bool)->usize{
 if !active{return original(out,id);}
 let Some(roll)=demons::seed_from_chara(id).and_then(demons::roll)else{return original(out,id);};
 let result=original(out,roll.source_chara());
 let source=ptr::read_unaligned((out+8)as*const usize);let row=chara_row(id,source);
 ptr::write_unaligned(out as*mut i32,id);ptr::write_unaligned((out+8)as*mut usize,row);result
}
pub unsafe extern "system" fn weapon(out:usize,id:i32)->usize{
 let original:Lookup=std::mem::transmute(WEAPON_ORIGINAL.load(Ordering::Acquire));
 weapon_call(original,out,id,ACTIVE.load(Ordering::Acquire),|seed,left,hand|{
  crate::demons_data::WEAPONS.iter().find(|w|w.id==hand.source).and_then(|w|crate::demons_lottery::generate(seed,left,w))
 })
}
unsafe fn weapon_call(original:Lookup,out:usize,id:i32,active:bool,mut generate:impl FnMut(u32,bool,demons::Hand)->Option<demons::Hand>)->usize{
 if !active{return original(out,id);}
 let Some((seed,left))=demons::seed_from_weapon(id)else{return original(out,id);};
 let Some(roll)=demons::roll(seed)else{return original(out,id);};let Some(hand)=(if left{roll.left}else{Some(roll.right)})else{return original(out,id);};
 let level=id%100;let root=id-level;
 // Let native code construct the whole 48-byte WeaponRef, especially its
 // embedded ReinforceWeaponRef and param version. Replace only ID/root/data.
 let result=original(out,hand.source+level);
 let source=ptr::read_unaligned((out+8)as*const usize);
 let row=if let Some(p)=cached(root){p}else if source==0{0}else{generate(seed,left,hand).map_or(0,|selected|weapon_row(root,source,selected))};
 ptr::write_unaligned(out as*mut i32,id);ptr::write_unaligned((out+8)as*mut usize,row);ptr::write_unaligned((out+0x10)as*mut i32,root);result
}

#[cfg(test)]mod tests{
 use super::*;
 use std::cell::RefCell;
 thread_local!{static INPUTS:RefCell<Vec<i32>>=const{RefCell::new(Vec::new())};}
 static SOURCE_CI:[u64;data::CI_SIZE/8]=[0xabababababababab;data::CI_SIZE/8];
 static SOURCE_WEAPON:[u64;data::WEAPON_SIZE/8]=[0xcdcdcdcdcdcdcdcd;data::WEAPON_SIZE/8];
 unsafe extern "system" fn ci(out:usize,id:i32)->usize{INPUTS.with(|v|v.borrow_mut().push(id));ptr::write_unaligned(out as*mut i32,id);ptr::write_unaligned((out+8)as*mut usize,SOURCE_CI.as_ptr()as usize);0x1234}
 unsafe extern "system" fn wep(out:usize,id:i32)->usize{INPUTS.with(|v|v.borrow_mut().push(id));ptr::write_bytes(out as*mut u8,0x79,48);ptr::write_unaligned(out as*mut i32,id);ptr::write_unaligned((out+8)as*mut usize,SOURCE_WEAPON.as_ptr()as usize);ptr::write_unaligned((out+0x10)as*mut i32,id/100*100);0x5678}
 #[test]fn dynamic_refs_preserve_native_layout_source_data_and_reinforcement(){unsafe{
  let seed=701;let roll=demons::roll(seed).unwrap();let id=demons::chara_id(seed).unwrap();let mut out=[0u64;6];let out=out.as_mut_ptr()as usize;
  assert_eq!(chara_call(ci,out,id,true),0x1234);assert_eq!(ptr::read_unaligned(out as*const i32),id);
  let row=ptr::read_unaligned((out+8)as*const usize);assert_ne!(row,SOURCE_CI.as_ptr()as usize);assert_eq!(row%16,0);
  let bytes=std::slice::from_raw_parts(row as*const u8,data::CI_SIZE);let mut expected=std::slice::from_raw_parts(SOURCE_CI.as_ptr()as*const u8,data::CI_SIZE).to_vec();
  for &offset in &data::CI_WEAPONS{expected[offset..offset+4].copy_from_slice(&(-1i32).to_le_bytes());}
  expected[data::CI_WEAPONS[0]..data::CI_WEAPONS[0]+4].copy_from_slice(&demons::weapon_id(seed,false).unwrap().to_le_bytes());
  if roll.left.is_some(){expected[data::CI_WEAPONS[3]..data::CI_WEAPONS[3]+4].copy_from_slice(&demons::weapon_id(seed,true).unwrap().to_le_bytes());}assert_eq!(bytes,expected);
  assert_eq!(chara_call(ci,out,id,true),0x1234);assert_eq!(ptr::read_unaligned((out+8)as*const usize),row);
  let wid=demons::weapon_id(seed,false).unwrap()+12;assert_eq!(weapon_call(wep,out,wid,true,|_,_,hand|Some(hand)),0x5678);assert_eq!(ptr::read_unaligned(out as*const i32),wid);assert_eq!(ptr::read_unaligned((out+0x10)as*const i32),wid-12);
  assert_eq!(std::slice::from_raw_parts((out+0x18)as*const u8,24),[0x79;24]);let wrow=ptr::read_unaligned((out+8)as*const usize);
  let mut expected=std::slice::from_raw_parts(SOURCE_WEAPON.as_ptr()as*const u8,data::WEAPON_SIZE).to_vec();expected[data::WEAPON_ART..data::WEAPON_ART+4].copy_from_slice(&roll.right.art.to_le_bytes());expected[data::WEAPON_SPELL..data::WEAPON_SPELL+4].copy_from_slice(&roll.right.spell.to_le_bytes());assert_eq!(std::slice::from_raw_parts(wrow as*const u8,data::WEAPON_SIZE),expected);
  assert_eq!(SOURCE_WEAPON,[0xcdcdcdcdcdcdcdcd;data::WEAPON_SIZE/8]);assert_eq!(SOURCE_CI,[0xabababababababab;data::CI_SIZE/8]);
  let inputs=INPUTS.with(|v|v.replace(Vec::new()));assert_eq!(inputs,vec![roll.source_chara(),roll.source_chara(),roll.right.source+12]);
 }}
 #[test]fn normal_ids_and_disabled_mod_pass_through_without_aliasing(){unsafe{
  let mut out=[0u64;6];let out=out.as_mut_ptr()as usize;chara_call(ci,out,35000,true);weapon_call(wep,out,3150000,true,|_,_,hand|Some(hand));let id=demons::chara_id(9).unwrap();chara_call(ci,out,id,false);
  assert_eq!(ptr::read_unaligned(out as*const i32),id);assert_eq!(ptr::read_unaligned((out+8)as*const usize),SOURCE_CI.as_ptr()as usize);assert_eq!(INPUTS.with(|v|v.replace(Vec::new())),vec![35000,3150000,id]);
 }}
}

#[cfg(test)]mod native_tests;
