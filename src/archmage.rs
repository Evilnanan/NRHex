//! Per-instance secondary spells. Primary spells, inventory and drops stay original.
use std::collections::BTreeMap;

pub const OWNER:i32=95002100;
pub const MARKER:i32=6999936;
pub const BUSY:i32=6999937;
pub const LEFT_0:i32=6999938;
pub const LEFT_1:i32=6999939;
pub const RIGHT_0:i32=6999940;
pub const RIGHT_1:i32=6999941;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Catalyst {pub handle:u32,pub base:i32,pub original:[i32;2]}
#[derive(Clone,Copy,Debug)]
struct Entry {catalyst:Catalyst,spells:[i32;2]}
#[derive(Default)]
pub struct Archmage {
 entries:BTreeMap<u32,Entry>,drops:BTreeMap<u32,Entry>,pending:BTreeMap<(u32,usize),Catalyst>,
 context:usize,owned:bool,busy:bool,quiet:u8,dirty:bool,
}
pub fn kind(id:i32)->Option<u8>{crate::archmage_data::SPELLS.binary_search_by_key(&id,|s|s.0).ok().map(|i|crate::archmage_data::SPELLS[i].1)}
pub fn pool(original:i32)->Vec<i32>{
 let Some(kind)=kind(original) else{return vec![];};
 crate::archmage_data::SPELLS.iter().filter_map(|&(id,k)|(k==kind&&crate::archmage_data::RANDOM_SPELLS.binary_search(&id).is_ok()).then_some(id)).collect()
}
pub fn pulse(effect:i32)->Option<(i32,usize)>{match effect{LEFT_0=>Some((0,0)),LEFT_1=>Some((0,1)),RIGHT_0=>Some((1,0)),RIGHT_1=>Some((1,1)),_=>None}}
impl Archmage {
 pub fn synchronize(&mut self,context:usize,owned:bool,alive:bool){
  if self.context!=context {self.dirty=false;}
  else if !owned&&!self.entries.is_empty(){self.dirty=true;}
  if self.context!=context||!owned {self.entries.clear();self.drops.clear();self.pending.clear();self.busy=false;self.quiet=0;}
  self.context=context;self.owned=owned;
  if !alive{self.pending.clear();self.busy=false;self.quiet=0;}
 }
 pub fn selected(&self,c:Catalyst)->Option<[i32;2]>{
  if !self.owned{return None;}
  self.entries.get(&c.handle).filter(|e|e.catalyst==c).map(|e|e.spells)
 }
 pub fn notify(&mut self,used:Option<(Catalyst,usize)>){
  if !self.owned{return;}
  self.busy=true;self.quiet=0;
  if let Some((c,slot))=used{
   if slot==1&&kind(c.original[slot]).is_some(){self.pending.insert((c.handle,slot),c);}
  }
 }
 pub fn tick(&mut self)->Vec<(Catalyst,usize)>{
  if self.busy{self.busy=false;self.quiet=0;return vec![];}
  self.quiet=self.quiet.saturating_add(1);
  if self.quiet<2{return vec![];}
  std::mem::take(&mut self.pending).into_iter().map(|((_,slot),c)|(c,slot)).collect()
 }
 pub fn drop_boundary(&mut self,c:Catalyst)->Vec<(Catalyst,usize)>{
  // Dropping the casting catalyst ends its chain even during recovery.
  // Resolve before recording the ground UID so dropping cannot cancel a roll.
  (1..2).filter_map(|slot|self.pending.remove(&(c.handle,slot)).filter(|pending|*pending==c).map(|c|(c,slot))).collect()
 }
 pub fn reroll(&mut self,c:Catalyst,slot:usize,spell:i32)->bool{
  if !self.owned||slot!=1||crate::archmage_data::RANDOM_SPELLS.binary_search(&spell).is_err()||kind(spell).is_none()||kind(spell)!=kind(c.original[slot]){return false;}
  let mut spells=self.selected(c).unwrap_or(c.original);spells[slot]=spell;
  self.entries.insert(c.handle,Entry{catalyst:c,spells});self.dirty=true;true
 }
 pub fn dropped(&mut self,uid:u32,c:Catalyst){
  if uid==0{return;}
  if let Some(spells)=self.selected(c){self.drops.insert(uid,Entry{catalyst:c,spells});}
 }
 pub fn picked_up(&mut self,uid:u32,c:Catalyst)->bool{
  if !self.owned{return false;}
  let Some(e)=self.drops.get(&uid).copied() else{return false;};
  if e.catalyst.base!=c.base||e.catalyst.original!=c.original{return false;}
  self.entries.insert(c.handle,Entry{catalyst:c,spells:e.spells});self.dirty=true;true
 }
 pub fn network_pending(&self)->bool{self.dirty}
 pub fn network_sent(&mut self){self.dirty=false;}
}
pub fn snapshot(data:&mut [u8;0x48],c:Catalyst,spells:[i32;2]){
 if i32::from_le_bytes(data[..4].try_into().unwrap())!=c.base{return;}
 for slot in 1..2{
  if kind(spells[slot]).is_some()&&kind(spells[slot])==kind(c.original[slot]){
   let offset=0x14+4*slot;data[offset..offset+4].copy_from_slice(&spells[slot].to_le_bytes());
  }
 }
}

#[cfg(test)]mod tests{
 use super::*;
 fn catalyst(handle:u32)->Catalyst{Catalyst{handle,base:33000000,original:[4000,4001]}}
 fn state()->Archmage{let mut s=Archmage::default();s.synchronize(1,true,true);s}
 #[test]fn continuous_and_charged_casts_only_roll_used_slots_after_whole_chain(){
  let mut s=state();let c=catalyst(1);
  for _ in 0..100{s.notify(None);assert!(s.tick().is_empty());}
  assert!(s.tick().is_empty());assert!(s.tick().is_empty());
  for _ in 0..100{s.notify(Some((c,1)));assert!(s.tick().is_empty());assert_eq!(s.selected(c),None);}
  assert!(s.tick().is_empty());assert_eq!(s.tick(),[(c,1)]);assert!(s.tick().is_empty());
  assert!(s.reroll(c,1,4010));assert_eq!(s.selected(c),Some([4000,4010]));
  s.notify(Some((c,0)));s.notify(Some((c,1)));s.tick();s.tick();assert_eq!(s.tick(),[(c,1)]);
 }
 #[test]fn identical_and_duplicate_spell_catalysts_track_handle_and_slot_independently(){
  let mut s=state();let a=Catalyst{original:[4000,4000],..catalyst(1)};let b=Catalyst{handle:2,..a};
  assert!(s.reroll(a,1,4010));assert!(s.reroll(b,1,4020));
  assert_eq!(s.selected(a),Some([4000,4010]));assert_eq!(s.selected(b),Some([4000,4020]));
  assert_eq!(s.selected(Catalyst{base:33040000,..a}),None);
  assert_eq!(s.selected(Catalyst{original:[4000,4001],..a}),None);
 }
 #[test]fn drop_pickup_restores_only_exact_uid_and_survives_death_without_reroll_exploit(){
  let mut s=state();let a=catalyst(1);let b=catalyst(2);
  assert!(!s.reroll(a,0,4010));s.reroll(a,1,4020);s.reroll(b,1,4030);
  s.dropped(100,a);s.dropped(200,b);s.synchronize(1,true,false);
  let a2=catalyst(3);let b2=catalyst(4);
  assert!(s.picked_up(200,b2));assert!(s.picked_up(100,a2));
  assert_eq!(s.selected(a2),Some([4000,4020]));assert_eq!(s.selected(b2),Some([4000,4030]));
  s.reroll(a2,1,4040);s.dropped(300,a2);let a3=catalyst(5);assert!(s.picked_up(300,a3));
  assert_eq!(s.selected(a3),Some([4000,4040]));
  assert!(!s.picked_up(100,Catalyst{base:34000000,..catalyst(6)}));
  assert!(!s.picked_up(100,Catalyst{original:[4001,4000],..catalyst(6)}));
  assert!(!s.picked_up(999,catalyst(6)));
  s.notify(Some((a3,0)));s.synchronize(1,true,false);assert!(s.tick().is_empty());assert!(s.tick().is_empty());
  s.synchronize(2,true,true);assert_eq!(s.selected(a3),None);assert!(!s.picked_up(300,catalyst(6)));
 }
 #[test]fn kinds_empty_slots_and_skill_slot_never_cross_or_enter_random_pool(){
  let mut s=state();let c=catalyst(1);
  assert!(!s.reroll(c,1,6000));assert!(!s.reroll(c,2,4001));assert!(!s.reroll(c,1,8000));
  let empty=Catalyst{original:[4000,-1],..c};s.notify(Some((empty,1)));s.notify(Some((empty,2)));s.tick();s.tick();assert!(s.tick().is_empty());
  assert!(pool(4000).iter().all(|&i|kind(i)==Some(0)));assert!(pool(6000).iter().all(|&i|kind(i)==Some(1)));assert!(pool(-1).is_empty());
  assert_eq!(pulse(LEFT_1),Some((0,1)));assert_eq!(pulse(RIGHT_0),Some((1,0)));assert_eq!(pulse(BUSY),None);
 }
 #[test]fn enchantments_are_never_random_choices_but_original_enchantment_slots_can_still_reroll(){
  let excluded=[4460,4470,4490,6250,6320,6740,6770,6960,7230];
  for original in [4000,6000,4460,4470,6740]{
   let choices=pool(original);assert!(!choices.is_empty());
   assert!(choices.iter().all(|id|!excluded.contains(id)&&kind(*id)==kind(original)));
  }
  let mut s=state();let c=Catalyst{original:[4460,4470],..catalyst(1)};
  assert_eq!(kind(4460),Some(0));assert_eq!(kind(6740),Some(1));
  s.notify(Some((c,1)));s.tick();s.tick();assert_eq!(s.tick(),[(c,1)]);
  assert!(s.reroll(c,1,4000));assert_eq!(s.selected(c),Some([4460,4000]));
  for id in excluded{assert!(!s.reroll(c,1,id));}
 }
 #[test]fn dropping_during_recovery_consumes_only_that_catalysts_pending_roll(){
  let mut s=state();let a=catalyst(1);let b=catalyst(2);
  s.notify(Some((a,0)));s.notify(Some((a,1)));s.notify(Some((b,1)));
  assert_eq!(s.drop_boundary(a),[(a,1)]);assert!(s.drop_boundary(a).is_empty());
  s.reroll(a,0,4010);s.reroll(a,1,4020);s.dropped(123,a);assert!(s.picked_up(123,catalyst(3)));
  assert_eq!(s.selected(catalyst(3)),Some([4000,4020]));
  s.tick();s.tick();assert_eq!(s.tick(),[(b,1)]);
 }
 #[test]fn snapshot_changes_only_the_secondary_native_magic_field(){
  let c=catalyst(1);let mut data=[0x67;0x48];data[..4].copy_from_slice(&c.base.to_le_bytes());let before=data;
  let mut expected=before;expected[0x18..0x1c].copy_from_slice(&4020i32.to_le_bytes());
  snapshot(&mut data,c,[4010,4020]);assert_eq!(data,expected);
  let mut wrong=before;snapshot(&mut wrong,Catalyst{base:34000000,..c},[4010,4020]);assert_eq!(wrong,before);
  let mut invalid=before;snapshot(&mut invalid,c,[6000,-1]);assert_eq!(invalid,before);
 }
 #[test]fn losing_ownership_restores_network_state_and_clears_drops(){
  let mut s=state();let c=catalyst(1);s.reroll(c,1,4010);s.dropped(123,c);s.network_sent();
  s.synchronize(1,false,true);assert!(s.network_pending());assert_eq!(s.selected(c),None);
  s.network_sent();s.synchronize(1,true,true);assert!(!s.picked_up(123,catalyst(2)));
 }
 #[test]fn nt_flagged_incantations_are_recognized_and_reroll_after_the_chain(){
  for original in [6900,7000,6110] { // Lightning Spear, Dragonfire, Flame of the Fell God.
   let mut s=state();let c=Catalyst{original:[original,original],..catalyst(1)};
   assert_eq!(kind(original),Some(1));assert!(pool(6000).contains(&original));
   s.notify(Some((c,0)));s.tick();s.tick();assert!(s.tick().is_empty());
   for _ in 0..30 {s.notify(Some((c,1)));assert!(s.tick().is_empty());assert_eq!(s.selected(c),None);}
   assert!(s.tick().is_empty());assert_eq!(s.tick(),[(c,1)]);
   assert!(s.reroll(c,1,6000));assert_eq!(s.selected(c),Some([original,6000]));
   assert!(!s.reroll(c,1,4000));assert!(!s.reroll(c,1,6250));
   s.dropped(123,c);let picked=Catalyst{handle:2,..c};assert!(s.picked_up(123,picked));
   assert_eq!(s.selected(picked),Some([original,6000]));
  }
 }
 #[test]fn primary_casts_never_roll_even_with_identical_spell_ids(){
  let mut s=state();let c=Catalyst{original:[4000,4000],..catalyst(1)};
  for _ in 0..30{s.notify(Some((c,0)));s.tick();}s.tick();assert!(s.tick().is_empty());
  assert!(!s.reroll(c,0,4010));assert_eq!(s.selected(c),None);
  s.notify(Some((c,1)));s.tick();s.tick();assert_eq!(s.tick(),[(c,1)]);
  assert!(s.reroll(c,1,4010));assert_eq!(s.selected(c),Some([4000,4010]));
 }

}
