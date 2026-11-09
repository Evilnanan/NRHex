//! Per-recipient impact tier, measured from the stationary wave center.
use crate::{ptr,read_i32,spatial_ready,position};
fn tier(center:[f32;4],target:[f32;4])->Option<u8>{
 if !center[..3].iter().chain(&target[..3]).all(|v|v.is_finite()){return None;}
 let squared=(0..3).map(|i|(f64::from(target[i])-f64::from(center[i])).powi(2)).sum::<f64>();
 Some(if squared<=16.{7}else{5})
}
pub struct Guard(Option<(usize,u8,u8)>);
impl Drop for Guard{fn drop(&mut self){if let Some((hit,level,override_id))=self.0{unsafe{
 ptr::write((hit+0x24)as*mut u8,level);ptr::write((hit+0x1e)as*mut u8,override_id);
}}}}
unsafe fn apply(hit:usize,level:u8)->Guard{
 let old=(hit,ptr::read((hit+0x24)as*const u8),ptr::read((hit+0x1e)as*const u8));
 ptr::write((hit+0x24)as*mut u8,level);
 // E6D510 maps override 6 -> impact 7 and override 10 -> impact 5.
 // 6D1B60 preserves an explicit override instead of using dmgLevel_vsPlayer.
 ptr::write((hit+0x1e)as*mut u8,if level==7{6}else{10});
 Guard(Some(old))
}
pub unsafe fn contact(hit:usize,target:usize,bullet:usize)->Guard{
 if hit==0||bullet==0||read_i32(hit+0x40)!=crate::soul_critical::ATTACK||read_i32(hit+0x44)!=1
  ||read_i32(bullet+0xa4)!=crate::soul_critical::BULLET||!spatial_ready(target){return Guard(None);}
 let center=ptr::read_unaligned((bullet+0x10)as*const[f32;4]);
 match tier(center,position(target,0)){Some(level)=>apply(hit,level),None=>Guard(None)}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn inner_boundary_and_outer_ring_use_wave_center_in_three_dimensions(){
  let c=[10.,20.,30.,1.];
  for(d,want)in [(0.,7),(3.999,7),(4.,7),(4.001,5),(10.,5)]{assert_eq!(tier(c,[10.+d,20.,30.,1.]),Some(want));}
  assert_eq!(tier(c,[10.,24.,30.,1.]),Some(7));
  assert_eq!(tier(c,[13.,23.,30.,1.]),Some(5));
  assert_eq!(tier(c,[f32::NAN,20.,30.,1.]),None);
 }
 #[test]fn contact_inputs_restore_without_touching_damage_or_knockback(){unsafe{
  let mut hit=[0xa5u8;0x300];let p=hit.as_mut_ptr()as usize;
  for level in [7,5]{let before=hit;{let _g=apply(p,level);
   for i in 0..hit.len(){assert_eq!(hit[i],match i{0x24=>level,0x1e=>if level==7{6}else{10},_=>before[i]});}
   {let _nested=apply(p,5);assert_eq!(hit[0x24],5);}assert_eq!(hit[0x24],level);
  }assert_eq!(hit,before);}
 }}
}
