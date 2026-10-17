//! Linear-space sampling without replacement, with per-expedition exposure rounds.
use crate::{hex_pool::HEX_COUNT,reward_choices::{HexSet,all,mask}};
pub struct Single{pub candidates:Vec<usize>,seen:HexSet,rollover:bool}
impl Single{
 pub fn new(unavailable:&HexSet,seen:&HexSet,visible:&HexSet)->Self{Self::with_count(HEX_COUNT,unavailable,seen,visible)}
 pub fn with_count(count:usize,unavailable:&HexSet,seen:&HexSet,visible:&HexSet)->Self{
  let eligible:HexSet=all(count).difference(unavailable).copied().filter(|i|!visible.contains(i)).collect();
  let unseen:HexSet=eligible.difference(seen).copied().collect();let rollover=unseen.is_empty();
  Self{candidates:if rollover{eligible}else{unseen}.into_iter().collect(),seen:seen.intersection(&all(count)).copied().collect(),rollover}
 }
 pub fn next_seen(&self,choice:usize)->Option<HexSet>{
  if !self.candidates.contains(&choice){return None;}
  let mut seen=if self.rollover{HexSet::new()}else{self.seen.clone()};seen.insert(choice);Some(seen)
 }
}
pub struct Round{pub candidates:Vec<usize>,seen:HexSet,tail:HexSet,rollover:bool}
impl Round{
 pub fn new(unavailable:&HexSet,seen:&HexSet)->Self{Self::with_count(HEX_COUNT,unavailable,seen)}
 pub fn with_count(count:usize,unavailable:&HexSet,seen:&HexSet)->Self{
  let available:HexSet=all(count).difference(unavailable).copied().collect();
  let unseen:HexSet=available.difference(seen).copied().collect();let rollover=unseen.len()<3;
  Self{candidates:if available.len()<3{HexSet::new()}else if rollover{available}else{unseen.clone()}.into_iter().collect(),
   seen:seen.intersection(&all(count)).copied().collect(),tail:if rollover{unseen}else{HexSet::new()},rollover}
 }
 fn pools(&self)->(Vec<usize>,Vec<usize>){
  (self.tail.iter().copied().collect(),self.candidates.iter().copied().filter(|i|!self.tail.contains(i)).collect())
 }
 pub fn draw(&self,mut random:impl FnMut(usize)->Option<usize>)->Option<[usize;3]>{
  if self.candidates.len()<3{return None;}
  let (mut tail,mut rest)=self.pools();let mut out=[0;3];
  for slot in &mut out{
   let pool=if tail.is_empty(){&mut rest}else{&mut tail};let i=random(pool.len())?;
   if i>=pool.len(){return None;}*slot=pool.swap_remove(i);
  }Some(out)
 }
 pub fn preferred(&self,preferred:[usize;3])->Option<[usize;3]>{
  if self.candidates.len()<3{return None;}
  let rank=|i:&usize|preferred.iter().position(|p|p==i).unwrap_or(3+*i);
  let (mut tail,mut rest)=self.pools();tail.sort_by_key(rank);rest.sort_by_key(rank);
  tail.extend(rest);Some(tail[..3].try_into().unwrap())
 }
 pub fn next_seen(&self,choice:[usize;3])->Option<HexSet>{
  let offered=mask(choice);
  if offered.len()!=3||!choice.iter().all(|i|self.candidates.contains(i))||mask(choice[..self.tail.len()].iter().copied())!=self.tail{return None;}
  Some(if self.rollover{offered.difference(&self.tail).copied().collect()}else{self.seen.union(&offered).copied().collect()})
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn visits_every_eligible_item_before_any_repeat_at_catalog_sizes_above_word_boundaries(){
  for count in [3,4,28,33,64,96,129,256,1000]{
   let unavailable=if count>4{mask([1,3])}else{HexSet::new()};let eligible:HexSet=all(count).difference(&unavailable).copied().collect();
   let mut seen=HexSet::new();let mut visited=HexSet::new();let mut random=0x12345678u64;
   for _ in 0..count*3{
    let r=Round::with_count(count,&unavailable,&seen);
    let chosen=r.draw(|n|{random=random.wrapping_mul(6364136223846793005).wrapping_add(1);Some((random>>32)as usize%n)}).unwrap();
    assert_eq!(mask(chosen).len(),3);
    for i in chosen{if visited==eligible{visited.clear();}assert!(!unavailable.contains(&i));assert!(visited.insert(i),"count {count}, repeated {i}");}
    seen=r.next_seen(chosen).unwrap();
   }
  }
 }
 #[test]fn tail_is_offered_first_then_new_round_commits_only_its_new_options(){
  for tail in [mask([6]),mask([6,8])]{
   let unavailable=mask([0,3]);let seen:HexSet=all(28).difference(&unavailable).copied().filter(|i|!tail.contains(i)).collect();
   let r=Round::with_count(28,&unavailable,&seen);
   for chosen in [r.preferred([0,3,9]).unwrap(),r.draw(|_|Some(0)).unwrap()]{
    assert_eq!(mask(chosen[..tail.len()].iter().copied()),tail);assert_eq!(r.next_seen(chosen),Some(mask(chosen).difference(&tail).copied().collect()));
   }
   assert_eq!(r.seen,seen);assert_eq!(r.next_seen([0,1,2]),None);
  }
 }
 #[test]fn all_options_shown_are_excluded_regardless_of_which_was_selected(){
  let first=Round::new(&HexSet::new(),&HexSet::new());let c=first.preferred([0,1,2]).unwrap();let seen=first.next_seen(c).unwrap();
  let next=Round::new(&mask([0]),&seen);assert!(next.candidates.iter().all(|i|!seen.contains(i)));assert_eq!(next.next_seen(c),None);
 }
 #[test]fn single_refresh_excludes_owned_visible_and_seen_and_can_roll_over(){
  let visible=mask([0,65,128]);let unavailable=mask([2,64]);let seen=mask([0,65,128,129]);
  let r=Single::with_count(160,&unavailable,&seen,&visible);
  for &i in &r.candidates{assert!(!unavailable.contains(&i)&&!visible.contains(&i)&&!seen.contains(&i));let mut expected=seen.clone();expected.insert(i);assert_eq!(r.next_seen(i),Some(expected));}
  let r=Single::with_count(160,&unavailable,&all(160),&visible);for &i in &r.candidates{assert_eq!(r.next_seen(i),Some(mask([i])));}
  assert_eq!(r.next_seen(65),None);
 }
 #[test]fn small_remaining_pools_never_duplicate_or_include_owned_and_rng_failure_consumes_nothing(){
  for left in [0,1,2]{let excluded:HexSet=(left..256).collect();let r=Round::with_count(256,&excluded,&all(256));assert!(r.candidates.is_empty());assert_eq!(r.draw(|_|Some(0)),None);assert_eq!(r.preferred([0,1,2]),None);}
  let r=Round::with_count(256,&HexSet::new(),&mask([65,128]));assert_eq!(r.draw(|_|None),None);assert_eq!(r.draw(|n|Some(n)),None);assert_eq!(r.seen,mask([65,128]));assert_eq!(r.next_seen([3,3,4]),None);
 }
 #[test]fn draw_reaches_all_distinct_triples_uniformly_without_storing_them(){
  let r=Round::with_count(5,&HexSet::new(),&HexSet::new());let mut results=std::collections::BTreeMap::new();
  for a in 0..5{for b in 0..4{for c in 0..3{let mut rolls=[a,b,c].into_iter();let chosen=r.draw(|_|rolls.next()).unwrap();*results.entry(chosen).or_insert(0)+=1;}}}
  assert_eq!(results.len(),60);assert!(results.values().all(|&count|count==1));
 }
}
