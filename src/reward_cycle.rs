//! Per-expedition exposure rounds. Plans never consume history until delivery.
use crate::hex_pool::{HEX_COUNT,ORDERS};
pub const ALL:u32=(1u32<<HEX_COUNT)-1;
pub fn mask(order:[usize;3])->u32{order.into_iter().fold(0,|m,i|m|(1<<i))}

/// A reroll draws one new option while keeping the other two on screen.
pub struct Single {pub candidates:Vec<usize>,seen:u32,rollover:bool}
impl Single {
 pub fn new(unavailable:u32,seen:u32,visible:u32)->Self {
  let eligible=ALL&!unavailable&!visible;
  let unseen=eligible&!seen;
  let rollover=unseen==0;
  let pool=if rollover{eligible}else{unseen};
  Self{candidates:(0..HEX_COUNT).filter(|i|pool&(1<<i)!=0).collect(),seen:seen&ALL,rollover}
 }
 pub fn next_seen(&self,choice:usize)->Option<u32>{
  self.candidates.contains(&choice).then(||if self.rollover{1<<choice}else{self.seen|(1<<choice)})
 }
}

pub struct Round {
 pub candidates:Vec<usize>,
 seen:u32,
 tail:u32,
 rollover:bool,
}
impl Round {
 pub fn new(unavailable:u32,seen:u32)->Self{
  let available=ALL&!unavailable;let unseen=available&!seen;
  let rollover=unseen.count_ones()<3;
  let tail=if rollover{unseen}else{0};
  let excluded=if rollover{unavailable}else{unavailable|seen};
  let candidates=ORDERS.iter().enumerate().filter_map(|(i,&o)|{
   if !crate::reward_choices::eligible(o,excluded){return None;}
   // Finish the old round first, both in membership and displayed order.
   let tail_len=tail.count_ones() as usize;
   let prefix=o[..tail_len].iter().fold(0,|m,&i|m|(1<<i));
   (prefix==tail).then_some(i)
  }).collect();
  Self{candidates,seen:seen&ALL,tail,rollover}
 }
 pub fn preferred(&self,preferred:[usize;3])->Option<usize>{
  let rank=|i|preferred.iter().position(|&p|p==i).unwrap_or(3+i);
  self.candidates.iter().copied().min_by_key(|&i|ORDERS[i].map(rank))
 }
 pub fn next_seen(&self,choice:usize)->Option<u32>{
  if !self.candidates.contains(&choice){return None;}
  let offered=mask(ORDERS[choice]);
  Some(if self.rollover{offered&!self.tail}else{self.seen|offered})
 }
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn single_refresh_never_repeats_visible_owned_or_excluded_options(){
  let visible=7;let unavailable=(1<<3)|(1<<4);let r=Single::new(unavailable,visible|(1<<5),visible);
  assert!(!r.candidates.is_empty());for &i in &r.candidates{assert_eq!((visible|unavailable|(1<<5))&(1<<i),0);assert_eq!(r.next_seen(i),Some(visible|(1<<5)|(1<<i)));}
 }
 #[test]fn single_refresh_finishes_round_before_reset_and_commits_only_the_new_option(){
  let last=1<<7;let r=Single::new(0,ALL&!last,7);assert_eq!(r.candidates,vec![7]);assert_eq!(r.next_seen(7),Some(ALL));
  let r=Single::new(1<<5,ALL,7);for &i in &r.candidates{assert_eq!((7|(1<<5))&(1<<i),0);assert_eq!(r.next_seen(i),Some(1<<i));}assert_eq!(r.next_seen(0),None);
  assert!(Single::new(ALL&!7,ALL,7).candidates.is_empty());
 }
 #[test]fn every_shown_option_is_excluded_even_when_only_one_was_chosen(){
  let first=Round::new(0,0);let c=first.preferred([0,1,2]).unwrap();let seen=first.next_seen(c).unwrap();assert_eq!(seen,7);
  let next=Round::new(1,seen);assert!(next.candidates.iter().all(|&i|mask(ORDERS[i])&7==0));assert_ne!(next.preferred([0,1,2]),Some(c));
 }
 #[test]fn final_one_or_two_are_shown_first_then_new_round_fills_without_duplicates(){
  for tail in [1<<6,(1<<6)|(1<<8)]{
   let unavailable=(1<<0)|(1<<3);let seen=ALL&!unavailable&!tail;let round=Round::new(unavailable,seen);assert!(!round.candidates.is_empty());
   for &choice in &round.candidates{
    let o=ORDERS[choice];let shown=mask(o);assert_eq!(shown.count_ones(),3);assert_eq!(shown&unavailable,0);assert_eq!(shown&tail,tail);
    let prefix=o[..tail.count_ones()as usize].iter().fold(0,|m,&i|m|(1<<i));assert_eq!(prefix,tail);
    assert_eq!(round.next_seen(choice),Some(shown&!tail));
   }
   let c=round.preferred([0,3,9]).unwrap();assert_eq!(mask(ORDERS[c])&tail,tail);
  }
 }
 #[test]fn exhausted_round_restarts_but_owned_and_role_terrain_exclusions_never_return(){
  let cage=crate::hex_pool::OWNER_IDS.iter().position(|&id|id==95001600).unwrap();
  for hero in 1..=10{
   let excluded=crate::reward_choices::role_exclusions(crate::reward_choices::terrain_exclusions(1,true),Some(hero));
   let round=Round::new(excluded,ALL);assert!(!round.candidates.is_empty());
   for &c in &round.candidates{assert_eq!(mask(ORDERS[c])&excluded,0);assert!(!ORDERS[c].contains(&cage));assert_eq!(round.next_seen(c),Some(mask(ORDERS[c])));}
  }
 }
 #[test]fn unsuccessful_delivery_does_not_mutate_history_or_reset_at_a_boundary(){
  let seen=ALL&!(1<<7);let round=Round::new(0,seen);let c=round.preferred([1,2,3]).unwrap();
  assert_eq!(round.seen,seen);assert_eq!(Round::new(0,seen).candidates,round.candidates);assert_ne!(round.next_seen(c),Some(seen));assert_eq!(round.next_seen(usize::MAX),None);
 }
 #[test]fn full_rounds_visit_every_eligible_item_before_any_return(){
  let unavailable=ALL&!((1<<10)-1);let mut seen=0;let mut visited=0;
  for turn in 0..12{
   let round=Round::new(unavailable,seen);let c=round.preferred([0,1,2]).unwrap();let o=ORDERS[c];
   for i in o{
    if visited==!unavailable&ALL{visited=0;}
    assert_eq!(visited&(1<<i),0,"repeat before exhaustion at turn {turn}");visited|=1<<i;
   }
   seen=round.next_seen(c).unwrap();
  }
 }
 #[test]fn fewer_than_three_unowned_items_cannot_form_duplicate_or_owned_choices(){
  for available in [0,1,3]{let round=Round::new(ALL&!available,ALL);assert!(round.candidates.is_empty());assert_eq!(round.preferred([0,1,2]),None);}
 }
}
