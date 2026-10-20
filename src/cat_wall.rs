//! The native behavior graph selects 202420 / velocity row 510 when jumping
//! into a wall. Its -1.3 impulse pushes away from an inward-sloping surface.
//! Borrow the native forward-wall-jump speed only for a living local Cat owner.
pub const INTO_WALL_ROW:i32=510;
pub const NATIVE_BACKWARD_SPEED:f32=-1.3;
pub const FORWARD_SPEED:f32=4.8;
pub fn horizontal(row:i32,native:f32,owner:bool)->f32{
 if owner&&row==INTO_WALL_ROW&&(native-NATIVE_BACKWARD_SPEED).abs()<0.00001{FORWARD_SPEED}else{native}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn only_the_native_into_wall_impulse_of_an_owner_is_reversed(){
  for owned in [false,true]{for row in [-1,100,101,111,121,131,500,501,510,511,999]{
   assert_eq!(horizontal(row,-1.3,owned),if owned&&row==510{4.8}else{-1.3});
  }}
  // Do not override a changed parameter from an unrelated mod or invalid data.
  for speed in [0.,-4.8,4.8,1.3,f32::INFINITY,f32::NEG_INFINITY,f32::NAN]{assert_eq!(horizontal(510,speed,true).to_bits(),speed.to_bits());}
 }
}

// Stateless HKS query routed through the existing shared env hook. The second
// script argument remains the stable owner ID; no effect is added or removed.
pub const QUERY:i32=990008;
fn outside_nightlord(map:Option<u32>)->bool{
 map.is_some_and(|id|!matches!(id>>24,0|16|18|19|255))
}
// Same packed source map/fallback as native latent-power creation at576D44.
// Read at every request: loaded maps/actor pointers never become a saved flag.
pub unsafe fn current_map(player:usize,base:usize)->Option<u32>{
 if player==0{return None;}
 let mut map=(crate::read_ptr(player+8)>>32)as u32;
 if map==0||map==u32::MAX{
  let context=crate::read_ptr(base+crate::MAP_CONTEXT_RVA);if context==0{return None;}
  map=std::ptr::read_unaligned((context+0x2c)as*const u32);
 }
 (map!=0&&map!=u32::MAX).then_some(map)
}
pub unsafe fn enabled(player:usize,base:usize)->bool{
 if player==0||!crate::is_local(player,base)||!crate::has_effect(player,crate::cat::OWNER,base){return false;}
 let stats=crate::native_actor::stats(player);
 stats!=0&&!crate::native_actor::incapacitated(player,stats)&&outside_nightlord(current_map(player,base))
}
#[cfg(test)]#[path="cat_arena_native_tests.rs"]mod arena_tests;
