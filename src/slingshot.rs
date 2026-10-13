//! An accepted sprint animation request arms a launch; one layer per input press.
pub const OWNER:i32=95002300;
pub const MARKER:i32=6999960;
pub const START:i32=6999961;
pub const JUMP:i32=6999962;
pub const BUSY:i32=6999963;
pub const CANCEL:i32=6999964;
pub const INPUT_DOWN:i32=6999965;
pub const INPUT_UP:i32=6999966;
pub const REQUEST:i32=6999967;
pub const BUFFS:[i32;3]=[95002301,95002302,95002303];
// Native TAE 202050 event 530, shared by all nine player animation sets.
pub const SPRINT_JUMP_ROW:i32=131;
pub fn ground_contact(current:bool,completed:bool)->bool{current||completed}
#[derive(Default)]
pub struct State {
 pub player:usize,pub stats:usize,
 pending:u8,window:f32,flight:u8,heartbeat:f32,launch_wait:f32,
 seen_air:bool,impulse_used:bool,
 input_down:bool,press_available:bool,
}
impl State {
 pub fn synchronize(&mut self,player:usize,stats:usize,owned:bool,alive:bool){
  if self.player!=player||self.stats!=stats||!owned||!alive{*self=Self{player,stats,..Self::default()};}
 }
 pub fn notify(&mut self,token:i32,_grounded:bool){
  match token {
   // A held button can request the same event on several updates. Only a new
   // input edge enables another layer; action-node activation can arrive later.
   INPUT_DOWN=>{if !self.input_down{self.press_available=true;}self.input_down=true;},
   INPUT_UP=>{self.input_down=false;},
   REQUEST if self.press_available=>{self.press_available=false;self.pending=if self.window>0.{(self.pending+1).min(3)}else{1};self.window=1.;self.flight=0;},
   CANCEL=>{self.pending=0;self.window=0.;self.press_available=false;},
   // A jump-attack transition can activate the jump node again, without a new
   // native takeoff. Do not overwrite the already reserved/confirmed layers.
   JUMP if self.flight==0=>{self.flight=if self.window>0.{self.pending}else{0};self.pending=0;self.window=0.;self.seen_air=false;self.impulse_used=false;self.launch_wait=0.25;self.heartbeat=0.1;},
   BUSY=>{if self.flight>0{self.heartbeat=0.1;}},
   _=>{},
  }
 }
 pub fn tick(&mut self,dt:f32,grounded:bool){
  if !dt.is_finite()||dt<=0.{return;}
  self.window=(self.window-dt).max(0.);if self.window==0.{self.pending=0;}
  self.heartbeat=(self.heartbeat-dt).max(0.);
  self.launch_wait=(self.launch_wait-dt).max(0.);
  if !grounded{self.seen_air=true;self.pending=0;self.window=0.;}
  if grounded&&self.flight>0&&self.heartbeat==0.&&(self.seen_air||self.launch_wait==0.){self.flight=0;}
 }
 pub fn impulse(&mut self,row:i32)->f32 {
  // The HKS jump callback only reserves the layers. Confirm the actual native
  // sprint takeoff here; ordinary jumps consume the reservation without a buff.
  if matches!(row,100|101|111|121){self.flight=0;return 1.;}
  // Wall jumps, spiritsprings, knockbacks and ride-off launches are untouched.
  if row!=SPRINT_JUMP_ROW{return 1.;}
  // The original function consumes its velocity row. Another row 131 is a
  // genuinely new takeoff, even within the previous landing heartbeat grace.
  if self.impulse_used{self.flight=0;return 1.;}
  if self.flight==0{return 1.;}
  self.impulse_used=true;1.+0.5*self.flight as f32
 }
 pub fn buff(&self)->Option<i32>{if !self.impulse_used{return None;}self.flight.checked_sub(1).map(|i|BUFFS[i as usize])}
}
#[cfg(test)]mod tests{
 use super::*;
 fn request(s:&mut State){s.notify(INPUT_UP,true);s.notify(INPUT_DOWN,true);s.notify(REQUEST,true);}
 fn start()->State{let mut s=State::default();s.synchronize(1,2,true,true);request(&mut s);s}
 #[test]fn repeated_starts_refresh_window_and_cap_at_three(){let mut s=start();for _ in 0..5{s.tick(0.75,true);request(&mut s);}s.tick(0.99,true);s.notify(JUMP,true);assert_eq!(s.impulse(131),2.5);assert_eq!(s.buff(),Some(BUFFS[2]));}
 #[test]fn holding_sprint_does_not_stack_or_refresh(){let mut s=start();s.tick(1.01,true);s.notify(JUMP,true);assert_eq!(s.impulse(131),1.);assert_eq!(s.buff(),None);}
 #[test]fn expired_starts_restart_at_one(){let mut s=start();request(&mut s);s.tick(1.1,true);request(&mut s);s.notify(JUMP,true);assert_eq!(s.impulse(131),1.5);}
 #[test]fn one_impulse_only_and_other_physics_events_unchanged(){let mut s=start();s.notify(JUMP,true);assert_eq!(s.impulse(1000),1.);assert_eq!(s.impulse(131),1.5);assert_eq!(s.impulse(131),1.);}
 #[test]fn all_hits_and_landing_attack_keep_locked_layers(){let mut s=start();request(&mut s);s.notify(JUMP,true);s.impulse(131);s.tick(0.1,false);for _ in 0..20{s.notify(BUSY,true);s.tick(0.016,true);for _ in 0..4{assert_eq!(s.buff(),Some(BUFFS[1]));}}s.tick(0.11,true);assert_eq!(s.buff(),None);}
 #[test]fn landing_without_attack_and_cancelled_takeoff_expire(){let mut s=start();s.notify(JUMP,true);s.tick(0.3,true);assert_eq!(s.buff(),None);let mut s=start();s.notify(JUMP,true);s.impulse(131);s.tick(0.5,false);s.tick(0.1,true);assert_eq!(s.buff(),None);}
 #[test]fn ownership_death_and_actor_changes_clear_everything(){for (p,st,own,alive) in [(1,2,false,true),(1,2,true,false),(3,2,true,true),(1,4,true,true)]{let mut s=start();s.notify(JUMP,true);s.impulse(131);s.synchronize(p,st,own,alive);assert_eq!(s.buff(),None);assert_eq!(s.impulse(131),1.);}}
 #[test]fn walking_off_ledge_consumes_pending_and_pause_preserves_window(){let mut s=start();s.tick(f32::NAN,true);s.tick(0.,true);s.notify(JUMP,true);assert_eq!(s.impulse(131),1.5);let mut s=start();s.tick(0.016,false);s.notify(JUMP,false);assert_eq!(s.impulse(131),1.);}
 #[test]fn ordinary_jumps_never_boost_or_leave_reserved_layers(){for row in [100,101,111,121]{for layers in 1..=3{let mut s=start();for _ in 1..layers{request(&mut s);}s.notify(JUMP,true);assert_eq!(s.buff(),None);assert_eq!(s.impulse(row),1.);assert_eq!(s.buff(),None);assert_eq!(s.impulse(131),1.);}}}
 #[test]fn sprint_takeoff_confirms_each_layer_and_damage_bonus(){for layers in 1..=3{let mut s=start();for _ in 1..layers{request(&mut s);}s.notify(JUMP,true);assert_eq!(s.buff(),None);assert_eq!(s.impulse(131),1.+0.5*layers as f32);assert_eq!(s.buff(),Some(BUFFS[layers-1]));assert_eq!(s.impulse(131),1.);assert_eq!(s.buff(),None);}}
 #[test]fn accepted_rapid_starts_stack_despite_in_progress_ground_flag(){let mut s=State::default();s.synchronize(1,2,true,true);for _ in 0..3{request(&mut s);s.tick(0.15,ground_contact(false,true));}s.notify(JUMP,true);assert_eq!(s.impulse(131),2.5);assert_eq!(s.buff(),Some(BUFFS[2]));}
 #[test]fn explicit_cancel_clears_layers_and_next_start_begins_at_one(){let mut s=start();request(&mut s);request(&mut s);s.notify(CANCEL,true);s.notify(CANCEL,true);s.notify(JUMP,true);assert_eq!(s.impulse(131),1.);assert_eq!(s.buff(),None);request(&mut s);s.notify(JUMP,true);assert_eq!(s.impulse(131),1.5);}
 #[test]fn confirmed_air_clears_pending_but_contact_refresh_does_not(){let mut s=start();for (current,completed) in [(false,true),(true,false),(true,true)]{s.tick(0.02,ground_contact(current,completed));assert_eq!(s.pending,1);}s.tick(0.02,ground_contact(false,false));s.notify(JUMP,false);assert_eq!(s.impulse(131),1.);}
 #[test]fn early_landing_keeps_entire_falling_attack_until_its_callbacks_end(){let mut s=start();request(&mut s);request(&mut s);s.notify(JUMP,true);assert_eq!(s.impulse(131),2.5);s.tick(0.016,false);s.notify(BUSY,false);for frame in 0..120{s.notify(BUSY,true);s.tick(1./60.,true);if [30,45,60,90,119].contains(&frame){assert_eq!(s.buff(),Some(BUFFS[2]));}}s.tick(0.11,true);assert_eq!(s.buff(),None);s.notify(BUSY,true);assert_eq!(s.buff(),None);s.notify(JUMP,true);assert_eq!(s.impulse(131),1.);}
 #[test]fn recorded_jump_attack_reactivation_does_not_erase_three_layers(){
  let mut s=start();request(&mut s);request(&mut s);s.notify(JUMP,true);
  s.notify(JUMP,true);assert_eq!(s.impulse(131),2.5);
  s.tick(0.067,false);s.notify(JUMP,false);
  assert_eq!(s.buff(),Some(BUFFS[2]));
  for _ in 0..90{s.notify(BUSY,true);s.tick(1./60.,true);assert_eq!(s.buff(),Some(BUFFS[2]));}
 }
 #[test]fn new_native_takeoff_cannot_inherit_previous_landing_bonus(){
  let mut s=start();s.notify(JUMP,true);s.impulse(131);s.tick(0.2,false);
  s.notify(BUSY,true);s.tick(0.016,true);s.notify(JUMP,true);
  assert_eq!(s.impulse(131),1.);assert_eq!(s.buff(),None);
  request(&mut s);s.notify(JUMP,true);assert_eq!(s.impulse(131),1.5);
 }

 #[test]fn rapid_animation_requests_count_without_node_reactivation(){
  let mut s=start();s.tick(0.05,true);request(&mut s);s.tick(0.05,true);request(&mut s);
  s.notify(JUMP,true);assert_eq!(s.impulse(131),2.5);
 }
 #[test]fn held_press_requests_and_delayed_activation_do_not_double_count(){
  let mut s=start();for _ in 0..10{s.notify(INPUT_DOWN,true);s.notify(REQUEST,true);s.notify(START,true);s.tick(0.016,true);}
  s.notify(JUMP,true);assert_eq!(s.impulse(131),1.5);
 }
 #[test]fn rejected_input_never_counts_and_late_request_uses_same_press_once(){
  let mut s=State::default();s.notify(INPUT_DOWN,true);s.notify(INPUT_UP,true);
  assert_eq!(s.pending,0);s.notify(REQUEST,true);assert_eq!(s.pending,1);
  for _ in 0..5{s.notify(INPUT_UP,true);s.notify(REQUEST,true);}assert_eq!(s.pending,1);
 }
 #[test]fn cancellation_consumes_the_press_but_next_press_can_start_fresh(){
  let mut s=start();s.notify(INPUT_UP,true);s.notify(INPUT_DOWN,true);s.notify(CANCEL,true);
  s.notify(REQUEST,true);assert_eq!(s.pending,0);
  request(&mut s);s.notify(JUMP,true);assert_eq!(s.impulse(131),1.5);
 }
 #[test]fn holding_past_timeout_cannot_rearm_without_a_new_press(){
  let mut s=start();s.tick(1.1,true);s.notify(INPUT_DOWN,true);s.notify(REQUEST,true);
  s.notify(JUMP,true);assert_eq!(s.impulse(131),1.);request(&mut s);s.notify(JUMP,true);assert_eq!(s.impulse(131),1.5);
 }

}
