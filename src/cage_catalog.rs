mod data { include!(concat!(env!("OUT_DIR"), "/cage_data.rs")); }

pub fn npc(id:i32)->bool {
 (95300000..95300084).contains(&id)||data::EXTRA_NPCS.binary_search(&id).is_ok()
}
pub fn reward_group(offset:u32)->bool {
 (5937..5950).contains(&offset)||data::EXTRA_REWARD_GROUPS.binary_search(&offset).is_ok()
}

#[cfg(test)]
mod tests {
 use super::*;
 #[test] fn generated_catalog_is_sorted_and_disjoint(){
  assert!(data::EXTRA_NPCS.windows(2).all(|w|w[0]<w[1]));
  assert!(data::EXTRA_REWARD_GROUPS.windows(2).all(|w|w[0]<w[1]));
  for &id in data::EXTRA_NPCS{assert!((95300200..95300400).contains(&id));assert!(npc(id));}
  for &id in data::EXTRA_REWARD_GROUPS{assert!((6000..6100).contains(&id));assert!(reward_group(id));}
  for id in [95300084,95300199,95300400,48000010]{assert!(!npc(id));}
  assert!(!reward_group(5950));assert!(!reward_group(6100));
 }
}
