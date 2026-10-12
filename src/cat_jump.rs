//! Native jump category 102; env(224) height is in world metres.
pub const HEIGHT_BASELINE_METRES:f32=1.;
pub const HEIGHT_BONUS_COEFFICIENT:f64=1.;
// Current native ExecJump also permits the Great Hollow pseudo-stream effect.
pub const CRYSTAL_SPRING_EFFECT:i32=98215;
pub fn spring_launch_available(region_height:f32,crystal_ready:bool)->bool {
 (region_height.is_finite()&&region_height>0.)||crystal_ready
}
pub const JUMP_CATEGORY:usize=102;
pub const LANDING_GRACE_MS:u64=250;
pub fn is_jump(mask:&[u32;8])->bool {mask[JUMP_CATEGORY/32]&(1<<(JUMP_CATEGORY%32))!=0}
pub fn bonus(height:f32)->f64 {
 if !height.is_finite()||height<=0. {return 0.;}
 (HEIGHT_BONUS_COEFFICIENT*(1.+(height as f64/HEIGHT_BASELINE_METRES as f64).log2())).max(0.)
}
pub fn boosted(damage:i32,height:f32)->i32 {
 if damage<=0{return damage;}
 ((damage as f64*(1.+bonus(height))).floor().min(i32::MAX as f64)) as i32
}
// Each hit uses the current native height independently. Only the raw landing
// height is retained for 250 ms; hits and grounded samples never renew it.
#[derive(Clone,Copy)]
pub struct Sample {pub player:usize,pub stats:usize,pub airborne:bool,pub native_height:f32,pub world_y:f32,pub spring_available:bool}
#[derive(Default)]
pub struct Landing {player:usize,stats:usize,initialized:bool,airborne:bool,height:f32,landed_at:u64,last_y:f32,spring_candidate:bool,spring_flight:bool,apex:f32}
impl Landing {
 pub fn observe_frame(&mut self,sample:Sample,owned:bool,alive:bool,now:u64) {
  let Sample{player,stats,airborne,native_height,world_y,spring_available}=sample;
  if self.player!=player||self.stats!=stats||!owned||!alive {
   *self=Self{player,stats,..Self::default()};
  }
  if !owned||!alive||!native_height.is_finite()||!world_y.is_finite(){self.initialized=false;self.height=0.;self.spring_candidate=false;self.spring_flight=false;return;}
  if !self.initialized {
   self.initialized=true;self.airborne=airborne;
   self.spring_candidate=!airborne&&spring_available;self.spring_flight=airborne&&spring_available;self.apex=world_y;self.last_y=world_y;
   self.height=if airborne{native_height.max(0.)}else{0.};return;
  }
  if airborne {
   if !self.airborne {
    // env366 may stop returning the region as soon as the launch leaves it.
    self.spring_flight=self.spring_candidate||spring_available;self.apex=self.last_y.max(world_y);
   }
   self.apex=self.apex.max(world_y);
   self.height=native_height.max(if self.spring_flight{self.apex-world_y}else{0.}).max(0.);
  }else if self.airborne {
   self.height=self.height.max(native_height).max(if self.spring_flight{self.apex-world_y}else{0.}).max(0.);
   self.landed_at=now;
  }else if now.saturating_sub(self.landed_at)>LANDING_GRACE_MS {
   self.height=0.;self.spring_flight=false;
  }
  self.spring_candidate=!airborne&&spring_available;self.airborne=airborne;self.last_y=world_y;
 }
 pub fn height_frame(&mut self,sample:Sample,now:u64)->f32 {
  // A hit can observe the first grounded frame before the update callback.
  self.observe_frame(sample,true,true,now);
  if self.initialized&&(sample.airborne||now.saturating_sub(self.landed_at)<=LANDING_GRACE_MS){self.height}else{0.}
 }
}
#[cfg(test)]
mod tests {
 use super::*;
 impl Landing {
  fn observe(&mut self,player:usize,stats:usize,owned:bool,alive:bool,airborne:bool,native_height:f32,now:u64){self.observe_frame(Sample{player,stats,airborne,native_height,world_y:0.,spring_available:false},owned,alive,now);}
  fn height(&mut self,player:usize,stats:usize,native_height:f32,airborne:bool,now:u64)->f32{self.height_frame(Sample{player,stats,airborne,native_height,world_y:0.,spring_available:false},now)}
 }
 fn spring_sample(airborne:bool,y:f32,native:f32,available:bool)->Sample {Sample{player:1,stats:2,airborne,world_y:y,native_height:native,spring_available:available}}
 #[test]fn logarithmic_anchors_and_invalid_heights(){
  for (height,expected) in [(0.,0.),(0.25,0.),(0.5,0.),(1.,1.),(2.,2.),(4.,3.),(8.,4.),(16.,5.)] {assert!((bonus(height)-expected).abs()<1e-12);}
  for height in [f32::NAN,f32::INFINITY,f32::NEG_INFINITY,-1.] {assert_eq!(bonus(height),0.);}
  assert!(bonus(10.)>bonus(9.));
 }
 #[test]fn crystal_launch_hint_works_without_a_bigjump_region(){
  assert_eq!(CRYSTAL_SPRING_EFFECT,98215);
  for height in [0.,-1.,f32::NAN,f32::INFINITY] {
   assert!(!spring_launch_available(height,false));assert!(spring_launch_available(height,true));
  }
  assert!(spring_launch_available(100.,false));
  let mut state=Landing::default();
  state.observe_frame(spring_sample(false,-796.,0.,spring_launch_available(0.,true)),true,true,1000);
  state.observe_frame(spring_sample(true,-700.,-96.,false),true,true,1100);
  assert_eq!(state.height_frame(spring_sample(true,-704.,-92.,false),1200),4.);
  assert_eq!(boosted(100,4.),400);
  assert_eq!(state.height_frame(spring_sample(false,-796.,0.,false),1300),96.);
  assert_eq!(state.height_frame(spring_sample(false,-796.,0.,false),1551),0.);
 }
 #[test]fn only_native_jump_category_matches(){
  for category in 0..256 {let mut mask=[0;8];mask[category/32]|=1<<(category%32);assert_eq!(is_jump(&mask),category==102);}
 }
 #[test]fn preserves_non_damage_rounds_and_saturates(){
  for (damage,height,expected) in [(100,1.,200),(100,2.,300),(100,4.,400),(101,1.,202),(101,2f32.sqrt(),252),(0,60.,0),(-1,60.,-1),(i32::MAX,60.,i32::MAX)] {assert_eq!(boosted(damage,height),expected);}
 }
 #[test]fn landing_record_expires_without_grounded_samples_renewing_it(){
  let mut state=Landing::default();state.observe(1,2,true,true,true,4.,1000);
  assert_eq!(state.height(1,2,0.,false,1010),4.);
  for now in [1050,1100,1259,1260]{assert_eq!(state.height(1,2,8.,false,now),4.);}
  for now in [1261,2000]{assert_eq!(state.height(1,2,8.,false,now),0.);}
 }
 #[test]fn new_takeoff_discards_prior_landing(){
  let mut state=Landing::default();state.observe(1,2,true,true,true,8.,1000);
  assert_eq!(state.height(1,2,0.,false,1010),8.);
  assert_eq!(state.height(1,2,0.,true,1100),0.);
  assert_eq!(state.height(1,2,1.,true,1110),1.);
 }
 #[test]fn death_owner_loss_and_character_replacement_discard_prior_fall(){
  for (player,stats,owned,alive) in [(3,2,true,true),(1,3,true,true),(1,2,false,true),(1,2,true,false)] {
   let mut state=Landing::default();assert_eq!(state.height(1,2,30.,true,1000),30.);
   state.observe(player,stats,owned,alive,false,0.,1010);
   assert_eq!(state.height(player,stats,0.,false,1020),0.);
  }
 }
 #[test]fn uses_current_native_descent_not_tracked_apex(){
  let mut state=Landing::default();assert_eq!(state.height(1,2,4.,true,1000),4.);
  assert_eq!(state.height(1,2,1.,true,1010),1.);
  assert_eq!(state.height(1,2,0.,false,1020),1.);
 }
 #[test]fn invalid_height_discards_record(){
  let mut state=Landing::default();state.height(1,2,4.,true,1000);
  assert_eq!(state.height(1,2,f32::NAN,false,1010),0.);
  assert_eq!(state.height(1,2,0.,false,1020),0.);
 }
 #[test]fn four_segments_recompute_damage_and_late_segments_lose_bonus(){
  let mut state=Landing::default();
  assert_eq!(boosted(50,state.height(1,2,2.,true,1000)),150);
  assert_eq!(boosted(50,state.height(1,2,4.,true,1005)),200);
  state.observe(1,2,true,true,false,0.,1010);
  assert_eq!(boosted(50,state.height(1,2,0.,false,1260)),200);
  assert_eq!(boosted(50,state.height(1,2,0.,false,1261)),50);
 }
 #[test]fn repeated_targets_and_hits_do_not_extend_landing_grace(){
  let mut state=Landing::default();state.height(1,2,8.,true,1000);
  for now in [1010,1010,1100,1200,1260]{assert_eq!(state.height(1,2,0.,false,now),8.);}
  for now in [1261,1300,1900]{assert_eq!(state.height(1,2,0.,false,now),0.);}
 }
 #[test]fn spring_rise_has_no_bonus_and_descent_uses_actual_apex(){
  let mut state=Landing::default();state.observe_frame(spring_sample(false,100.,0.,true),true,true,1000);
  for (now,y) in [(1010,110.),(1100,140.),(1200,150.)]{assert_eq!(state.height_frame(spring_sample(true,y,100.-y,false),now),0.);}
  assert_eq!(state.height_frame(spring_sample(true,140.,-40.,false),1300),10.);
  assert_eq!(state.height_frame(spring_sample(true,120.,-20.,false),1400),30.);
 }
 #[test]fn spring_candidate_survives_leaving_region_on_first_airborne_frame(){
  let mut state=Landing::default();state.observe_frame(spring_sample(false,100.,0.,true),true,true,1000);
  state.observe_frame(spring_sample(true,148.,-48.,false),true,true,1010);
  assert_eq!(state.height_frame(spring_sample(true,140.,-40.,false),1100),8.);
 }
 #[test]fn normal_flight_keeps_native_height_even_with_a_large_world_y_apex(){
  let mut state=Landing::default();state.observe_frame(spring_sample(false,100.,0.,false),true,true,1000);
  state.observe_frame(spring_sample(true,150.,-50.,false),true,true,1010);
  assert_eq!(state.height_frame(spring_sample(true,140.,-40.,false),1100),0.);
  assert_eq!(state.height_frame(spring_sample(true,98.,2.,false),1200),2.);
 }
 #[test]fn entering_spring_region_midair_does_not_convert_an_existing_normal_flight(){
  let mut state=Landing::default();state.observe_frame(spring_sample(false,100.,0.,false),true,true,1000);
  state.observe_frame(spring_sample(true,150.,-50.,false),true,true,1010);
  assert_eq!(state.height_frame(spring_sample(true,140.,-40.,true),1100),0.);
  assert_eq!(state.height_frame(spring_sample(true,130.,-30.,false),1200),0.);
 }
 #[test]fn spring_four_hits_recompute_and_landing_grace_is_not_renewed(){
  let mut state=Landing::default();state.observe_frame(spring_sample(false,100.,0.,true),true,true,1000);
  state.observe_frame(spring_sample(true,148.,-48.,false),true,true,1010);
  assert_eq!(state.height_frame(spring_sample(true,144.,-44.,false),1100),4.);
  assert_eq!(state.height_frame(spring_sample(true,140.,-40.,false),1200),8.);
  assert_eq!(state.height_frame(spring_sample(false,100.,0.,false),1300),48.);
  assert_eq!(state.height_frame(spring_sample(false,100.,0.,false),1550),48.);
  assert_eq!(state.height_frame(spring_sample(false,100.,0.,false),1551),0.);
  assert_eq!(state.height_frame(spring_sample(false,100.,8.,false),1600),0.);
 }
 #[test]fn new_normal_takeoff_cannot_inherit_previous_spring_peak(){
  let mut state=Landing::default();state.observe_frame(spring_sample(false,100.,0.,true),true,true,1000);
  state.observe_frame(spring_sample(true,150.,-50.,false),true,true,1010);
  state.observe_frame(spring_sample(false,100.,0.,false),true,true,1200);
  assert_eq!(state.height_frame(spring_sample(true,101.,-1.,false),1210),0.);
  assert_eq!(state.height_frame(spring_sample(true,98.,2.,false),1300),2.);
 }
 #[test]fn spring_tracking_is_discarded_on_death_owner_loss_or_invalid_position(){
  for (owned,alive,y) in [(false,true,100.),(true,false,100.),(true,true,f32::NAN)]{
   let mut state=Landing::default();state.observe_frame(spring_sample(false,100.,0.,true),true,true,1000);
   state.observe_frame(spring_sample(true,150.,-50.,false),true,true,1010);
   state.observe_frame(spring_sample(false,y,0.,false),owned,alive,1100);
   assert_eq!(state.height_frame(spring_sample(true,130.,-30.,false),1200),0.);
  }
 }
}
