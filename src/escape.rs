//! Low-HP activation with independent native buff and saved cooldown rows.
pub const OWNER:i32=95001200;
pub const BUFF:i32=95001201;
pub const COOLDOWN:i32=95001202;
pub const READY:i32=95001203;
pub const VISUAL:i32=95001204;
pub const DURATION:f32=10.0;
pub const COOLDOWN_SECONDS:f32=70.0;
pub fn low_hp(hp:i32,max:i32)->bool{hp>0&&max>0&&(hp as f32)/(max as f32)<=40.0_f32*0.01_f32}
#[derive(Default)]
pub struct Escape{player:usize,stats:usize,active:f32,cooldown:f32}
impl Escape{
 pub fn owned(&self)->bool{self.player!=0}
 pub fn ready(&self)->bool{self.cooldown<=0.0&&self.active<=0.0}
 pub fn tick(&mut self,player:usize,stats:usize,owned:bool,alive:bool,hp:i32,max:i32,saved_cooldown:bool,_present:bool,dt:f32)->(bool,bool){
  let dt=if dt.is_finite()&&dt>0.0{dt.min(0.25)}else{0.0};
  if !owned{*self=Self::default();return(false,false);}
  if self.player!=player||self.stats!=stats{self.player=player;self.stats=stats;}
  self.cooldown=(self.cooldown-dt).max(0.0);self.active=(self.active-dt).max(0.0);
  if !alive{self.active=0.0;return(false,false);}
  if self.active<=0.0&&!saved_cooldown&&self.cooldown<=0.0&&low_hp(hp,max){
   self.active=DURATION;self.cooldown=COOLDOWN_SECONDS;return(true,true);
  }
  (false,self.active>0.0)
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn ready_icon_is_hidden_immediately_and_returns_after_cooldown(){let mut e=Escape::default();assert!(e.ready());step(&mut e,100,false,0.0);assert!(!e.ready());for _ in 0..280{step(&mut e,1000,true,0.25);}assert!(e.ready());}
 #[test]fn original_threshold_matches_native_float_comparison_and_excludes_dead(){assert!(low_hp(399,1000));assert!(!low_hp(400,1000));assert!(!low_hp(401,1000));assert!(!low_hp(0,1000));assert!(!low_hp(5,0));assert!(!low_hp(i32::MAX,i32::MAX));}
 fn step(e:&mut Escape,hp:i32,cd:bool,dt:f32)->(bool,bool){e.tick(1,2,true,hp>0,hp,1000,cd,false,dt)}
 #[test]fn healing_keeps_window_and_cooldown_starts_at_activation(){let mut e=Escape::default();assert_eq!(step(&mut e,399,false,0.0),(true,true));for _ in 0..39{assert_eq!(step(&mut e,1000,false,0.25),(false,true));}assert_eq!(step(&mut e,1000,false,0.25),(false,false));for _ in 0..239{assert_eq!(step(&mut e,300,false,0.25),(false,false));}assert_eq!(step(&mut e,300,false,0.25),(true,true));}
 #[test]fn death_cancels_speed_without_resetting_cooldown(){let mut e=Escape::default();step(&mut e,100,false,0.0);assert_eq!(step(&mut e,0,false,0.25),(false,false));assert_eq!(e.tick(3,4,true,true,100,1000,false,false,0.25),(false,false));assert_eq!(e.tick(3,4,false,true,100,1000,false,false,0.25),(false,false));assert_eq!(step(&mut e,100,false,0.0),(true,true));}
 #[test]fn saved_cd_and_invalid_delta_do_not_retrigger(){let mut e=Escape::default();assert_eq!(step(&mut e,100,true,0.0),(false,false));assert_eq!(step(&mut e,100,false,0.0),(true,true));for dt in [f32::NAN,f32::INFINITY,-1.0]{assert_eq!(step(&mut e,100,false,dt),(false,true));}assert_eq!(e.active,10.0);}
}
