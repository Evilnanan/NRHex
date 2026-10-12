//! Native HKS charge callbacks consume at most one frame's regeneration budget.
pub const OWNER:i32=95002000;
pub const ENABLED:i32=6999920;
pub const PULSE:i32=6999921;
pub const RATE:f64=30.; // Stamina per game-time second while actively charging.
#[derive(Default)]
pub struct Chant { identity:(usize,usize), active:bool, dt:f64, used:bool, fraction:f64 }
impl Chant {
 pub fn frame(&mut self,player:usize,stats:usize,active:bool,dt:f32){
  if self.identity!=(player,stats)||!active||!self.used{self.fraction=0.;}
  self.identity=(player,stats);self.active=active;self.used=false;
  self.dt=if dt.is_finite()&&dt>0.&&dt<=1.{dt as f64}else{0.};
 }
 pub fn pulse(&mut self,player:usize,stats:usize,active:bool,full:bool)->i32{
  if !active||!self.active||self.identity!=(player,stats)||self.used{return 0;}
  self.used=true;
  if full{self.fraction=0.;return 0;}
  self.fraction+=RATE*self.dt;
  let whole=(self.fraction+1e-7).floor() as i32;
  self.fraction=(self.fraction-whole as f64).max(0.);whole
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn charge_rate_is_thirty_at_all_framerates(){for fps in [30,60,120,240]{let mut s=Chant::default();let mut recovered=0;for _ in 0..fps{s.frame(1,2,true,1./fps as f32);recovered+=s.pulse(1,2,true,false);assert_eq!(s.pulse(1,2,true,false),0);}assert_eq!(recovered,30);}}
 #[test]fn no_charge_no_recovery_or_stored_credit(){let mut s=Chant::default();s.frame(1,2,true,1./120.);assert_eq!(s.pulse(1,2,true,false),0);s.frame(1,2,true,1./120.);s.frame(1,2,true,1./120.);assert_eq!(s.pulse(1,2,true,false),0);}
 #[test]fn death_identity_full_and_invalid_delta(){for invalid in [f32::NAN,-1.,0.,2.]{let mut s=Chant::default();s.frame(1,2,true,invalid);assert_eq!(s.pulse(1,2,true,false),0);}let mut s=Chant::default();s.frame(1,2,true,1.);assert_eq!(s.pulse(1,3,true,false),0);assert_eq!(s.pulse(1,2,false,false),0);assert_eq!(s.pulse(1,2,true,true),0);s.frame(1,2,false,1.);assert_eq!(s.pulse(1,2,true,false),0);}
}
