//! Double the Joy: persist two draws, then grant directly through local common.
//! Only already owned/self/foreign character-exclusive powers are unavailable.
use crate::{has_effect,is_local,read_ptr,native_actor,hex_pool::{OWNER_IDS,HISTORY_BITS},reward_choices::HexSet};
use std::sync::Mutex;
pub const OWNER:i32=95004700;
pub const RECEIPT:i32=OWNER+1;
pub const READY:i32=OWNER+2;
pub const FIRST_MARKER:i32=96210000;
pub const POPUP_RVA:usize=0x7c5080;
pub const POPUP_PATTERN:&str="48 89 5C 24 18 55 56 57 48 83 EC 20 48 8B 05 3D 28 44 03 41 0F B6 D8 8B FA 48 8B E9 48 8B B0 80 00 00 00";
const MENU_RVA:usize=0x3c14a70;
fn marker(index:usize)->i32{FIRST_MARKER+HISTORY_BITS[index]as i32}
fn role_allowed(owner:i32,hero:Option<u8>)->bool{owner!=OWNER&&crate::reward_choices::exclusive_role(owner).is_none_or(|required|hero==Some(required))}
fn candidates(owned:&HexSet,selected:&[usize],hero:Option<u8>)->Vec<usize>{
 OWNER_IDS.iter().enumerate().filter_map(|(i,&owner)|(!owned.contains(&i)&&!selected.contains(&i)&&role_allowed(owner,hero)).then_some(i)).collect()
}
fn select(mut selected:Vec<usize>,owned:&HexSet,hero:Option<u8>,mut rng:impl FnMut(usize)->Option<usize>)->Option<Vec<usize>>{
 let mut pool=candidates(owned,&selected,hero);
 while selected.len()<2&&!pool.is_empty(){let n=rng(pool.len())?;if n>=pool.len(){return None;}selected.push(pool.remove(n));}
 Some(selected)
}
#[derive(Default)]struct State{selected:Option<Vec<usize>>,posted:std::collections::BTreeSet<i32>,announced:std::collections::BTreeSet<i32>}
static STATE:Mutex<State>=Mutex::new(State{selected:None,posted:std::collections::BTreeSet::new(),announced:std::collections::BTreeSet::new()});
pub fn reset(){if let Ok(mut state)=STATE.lock(){*state=State::default();}}
// Reserve a request before invoking native code, which may re-enter update.
fn post(id:i32)->bool{STATE.lock().map(|mut state|state.posted.insert(id)).unwrap_or(false)}
unsafe fn add(actor:usize,id:i32,base:usize){if post(id){crate::wraith::set(actor,id,true,base);}}
unsafe fn announce(id:i32,base:usize)->bool{
 // Use the local pickup branch, not EMEVD 2003[124]'s record-only flag.
 // The native deque owns ordering and presentation, including simultaneous gifts.
 let menu=read_ptr(base+MENU_RVA);
 if menu==0||read_ptr(base+crate::GAME_DATA_RVA)==0{return false;}
 let Ok(mut state)=STATE.lock()else{return false;};
 if !state.announced.insert(id){return true;}
 drop(state);
 let queue:unsafe extern "system" fn(usize,i32,u8)=std::mem::transmute(base+POPUP_RVA);
 queue(menu,id,0);true
}
pub unsafe fn update(actor:usize,base:usize){
 if actor==0||!is_local(actor,base)||!has_effect(actor,OWNER,base)||has_effect(actor,crate::reward_cycle_runtime::RESET,base){return;}
 let selected=STATE.lock().ok().and_then(|state|state.selected.clone());
 let saved=||OWNER_IDS.iter().enumerate().filter_map(|(i,&owner)|(owner!=OWNER&&has_effect(actor,marker(i),base)).then_some(i)).collect::<Vec<_>>();
 if has_effect(actor,RECEIPT,base){
  // Once complete, query only the two frozen targets, rather than the catalog.
  let cleanup=selected.unwrap_or_else(saved);
  if let Ok(mut state)=STATE.lock(){state.selected.get_or_insert_with(||cleanup.clone());}
  for i in cleanup{if has_effect(actor,marker(i),base){crate::wraith::set(actor,marker(i),false,base);}}
  if has_effect(actor,READY,base){crate::wraith::set(actor,READY,false,base);}return;
 }
 let stats=native_actor::stats(actor);if stats==0||native_actor::incapacitated(actor,stats){return;}
 let Some(hero)=crate::local_hero(base).filter(|h|(1..=10).contains(h))else{return;};
 let ready=has_effect(actor,READY,base);
 let selected=if let Some(selected)=selected{selected}else{
  let owned=crate::owned_hexes(actor,base);
  let selected=if ready{Some(saved())}else{select(saved(),&owned,Some(hero),crate::random_index)};
  let Some(selected)=selected else{return;};
  if let Ok(mut state)=STATE.lock(){state.selected=Some(selected.clone());}else{return;}selected
 };
 if selected.iter().any(|&i|!has_effect(actor,marker(i),base)){
  for &i in &selected{if !has_effect(actor,marker(i),base){add(actor,marker(i),base);}}return;
 }
 if !ready{add(actor,READY,base);return;}
 // Workers grant effects; announce only after each real owner is observable.
 // Do not consume the completion receipt while the native UI is unavailable.
 let mut complete=true;
 for &i in &selected{
  if !has_effect(actor,OWNER_IDS[i],base)||!announce(OWNER_IDS[i],base){complete=false;}
 }
 if complete{add(actor,RECEIPT,base);}
}
#[cfg(test)]mod tests{
 use super::*;use crate::reward_choices::{all,mask};
 fn index(id:i32)->usize{OWNER_IDS.iter().position(|&owner|owner==id).unwrap()}
 #[test]fn only_foreign_exclusives_owned_and_self_are_filtered_for_every_hero(){
  for hero in std::iter::once(None).chain((0..=255).map(Some)){
   let owned=mask([index(OWNER),index(95000300)]);let pool=candidates(&owned,&[],hero);
   for (i,&id)in OWNER_IDS.iter().enumerate(){assert_eq!(pool.contains(&i),!owned.contains(&i)&&match id{95001800=>hero==Some(6),95004200=>hero==Some(3),95004800=>hero==Some(5),95004900=>hero==Some(7),_=>id!=OWNER});}
  }
 }
 #[test]fn incompatible_and_normally_restricted_powers_remain_candidates(){
  for (owned,other)in [(95000900,95004500),(95002600,95000700),(95000700,95002600),(95004400,95003800),(95003800,95004400)]{
   assert!(candidates(&mask([index(owned)]),&[],Some(1)).contains(&index(other)));
  }
  let pool=candidates(&HexSet::new(),&[],Some(1));for id in [95000900,95003500,95002000,95000400,95001300,95001600,95002500,95003400]{assert!(pool.contains(&index(id)));}
 }
 #[test]fn draws_are_distinct_and_restored_partial_choices_fill_without_rerolling(){
  let owned=mask([index(OWNER)]);let pool=candidates(&owned,&[],Some(3));
  for first in 0..pool.len(){for second in 0..pool.len()-1{
   let mut draws=[first,second].into_iter();let selected=select(vec![],&owned,Some(3),|_|draws.next()).unwrap();assert_eq!(selected.len(),2);assert_ne!(selected[0],selected[1]);assert_eq!(selected[0],pool[first]);
  }}
  let first=index(95004500);let selected=select(vec![first],&owned,Some(3),|_|Some(0)).unwrap();assert_eq!(selected[0],first);assert_ne!(selected[1],first);
  assert_eq!(select(vec![],&owned,Some(1),|_|None),None);assert_eq!(select(vec![],&owned,Some(1),|n|Some(n)),None);
 }
 #[test]fn exhausted_pool_never_reintroduces_owned_powers_and_markers_are_stable(){
  let mut owned=all(OWNER_IDS.len());assert_eq!(select(vec![],&owned,Some(1),|_|panic!()),Some(vec![]));let one=index(95004500);owned.remove(&one);assert_eq!(select(vec![],&owned,Some(1),|_|Some(0)),Some(vec![one]));
  let markers:std::collections::BTreeSet<_>=(0..OWNER_IDS.len()).map(marker).collect();assert_eq!(markers.len(),OWNER_IDS.len());assert_eq!(marker(index(95004600)),FIRST_MARKER+43);
 }
}
#[cfg(test)]#[path="double_dip_native_tests.rs"]mod native_tests;
