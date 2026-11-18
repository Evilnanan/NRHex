//! User-defined outgoing impact remap, applied once at native hit construction.
pub const OWNER:i32=95001100;
pub fn applies(attack:i32,table:i32)->bool {!(table==1&&attack==600000120)}
pub fn remap(level:u8)->u8 {match level {0=>1,1=>2,2=>3,3=>7,8=>2,_=>level}}
#[cfg(test)]
mod tests {
 use super::*;
 #[test] fn native_marking_poison_contact_keeps_its_non_interrupting_impact(){
  assert!(!applies(600000120,1));
  assert!(applies(600000120,0));
  for attack in [600000110,300000290,550,560,0] {assert!(applies(attack,1));}
 }
 #[test] fn every_unlisted_impact_type_is_preserved(){
  for n in 0..=255u8 {assert_eq!(remap(n),match n {0=>1,1=>2,2=>3,3=>7,8=>2,_=>n});}
 }
}
