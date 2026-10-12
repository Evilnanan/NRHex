pub const MARKER:i32=95001602;
pub const FIRST_NPC:i32=95300000;
pub const NPC_COUNT:i32=84;

// Only the cage's own persistent marker may promote one of its NPC copies.
// Other native bits, including the actor's network state, must survive.
pub fn promoted_flags(npc:i32,marked:bool,flags:u8)->u8 {
 if marked&&(FIRST_NPC..FIRST_NPC+NPC_COUNT).contains(&npc){flags|1}else{flags}
}

#[cfg(test)]
mod tests {
 use super::*;
 #[test] fn promotion_is_bounded_and_preserves_every_other_bit(){
  for flags in 0..=255 {
   for npc in [FIRST_NPC-1,FIRST_NPC,FIRST_NPC+NPC_COUNT-1,FIRST_NPC+NPC_COUNT,34600020] {
    let promoted=promoted_flags(npc,true,flags);
    assert_eq!(promoted,if (FIRST_NPC..FIRST_NPC+NPC_COUNT).contains(&npc){flags|1}else{flags});
    assert_eq!(promoted_flags(npc,false,flags),flags);
    assert_eq!(promoted_flags(npc,true,promoted),promoted);
   }
  }
 }
}
