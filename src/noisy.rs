//! One native lava chain per self-ping; rearm after its final native cleanup.
use std::collections::BTreeSet;
pub const OWNER:i32=95001500;
pub const EMIT:i32=95001501;
pub const BULLET_BASE:i32=95001550;
pub const ATTACK_BASE:i32=95001560;
pub const MAX_STACKS:u8=15;
// CSBulletIns kind 2 is ordinary Bullet: the current PARAM reference lives
// at +5C0 (row +5C8, ID +5D0). Kind 3 at +5F0 describes its child emission;
// that ID can already name the next phase and must not freeze this phase.
pub const PARAM_ID_OFFSET:usize=0x5d0;
pub fn ordinary_reference(index:i32,kind:i32)->bool{(0..=4).contains(&index)&&kind==2}
#[derive(Default)]
pub struct Lava {player:usize,stats:usize,owned:bool,pending:bool,live:bool,open:bool,age:f32,stacks:u8,seen:bool,empty_frames:u8,instances:BTreeSet<usize>}
impl Lava {
 pub fn synchronize(&mut self,player:usize,stats:usize,owned:bool){
  if self.player!=player||self.stats!=stats||!owned { *self=Self{player,stats,owned,..Self::default()}; }
  self.owned=owned;
 }
 pub fn mark(&mut self,alive:bool){
  if !self.owned||!alive{return;}
  if self.live&&self.open {self.stacks=self.stacks.saturating_add(1).min(MAX_STACKS);}
  else if !self.live&&!self.pending {self.pending=true;self.live=true;self.open=true;self.age=0.;self.stacks=0;self.seen=false;self.empty_frames=0;self.instances.clear();}
 }
 pub fn tick(&mut self,dt:f32,alive:bool)->bool{
  if dt.is_finite()&&dt>0. {self.age+=dt;}
  // Allow the native parent/child replacement to complete across an update
  // boundary. Completion depends on actual instances, not a fixed duration.
  if self.live&&self.seen&&self.instances.is_empty(){self.empty_frames=self.empty_frames.saturating_add(1);}else{self.empty_frames=0;}
  if self.empty_frames>=2 {self.live=false;self.open=false;}
  // Failed native emission must not permanently lock placement. Normal seed
  // and placement stages last at most two seconds before the first lava SFX.
  if self.live&&!self.seen&&!self.pending&&self.age>4. {self.live=false;self.open=false;}
  let pending=std::mem::take(&mut self.pending);let emit=pending&&alive&&self.owned;
  if pending&&!emit {self.live=false;self.open=false;}
  emit
 }
 pub fn emitted(&mut self,ok:bool){if !ok {self.live=false;self.open=false;}}
 pub fn created(&mut self,player:usize,instance:usize,bullet:i32){
  if self.player!=player||!self.live||!lava_bullet(bullet){return;}
  self.instances.insert(instance);self.seen=true;self.empty_frames=0;
  if bullet>=BULLET_BASE+4 {self.open=false;}
 }
 pub fn removed(&mut self,instance:usize){self.instances.remove(&instance);}
 pub fn factor(&self,player:usize)->f32{if self.player==player&&self.live {factor(self.stacks)}else{1.}}
}
pub fn factor(stacks:u8)->f32{1.+0.3*stacks.min(MAX_STACKS) as f32}
pub fn lava_bullet(id:i32)->bool{(BULLET_BASE..=BULLET_BASE+7).contains(&id)}
pub fn lava_attack(id:i32,kind:i32)->bool{kind==1&&(id==ATTACK_BASE+4||id==ATTACK_BASE+6)}
pub fn visual_scale(factor:f32)->f32{factor.sqrt()}
pub fn scale_matrix(mut matrix:[[f32;4];4],scale:f32)->[[f32;4];4]{
 for row in matrix.iter_mut().take(3){for v in row.iter_mut().take(3){*v*=scale;}}
 matrix
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn repeated_marks_only_stack_the_existing_charge(){
  let mut l=Lava::default();l.synchronize(1,2,true);l.mark(true);assert!(l.tick(0.016,true));
  l.created(1,10,BULLET_BASE+2);assert_eq!(l.factor(1),1.);
  for _ in 0..20 {l.mark(true);}assert_eq!(l.factor(1),5.5);assert!(!l.tick(0.016,true));
  l.created(2,11,BULLET_BASE+4);assert!(l.open);l.created(1,11,BULLET_BASE+4);assert!(!l.open);
  l.mark(true);assert_eq!(l.stacks,15);assert!(!l.tick(20.,true));assert!(l.live);
 }
 #[test]fn only_final_native_cleanup_rearms_and_parent_child_gaps_do_not(){
  let mut l=Lava::default();l.synchronize(1,2,true);l.mark(true);assert!(l.tick(0.,true));
  l.created(1,10,BULLET_BASE+2);l.removed(10);l.tick(0.016,true);assert!(l.live);
  l.created(1,11,BULLET_BASE+4);l.created(1,12,BULLET_BASE+7);l.created(1,13,BULLET_BASE+7);
  l.removed(11);l.removed(12);l.tick(30.,true);l.mark(true);assert!(!l.tick(0.016,true));assert!(l.live);
  l.removed(13);l.tick(0.016,true);l.tick(0.016,true);assert!(!l.live);
  l.mark(true);assert!(l.tick(0.,true));assert_eq!(l.stacks,0);
 }
 #[test]fn death_ownership_identity_and_failed_emission_do_not_lock_placement(){
  let mut l=Lava::default();l.synchronize(1,2,true);l.mark(false);assert!(!l.tick(0.,true));
  l.mark(true);l.synchronize(3,4,true);assert!(!l.tick(0.,true));
  l.mark(true);l.synchronize(3,4,false);assert!(!l.tick(0.,true));
  l.synchronize(3,4,true);l.mark(true);assert!(l.tick(0.,true));l.emitted(false);assert!(!l.live);
  l.mark(true);l.tick(0.,true);l.tick(4.1,true);assert!(!l.live);
 }
 #[test]fn scale_only_changes_visual_basis(){
  let m=[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[42.,7.,-3.,1.]];
  let out=scale_matrix(m,visual_scale(5.5));assert_eq!(out[3],m[3]);assert_eq!(out[0][3],0.);assert!((out[0][0].powi(2)-5.5).abs()<1e-5);
  assert!(lava_attack(ATTACK_BASE+4,1));assert!(!lava_attack(12048214,1));assert!(!lava_attack(ATTACK_BASE+4,2));
 }
 #[test]fn ordinary_reference_uses_kind_two_and_accepts_all_native_stage_slots(){
  for index in 0..=4 {assert!(ordinary_reference(index,2));assert!(!ordinary_reference(index,1));assert!(!ordinary_reference(index,3));}
  assert!(!ordinary_reference(-1,2));assert!(!ordinary_reference(5,2));assert_eq!(PARAM_ID_OFFSET,0x5d0);
 }
}
