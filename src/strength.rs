//! User-defined outgoing impact remap, applied once at native hit construction.
pub const OWNER:i32=95001100;
pub fn remap(level:u8)->u8 {match level {0=>1,1=>2,2=>3,3=>7,8=>2,_=>level}}
#[cfg(test)]
mod tests {
 use super::*;
 #[test] fn every_unlisted_impact_type_is_preserved(){
  for n in 0..=255u8 {assert_eq!(remap(n),match n {0=>1,1=>2,2=>3,3=>7,8=>2,_=>n});}
 }
}
