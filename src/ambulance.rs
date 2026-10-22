//! Waive locomotion costs toward a downed ally. Share normalized outgoing
//! rescue progress across targets, redeem full segments and decay idle progress.
use crate::{has_effect,is_local,native_actor,ptr,read_i32,read_ptr,ACTIVE,BASE};
use std::{sync::{Mutex,atomic::{AtomicPtr,Ordering}}};
pub const OWNER:i32=95003400;
pub const RECOVERY_VISUAL:i32=OWNER+1;
pub const QUERY:i32=990034;
pub const ENV_RVA:usize=0x3f7b50;
pub const ENV_PATTERN:&str="48 8B C4 48 89 58 10 55 56 57 41 54 41 55 41 56 41 57 48 8D 68 A1 48 81 EC 00 01 00 00";
pub const NUMBER_RVA:usize=0xd40560;
pub const NUMBER_PATTERN:&str="E9 AB 8E 88 00 0F 85 B8 DE 59 01 E9 83 DE 59 01 48 89 5C 24 08 57 48 83";
pub const COST_RVA:usize=0x5085a0;
pub const COST_PATTERN:&str="48 8B 41 08 45 33 C9 4C 89 0A 48 85 C0 74 5A 0F 57 C9";
// Actual hit dispatch, after contact handling and before recipient authority.
pub const HIT_RVA:usize=0x43da77;
pub const HIT_PATTERN:&str="49 8B CE E8 A1 E6 FE FF 48 8B 10 48 8B C8 FF 92 18 01 00 00 84 C0 74 39";
pub const MODE_RVA:usize=0x6aeb20;
pub const MODE_PATTERN:&str="48 8B 05 31 47 56 03 48 85 C0 74 08 0F B6 80 1A 0E 00 00 C3";
pub const COMMON_RVA:usize=0xe76d50;
pub const COMMON_PATTERN:&str="48 89 5C 24 08 57 48 83 EC 30 48 8B F9 33 DB 48 89 19 89 59 08 48 8B 0D 14 1B DB 02 48 85 C9 0F 84 CC 00 00 00 45 33 C0 BA 92 00 00 00 E8 FE 78 02 00";
// Verify the complete native rescue arithmetic mirrored below, including tier
// offsets, f32 operation order, truncation and the native minimum-one rule.
pub const FORMULA_RVA:usize=0x66def9;
pub const FORMULA_PATTERN:&str="F3 41 0F 10 B0 50 01 00 00 E8 19 0C 04 00 84 C0 74 50 0F B6 9F 30 07 00 00 48 8D 4C 24 20 E8 34 8E 80 00 F3 0F 10 05 78 57 C8 02 48 8B 08 48 85 C9 74 2B 80 FB 01 75 0A F3 0F 10 81 0C 04 00 00 EB 1C 80 FB 02 75 0A F3 0F 10 81 10 04 00 00 EB 0D 80 FB 03 72 08 F3 0F 10 81 14 04 00 00 F3 0F 59 F0 F3 0F 2C C6 F3 0F 59 B5 58 02 00 00 F3 0F 2C DE 83 F8 01 7C 0A B8 01 00 00 00 3B D8 0F 4C D8";
pub static ENV_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static COST_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static HIT_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
#[derive(Clone,Copy,Default,PartialEq,Eq)]struct Identity{actor:usize,handle:u64,stats:usize}
const IDLE_GRACE:f64=5.;
const STAMINA_PER_SEGMENT:i32=40;
// 720 is divisible by all three native per-segment values (40/45/80).
// Whole rescue outputs therefore combine exactly across tiers. Public units
// use 40 points per segment; 30 points/second means 540 internal units/second.
const SEGMENT_UNITS:f64=720.;
const STANDARD_POINTS_PER_SEGMENT:f64=40.;
const DECAY_PER_SECOND:f64=30.;
#[derive(Default)]struct Progress{value:f64,idle:f64}
impl Progress{
 fn tick(&mut self,dt:f64){
  if !dt.is_finite()||dt<=0.{return;}
  let decay=(dt-(IDLE_GRACE-self.idle).max(0.)).max(0.);
  self.idle=(self.idle+dt).min(IDLE_GRACE);
  self.value=(self.value-DECAY_PER_SECOND*(SEGMENT_UNITS/STANDARD_POINTS_PER_SEGMENT)*decay).max(0.);
 }
 fn add(&mut self,power:i32,segment:f64)->i32{
  if power<=0||!segment.is_finite()||segment<=0.{return 0;}
  self.idle=0.;let value=self.value+f64::from(power)*(SEGMENT_UNITS/segment);
  let segments=(value/SEGMENT_UNITS).floor();self.value=value-segments*SEGMENT_UNITS;
  (segments*f64::from(STAMINA_PER_SEGMENT)).min(i32::MAX as f64)as i32
 }
}
#[derive(Default)]struct State{owner:Identity,frame:u64,sample:Option<(u64,f32,f32)>,progress:Progress}
static STATE:Mutex<State>=Mutex::new(State{owner:Identity{actor:0,handle:0,stats:0},frame:0,sample:None,progress:Progress{value:0.,idle:0.}});
impl State{
 fn bind(&mut self,owner:Identity){if self.owner!=owner{*self=Self{owner,..Self::default()};}}
 fn frame(&mut self,owner:Identity){self.bind(owner);self.frame=self.frame.wrapping_add(1);}
 fn tick(&mut self,owner:Identity,dt:f32){self.frame(owner);self.progress.tick(f64::from(dt));}
 fn sample(&mut self,owner:Identity,angle:f32,speed:f32){self.bind(owner);self.sample=Some((self.frame,angle,speed));}
 fn movement(&self,owner:Identity)->Option<(f32,f32)>{
  let(frame,angle,speed)=self.sample?;
  (owner==self.owner&&self.frame.wrapping_sub(frame)<=1&&angle.is_finite()&&speed.is_finite()&&speed>0.001).then_some((angle,speed))
 }
 fn credit(&mut self,owner:Identity,power:i32,segment:f64,current:i32,maximum:i32)->i32{
  self.bind(owner);
  // Consume every complete segment even at maximum stamina. Only incomplete
  // rescue output remains; no pending stamina is banked for a later frame.
  let reward=self.progress.add(power,segment);
  let room=(i64::from(maximum)-i64::from(current)).max(0).min(i64::from(i32::MAX))as i32;
  if maximum<=0{0}else{reward.min(room)}
 }
}
unsafe fn identity(actor:usize)->Identity{Identity{actor,handle:ptr::read_unaligned((actor+8)as*const u64),stats:native_actor::stats(actor)}}
unsafe fn owned(actor:usize,base:usize)->Option<Identity>{
 if !is_local(actor,base)||!has_effect(actor,OWNER,base){return None;}
 let owner=identity(actor);
 (owner.stats!=0&&!native_actor::incapacitated(actor,owner.stats)).then_some(owner)
}
pub unsafe fn update(actor:usize,base:usize,dt:f32){
 let owner=owned(actor,base);
 if let Ok(mut s)=STATE.lock(){if let Some(owner)=owner{s.tick(owner,dt);}else{*s=State::default();}}
}
// Coordinates use negative Z as forward; positive native yaw turns left.
// Havok MoveAngle is degrees around the character, independent of the camera.
fn within_cone(direction:[f32;2],offset:[f32;2])->bool{
 if !direction.iter().chain(offset.iter()).all(|v|v.is_finite()){return false;}
 let a=f64::from(direction[0]);let b=f64::from(direction[1]);let x=f64::from(offset[0]);let z=f64::from(offset[1]);
 let length=(a*a+b*b)*(x*x+z*z);let dot=a*x+b*z;
 length>1e-12&&dot>0.&&dot*dot>=length*(0.75-1e-7)
}
unsafe fn party_member(actor:usize,base:usize)->bool{
 let world=read_ptr(base+crate::WORLD_RVA);if world==0||actor==0{return false;}
 if read_ptr(world+0x174e8)==actor{return true;}
 let get:unsafe extern "system" fn(usize,i32)->usize=std::mem::transmute(base+0x5197e0);
 (1..=3).any(|slot|get(world,slot)==actor)
}
#[repr(C,align(16))]struct Vector([f32;4]);
unsafe fn heading(actor:usize,base:usize,angle:f32)->Option<[f32;2]>{
 if !crate::spatial_ready(actor)||!angle.is_finite(){return None;}
 let mut rotation=Vector([0.;4]);
 let get:unsafe extern "system" fn(usize,*mut f32)->*const f32=std::mem::transmute(base+crate::CHR_ROTATION_RVA);
 get(actor,rotation.0.as_mut_ptr());let yaw=rotation.0[1]-angle.to_radians();
 yaw.is_finite().then(||[-yaw.sin(),-yaw.cos()])
}
unsafe fn toward_down(owner:Identity,base:usize,angle:f32,speed:f32)->bool{
 if !speed.is_finite()||speed<=0.001{return false;}
 let Some(direction)=heading(owner.actor,base,angle)else{return false;};
 let world=read_ptr(base+crate::WORLD_RVA);if world==0{return false;}
 let origin=crate::position(owner.actor,base);
 let get:unsafe extern "system" fn(usize,i32)->usize=std::mem::transmute(base+0x5197e0);
 (1..=3).any(|slot|{
  let target=get(world,slot);
  if target==0||target==owner.actor||ptr::read((target+0x710)as*const u8)!=1||!crate::spatial_ready(target){return false;}
  let position=crate::position(target,base);
  within_cone(direction,[position[0]-origin[0],position[2]-origin[2]])
 })
}
type Env=unsafe extern "system" fn(usize,i32,usize)->i32;
pub unsafe extern "system" fn env(context:usize,code:i32,vm:usize)->i32{
 if matches!(code,crate::basics::ULTIMATE_QUERY|crate::basics::READY_QUERY){
  if ACTIVE.load(Ordering::Acquire)&&context!=0&&vm!=0&&crate::basics::sealed(read_ptr(context),BASE.load(Ordering::Acquire)){return 0;}
  return std::mem::transmute::<*mut u8,Env>(ENV_ORIGINAL.load(Ordering::Acquire))(context,if code==crate::basics::ULTIMATE_QUERY{1106}else{370},vm);
 }
 if matches!(code,crate::wraith::FRAME|crate::wraith::BUSY){if ACTIVE.load(Ordering::Acquire)&&context!=0&&vm!=0{crate::wraith::query(read_ptr(context),code,BASE.load(Ordering::Acquire));}return 0;}
 if code==crate::cat_wall::QUERY{
  if !ACTIVE.load(Ordering::Acquire)||context==0||vm==0{return 0;}
  return i32::from(crate::cat_wall::enabled(read_ptr(context),BASE.load(Ordering::Acquire)));
 }
 if code!=QUERY{return std::mem::transmute::<*mut u8,Env>(ENV_ORIGINAL.load(Ordering::Acquire))(context,code,vm);}
 if !ACTIVE.load(Ordering::Acquire)||context==0||vm==0{return 0;}
 let base=BASE.load(Ordering::Acquire);let actor=read_ptr(context);
 let Some(owner)=owned(actor,base)else{return 0;};
 let number:unsafe extern "system" fn(usize,i32)->f32=std::mem::transmute(base+NUMBER_RVA);
 let angle=number(vm,2);let speed=number(vm,3);
 if let Ok(mut s)=STATE.lock(){s.sample(owner,angle,speed);}
 i32::from(toward_down(owner,base,angle,speed))
}
// Same node eligibility and f32 addition order as 5085A0. Only two native
// locomotion effects are omitted; no effect flag or shared PARAM is mutated.
unsafe fn filtered_cost(manager:usize,out:usize)->usize{
 let mut rate=0f32;let mut points=0f32;let mut node=read_ptr(manager+8);
 while node!=0{
  let flags=ptr::read_unaligned((node+0x68)as*const u32);
  if flags&0x800c0003==0&&flags&4!=0&&!matches!(read_i32(node+8),100300|102022){
   let row=read_ptr(node);
   if row!=0{rate=ptr::read_unaligned((row+0xb0)as*const f32)+rate;points+=(read_i32(row+0xb4))as f32;}
  }
  node=read_ptr(node+0x38);
 }
 ptr::write_unaligned(out as*mut[f32;2],[rate,points]);out
}
pub unsafe extern "system" fn cost(manager:usize,out:usize)->usize{
 let original:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(COST_ORIGINAL.load(Ordering::Acquire));
 if !ACTIVE.load(Ordering::Acquire)||manager==0||out==0{return original(manager,out);}
 let base=BASE.load(Ordering::Acquire);let actor=read_ptr(manager+0x10);
 let Some(owner)=owned(actor,base)else{return original(manager,out);};
 if read_ptr(actor+0x190)!=manager{return original(manager,out);}
 let sample=STATE.lock().ok().and_then(|s|s.movement(owner));
 if sample.is_some_and(|(angle,speed)|toward_down(owner,base,angle,speed)){filtered_cost(manager,out)}else{original(manager,out)}
}
fn rescue_power(raw:f32,tier_rate:f32,hit_rate:f32)->i32{
 let scaled=raw*tier_rate;let final_value=scaled*hit_rate;
 if !scaled.is_finite()||!final_value.is_finite()||scaled<0.||final_value<0.||scaled>=2147483648.||final_value>=2147483648.{return 0;}
 let value=final_value as i32;if scaled as i32>=1{value.max(1)}else{value}
}
unsafe fn common_row(base:usize)->usize{
 let mut reference=[0usize;2];
 let get:unsafe extern "system" fn(usize)->usize=std::mem::transmute(base+COMMON_RVA);
 let result=get(reference.as_mut_ptr()as usize);read_ptr(result)
}
unsafe fn rescue_segment(target:usize,row:usize)->Option<f64>{
 if row==0{return None;}
 let tier=ptr::read((target+0x730)as*const u8).min(3);
 let offset=match tier{1=>0x380,2=>0x384,3=>0x388,_=>return None};
 let total=read_i32(row+offset);
 (total>0).then_some(f64::from(total)/f64::from(tier))
}
unsafe fn outgoing_power(target:usize,hit:usize,base:usize,row:usize)->i32{
 let mut rate=1f32;
 let mode:unsafe extern "system" fn()->u8=std::mem::transmute(base+MODE_RVA);
 if mode()!=0&&row!=0{
  let offset=match ptr::read((target+0x730)as*const u8){1=>0x40c,2=>0x410,3.. =>0x414,_=>0};
  if offset!=0{rate=ptr::read_unaligned((row+offset)as*const f32);}
 }
 rescue_power(ptr::read_unaligned((hit+0x150)as*const f32),rate,ptr::read_unaligned((hit+0x258)as*const f32))
}
unsafe extern "system" fn outgoing(module:usize,source:usize,hit:usize){
 if !ACTIVE.load(Ordering::Acquire)||module==0||hit==0{return;}
 let base=BASE.load(Ordering::Acquire);let Some(owner)=owned(source,base)else{return;};
 let target=read_ptr(module+8);
 if source==target||!party_member(target,base)||ptr::read((target+0x710)as*const u8)!=1{return;}
 // Deliberately never read +714 (remaining rescue). This is outgoing power,
 // including last-hit overflow, not a recipient-side gauge delta.
 let row=common_row(base);let Some(segment)=rescue_segment(target,row)else{return;};
 let power=outgoing_power(target,hit,base,row);
 let amount=STATE.lock().ok().map_or(0,|mut s|s.credit(owner,power,segment,read_i32(owner.stats+0x15c),read_i32(owner.stats+0x160)));
 if amount>0{
  let add:unsafe extern "system" fn(usize,i32,u8)=std::mem::transmute(base+crate::STAMINA_ADD_RVA);add(owner.stats,amount,0);
  let visual:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);visual(source,RECOVERY_VISUAL,0);
 }
}
// 43DA77 is inside 43D260, with RSP aligned to 16, R14 damage module,
// R12 resolved character source, RSI hit. Do not attach to 439E80: it is also
// used for previews/preflight and would award before actual contact dispatch.
#[unsafe(naked)]
pub unsafe extern "system" fn hit_detour(){
 core::arch::naked_asm!(
  "pushfq", "sub rsp,0xc8",
  "mov [rsp+0x20],rax", "mov [rsp+0x28],rcx", "mov [rsp+0x30],rdx",
  "mov [rsp+0x38],r8", "mov [rsp+0x40],r9", "mov [rsp+0x48],r10", "mov [rsp+0x50],r11",
  "movdqu [rsp+0x60],xmm0", "movdqu [rsp+0x70],xmm1", "movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3", "movdqu [rsp+0xa0],xmm4", "movdqu [rsp+0xb0],xmm5",
  "mov rcx,r14", "mov rdx,r12", "mov r8,rsi", "call {callback}",
  "movdqu xmm0,[rsp+0x60]", "movdqu xmm1,[rsp+0x70]", "movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]", "movdqu xmm4,[rsp+0xa0]", "movdqu xmm5,[rsp+0xb0]",
  "mov rax,[rsp+0x20]", "mov rcx,[rsp+0x28]", "mov rdx,[rsp+0x30]",
  "mov r8,[rsp+0x38]", "mov r9,[rsp+0x40]", "mov r10,[rsp+0x48]", "mov r11,[rsp+0x50]",
  "add rsp,0xc8", "popfq", "jmp qword ptr [rip+{original}]",
  callback=sym outgoing,original=sym HIT_ORIGINAL
 );
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn thirty_degree_boundary_and_invalid_or_coincident_positions(){
  for degrees in [0f32,29.9,30.,30.1,45.,90.,180.,-30.,-30.1]{let r=degrees.to_radians();assert_eq!(within_cone([0.,-1.],[r.sin()*10.,-r.cos()*10.]),degrees.abs()<=30.,"{degrees}");}
  for p in [[0.,0.],[f32::NAN,0.],[0.,f32::INFINITY]]{assert!(!within_cone([0.,-1.],p));}
 }
 #[test]fn mixed_tier_outputs_share_exact_segments_and_never_bank_stamina_overflow(){
  let owner=Identity{actor:1,handle:2,stats:3};let mut s=State::default();
  assert_eq!(s.credit(owner,20,40.,0,200),0); // half a tier-one segment
  assert_eq!(s.credit(owner,40,80.,0,200),40); // plus half a tier-three segment
  assert_eq!(s.progress.value,0.);
  for _ in 0..44{assert_eq!(s.credit(owner,1,45.,0,200),0);}
  assert_eq!(s.credit(owner,1,45.,0,200),40);assert_eq!(s.progress.value,0.);
  assert_eq!(s.credit(owner,125,40.,0,200),120);assert_eq!(s.progress.value,90.);
  assert_eq!(s.credit(owner,70,80.,200,200),0);assert_eq!(s.progress.value,0.);
  assert_eq!(s.credit(owner,80,80.,190,200),10);assert_eq!(s.progress.value,0.);
  assert_eq!(s.credit(owner,39,40.,0,200),0);
  let replacement=Identity{handle:10,..owner};assert_eq!(s.credit(replacement,1,40.,0,200),0);assert_eq!(s.progress.value,18.);
 }
 #[test]fn five_second_grace_then_thirty_standard_points_per_second_no_negative_debt(){
  let close=|a:f64,b:f64|assert!((a-b).abs()<1e-9,"{a} != {b}");
  let mut p=Progress::default();assert_eq!(p.add(79,80.),0);let start=p.value;
  p.tick(4.9);close(p.value,start);p.tick(0.1);close(p.value,start);
  p.tick(0.5);close(p.value,start-270.);p.tick(0.5);close(p.value,start-540.);
  assert_eq!(p.add(0,40.),0);p.tick(0.1);assert!((p.value-(start-594.)).abs()<1e-9); // zero output never renews grace
  assert_eq!(p.add(1,80.),0);let current=p.value;p.tick(5.);assert_eq!(p.value,current);
  p.tick(100.);assert_eq!(p.value,0.);p.tick(100.);assert_eq!(p.add(80,80.),40);
  p.add(79,80.);p.tick(5.5);close(p.value,start-270.); // only excess over five seconds decays
  for dt in [0.,-1.,f64::NAN,f64::INFINITY]{p.tick(dt);close(p.value,start-270.);}
 }
 #[test]fn game_time_partitioning_pause_and_cross_tier_output_refresh_shared_grace(){
  let owner=Identity{actor:1,handle:2,stats:3};let mut s=State::default();s.credit(owner,79,80.,0,200);
  for _ in 0..360{s.tick(owner,1./60.);}
  assert!((s.progress.value-171.).abs()<0.001); // 39.5 - 30 standard points
  let before=s.progress.value;s.tick(owner,0.);assert_eq!(s.progress.value,before);
  s.credit(owner,1,45.,0,200);let before=s.progress.value;s.tick(owner,5.);assert_eq!(s.progress.value,before);
  s.tick(owner,0.25);assert!((s.progress.value-(before-135.)).abs()<0.001);
  for (power,segment) in [(0,40.),(-1,40.),(1,f64::NAN),(1,0.)]{let value=s.progress.value;let idle=s.progress.idle;assert_eq!(s.credit(owner,power,segment,0,200),0);assert_eq!(s.progress.value,value);assert_eq!(s.progress.idle,idle);}
 }
 #[test]fn movement_expires_and_generation_changes_clear_it(){
  let a=Identity{actor:1,handle:2,stats:3};let mut s=State::default();s.frame(a);s.sample(a,20.,2.);assert_eq!(s.movement(a),Some((20.,2.)));s.frame(a);assert!(s.movement(a).is_some());s.frame(a);assert!(s.movement(a).is_none());
  s.sample(a,0.,0.);assert!(s.movement(a).is_none());s.sample(a,0.,2.);s.frame(Identity{handle:3,..a});assert!(s.movement(a).is_none());
 }
 #[test]fn filtered_periodic_sum_keeps_other_debits_recovery_and_native_flags(){unsafe{
  let mut manager=[0usize;3];let mut nodes=[[0usize;0x70/8];6];let mut rows=[[0u32;0xb8/4];6];
  for i in 0..6{nodes[i][0]=rows[i].as_mut_ptr()as usize;nodes[i][1]=[100300,102022,100320,99,101,103][i];nodes[i][0x68/8]=4;rows[i][0xb4/4]=[1,1,1,5,20,-3i32][i]as u32;rows[i][0xb0/4]=[0.,0.,1.,2.,100.,-2f32][i].to_bits();if i+1<6{nodes[i][0x38/8]=nodes[i+1].as_ptr()as usize;}}
  nodes[4][0x68/8]=4|0x80000;manager[1]=nodes[0].as_ptr()as usize;let saved=nodes;let mut out=[0f32;2];
  assert_eq!(filtered_cost(manager.as_ptr()as usize,out.as_mut_ptr()as usize),out.as_mut_ptr()as usize);assert_eq!(out,[1.,3.]);assert_eq!(nodes,saved);
 }}
 #[test]fn outgoing_power_matches_native_order_and_minimum(){
  assert_eq!(rescue_power(40.,1.,0.5),20);assert_eq!(rescue_power(40.,0.25,0.5),5);
  assert_eq!(rescue_power(1.,1.,0.01),1);assert_eq!(rescue_power(0.9,1.,0.01),0);
  assert_eq!(rescue_power(0.9,1.,2.),1);assert_eq!(rescue_power(1000.,1.,1.),1000);
  for v in [f32::NAN,f32::INFINITY,-1.,2147483648.]{assert_eq!(rescue_power(v,1.,1.),0);}
 }
}

#[cfg(test)]
#[path="ambulance_native_tests.rs"]mod native_tests;
