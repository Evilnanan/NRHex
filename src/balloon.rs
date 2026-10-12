//! Once per life transition, with the pre-transition maximum HP snapshot.
pub const OWNER:i32=95001700;
pub const MARKER:i32=6999900;
pub const DOWN:i32=6999901;
pub const REVIVED:i32=6999902;
pub const BEHAVIOR:i32=950017000;
pub const ATTACK:i32=95001760;
#[derive(Default)]
pub struct Balloon{player:usize,stats:usize,owned:bool,spent:bool,pending:bool,damage:i32,observed_down:bool,maximum:i32}
impl Balloon{
 pub fn synchronize(&mut self,player:usize,stats:usize,owned:bool){
  if self.player!=player||self.stats!=stats||!owned{*self=Self{player,stats,owned,..Self::default()};}
  self.owned=owned;
 }
 pub fn marker(&mut self,effect:i32,max_hp:i32){
  if !self.owned{return;}
  if effect==REVIVED{self.spent=false;}
  if effect==DOWN&&!self.spent&&max_hp>0{self.spent=true;self.pending=true;self.damage=max_hp;}
 }
 pub fn owned_by(&self,player:usize,stats:usize)->bool{self.owned&&self.player==player&&self.stats==stats}
 pub fn remember_max(&mut self,hp:i32){if self.owned&&hp>0{self.maximum=hp;}}
 pub fn maximum(&self)->i32{self.maximum}
 pub fn base_hp_for(&self,player:usize,stats:usize)->Option<i32>{(self.owned_by(player,stats)&&self.spent&&self.damage>0).then_some(self.damage)}
 pub fn synchronize_life(&mut self,player:usize,stats:usize,owned:bool,incapacitated:bool){let retained=incapacitated&&self.owned_by(player,stats);self.synchronize(player,stats,owned||retained);self.life(incapacitated);}
 pub fn take(&mut self)->Option<i32>{std::mem::take(&mut self.pending).then_some(self.damage)}
 pub fn take_for(&mut self,player:usize,stats:usize)->Option<i32>{if self.player==player&&self.stats==stats{self.take()}else{None}}
 pub fn life(&mut self,incapacitated:bool){if incapacitated{self.observed_down=true;}else if self.observed_down{self.spent=false;self.observed_down=false;}}
 #[cfg(test)]
 pub fn damage(&self,player:usize)->Option<i32>{(self.owned&&self.player==player&&self.damage>0).then_some(self.damage)}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn base_hp_survives_death_cleanup_without_baking_in_damage_modifiers(){
  let mut b=Balloon::default();b.synchronize(1,2,true);b.marker(DOWN,2000);
  b.synchronize_life(1,2,false,true);assert_eq!(b.take(),Some(2000));
  assert_eq!(b.base_hp_for(1,2),Some(2000));assert_eq!(b.base_hp_for(1,3),None);
  b.marker(DOWN,3000);assert_eq!(b.take(),None);assert_eq!(b.base_hp_for(1,2),Some(2000));
  b.synchronize_life(3,4,true,false);assert_eq!(b.base_hp_for(1,2),None);
 }
 #[test]fn near_death_followed_by_death_only_explodes_once_until_revived(){let mut b=Balloon::default();b.synchronize(1,2,true);b.marker(DOWN,1500);b.marker(DOWN,999);assert_eq!(b.take(),Some(1500));assert_eq!(b.take(),None);b.marker(DOWN,1600);assert_eq!(b.take(),None);b.marker(REVIVED,1600);b.marker(DOWN,1600);assert_eq!(b.take(),Some(1600));}
 #[test]fn identity_loss_and_invalid_max_hp_cannot_emit_or_reuse_stale_damage(){let mut b=Balloon::default();b.synchronize(1,2,true);b.marker(DOWN,0);assert_eq!(b.take(),None);b.marker(DOWN,1000);b.synchronize(1,2,false);assert_eq!(b.take(),None);assert_eq!(b.damage(1),None);b.synchronize(3,4,true);b.marker(DOWN,2000);assert_eq!(b.damage(1),None);assert_eq!(b.take(),Some(2000));}
 #[test]fn direct_death_retains_snapshot_and_rearming_requires_an_observed_down_transition(){let mut b=Balloon::default();b.synchronize(1,2,true);b.marker(DOWN,1500);b.life(false);b.marker(DOWN,2000);assert_eq!(b.take_for(1,3),None);assert_eq!(b.take_for(1,2),Some(1500));b.life(true);b.life(false);b.marker(DOWN,2000);assert_eq!(b.take_for(1,2),Some(2000));}
 #[test]fn native_death_cleanup_cannot_rearm_the_same_down_or_transfer_its_maximum(){let mut b=Balloon::default();b.synchronize_life(1,2,true,false);b.remember_max(1500);b.marker(DOWN,1500);assert_eq!(b.take(),Some(1500));b.synchronize_life(1,2,false,true);assert_eq!(b.maximum(),1500);b.marker(DOWN,1000);assert_eq!(b.take(),None);b.synchronize_life(1,2,true,false);b.marker(DOWN,1700);assert_eq!(b.take(),Some(1700));b.synchronize_life(1,3,false,true);assert!(!b.owned_by(1,3));assert_eq!(b.maximum(),0);}
}
