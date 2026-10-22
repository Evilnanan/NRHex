//! Byte-packed native saved records: eight history bits per active SpEffect.
//! Runtime sets are unbounded by machine-word size. Saved category10 has finite
//! native capacity; packing releases 24 slots at the current catalog size.
use crate::{has_effect,read_i32,read_ptr,reward_choices::HexSet,ptr};
use std::sync::Mutex;
pub const RESET:i32=95002825;
static HISTORY:Mutex<History>=Mutex::new(History::new());
struct History{seen:Option<HexSet>,dirty:bool,wait:f32,resetting:bool}
impl History{
 const fn new()->Self{Self{seen:None,dirty:false,wait:0.,resetting:false}}
 fn due(&mut self,reset:bool,dt:f32)->bool{
  if dt.is_finite()&&dt>0.{self.wait=(self.wait-dt).max(0.);}
  reset||self.seen.is_none()||self.resetting||self.wait<=0.
 }
 fn observe(&mut self,native:&HexSet,reset:bool)->Option<HexSet>{
  if reset{self.seen=Some(HexSet::new());self.resetting=true;}else{self.resetting=false;}
  let wanted=self.seen.get_or_insert_with(||native.clone());
  self.dirty=native!=wanted;self.wait=0.25;self.dirty.then(||wanted.clone())
 }
 fn seen(&self)->Option<HexSet>{if self.resetting{None}else{self.seen.clone()}}
 fn commit(&mut self,seen:HexSet){self.seen=Some(seen);self.dirty=true;self.wait=0.;}
}
fn encode(seen:&HexSet,bits:&[usize])->Vec<i32>{
 let count=bits.iter().copied().max().map_or(0,|n|n/8+1);let mut bytes=vec![0u8;count];
 for &i in seen{if let Some(&bit)=bits.get(i){bytes[bit/8]|=1<<(bit%8);}}
 bytes.into_iter().enumerate().filter_map(|(i,b)|(b!=0).then_some(crate::hex_pool::HISTORY_FIRST+i as i32*256+b as i32)).collect()
}
fn decode(records:&[i32],bits:&[usize])->HexSet{
 let mut bytes=std::collections::BTreeMap::new();
 for &id in records{let offset=id-crate::hex_pool::HISTORY_FIRST;if offset>=0&&offset%256!=0{bytes.entry(offset as usize/256).or_insert((offset%256)as u8);}}
 bits.iter().enumerate().filter_map(|(i,&bit)|bytes.get(&(bit/8)).is_some_and(|b|b&(1<<(bit%8))!=0).then_some(i)).collect()
}
unsafe fn records(manager:usize)->Vec<i32>{
 let mut result=Vec::new();let mut node=read_ptr(manager+8);
 for _ in 0..65536{
  if node==0{break;}
  let id=read_i32(node+8);
  if ptr::read_unaligned((node+0x68)as*const u32)&0x800c0003==0&&
   (crate::hex_pool::HISTORY_FIRST..crate::hex_pool::HISTORY_FIRST+crate::hex_pool::HISTORY_BYTES as i32*256).contains(&id)&&(id-crate::hex_pool::HISTORY_FIRST)%256!=0{
   result.push(id);
  }
  node=read_ptr(node+0x38);
 }result
}
pub unsafe fn update(player:usize,base:usize,dt:f32){
 let manager=read_ptr(player+0x190);if manager==0{return;}
 let reset=has_effect(player,RESET,base);if reset{crate::wraith::reset();crate::reward_reroll::reset();crate::potential_fission::reset(player,base);crate::demons_runtime::reset(player,base);crate::snack::reset(player,base);crate::momentum::reset(player,base);crate::heartstealer::reset(player,base);}
 let due=HISTORY.lock().map(|mut s|s.due(reset,dt)).unwrap_or(false);if !due{return;}
 let existing=records(manager);let native=decode(&existing,&crate::hex_pool::HISTORY_BITS);
 let wanted=HISTORY.lock().ok().and_then(|mut s|{s.observe(&native,reset);s.seen.clone()});
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);
 if let Some(wanted)=wanted{
  let ids=encode(&wanted,&crate::hex_pool::HISTORY_BITS);
  for &id in &ids{if !existing.contains(&id){apply(player,id,0);}}
  for id in existing{if !ids.contains(&id){remove(manager,id);}}
 }
 if reset&&records(manager).is_empty(){remove(manager,RESET);}
}
pub unsafe fn seen(player:usize,base:usize)->Option<HexSet>{update(player,base,0.);HISTORY.lock().ok().and_then(|s|s.seen())}
pub unsafe fn commit(player:usize,base:usize,seen:HexSet){if let Ok(mut s)=HISTORY.lock(){s.commit(seen);}update(player,base,0.);}
#[cfg(test)]mod tests{
 use super::*;use crate::reward_choices::{mask,all};
 #[test]fn packed_history_roundtrips_sparse_and_dense_catalogs_with_one_record_per_eight_slots(){
  for count in [28,33,64,96,128,256,1000]{
   let bits:Vec<_>=(0..count).map(|i|i+usize::from(i>=18)).collect();
   for seen in [HexSet::new(),all(count),(0..count).filter(|i|i%3==0).collect()]{
    let encoded=encode(&seen,&bits);assert_eq!(decode(&encoded,&bits),seen);assert!(encoded.len()<=(count+1).div_ceil(8));
    assert_eq!(encoded.iter().collect::<std::collections::BTreeSet<_>>().len(),encoded.len());
   }
  }
  assert!(encode(&all(28),&crate::hex_pool::HISTORY_BITS).len()<=4);
 }
 #[test]fn stale_records_never_overwrite_committed_history_during_deferred_native_changes(){
  let desired=mask([0,31,32,63,64,95,127,128,255,999]);let mut h=History::new();h.observe(&mask([0]),false);h.commit(desired.clone());
  for native in [HexSet::new(),mask([0]),mask([0,128]),desired.clone()]{h.observe(&native,false);assert_eq!(h.seen(),Some(desired.clone()));}
  assert!(!h.dirty);let mut restarted=History::new();restarted.observe(&desired,false);assert_eq!(restarted.seen(),Some(desired));
 }
 #[test]fn rollover_and_only_explicit_reset_control_the_saved_round(){
  let mut h=History::new();h.observe(&mask([0,1,63,128]),false);h.commit(mask([2,255]));
  for native in [mask([0,1,63,128]),mask([2,63,128]),mask([2,255])]{h.observe(&native,false);assert_eq!(h.seen(),Some(mask([2,255])));}
  assert_eq!(h.observe(&mask([2,255]),true),Some(HexSet::new()));assert_eq!(h.seen(),None);
  assert_eq!(h.observe(&HexSet::new(),false),None);assert_eq!(h.seen(),Some(HexSet::new()));
 }
 #[test]fn duplicate_transitional_records_use_the_newest_native_link_head(){
  let first=crate::hex_pool::HISTORY_FIRST;assert_eq!(decode(&[first+1,first+255],&[0,1,2]),mask([0]));
  assert_eq!(decode(&[first+255,first+1],&[0,1,2]),mask([0,1,2]));
 }
 #[test]fn compiled_packed_update_retains_deferred_history_and_waits_for_reset_cleanup(){unsafe{
  use crate::win::*;use std::cell::{Cell,RefCell};
  thread_local!{static RESETTING:Cell<bool>=const{Cell::new(false)};static CALLS:RefCell<Vec<(bool,i32)>>=const{RefCell::new(Vec::new())};}
  unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from(id==RESET&&RESETTING.with(Cell::get))}
  unsafe extern "system" fn add(_:usize,id:i32,flag:u8)->u8{assert_eq!(flag,0);CALLS.with(|c|c.borrow_mut().push((true,id)));1}
  unsafe extern "system" fn remove(_:usize,id:i32)->u8{CALLS.with(|c|c.borrow_mut().push((false,id)));1}
  let image=VirtualAlloc(ptr::null_mut(),0x600000,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!image.is_null());let base=image as usize;
  for(rva,dest)in [(crate::HAS_EFFECT_RVA,has as*const()as usize),(crate::APPLY_SELF_RVA,add as*const()as usize),(crate::REMOVE_EFFECT_RVA,remove as*const()as usize)]{
   let mut bytes=vec![0x48,0xb8];bytes.extend_from_slice(&dest.to_le_bytes());bytes.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(bytes.as_ptr(),(base+rva)as*mut u8,bytes.len());
  }
  let mut player=[0u64;0x200/8];let player=player.as_mut_ptr()as usize;let mut manager=[0u64;4];let manager=manager.as_mut_ptr()as usize;ptr::write_unaligned((player+0x190)as*mut usize,manager);
  let mut nodes=[[0u64;0x70/8];8];
  let mut set_native=|ids:&[i32]|{
   for node in &mut nodes{node.fill(0);}
   for (i,id)in ids.iter().enumerate(){let p=nodes[i].as_mut_ptr()as usize;ptr::write_unaligned((p+8)as*mut i32,*id);if i+1<ids.len(){ptr::write_unaligned((p+0x38)as*mut usize,nodes[i+1].as_mut_ptr()as usize);}}
   ptr::write_unaligned((manager+8)as*mut usize,if ids.is_empty(){0}else{nodes[0].as_mut_ptr()as usize});
  };
  let previous={let mut h=HISTORY.lock().unwrap();std::mem::replace(&mut *h,History::new())};
  let first=crate::hex_pool::HISTORY_FIRST;set_native(&[first+255,first+512+32]);
  let desired=mask([0,8,17,27]);let wanted=encode(&desired,&crate::hex_pool::HISTORY_BITS);
  commit(player,base,desired.clone());
  assert_eq!(HISTORY.lock().unwrap().seen(),Some(desired.clone()));
  let calls=CALLS.with(|c|c.replace(Vec::new()));for &id in &wanted{assert!(calls.contains(&(true,id)));}assert!(calls.contains(&(false,first+255)));
  // Partial native acknowledgement cannot erase the desired packed bytes.
  set_native(&[wanted[0],first+255]);update(player,base,0.3);assert_eq!(HISTORY.lock().unwrap().seen(),Some(desired.clone()));CALLS.with(|c|c.borrow_mut().clear());
  set_native(&wanted);update(player,base,0.3);assert!(CALLS.with(|c|c.borrow().is_empty()));assert!(!HISTORY.lock().unwrap().dirty);
  // Reconcile even after a previous clean acknowledgement.
  set_native(&wanted[1..]);update(player,base,0.3);assert!(CALLS.with(|c|c.borrow().contains(&(true,wanted[0]))));CALLS.with(|c|c.borrow_mut().clear());
  RESETTING.with(|c|c.set(true));set_native(&wanted);update(player,base,0.);assert_eq!(HISTORY.lock().unwrap().seen(),None);
  assert!(!CALLS.with(|c|c.borrow().contains(&(false,RESET))));
  CALLS.with(|c|c.borrow_mut().clear());set_native(&[]);update(player,base,0.);assert!(CALLS.with(|c|c.borrow().contains(&(false,RESET))));
  RESETTING.with(|c|c.set(false));update(player,base,0.);assert_eq!(HISTORY.lock().unwrap().seen(),Some(HexSet::new()));
  CALLS.with(|c|c.borrow_mut().clear());set_native(&[first+512+2]);update(player,base,0.3);
  assert!(CALLS.with(|c|c.borrow().contains(&(false,first+512+2)))); // retired bit decodes empty but still needs removal
  *HISTORY.lock().unwrap()=previous;VirtualFree(image,0,MEM_RELEASE);
 }}

}
