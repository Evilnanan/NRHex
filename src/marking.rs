//! Ironeye's direct Marking hit uses Stamp (Upward Cut)'s native reaction.
use crate::{ptr,read_i32};
pub const OWNER:i32=95004200;
pub const ATTACK:i32=600000110;
pub const IMPACT:u8=9;
pub unsafe fn constructed(hit:usize,context:usize,owned:impl FnOnce()->bool){
 // Caller has already matched context[0] against the local ChrIns. Never
 // change a shared AtkParam, poison-relic follow-up, projectile or other skill.
 if hit!=0&&context!=0&&read_i32(context+8)==ATTACK&&read_i32(context+0xc)==1&&owned(){
  ptr::write((hit+0x24)as*mut u8,IMPACT);
 }
}
pub const CARRIER:i32=OWNER+1;
pub const MARK_HIT:i32=OWNER+2;
pub const BURST:i32=OWNER+3;
pub const BEGIN:i32=OWNER+4;
pub const QUERY:i32=990042;
#[derive(Clone,Copy,Default,PartialEq,Eq,Debug)]
struct Identity{actor:usize,handle:u64,stats:usize}
#[derive(Clone,Copy,Default,PartialEq,Eq,Debug)]
enum Phase{#[default]Idle,Slide,Followup{family:i32,layers:u8},Spent}
#[derive(Default)]struct Combo{owner:Identity,phase:Phase,targets:Vec<Identity>}
impl Combo{
 fn sync(&mut self,owner:Identity){if self.owner!=owner{*self=Self{owner,..Self::default()};}}
 fn begin(&mut self){self.phase=Phase::Slide;self.targets.clear();}
 fn mark(&mut self,target:Identity){if self.phase==Phase::Slide&&!self.targets.contains(&target){self.targets.push(target);}}
 fn event(&mut self,mode:i32){
  let family=mode%10;let layer=if mode>=11{2}else{1};
  let followup=(1..=4).contains(&family)&&(mode==family||mode==family+10);
  self.phase=match self.phase{
   Phase::Slide if followup&&!self.targets.is_empty()=>Phase::Followup{family,layers:layer},
   Phase::Followup{family:old,layers}if followup&&family==old&&layers&layer==0=>Phase::Followup{family,layers:layers|layer},
   _=>Phase::Idle
  };
  if self.phase==Phase::Idle{self.targets.clear();}
 }
 fn hit(&mut self,target:Identity)->bool{
  if !matches!(self.phase,Phase::Followup{..}){return false;}
  self.phase=Phase::Spent;let matched=self.targets.contains(&target);self.targets.clear();matched
 }
}
static COMBO:std::sync::Mutex<Combo>=std::sync::Mutex::new(Combo{owner:Identity{actor:0,handle:0,stats:0},phase:Phase::Idle,targets:Vec::new()});
unsafe fn identity(actor:usize)->Identity{Identity{actor,handle:ptr::read_unaligned((actor+8)as*const u64),stats:crate::native_actor::stats(actor)}}
unsafe fn owned(player:usize,base:usize)->Option<Identity>{
 if player==0||!crate::is_local(player,base)||!crate::has_effect(player,OWNER,base)||crate::has_effect(player,crate::reward_cycle_runtime::RESET,base){return None;}
 let id=identity(player);(id.stats!=0&&!crate::native_actor::incapacitated(player,id.stats)).then_some(id)
}
pub fn reset(){if let Ok(mut s)=COMBO.lock(){*s=Combo::default();}}
pub unsafe fn update(player:usize,base:usize){if let Some(owner)=owned(player,base){if let Ok(mut s)=COMBO.lock(){s.sync(owner);}}else{reset();}}
pub unsafe fn pulse(player:usize,effect:i32,base:usize)->bool{
 if effect!=BEGIN{return false;}
 if let Some(owner)=owned(player,base){if let Ok(mut s)=COMBO.lock(){s.sync(owner);s.begin();}}true
}
pub unsafe fn event(player:usize,mode:i32,base:usize){
 if let Some(owner)=owned(player,base){if let Ok(mut s)=COMBO.lock(){s.sync(owner);s.event(mode);}}
}
pub unsafe fn inapplicable(node:usize,descriptor:usize)->bool{
 if node==0||descriptor==0{return false;}let row=crate::read_ptr(node);if row==0||read_i32(row+0x12c)!=CARRIER{return false;}
 let table=read_i32(descriptor+0x38);let attack=read_i32(descriptor+0x34);
 if table==1&&attack==ATTACK{return false;}
 let mask=ptr::read_unaligned((descriptor+4)as*const[u32;8]);
 !matches!(ptr::read(descriptor as*const u8),1|2)||mask[4]&2==0||table==1&&matches!(attack,300..=307|310..=313|322|550|551|560|561)
}
pub unsafe fn route(target:usize,effect:i32,source:usize,context:usize,base:usize)->Option<i32>{
 let owner=owned(source,base)?;
 if target==0||target==source||context==0{return None;}
 let vt=crate::read_ptr(target);if vt==0{return None;}let getter=crate::read_ptr(vt+0x10);if getter==0{return None;}
 let kind:unsafe extern "system" fn(usize)->i32=std::mem::transmute(getter);if kind(target)!=1{return None;}
 let id=identity(target);if id.stats==0||read_i32(id.stats+0x140)<=0{return None;}
 let mut s=COMBO.lock().ok()?;s.sync(owner);
 if effect==MARK_HIT{s.mark(id);None}else if effect==CARRIER&&s.hit(id){Some(BURST)}else{None}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn only_owned_pc_marking_changes_the_outgoing_reaction(){unsafe{
  for owned in [false,true]{for attack in [ATTACK,600000120,300000290,0,-1]{for table in [0,1,2]{
   let mut context=[0u32;8];context[2]=attack as u32;context[3]=table;
   for original in [0,1,2,3,7,9,255]{
    let mut hit=[0xabu8;0x300];hit[0x24]=original;let before=hit;
    constructed(hit.as_mut_ptr()as usize,context.as_ptr()as usize,||owned);
    for i in 0..hit.len(){assert_eq!(hit[i],if i==0x24&&owned&&attack==ATTACK&&table==1{IMPACT}else{before[i]});}
   }
  }}}
 }}
 fn id(actor:usize)->Identity{Identity{actor,handle:actor as u64,stats:actor+100}}
 #[test]fn followup_is_one_attack_and_only_the_slid_target_can_burst(){
  for mode in 1..=4{
   let mut s=Combo::default();s.sync(id(1));s.begin();s.mark(id(2));s.mark(id(2));assert_eq!(s.targets.len(),1);
   s.event(mode);s.event(mode+10);assert!(s.hit(id(2)));assert!(!s.hit(id(2)));
   s.begin();s.mark(id(2));s.event(mode);assert!(!s.hit(id(3)));assert!(!s.hit(id(2)));
   s.begin();s.mark(id(2));s.event(mode);s.event(0);assert!(!s.hit(id(2)));
   s.begin();s.mark(id(2));s.event(0);s.event(mode);assert!(!s.hit(id(2)));
   s.begin();s.mark(id(2));s.event(mode);s.event(mode);assert!(!s.hit(id(2)));
   // An unhit new slide replaces the previous target set, including misses.
   s.begin();s.mark(id(2));s.begin();s.event(mode);assert!(!s.hit(id(2)));
  }
 }
 #[test]fn no_ownership_identity_or_target_generation_can_be_reused(){
  let mut s=Combo::default();s.sync(id(1));s.begin();s.mark(id(2));s.event(1);
  assert!(!s.hit(Identity{handle:999,..id(2)}));
  s.begin();s.mark(id(2));s.event(1);s.sync(Identity{stats:999,..id(1)});assert!(!s.hit(id(2)));
  s.begin();s.mark(id(2));s.event(1);s.sync(Identity{handle:999,..id(1)});assert!(!s.hit(id(2)));
 }
 #[test]fn carrier_accepts_native_backstep_category_and_only_real_marking_contacts(){unsafe{
  let mut row=[0u64;0x400/8];let rp=row.as_mut_ptr()as usize;ptr::write_unaligned((rp+0x12c)as*mut i32,CARRIER);let node=[rp];
  let mut descriptor=[0u8;0x40];let d=descriptor.as_mut_ptr()as usize;descriptor[0]=1;
  for category in 0..256{
   descriptor[4..36].fill(0);descriptor[4+category/8]|=1<<(category%8);ptr::write_unaligned((d+0x34)as*mut i32,10000);ptr::write_unaligned((d+0x38)as*mut i32,1);
   assert_eq!(inapplicable(node.as_ptr()as usize,d),category!=129);
  }
  for attack in [ATTACK,600000120,304]{ptr::write_unaligned((d+0x34)as*mut i32,attack);descriptor[4..36].fill(0);assert_eq!(inapplicable(node.as_ptr()as usize,d),attack!=ATTACK);}
  ptr::write_unaligned((d+0x34)as*mut i32,10000);descriptor[20]=2;
  for kind in 0..12{descriptor[0]=kind;assert_eq!(inapplicable(node.as_ptr()as usize,d),!matches!(kind,1|2));}
 }}
 #[test]fn only_ironeye_draws_marking_even_after_affinity_overrides(){
  let index=crate::hex_pool::OWNER_IDS.iter().position(|&id|id==OWNER).unwrap();
  for hero in std::iter::once(None).chain((0..=10).map(Some)){for affinity in [None,Some(1100),Some(57),Some(61),Some(1600),Some(1700)]{for party in [None,Some(1),Some(2),Some(3)]{
   let excluded=crate::reward_choices::party_exclusions(crate::reward_choices::weapon_exclusions(crate::reward_choices::role_exclusions(Default::default(),hero),affinity),party);
   assert_eq!(!excluded.contains(&index),hero==Some(3));
   let round=crate::reward_cycle::Round::new(&excluded,&Default::default());assert_eq!(round.candidates.contains(&index),hero==Some(3));
   assert_eq!(crate::reward_cycle::Single::new(&excluded,&Default::default(),&crate::reward_choices::mask([0,1])).candidates.contains(&index),hero==Some(3));
  }}}
  assert_eq!(crate::hex_pool::HISTORY_BITS[index],39);
 }
}
#[cfg(test)]#[path="marking_native_tests.rs"]mod native_tests;

#[cfg(test)]#[path="marking_poison_native_tests.rs"]mod poison_native_tests;
