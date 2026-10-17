//! Ability filters. No serialized triple catalog or fixed-width ownership mask.
use crate::hex_pool::{HEX_COUNT,OWNER_IDS};
pub type HexSet=std::collections::BTreeSet<usize>;
pub fn all(count:usize)->HexSet{(0..count).collect()}
pub fn mask(items:impl IntoIterator<Item=usize>)->HexSet{items.into_iter().collect()}
fn index(owner:i32)->usize{OWNER_IDS.iter().position(|&id|id==owner).expect("Missing ability in catalog")}
pub fn terrain_exclusions(mut owned:HexSet,great_hollow:bool)->HexSet{
 if great_hollow{owned.insert(index(95001600));}owned
}
pub fn role_exclusions(mut unavailable:HexSet,hero:Option<u8>)->HexSet{
 let sword=index(95000900);let has_sword=unavailable.contains(&sword);
 for (id,excluded)in [
  (95000900,!matches!(hero,Some(4|6|7))),
  (95002000,!matches!(hero,Some(4|6|7))),
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
 order.iter().all(|i|*i<HEX_COUNT&&!owned.contains(i))&&mask(order).len()==3
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
