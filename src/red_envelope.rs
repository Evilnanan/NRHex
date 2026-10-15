//! One native field-boss lottery, selected by level when Red Envelope is acquired.
use crate::{has_effect,read_i32,read_ptr};
use std::sync::{Mutex,atomic::Ordering};
pub const OWNER:i32=95000100;
// Persist the chosen tier before delivery. Level changes while queued cannot reroll it.
pub const MARKERS:[i32;3]=[95000103,95000104,95000105];
pub const RESET:i32=95000106;
pub const LOTS:[i32;2]=[95410000,95410010];
pub fn bonus_lot(lot:i32)->bool { LOTS.iter().any(|&base|(base..base+3).contains(&lot)) }

#[derive(Default)]
struct Delivery { selected:Option<usize>,issued:bool,wait:f32 }
impl Delivery {
 fn observe(&mut self,saved:u8,reset:bool){
  if reset{*self=Self::default();return;}
  if self.selected.is_none(){self.selected=if saved&2!=0{Some(1)}else if saved&1!=0{Some(0)}else{None};}
  self.issued|=saved&4!=0;
 }
 fn select(&mut self,level:i32){
  if self.selected.is_none()&&(1..=15).contains(&level){self.selected=Some(usize::from(level>=7));}
 }
 fn markers(&self)->u8{self.selected.map_or(0,|i|1<<i)|if self.issued{4}else{0}}
 fn next(&mut self,ready:bool,ball:bool,dt:f32)->Option<usize>{
  if dt.is_finite()&&dt>0.{self.wait=(self.wait-dt).max(0.);}
  if !ready||ball||self.wait>0.||self.issued{return None;}
  self.selected
 }
 fn created(&mut self,count:u32){
  // Never repeat an already visible reward after a partial native failure.
  if count>0{self.issued=true;}
  self.wait=0.25;
 }
}
static DELIVERY:Mutex<Delivery>=Mutex::new(Delivery{selected:None,issued:false,wait:0.});

pub unsafe fn update(player:usize,base:usize,dt:f32){
 let manager=read_ptr(player+0x190);if manager==0{return;}
 let reset=has_effect(player,RESET,base);
 let owned=has_effect(player,OWNER,base);
 let Ok(mut state)=DELIVERY.try_lock()else{return;};
 let saved=MARKERS.iter().enumerate().fold(0,|m,(i,&id)|m|if has_effect(player,id,base){1<<i}else{0});
 state.observe(saved,reset);
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);
 if reset{
  let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);
  for id in MARKERS{if has_effect(player,id,base){remove(manager,id);}}
  if MARKERS.iter().all(|&id|!has_effect(player,id,base)){remove(manager,RESET);}
  return;
 }
 // Do not mistake character restoration or missing ownership for a new run.
 if !owned{return;}
 if state.selected.is_none(){
  let game=read_ptr(base+crate::GAME_DATA_RVA);
  if game==0{return;}
  let data=read_ptr(game+8);if data==0{return;}
  state.select(read_i32(data+0x68));
 }
 // Reserve in RAM immediately and retry persistence until native acknowledgement.
 let wanted=state.markers();
 for (i,&id) in MARKERS.iter().enumerate(){if wanted&(1<<i)!=0&&saved&(1<<i)==0{apply(player,id,0);}}
 let stats=crate::native_actor::stats(player);
 let ready=stats!=0&&!crate::native_actor::incapacitated(player,stats)&&crate::spatial_ready(player)
  &&has_effect(player,crate::level_rewards::EXPEDITION,base)&&has_effect(player,95000101,base)
  &&saved&wanted==wanted;
 let ball=crate::unclaimed_hex_ball(base);
 let Some(index)=state.next(ready,ball,dt)else{return;};
 if crate::CALLBACK_LOCK.swap(true,Ordering::AcqRel){return;}
 let count=crate::spawn_personal_lot(player,base,LOTS[index]);
 state.created(count);
 if count>0{apply(player,MARKERS[2],0);}
 if count!=3{crate::nlog_error!("Red Envelope native reward {} created {} choices; receipt reserved={}",LOTS[index],count,count>0);}
 crate::CALLBACK_LOCK.store(false,Ordering::Release);
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn level_seven_boundary_is_captured_once_and_waits_for_existing_ball(){
  for level in 1..=15{
   let mut d=Delivery::default();d.select(level);let index=usize::from(level>=7);
   assert_eq!(d.next(true,true,1.),None);
   d.select(if level<7{15}else{1});assert_eq!(d.next(true,false,0.),Some(index));
   assert_eq!(d.markers(),1<<index);d.created(3);
   for saved in [0,1<<index,(1<<index)|4]{d.observe(saved,false);assert_eq!(d.next(true,false,1.),None);}
  }
  for level in [-1,0,16,999]{let mut d=Delivery::default();d.select(level);assert_eq!(d.next(true,false,1.),None);}
 }
 #[test]fn reload_revival_and_new_run_preserve_selected_tier_and_delivery(){
  for index in 0..2{
   let mut d=Delivery::default();d.observe(1<<index,false);d.select(if index==0{15}else{1});
   assert_eq!(d.next(false,false,1.),None);d.observe(0,false);
   assert_eq!(d.next(true,false,1.),Some(index));
   d.observe((1<<index)|4,false);assert_eq!(d.next(true,false,1.),None);
   let mut restarted=Delivery::default();restarted.observe(d.markers(),false);assert_eq!(restarted.next(true,false,1.),None);
   d.observe(7,true);assert_eq!(d.next(true,false,1.),None);assert_eq!(d.markers(),0);
   d.select(7);assert_eq!(d.next(true,false,1.),Some(1));
  }
 }
 #[test]fn failed_creation_retries_but_partial_creation_never_duplicates(){
  let mut d=Delivery::default();d.select(6);d.created(0);assert_eq!(d.next(true,false,0.1),None);
  assert_eq!(d.next(true,false,1.),Some(0));d.created(1);
  assert_eq!(d.next(true,false,1.),None);assert!(d.issued);
  for id in [6100000,6110200,95200000,95410003,95410009,95410013]{assert!(!bonus_lot(id));}
  for base in LOTS{for id in base..base+3{assert!(bonus_lot(id));}}
 }
}
