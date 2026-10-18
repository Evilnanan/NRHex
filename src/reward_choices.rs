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
pub fn caster_role_eligible(hero:Option<u8>,has_archmage:bool)->bool{
 matches!(hero,Some(4|6|7))||(has_archmage&&matches!(hero,Some(1..=10)))
}
pub fn role_exclusions(mut unavailable:HexSet,hero:Option<u8>)->HexSet{
 let sword=index(95000900);let has_sword=unavailable.contains(&sword);
 let caster_allowed=caster_role_eligible(hero,unavailable.contains(&index(95002100)));
 for (id,excluded)in [
  (95000900,!caster_allowed),
  (95003500,!caster_allowed),
  (95002000,!caster_allowed),
  (95001800,hero!=Some(6)),
  (95001100,matches!(hero,Some(6|7))&&!has_sword),
  (95001300,matches!(hero,Some(2|4|6))),
  (95000400,!matches!(hero,Some(1|3|5|7|8))),
  (95002600,matches!(hero,Some(6|7))&&!has_sword),
  (95002400,matches!(hero,Some(6|7))),
 ]{if excluded{unavailable.insert(index(id));}}
 unavailable
}
pub fn eligible(order:[usize;3],owned:&HexSet)->bool{
 let unavailable=pool_exclusions(owned.clone());
 order.iter().all(|i|*i<HEX_COUNT&&!unavailable.contains(i))&&mask(order).len()==3
}
#[cfg(test)]pub fn candidates(owned:&HexSet)->Vec<usize>{(0..HEX_COUNT).filter(|i|!owned.contains(i)).collect()}
#[cfg(test)]pub fn preferred(order:[usize;3],owned:&HexSet)->Option<[usize;3]>{
 crate::reward_cycle::Round::new(owned,&HexSet::new()).preferred(order)
}
pub fn is_hex_good(encoded:u32)->bool{
 encoded&0xf0000000==0x40000000&&OWNER_IDS.contains(&((encoded&0x0fffffff)as i32))
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn archmage_unlocks_only_the_three_caster_role_filters_in_every_draw_route(){
  let mage=index(95002100);let trio=mask([index(95000900),index(95002000),index(95003500)]);
  for hero in std::iter::once(None).chain((0..=255).map(Some)){
   let original=role_exclusions(HexSet::new(),hero);
   for has_mage in [false,true]{
    let owned=if has_mage{mask([mage])}else{HexSet::new()};
    let excluded=role_exclusions(ownership_exclusions(owned.clone()),hero);
    let allowed=matches!(hero,Some(4|6|7))||(has_mage&&matches!(hero,Some(1..=10)));
    let mut expected=original.clone();if has_mage{expected.insert(mage);if allowed{for i in &trio{expected.remove(i);}}}
    assert_eq!(excluded,expected); // No other role restriction is relaxed.
    for &ability in &trio{
     for seen in [HexSet::new(),all(HEX_COUNT)]{
      let round=crate::reward_cycle::Round::new(&excluded,&seen);assert_eq!(round.candidates.contains(&ability),allowed);
      assert_eq!(round.preferred([ability,0,1]).unwrap().contains(&ability),allowed);
      assert_eq!(crate::reward_cycle::Single::new(&excluded,&seen,&mask([0,1])).candidates.contains(&ability),allowed);
      let mut owned_ability=owned.clone();owned_ability.insert(ability);
      let blocked=role_exclusions(ownership_exclusions(owned_ability),hero);
      assert!(!crate::reward_cycle::Round::new(&blocked,&seen).candidates.contains(&ability));
      assert!(!crate::reward_cycle::Single::new(&blocked,&seen,&mask([0,1])).candidates.contains(&ability));
     }
    }
   }
  }
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
   let excluded=role_exclusions(terrain_exclusions(ownership_exclusions(owned),true),Some(hero));
   assert!(excluded.contains(&jiahao));assert!(!excluded.contains(&pact));assert!(!excluded.contains(&shell));
  }
  // Sword's role exception cannot unlock a pact excluded by an owned conflict.
  for conflict in [shell,jiahao]{
   let excluded=role_exclusions(ownership_exclusions(mask([index(95000900),conflict])),Some(6));
   assert!(excluded.contains(&pact));
  }
 }
 #[test]fn disabled_ability_keeps_its_stable_slot_but_never_appears_in_draws_or_refreshes(){
  let disabled=index(95003300);assert_eq!(disabled,29);assert_eq!(crate::hex_pool::HISTORY_BITS[disabled],30);
  assert!(is_hex_good(0x40000000|95003300));
  for hero in std::iter::once(None).chain((0..=255).map(Some)){
   let unavailable=role_exclusions(HexSet::new(),hero);
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
    let excluded=role_exclusions(terrain_exclusions(mask([5]),hollow),hero);
    let expected=[
     (95000900,matches!(hero,Some(4|6|7))),
     (95002000,matches!(hero,Some(4|6|7))),
     (95001800,hero==Some(6)),
     (95001100,!matches!(hero,Some(6|7))),
     (95001300,!matches!(hero,Some(2|4|6))),
     (95000400,matches!(hero,Some(1|3|5|7|8))),
     (95002600,!matches!(hero,Some(6|7))),
     (95002400,!matches!(hero,Some(6|7))),
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
   let sword=index(95000900);let excluded=role_exclusions(terrain_exclusions(mask([sword,5]),true),Some(hero));
   for owner in [95001100,95002600]{let i=index(owner);assert!(!excluded.contains(&i));assert!(preferred([i,0,1],&excluded).unwrap().contains(&i));}
   for owner in [95000900,95002400,95001600]{assert!(excluded.contains(&index(owner)));}
  }
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
