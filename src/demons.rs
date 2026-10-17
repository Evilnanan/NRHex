//! Value-only runtime rolls. The native 48-byte summon descriptor transports
//! one seed; lookup order, client timing and later RNG calls cannot change it.
use crate::demons_data::{LEFT_TYPES, MAIN_TYPES, WEAPONS};
pub const OWNER:i32=95003300;
pub const DRAIN:i32=95003301;
pub const COOLDOWN:i32=95003302;
pub const FIRST_NPC:i32=98300000;
pub const SEEDS:u32=1<<19;
const FIRST_CI:i32=1_000_000_000;
const FIRST_WEAPON:i32=100_000_000;
pub struct Weapon{pub id:i32,pub kind:i32,pub art:i32,pub spell:i32,pub primary:i32,pub art_table:i32,pub spell_table:i32}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Hand{pub source:i32,pub art:i32,pub spell:i32}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Roll{pub hero:u8,pub skin:u8,pub right:Hand,pub left:Option<Hand>}
struct Rng(u64);
impl Rng{
 fn next(&mut self)->u64{self.0=self.0.wrapping_add(0x9e3779b97f4a7c15);let mut z=self.0;z=(z^(z>>30)).wrapping_mul(0xbf58476d1ce4e5b9);z=(z^(z>>27)).wrapping_mul(0x94d049bb133111eb);z^(z>>31)}
 fn bounded(&mut self,n:u64)->u64{assert!(n>0);let reject=n.wrapping_neg()%n;loop{let value=self.next();if value>=reject{return value%n;}}}
 fn hand(&mut self,preferred:i32)->Hand{
  let same=self.bounded(5)!=0;
  let eligible=|w:&&Weapon| (w.kind==preferred)==same;
  let count=WEAPONS.iter().filter(eligible).count();
  let w=WEAPONS.iter().filter(eligible).nth(self.bounded(count as u64)as usize).unwrap();
  Hand{source:w.id,art:w.art,spell:w.spell}
 }
}
pub fn roll(seed:u32)->Option<Roll>{
 if seed>=SEEDS{return None;}
 let mut rng=Rng(u64::from(seed)^0xd8f2_b9a4_6143_7721);let hero=rng.bounded(10)as u8;let skin=rng.bounded(6)as u8;
 let right=rng.hand(MAIN_TYPES[hero as usize]);let left=(LEFT_TYPES[hero as usize]>=0).then(||rng.hand(LEFT_TYPES[hero as usize]));
 Some(Roll{hero,skin,right,left})
}
pub fn chara_id(seed:u32)->Option<i32>{(seed<SEEDS).then_some(FIRST_CI+seed as i32)}
pub fn seed_from_chara(id:i32)->Option<u32>{let seed=id.checked_sub(FIRST_CI)?;(seed>=0&&(seed as u32)<SEEDS).then_some(seed as u32)}
pub fn weapon_id(seed:u32,left:bool)->Option<i32>{(seed<SEEDS).then_some(FIRST_WEAPON+seed as i32*200+i32::from(left)*100)}
pub fn seed_from_weapon(id:i32)->Option<(u32,bool)>{
 let value=id.checked_sub(FIRST_WEAPON)?;
 if value<0||value>=SEEDS as i32*200{return None;}
 Some(((value/200)as u32,value%200>=100))
}
pub fn is_npc(id:i32)->bool{(FIRST_NPC..FIRST_NPC+60).contains(&id)}
impl Roll{
 pub fn npc(self)->i32{FIRST_NPC+i32::from(self.hero)*6+i32::from(self.skin)}
 pub fn source_chara(self)->i32{35000+i32::from(self.hero)*100+i32::from(self.skin)}
}

#[derive(Default)]
pub struct Summoning{
 pub live:u64,
 pub cooldown:f32,
 madness:bool,
 // A consumed madness may retry placement briefly, never accumulate queues.
 pending:Option<u32>,
 retry:f32,
 placement_left:f32,
 pub cooldown_dirty:bool,
}
impl Summoning{
 pub fn observe(&mut self,owned:bool,madness:bool,alive:bool,native_cooldown:bool,dt:f32,seed:u32)->Option<u32>{
  let dt=if dt.is_finite()&&dt>0.{dt}else{0.};
  self.cooldown=(self.cooldown-dt).max(0.);self.retry=(self.retry-dt).max(0.);self.placement_left=(self.placement_left-dt).max(0.);
  let rising=madness&&!self.madness;self.madness=madness;
  if !owned||!alive{self.pending=None;return None;}
  if self.live!=0||self.cooldown>0.||native_cooldown{self.pending=None;return None;}
  if rising{self.pending=Some(seed);self.retry=0.;self.placement_left=3.;}
  if self.placement_left<=0.{self.pending=None;}
  if self.retry<=0.{if let Some(seed)=self.pending{self.retry=0.25;return Some(seed);}}
  None
 }
 pub fn created(&mut self,handle:u64){if handle!=0&&handle!=u64::MAX{self.live=handle;self.pending=None;}}
 pub fn retired(&mut self,handle:u64){if self.live==handle&&handle!=0{self.live=0;self.cooldown=60.;self.cooldown_dirty=true;self.pending=None;}}
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn runtime_roles_skins_and_weapons_replay_with_native_pool_references(){
  let mut heroes=[0u32;10];let mut skins=[[false;6];10];let mut preferred=0;let count=40000;
  for seed in 0..count{
   let a=roll(seed).unwrap();assert_eq!(a,roll(seed).unwrap());heroes[a.hero as usize]+=1;skins[a.hero as usize][a.skin as usize]=true;
   for h in std::iter::once(a.right).chain(a.left){let w=WEAPONS.iter().find(|w|w.id==h.source).unwrap();assert_eq!((h.art,h.spell),(w.art,w.spell));assert!(w.art_table>=-1&&w.spell_table>=-1);}
   preferred+=u32::from(WEAPONS.iter().find(|w|w.id==a.right.source).unwrap().kind==MAIN_TYPES[a.hero as usize]);
  }
  assert!(heroes.iter().all(|&n|(3500..4500).contains(&n)));assert!(skins.into_iter().flatten().all(|v|v));assert!((31000..33000).contains(&preferred));
 }
 #[test]fn virtual_ids_roundtrip_all_seeds_with_native_item_mask_and_reinforcement_suffixes(){
  for seed in 0..SEEDS{
   assert_eq!(seed_from_chara(chara_id(seed).unwrap()),Some(seed));
   for left in [false,true]{let id=weapon_id(seed,left).unwrap();assert!(id<0x10000000);assert_eq!(id%100,0);for level in [0,1,25,99]{assert_eq!(seed_from_weapon(id+level),Some((seed,left)));}}
  }
  assert_eq!(roll(SEEDS),None);assert_eq!(seed_from_chara(-1),None);assert_eq!(seed_from_weapon(99030000),None);assert_eq!(seed_from_weapon(FIRST_WEAPON+SEEDS as i32*200),None);
 }
 #[test]fn one_madness_one_sinner_and_cooldown_starts_only_on_death(){
  let mut s=Summoning::default();assert_eq!(s.observe(true,false,true,false,1.,10),None);
  assert_eq!(s.observe(true,true,true,false,0.1,10),Some(10));s.created(77);
  for i in 0..120{assert_eq!(s.observe(true,i%2==0,true,false,1.,11),None);assert_eq!(s.cooldown,0.);}
  s.retired(78);assert_eq!(s.live,77);s.retired(77);assert_eq!(s.cooldown,60.);assert!(s.cooldown_dirty);
  for i in 0..59{assert_eq!(s.observe(true,i%2==0,true,false,1.,11),None);}
  // Expiring cooldown cannot replay madness that already happened.
  assert_eq!(s.observe(true,true,true,false,1.,11),None);
  s.observe(true,false,true,false,0.1,11);assert_eq!(s.observe(true,true,true,false,0.1,11),Some(11));
 }
 #[test]fn placement_retries_keep_same_roll_but_death_and_expiry_cancel_it(){
  let mut s=Summoning::default();assert_eq!(s.observe(true,true,true,false,0.1,1),Some(1));assert_eq!(s.observe(true,true,true,false,0.25,2),Some(1));
  assert_eq!(s.observe(true,true,true,false,3.,3),None);s.observe(true,false,true,false,0.1,3);
  assert_eq!(s.observe(true,true,true,false,0.1,3),Some(3));assert_eq!(s.observe(true,true,false,false,0.1,3),None);
  assert_eq!(s.observe(true,true,true,false,0.3,3),None);
 }
}
