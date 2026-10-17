//! Reuse the exact SwordArtsTable/MagicTable selection used by native weapon
//! rewards, including weights, unavailable rows and primary-spell exclusion.
//! Only the RNG input is scoped so every peer reconstructs the same result.
use crate::{demons::{Hand,Weapon},ACTIVE,BASE};
use std::{cell::Cell,ptr,sync::atomic::{AtomicPtr,Ordering}};
pub const ART_TABLE:usize=0xe86620;
pub const ART_PICK:usize=0xe86730;
pub const ART_GET:usize=0xe86840;
pub const SPELL_TABLE:usize=0xe7ae50;
pub const SPELL_PICK:usize=0xe7b030;
pub const SPELL_GET:usize=0xe7b010;
pub const RNG_RVA:usize=0xf561d0;
pub static RNG_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
// Filled from the hash-guarded supported executable, also checked by scancheck.
pub const ART_TABLE_PATTERN:&str="E9 01 E7 07 00 48 89 44 24 F8 48 8D 64 24 F8 48 8D 05 29 6E 7F 06 48 89 5C 24 F8 48 8D 64 24 F8 48 8B 5C 24 08 48 89 44 24 08 48 89 D8 48 8D 64 24 08 48 8B 5C 24 F8 48 8D 64 24 08 FF 64 24 F8";
pub const ART_PICK_PATTERN:&str="48 89 6C 24 20 57 48 83 EC 20 48 83 39 00 8B EA 48 8B F9 0F 84 D8 00 00 00 E9 FF D3 0F 06 48 C1 E0 05 48 03 C1 48 03 D0 48 3B DA 50 48 8D 05 72 BF B3 FF 48 87 04 24 C3 0F 1F 84 00 00 00 00 00";
pub const ART_GET_PATTERN:&str="48 83 EC 28 E8 27 00 00 00 48 85 C0 74 08 8B 40 04 48 83 C4 28 C3 B8 FF FF FF FF 48 83 C4 28 C3 48 83 39 00 0F 95 C0 C3 17 AE 49 42 01 00 00 00 48 83 EC 18 45 33 C0 3B 51 14 73 7B 4C 8B 09 4C";
pub const SPELL_TABLE_PATTERN:&str="E9 7F 35 4A 06 48 8B 45 20 8B 40 60 85 C0 41 54 49 BC 7A 23 BB 45 01 00 00 00 4C 87 24 24 50 51 48 8B 44 24 10 48 B9 04 FF B8 46 01 00 00 00 48 0F 44 C1 48 89 44 24 10 59 58 C3 E9 EA 74 D3 04";
pub const SPELL_PICK_PATTERN:&str="48 89 5C 24 10 55 56 57 41 54 41 55 41 56 41 57 48 8B EC 48 83 EC 50 4C 8B EA 4C 8B F9 41 BC FF FF FF FF 48 83 39 00 0F 84 61 01 00 00 E8 7E 43 2D 01 48 8B C8 48 89 45 E0 48 8B 00 45 33 C0 48";
pub const SPELL_GET_PATTERN:&str="48 83 EC 28 E8 D7 01 00 00 48 85 C0 74 08 8B 40 04 48 83 C4 28 C3 B8 FF FF FF FF 48 83 C4 28 C3 48 89 5C 24 10 55 56 57 41 54 41 55 41 56 41 57 48 8B EC 48 83 EC 50 4C 8B EA 4C 8B F9 41 BC FF";
pub const RNG_PATTERN:&str="48 83 EC 28 48 8B 01 FF 50 08 C1 E8 09 05 00 00 80 3F 89 44 24 30 F3 0F 10 44 24 30 F3 0F 5C 05 A8 D4 39 02 48 83 C4 28 C3 32 C0 C3 CC 7C 19 E6 48 83 EC 48 48 8B 01 0F 29 74 24 30 0F 28 F1 0F";
thread_local!{static DRAW:Cell<Option<u64>>=const{Cell::new(None)};}
struct Scope(Option<u64>);
impl Scope{fn enter(seed:u64)->Self{Self(DRAW.with(|c|c.replace(Some(seed))))}}
impl Drop for Scope{fn drop(&mut self){DRAW.with(|c|c.set(self.0));}}
fn next_float(state:&mut u64)->f32{
 *state=state.wrapping_add(0x9e3779b97f4a7c15);let mut z=*state;z=(z^(z>>30)).wrapping_mul(0xbf58476d1ce4e5b9);z=(z^(z>>27)).wrapping_mul(0x94d049bb133111eb);z^=z>>31;
 f32::from_bits(0x3f800000|((z as u32)>>9))-1.
}
pub unsafe extern "system" fn random(context:usize)->f32{
 if ACTIVE.load(Ordering::Acquire){if let Some(value)=DRAW.with(|c|{let mut state=c.get()?;let value=next_float(&mut state);c.set(Some(state));Some(value)}){return value;}}
 let original:unsafe extern "system" fn(usize)->f32=std::mem::transmute(RNG_ORIGINAL.load(Ordering::Acquire));original(context)
}
pub unsafe fn generate(seed:u32,left:bool,w:&Weapon)->Option<Hand>{generate_at(BASE.load(Ordering::Acquire),seed,left,w)}
unsafe fn generate_at(base:usize,seed:u32,left:bool,w:&Weapon)->Option<Hand>{
 let _scope=Scope::enter(u64::from(seed)^if left{0x7341_29ea_b80d_6c51}else{0x9a81_e32f_0685_4db7});
 let mut result=Hand{source:w.id,art:w.art,spell:w.spell};
 let mut table=[0u64;3];let p=table.as_mut_ptr()as usize;
 if w.art_table>=0{
  let init:unsafe extern "system" fn(usize,i32)->usize=std::mem::transmute(base+ART_TABLE);init(p,w.art_table);if table[0]==0{return None;}
  let pick:unsafe extern "system" fn(usize,i32)->i32=std::mem::transmute(base+ART_PICK);let index=pick(p,-1);
  if index>=0{let get:unsafe extern "system" fn(usize,i32)->i32=std::mem::transmute(base+ART_GET);let id=get(p,index);if id>=0{result.art=id;}}
 }
 if w.spell_table>=0{
  let init:unsafe extern "system" fn(usize,i32)->usize=std::mem::transmute(base+SPELL_TABLE);init(p,w.spell_table);if table[0]==0{return None;}
  let excluded=[w.primary,-1];let pick:unsafe extern "system" fn(usize,*const i32)->i32=std::mem::transmute(base+SPELL_PICK);let index=pick(p,excluded.as_ptr());
  if index>=0{let get:unsafe extern "system" fn(usize,i32)->i32=std::mem::transmute(base+SPELL_GET);let id=get(p,index);if id>=0{result.spell=id;}}
 }
 Some(result)
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn seeded_native_draw_scope_replays_and_restores_enclosing_rng_input(){
  assert_eq!(DRAW.with(Cell::get),None);let first;
  {let _s=Scope::enter(42);first=next_scoped();{let _nested=Scope::enter(77);for _ in 0..10{let v=next_scoped();assert!((0.0..1.0).contains(&v));}}assert_ne!(DRAW.with(Cell::get),Some(77));}
  assert_eq!(DRAW.with(Cell::get),None);{let _s=Scope::enter(42);assert_eq!(next_scoped(),first);}assert_eq!(DRAW.with(Cell::get),None);
 }
 fn next_scoped()->f32{DRAW.with(|c|{let mut s=c.get().unwrap();let result=next_float(&mut s);c.set(Some(s));result})}
}
#[cfg(test)]mod native_tests;
