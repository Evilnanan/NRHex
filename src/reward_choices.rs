//! Ability filters. No serialized triple catalog or fixed-width ownership mask.
use crate::hex_pool::{HEX_COUNT,OWNER_IDS};
pub type HexSet=std::collections::BTreeSet<usize>;
pub fn all(count:usize)->HexSet{(0..count).collect()}
pub fn mask(items:impl IntoIterator<Item=usize>)->HexSet{items.into_iter().collect()}
fn index(owner:i32)->usize{OWNER_IDS.iter().position(|&id|id==owner).expect("Missing ability in catalog")}
pub fn pool_exclusions(mut unavailable:HexSet)->HexSet{
 unavailable.insert(index(95003300));unavailable
}
// Run on actual ownership before role/terrain restrictions add unavailable
// entries: a role-ineligible ability must not imply that it is owned.
pub fn ownership_exclusions(mut owned:HexSet)->HexSet{
 let sniper=index(crate::sniper::OWNER);let sword=index(crate::sword::OWNER);
 if owned.contains(&sniper)||owned.contains(&sword){owned.extend([sniper,sword]);}
 let basics=index(crate::basics::OWNER);
 let ultimate=crate::basics::CONFLICTS.map(index);
 if owned.contains(&basics){owned.extend(ultimate);}
 else if ultimate.iter().any(|i|owned.contains(i)){owned.insert(basics);}
 let pact=index(95002600);let shell=index(95000700);let jiahao=index(95001300);
 if owned.contains(&pact){owned.extend([shell,jiahao]);}
 else if owned.contains(&shell)||owned.contains(&jiahao){owned.insert(pact);}
 owned
}
pub fn terrain_exclusions(mut owned:HexSet,great_hollow:bool)->HexSet{
 if great_hollow{owned.insert(index(95001600));}owned
}
pub fn party_exclusions(mut unavailable:HexSet,difficulty_count:Option<i32>)->HexSet{
 if difficulty_count!=Some(1){unavailable.insert(index(95002500));}
 if !difficulty_count.is_some_and(|count|count>1){unavailable.insert(index(95003400));}unavailable
}
pub fn weapon_exclusions(mut unavailable:HexSet,affinity:Option<i32>)->HexSet{
 let sword=index(95000900);let has_sword=unavailable.contains(&sword);
 let caster=affinity.is_some_and(crate::weapon_affinity::is_caster);
 let caster_allowed=caster||unavailable.contains(&index(95002100));
 let physical=affinity.is_some()&&!caster;
 for (id,excluded)in [
  (95000900,!caster_allowed),
  (95003500,!caster_allowed),
  (95002000,!caster_allowed),
  (95001100,!physical&&!has_sword),
  (95002600,!physical&&!has_sword),
  (95002400,!physical),
  (95003100,!physical),
 ]{if excluded{unavailable.insert(index(id));}}
 unavailable
}
pub fn role_exclusions(mut unavailable:HexSet,hero:Option<u8>)->HexSet{
 for (id,excluded)in [
  (95001800,hero!=Some(6)),
  (crate::marking::OWNER,hero!=Some(3)),
  (95001300,matches!(hero,Some(2|4|6))),
  (95000400,!matches!(hero,Some(1|3|5|7|8))),
 ]{if excluded{unavailable.insert(index(id));}}
 unavailable
}
// Evaluate owned exceptions before conflict/role exclusions can impersonate
// ownership (Sniper excludes Sword but does not grant Sword's melee access).
pub fn combat_exclusions(owned:HexSet,hero:Option<u8>,affinity:Option<i32>)->HexSet{
 let mut unavailable=weapon_exclusions(owned.clone(),affinity);
 unavailable.extend(role_exclusions(HexSet::new(),hero));
 unavailable.extend(ownership_exclusions(owned));unavailable
}
pub fn eligible(order:[usize;3],owned:&HexSet)->bool{
 let unavailable=pool_exclusions(owned.clone());
 order.iter().all(|i|*i<HEX_COUNT&&!unavailable.contains(i))&&mask(order).len()==3
}
#[cfg(test)]pub fn candidates(owned:&HexSet)->Vec<usize>{(0..HEX_COUNT).filter(|i|!owned.contains(i)).collect()}
// Default HeroParam discovery values, for legacy tests without relic overrides.
#[cfg(test)]pub fn default_filters(owned:HexSet,hero:Option<u8>)->HexSet{
 weapon_exclusions(role_exclusions(owned,hero),hero.filter(|h|(1..=10).contains(h)).map(|h|1000+i32::from(h)*100))
}
#[cfg(test)]pub fn preferred(order:[usize;3],owned:&HexSet)->Option<[usize;3]>{
 crate::reward_cycle::Round::new(owned,&HexSet::new()).preferred(order)
}
pub fn is_hex_good(encoded:u32)->bool{
 encoded&0xf0000000==0x40000000&&OWNER_IDS.contains(&((encoded&0x0fffffff)as i32))
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn live_weapon_affinity_controls_seven_abilities_and_keeps_owned_exceptions(){
  for hero in std::iter::once(None).chain((0..=10).map(Some)){for affinity in [None,Some(57),Some(61),Some(1600),Some(1700),Some(1),Some(1100),Some(1400),Some(2000)]{for flags in 0..4{
   let owned=mask([(95002100,flags&1!=0),(95000900,flags&2!=0)].into_iter().filter_map(|(id,on)|on.then_some(index(id))));
   let excluded=combat_exclusions(owned.clone(),hero,affinity);
   let caster=matches!(affinity,Some(57|61|1600|1700));let physical=affinity.is_some()&&!caster;
   let expected=[(95000900,(caster||flags&1!=0)&&flags&2==0),(95002000,caster||flags&1!=0),(95003500,caster||flags&1!=0),
    (95001100,physical||flags&2!=0),(95002600,physical||flags&2!=0),(95002400,physical),(95003100,physical),
    (95001800,hero==Some(6)),(95001300,!matches!(hero,Some(2|4|6))),(95000400,matches!(hero,Some(1|3|5|7|8)))];
   for (id,allowed)in expected{let ability=index(id);
    for seen in [HexSet::new(),all(HEX_COUNT)]{
     let round=crate::reward_cycle::Round::new(&excluded,&seen);assert_eq!(round.candidates.contains(&ability),allowed,"{hero:?}/{affinity:?}/{flags}/{id}");
     assert_eq!(round.preferred([ability,0,1]).unwrap().contains(&ability),allowed);
     assert_eq!(crate::reward_cycle::Single::new(&excluded,&seen,&mask([0,1])).candidates.contains(&ability),allowed);
     let mut owned_ability=owned.clone();owned_ability.insert(ability);
     let blocked=combat_exclusions(owned_ability,hero,affinity);
     assert!(!crate::reward_cycle::Round::new(&blocked,&seen).candidates.contains(&ability));
    }
   }
  }}}
 }
 #[test]fn night_blessing_draws_and_refreshes_follow_live_difficulty_count(){
  let blessing=index(95002500);let owned=mask([5]);
  for count in [None,Some(0),Some(1),Some(2),Some(3),Some(4),Some(1)]{
   let excluded=party_exclusions(owned.clone(),count);let allowed=count==Some(1);
   assert_eq!(excluded.difference(&owned).copied().collect::<HexSet>(),mask([(blessing,!allowed),(index(95003400),!count.is_some_and(|n|n>1))].into_iter().filter_map(|(id,blocked)|blocked.then_some(id))));
   for seen in [HexSet::new(),all(HEX_COUNT)]{
    let round=crate::reward_cycle::Round::new(&excluded,&seen);
    assert_eq!(round.candidates.contains(&blessing),allowed);
    assert_eq!(round.preferred([blessing,0,1]).unwrap().contains(&blessing),allowed);
    assert_eq!(crate::reward_cycle::Single::new(&excluded,&seen,&mask([0,1])).candidates.contains(&blessing),allowed);
   }
   let mut owned_blessing=owned.clone();owned_blessing.insert(blessing);
   assert!(!preferred([blessing,0,1],&party_exclusions(owned_blessing,count)).unwrap().contains(&blessing));
  }
 }
 #[test]fn ambulance_is_multiplayer_only_in_all_choice_routes(){
  let ambulance=index(95003400);let blessing=index(95002500);
  for count in [None,Some(-1),Some(0),Some(1),Some(2),Some(3),Some(4)]{
   let excluded=party_exclusions(HexSet::new(),count);let allowed=count.is_some_and(|n|n>1);
   let round=crate::reward_cycle::Round::new(&excluded,&HexSet::new());
   assert_eq!(round.candidates.contains(&ambulance),allowed);
   assert_eq!(round.preferred([ambulance,0,1]).unwrap().contains(&ambulance),allowed);
   assert_eq!(crate::reward_cycle::Single::new(&excluded,&HexSet::new(),&mask([0,1])).candidates.contains(&ambulance),allowed);
   assert_eq!(excluded.contains(&blessing),count!=Some(1));
  }
 }
 #[test]fn pact_conflicts_follow_actual_ownership_in_draws_focused_balls_and_refreshes(){
  let pact=index(95002600);let shell=index(95000700);let jiahao=index(95001300);
  for bits in 0..8{
   let owned=mask([pact,shell,jiahao].into_iter().enumerate().filter_map(|(i,id)|(bits&(1<<i)!=0).then_some(id)));
   let excluded=ownership_exclusions(owned.clone());
   let expected=[(pact,bits!=0),(shell,bits&3!=0),(jiahao,bits&5!=0)];
   for seen in [HexSet::new(),all(HEX_COUNT)]{
    let round=crate::reward_cycle::Round::new(&excluded,&seen);
    let focused=round.preferred([pact,shell,jiahao]).unwrap();
    let random=round.draw(|n|Some(n-1)).unwrap();
    let refresh=crate::reward_cycle::Single::new(&excluded,&seen,&mask([0,1]));
    for (id,blocked) in expected{
     assert_eq!(round.candidates.contains(&id),!blocked,"ownership {bits}, ability {id}");
     assert_eq!(refresh.candidates.contains(&id),!blocked);
     if blocked{assert!(!focused.contains(&id));assert!(!random.contains(&id));}
    }
   }
   // Conflicts do not grant or remove actual effects, nor block unrelated options.
   assert!(owned.is_subset(&excluded));assert!(!excluded.contains(&0));
  }
 }
 #[test]fn role_restrictions_never_count_as_ownership_for_pact_conflicts(){
  let pact=index(95002600);let shell=index(95000700);let jiahao=index(95001300);
  for hero in [2,4,6]{
   let mut owned=HexSet::new();if hero==6{owned.insert(index(95000900));}
   let excluded=default_filters(terrain_exclusions(ownership_exclusions(owned),true),Some(hero));
   assert!(excluded.contains(&jiahao));assert!(!excluded.contains(&pact));assert!(!excluded.contains(&shell));
  }
  // Sword's role exception cannot unlock a pact excluded by an owned conflict.
  for conflict in [shell,jiahao]{
   let excluded=default_filters(ownership_exclusions(mask([index(95000900),conflict])),Some(6));
   assert!(excluded.contains(&pact));
  }
 }
 #[test]fn disabled_ability_keeps_its_stable_slot_but_never_appears_in_draws_or_refreshes(){
  let disabled=index(95003300);assert_eq!(disabled,29);assert_eq!(crate::hex_pool::HISTORY_BITS[disabled],30);
  assert!(is_hex_good(0x40000000|95003300));
  for hero in std::iter::once(None).chain((0..=255).map(Some)){
   let unavailable=default_filters(HexSet::new(),hero);
   for seen in [HexSet::new(),all(HEX_COUNT)]{
    let round=crate::reward_cycle::Round::new(&unavailable,&seen);
    assert!(!round.candidates.contains(&disabled));
    assert!(!round.preferred([disabled,0,1]).unwrap().contains(&disabled));
    assert!(!round.draw(|count|Some(count-1)).unwrap().contains(&disabled));
    assert!(!crate::reward_cycle::Single::new(&unavailable,&seen,&mask([0,1])).candidates.contains(&disabled));
   }
  }
  assert!(!eligible([disabled,0,1],&HexSet::new()));
  assert_eq!(crate::reward_cycle::Round::new(&HexSet::new(),&HexSet::new()).candidates.len(),HEX_COUNT-1);
 }
 #[test]fn all_roles_preserve_their_filters_in_random_focused_and_refresh_paths(){
  for hero in std::iter::once(None).chain((0..=255).map(Some)){
   for hollow in [false,true]{
    let excluded=default_filters(terrain_exclusions(mask([5]),hollow),hero);
    let expected=[
     (95000900,matches!(hero,Some(6|7))),
     (95002000,matches!(hero,Some(6|7))),
     (95001800,hero==Some(6)),
     (95001100,matches!(hero,Some(1..=10))&&!matches!(hero,Some(6|7))),
     (95001300,!matches!(hero,Some(2|4|6))),
     (95000400,matches!(hero,Some(1|3|5|7|8))),
     (95002600,matches!(hero,Some(1..=10))&&!matches!(hero,Some(6|7))),
     (95002400,matches!(hero,Some(1..=10))&&!matches!(hero,Some(6|7))),
     (95003100,matches!(hero,Some(1..=10))&&!matches!(hero,Some(6|7))),
     (95001600,!hollow),
    ];
    for (owner,allowed)in expected{
     let i=index(owner);assert_eq!(candidates(&excluded).contains(&i),allowed,"{hero:?}/{owner}/{hollow}");
     assert_eq!(preferred([i,0,1],&excluded).unwrap().contains(&i),allowed);
     for seen in [HexSet::new(),all(HEX_COUNT)]{
      assert_eq!(crate::reward_cycle::Round::new(&excluded,&seen).candidates.contains(&i),allowed);
      assert_eq!(crate::reward_cycle::Single::new(&excluded,&seen,&mask([5])).candidates.contains(&i),allowed);
     }
     let mut owned=excluded.clone();owned.insert(i);assert!(!preferred([i,0,1],&owned).unwrap().contains(&i));
    }
   }
  }
 }
 #[test]fn sword_ownership_unlocks_mighty_and_pact_but_never_owned_clash_or_terrain_filters(){
  for hero in [6,7]{
   let sword=index(95000900);let excluded=default_filters(terrain_exclusions(mask([sword,5]),true),Some(hero));
   for owner in [95001100,95002600]{let i=index(owner);assert!(!excluded.contains(&i));assert!(preferred([i,0,1],&excluded).unwrap().contains(&i));}
   for owner in [95000900,95002400,95003100,95001600]{assert!(excluded.contains(&index(owner)));}
  }
 }
 #[test]fn erosion_role_exclusion_is_not_relaxed_by_sword_or_archmage(){
  let erosion=index(95003100);
  for hero in 1..=10{for owned in [HexSet::new(),mask([index(95000900)]),mask([index(95002100)]),mask([index(95000900),index(95002100)])]{
   let allowed=!matches!(hero,6|7);let excluded=default_filters(owned,Some(hero));
   for seen in [HexSet::new(),all(HEX_COUNT)]{
    let round=crate::reward_cycle::Round::new(&excluded,&seen);
    assert_eq!(round.candidates.contains(&erosion),allowed);
    assert_eq!(round.preferred([erosion,0,1]).unwrap().contains(&erosion),allowed);
    assert_eq!(crate::reward_cycle::Single::new(&excluded,&seen,&mask([0,1])).candidates.contains(&erosion),allowed);
   }
  }}
 }
 #[test]fn focused_preferences_preserve_order_and_replace_owned_or_duplicate_choices(){
  assert_eq!(preferred([9,3,6],&mask([3])),Some([9,6,0]));
  assert_eq!(preferred([9,9,6],&HexSet::new()),Some([9,6,0]));
  assert_eq!(preferred([9,3,6],&all(HEX_COUNT)),None);
  assert!(is_hex_good(0x40000000|95003100));assert!(!is_hex_good(95003100));assert!(!is_hex_good(0x40000000|95001900));
 }
 #[test]fn catalog_contains_distinct_live_abilities_and_one_native_template(){
  assert_eq!(OWNER_IDS.len(),HEX_COUNT);assert_eq!(OWNER_IDS.iter().collect::<std::collections::HashSet<_>>().len(),HEX_COUNT);
  assert!(!OWNER_IDS.contains(&95001900));assert_eq!(OWNER_IDS[27],95003100);
  assert!(!crate::red_envelope::LOTS.contains(&crate::hex_pool::TEMPLATE_LOT));
 }
}
