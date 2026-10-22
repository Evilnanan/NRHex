use std::collections::BTreeMap;
use crate::sword_map;
#[cfg(test)]
#[path = "sword_csv.rs"]
mod sword_csv;
include!(concat!(env!("OUT_DIR"), "/sword_bonuses.rs"));
pub const OWNER:i32=95000900;
pub const CAST_OWNER:i32=95000912; // HKS f32 IDs must be divisible by eight.
pub const CAST_PULSE:i32=95000904;
pub const CAST_RIGHT:i32=95000944;
pub const CAST_LEFT:i32=95000952;
pub const RECOVERY_RIGHT:i32=95000960;
pub const RECOVERY_LEFT:i32=95000968;
pub const WEAPON_RIGHT:i32=95000940;
pub const WEAPON_LEFT:i32=95000941;
pub const GUARD:i32=95000908; // Native int only; not passed through HKS f32.
pub const POISE:i32=95000916;
pub const LIFESTEAL_SCALE:f64=2.;
pub const NEAR_DISTANCE:f32=5.;
pub const CASTER_HEIGHT:f32=1.; // Mighty + Sword ordinary projectile synergy only.

pub struct Bonuses {percentages:BTreeMap<i32,f32>}
impl Bonuses {
 pub fn defaults()->Self{Self{percentages:COMPILED_BONUSES.iter().copied().collect()}}
 #[cfg(test)]
 pub fn read(text:&str)->Result<Self,String>{
  sword_csv::read(text,sword_map::DEFAULT_BONUSES.iter().map(|&(id,_)|id)).map(|percentages|Self{percentages})
 }
 pub fn for_magic(&self,id:i32)->Option<f32>{self.percentages.get(&id).copied()}
 pub fn for_attack(&self,attack:i32,kind:i32)->Option<f32>{
  if kind!=1{return None;} // Verified live: player Glintstone Pebble is kind 1.
  let i=sword_map::ATTACKS.binary_search_by_key(&attack,|a|a.0).ok()?;
  // Shared attack IDs use the smallest percentage of their mapped spells.
  Some(sword_map::ATTACKS[i].1.iter().map(|id|self.percentages[id]).fold(f32::INFINITY,f32::min))
 }
 pub fn factor(&self,attack:i32,kind:i32)->Option<f32>{self.for_attack(attack,kind).map(|p|1.+p/100.)}

}
pub fn distance(a:[f32;4],b:[f32;4])->Option<f32>{
 let d=(0..3).map(|i|(a[i]-b[i]).powi(2)).sum::<f32>().sqrt();
 d.is_finite().then_some(d)
}
// The physics root is at the feet. Measure from the torso so an elevated
// contact does not spend part of the melee range on the caster's own height.
// Keep world-space height and the real contact point; do not flatten the range.
pub fn hit_distance(mut feet:[f32;4],contact:[f32;4])->Option<f32>{
 feet[1]+=CASTER_HEIGHT;
 distance(feet,contact)
}
pub fn near(distance:Option<f32>,limit:f32)->bool{distance.is_some_and(|d|d<=limit)}
pub fn boosted(damage:i32,factor:f32)->i32{((damage as f64*factor as f64).round()).clamp(0.,i32::MAX as f64) as i32}
// Native categories are a 256-bit mask. Add tags while retaining all spell tags.
pub fn add_melee_category(mask:&mut [u32;8]){
 let category=130usize;mask[category/32]|=1u32<<(category%32);
 if mask[110/32]&(1<<(110%32))!=0{mask[100/32]|=1<<(100%32);}
}
// Retry only category-restricted melee effects on tagged spell descriptors.
// Native apply-type 3/4 filtering otherwise rejects those effects before tags.
pub fn extra_melee_application(apply_type:u8,mask:&[u32;8],filters:[u8;3])->bool{
 let contains=|category:u32|mask[category as usize/32]&(1<<(category%32))!=0;
 matches!(apply_type,3|4)&&contains(130)
  &&(filters.contains(&130)||(contains(110)&&contains(100)&&filters.contains(&100)))
}
pub fn reduced(damage:i32,percentage:f32)->i32{
 if damage<=0{return damage;}
 if !percentage.is_finite() || percentage<0. {return damage;}
 // Multiply before dividing so whole percentages such as 35/70 do not
 // lose an extra point to rounding a precomputed binary fraction.
 let remaining=(100.-percentage as f64).clamp(0.,100.);
 (damage as f64*remaining/100.).floor().clamp(0.,i32::MAX as f64) as i32
}
pub fn heal_amount(damage:i32,percentage:f32,nullified:bool)->u32{
 // Like native Soulblood Song, heal from completed hit damage, even when
 // no-death/locked HP prevents HP loss. Glancing/immune hits still cannot heal.
 if nullified||damage<=0{return 0;}
 (damage as f64*percentage as f64*LIFESTEAL_SCALE/100.).floor().clamp(0.,65535.) as u32
}

pub fn casting(player:usize,owned:bool,alive:bool,pulse_player:usize,last:u64,now:u64)->bool{
 owned&&alive&&player!=0&&player==pulse_player&&last!=0&&now>=last&&now-last<100
}
pub fn regain_marker_change(owned:bool,alive:bool,present:bool)->Option<bool>{
 if !owned&&present {Some(false)}
 else if owned&&alive&&!present {Some(true)}
 else {None}
}
pub fn casting_effect(id:i32,casting:bool,_hand:i32)->bool{
 // Retired hand visuals stay in the runtime cleanup list, but are never applied.
 casting&&matches!(id,GUARD|POISE)
}
// Recovery getters may already expose the queued spell or no active spell.
// Retain the spell that actually cast; never revive an expired/other actor cast.
pub fn pulse_magic(recovery:bool,continuous:bool,previous:i32,current:i32)->Option<i32>{
 if recovery{return (continuous&&previous>=0).then_some(previous);}
 Some(if current>=0 {current} else if continuous {previous} else {-1})
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn regain_marker_is_not_removed_at_zero_hp_or_readded_while_incapacitated(){
  for owned in [false,true]{for alive in [false,true]{for present in [false,true]{
   let expected=match (owned,alive,present){
    (false,_,true)=>Some(false),
    (true,true,false)=>Some(true),
    _=>None,
   };
   assert_eq!(regain_marker_change(owned,alive,present),expected);
  }}}
 }
 #[test]fn regain_lifecycle_keeps_native_cleanup_separate_from_casting_protection(){
  let mut present=false;
  for (owned,alive,native_cleanup,expected) in [
   (true,true,false,true),
   (true,false,false,true),
   (true,false,true,false),
   (true,false,false,false),
   (true,true,false,true),
   (false,true,false,false),
   (false,false,false,false),
  ]{
   if native_cleanup {present=false;}
   if let Some(desired)=regain_marker_change(owned,alive,present){present=desired;}
   assert_eq!(present,expected);
   let active=casting(1,owned,alive,1,100,101);
   for effect in [GUARD,POISE]{assert_eq!(casting_effect(effect,active,0),owned&&alive);}
  }
 }
 #[test]fn recovery_retains_cast_spell_without_reviving_stale_protection(){
  assert_eq!(pulse_magic(true,true,4000,5000),Some(4000));
  assert_eq!(pulse_magic(true,true,4000,-1),Some(4000));
  assert_eq!(pulse_magic(true,false,4000,4000),None);
  assert_eq!(pulse_magic(true,true,-1,4000),None);
  assert_eq!(pulse_magic(false,true,4000,-1),Some(4000));
  assert_eq!(pulse_magic(false,true,4000,5000),Some(5000));
  assert_eq!(pulse_magic(false,false,4000,-1),Some(-1));
 }
 #[test]fn casting_keeps_protection_without_either_retired_hand_visual(){
  for hand in [-1,0,1] {
   for (owned,alive,player,last,now) in [(true,true,1,100,199),(false,true,1,100,199),(true,false,1,100,199),(true,true,2,100,199),(true,true,1,100,200)]{
    let active=casting(player,owned,alive,1,last,now);
    assert!(!casting_effect(WEAPON_RIGHT,active,hand));
    assert!(!casting_effect(WEAPON_LEFT,active,hand));
    assert_eq!(casting_effect(GUARD,active,hand),active);
    assert_eq!(casting_effect(POISE,active,hand),active);
    assert!(!casting_effect(OWNER,active,hand));
   }
  }
 }
 #[test]fn linear_reduction_reference_points_and_monotonicity(){
  for (p,expected) in [(0.,100000),(25.,75000),(35.,65000),(50.,50000),(60.,40000),(70.,30000),(100.,0),(200.,0)] {
   assert_eq!(reduced(100000,p),expected);
  }
  let mut previous=100000;
  for p in 0..=1000 {let current=reduced(100000,p as f32);assert!(current<=previous&&current>=0);previous=current;}
  for p in 0..=100{assert_eq!(reduced(100,p as f32),100-p);}
  for p in [-1.,f32::NAN,f32::INFINITY] {assert_eq!(reduced(100,p),100);}
 }
 #[test]fn extra_melee_application_only_allows_tagged_magic_or_prayer_and_relevant_category_filters(){
  let mut mask=[0;8];add_melee_category(&mut mask);
  for kind in [3,4]{assert!(extra_melee_application(kind,&mask,[130,0,0]));}
  for kind in [0,1,2,5,9,10,11,12,255]{assert!(!extra_melee_application(kind,&mask,[130,111,112]));}
  for filters in [[0,0,0],[7,0,0],[17,18,19],[111,112,0]]{assert!(!extra_melee_application(3,&mask,filters));}
  assert!(!extra_melee_application(3,&[0;8],[130,0,0]));
  mask[130/32]&=!(1<<(130%32));assert!(!extra_melee_application(3,&mask,[111,112,0]));
 }
 #[test]fn charged_spell_damage_retry_requires_sword_and_native_charge_tags(){
  for charged in [false,true]{
   let mut mask=[0;8];if charged{mask[110/32]|=1<<(110%32);}
   add_melee_category(&mut mask);
   for kind in 0..=12{assert_eq!(extra_melee_application(kind,&mask,[100,0,0]),charged&&matches!(kind,3|4));}
   assert!(!extra_melee_application(3,&mask,[111,112,0]));
   if charged{for missing in [100,110,130]{let mut incomplete=mask;incomplete[missing/32]&=!(1<<(missing%32));assert!(!extra_melee_application(3,&incomplete,[100,0,0]));}}
  }
 }
 fn table(override_id:i32,value:&str)->String{"MagicID,增伤百分比\n".to_string()+&sword_map::DEFAULT_BONUSES.iter().map(|(id,v)|format!("{id},{}\n",if *id==override_id{value.to_string()}else{v.to_string()})).collect::<String>()}
 #[test]fn table_bonus_keeps_damage_and_scales_lifesteal(){for (value,factor) in [("0",1.),("10",1.1),("30",1.3),("50",1.5),("100",2.)]{let b=Bonuses::read(&table(4021,value)).unwrap();assert_eq!(b.factor(40210,1),Some(factor));assert_eq!(b.factor(40210,0),None);assert_eq!(b.factor(-1,1),None);let p=b.for_magic(4021).unwrap();assert_eq!(b.for_attack(40210,1),Some(p));assert_eq!(heal_amount(100,p,false),(p as f64*2.).floor() as u32);}}
 #[test]fn recorded_recluse_staff_hits_use_pc_kind_one(){let r=Bonuses::defaults();for attack in [40000,40070]{assert!(r.factor(attack,1).unwrap()>=1.);assert_eq!(r.factor(attack,0),None);}}
 #[test]fn casting_expires_and_cannot_transfer_to_dead_or_other_owner(){assert!(casting(1,true,true,1,100,199));assert!(!casting(1,true,true,1,100,200));assert!(!casting(1,true,true,2,100,150));assert!(!casting(1,false,true,1,100,150));assert!(!casting(1,true,false,1,100,150));}
 #[test]fn bonus_table_rejects_invalid_values_and_missing_or_duplicate_identity(){for v in ["-1","NaN","inf","100.01","oops"]{assert!(Bonuses::read(&table(4021,v)).is_err());}assert!(Bonuses::read("MagicID,增伤百分比\n4021,50").is_err());assert!(Bonuses::read(&(table(4021,"50")+"4021,50\n")).is_err());assert!(Bonuses::read(&(table(4021,"50")+"999999,50\n")).is_err());}
 #[test]fn blank_bonus_means_zero_and_fractional_bonus_is_supported(){assert_eq!(Bonuses::read(&table(4021,"")).unwrap().factor(40210,1),Some(1.));assert_eq!(Bonuses::read(&table(4021,"12.5")).unwrap().factor(40210,1),Some(1.125));}
 #[test]fn compiled_table_preserves_user_values_for_every_spell_and_mapped_attack(){let expected=Bonuses::read(include_str!("../data/balance/sword.csv")).unwrap();let b=Bonuses::defaults();assert_eq!(b.percentages,expected.percentages);for (attack,ids) in sword_map::ATTACKS{let p=ids.iter().map(|id|expected.percentages[id]).fold(f32::INFINITY,f32::min);assert_eq!(b.factor(*attack,1),Some(1.+p/100.));}assert_eq!(b.percentages.len(),160);}
 #[test]fn healing_uses_full_hit_damage_without_requiring_target_hp_loss(){assert_eq!(heal_amount(300,30.,false),180);assert_eq!(heal_amount(65,20.,false),26);assert_eq!(heal_amount(0,30.,false),0);assert_eq!(heal_amount(-1,30.,false),0);assert_eq!(heal_amount(i32::MAX,100.,false),65535);assert_eq!(heal_amount(300,0.,false),0);assert_eq!(heal_amount(300,12.5,false),75);assert_eq!(heal_amount(100,100.,false),200);}
 #[test]fn glancing_and_immune_hits_never_heal_despite_displayed_damage(){for damage in [1,5,300,i32::MAX]{assert_eq!(heal_amount(damage,100.,true),0);}}
 #[test]fn added_melee_category_keeps_all_original_tags_without_adding_skill_and_is_idempotent(){for original in 0..256{let mut mask=[0u32;8];mask[original/32]|=1<<(original%32);let saved=mask;add_melee_category(&mut mask);for category in [original,130]{assert_ne!(mask[category/32]&(1<<(category%32)),0);}for bit in 0..256{let expected=saved[bit/32]&(1<<(bit%32))!=0||bit==130||(original==110&&bit==100);assert_eq!(mask[bit/32]&(1<<(bit%32))!=0,expected);}let once=mask;add_melee_category(&mut mask);assert_eq!(mask,once);}}
 #[test]fn shared_percentage_headers_are_unambiguous(){for name in ["加成","吸血百分比","效果百分比","增伤百分比"]{assert!(Bonuses::read(&table(4021,"30").replace("增伤百分比",name)).is_ok());}assert!(Bonuses::read("MagicID,吸血百分比,增伤百分比\n4021,20,30").is_err());assert!(Bonuses::read("MagicID,加成,吸血百分比\n4021,20,30").is_err());}
 #[test]fn incoming_reduction_is_linear_and_preserves_non_damage(){assert_eq!(reduced(65,30.),45);assert_eq!(reduced(100,0.),100);assert_eq!(reduced(100,100.),0);assert_eq!(reduced(-5,50.),-5);assert_eq!(reduced(0,50.),0);assert_eq!(reduced(100,12.5),87);}
 #[test]fn torso_range_handles_elevated_contacts_without_expanding_horizontal_range(){
  let feet=[10.,20.,30.,1.];
  let elevated=[14.8,22.,30.,1.];
  assert!(!near(distance(feet,elevated),NEAR_DISTANCE));
  assert!(near(hit_distance(feet,elevated),NEAR_DISTANCE));
  assert!(near(hit_distance(feet,[15.,21.,30.,1.]),NEAR_DISTANCE));
  assert!(!near(hit_distance(feet,[15.001,21.,30.,1.]),NEAR_DISTANCE));
  // Retain height limits above and below the torso, rather than a cylinder.
  assert_eq!(hit_distance(feet,[10.,26.,30.,1.]),Some(5.));
  assert!(!near(hit_distance(feet,[10.,26.001,30.,1.]),NEAR_DISTANCE));
  assert!(!near(hit_distance(feet,[10.,15.999,30.,1.]),NEAR_DISTANCE));
  assert!(near(hit_distance(feet,[10.,20.,30.,1.]),NEAR_DISTANCE));
  for invalid in [f32::NAN,f32::INFINITY,f32::NEG_INFINITY]{
   for axis in 0..3{
    let mut p=feet;p[axis]=invalid;assert_eq!(hit_distance(p,elevated),None);
    let mut p=elevated;p[axis]=invalid;assert_eq!(hit_distance(feet,p),None);
   }
  }
 }
 #[test]fn range_is_three_dimensional_and_rejects_nan(){assert_eq!(distance([0.;4],[0.,3.,4.,0.]),Some(5.));assert_eq!(distance([f32::NAN;4],[0.;4]),None);}
 #[test]fn near_boundary_includes_limit_but_rejects_far_and_unknown(){assert!(near(distance([0.;4],[0.,0.,5.,0.]),5.));assert!(!near(distance([0.;4],[0.,0.,5.001,0.]),5.));assert!(!near(distance([0.;4],[0.,5.001,0.,0.]),5.));assert!(!near(distance([f32::NAN;4],[0.;4]),5.));assert!(!near(None,5.));}

}
