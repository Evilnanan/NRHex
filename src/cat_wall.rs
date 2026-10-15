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
