//! Delay only HP losses accepted by the native regain branch. Every hit owns
//! its own two-second schedule; paid damage still feeds the one native pool.
use crate::{has_effect,native_actor,read_i32,read_ptr,ACTIVE,BASE};
use std::{cell::Cell,ptr,sync::{Mutex,atomic::{AtomicPtr,Ordering}}};
pub const OWNER:i32=95003000;
pub const FLASK:i32=95003001;
pub const RVA:usize=0x426bae;
pub const PATTERN:&str="8B 8B 40 01 00 00 0F 57 C9 8B C1 2B C5 0F 2F F1 0F";
pub const SET_RVA:usize=0x426b00;
pub const SET_PATTERN:&str="48 89 5C 24 08 48 89 6C 24 18 48 89 74 24 20 89 54 24 10 57 48 83 EC 30 8B A9 40 01 00 00 41";
pub const FINISH_RVA:usize=0x43ced0;
pub const FINISH_PATTERN:&str="48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 20 44 88 44 24 18 57 41 54 41 55 41 56 41 57 48 83 EC";
// Native reset clears protected quota and its timer together. Keep only these
// two fields for this local owner; the next native store still clears pending HP.
pub const PROTECT_RVA:usize=0x426b79;
pub const PROTECT_RESUME_RVA:usize=0x426b84;
pub const PROTECT_PATTERN:&str="48 C7 83 6C 01 00 00 00 00 00 00 C7 83 74 01 00 00 00 00 00 00";
pub static PROTECT_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static PROTECT_RESUME:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
const SECONDS:f64=2.;
#[derive(Clone,Copy,Debug)]
struct Terms {rate:f32,window:f32,refresh:u8}
#[derive(Debug)]
struct Debt {amount:i32,paid:i32,elapsed:f64,terms:Terms}
#[derive(Default)]
struct Schedule {object:(usize,usize),hits:Vec<Debt>}
impl Schedule {
 fn sync(&mut self,object:(usize,usize),active:bool){
  if !active||self.object!=object{self.object=object;self.hits.clear();}
 }
 fn add(&mut self,amount:i32,terms:Terms){
  if amount>0{self.hits.push(Debt{amount,paid:0,elapsed:0.,terms});}
 }
 fn tick(&mut self,dt:f32)->Vec<(i32,Terms)>{
  let mut payments=Vec::new();
  if !dt.is_finite()||dt<=0.{return payments;}
  for hit in &mut self.hits{
   hit.elapsed=(hit.elapsed+f64::from(dt)).min(SECONDS);
   // Cumulative rounding conserves single HP hits and low frame-rate updates.
   let target=if hit.elapsed>=SECONDS{hit.amount}else{(f64::from(hit.amount)*hit.elapsed/SECONDS).floor()as i32};
   let due=target-hit.paid;hit.paid=target;
   if due>0{payments.push((due,hit.terms));}
  }
  self.hits.retain(|h|h.paid<h.amount);payments
 }
}
static SCHEDULE:Mutex<Schedule>=Mutex::new(Schedule{object:(0,0),hits:Vec::new()});
thread_local!{static SETTLING:Cell<bool>=const{Cell::new(false)};}
struct Settlement;
impl Drop for Settlement{fn drop(&mut self){SETTLING.with(|s|s.set(false));}}

// This runs inside an accepted native reset, after HP was clamped but before
// the original protected quota/time store. Do not create quota or extend time:
// later eligible payments use the unchanged native accumulation/refresh path.
unsafe extern "system" fn preserve_protection(stats:usize,before:i32)->u8{
 if !ACTIVE.load(Ordering::Acquire)||stats==0||before<=0||read_i32(stats+0x140)>=before{return 0;}
 let base=BASE.load(Ordering::Acquire);let world=read_ptr(base+crate::WORLD_RVA);if world==0{return 0;}
 let player=read_ptr(world+0x174e8);
 if player==0||native_actor::stats(player)!=stats||matches!(ptr::read((player+0x710)as*const u8),1|2){return 0;}
 (has_effect(player,OWNER,base)&&crate::has_state(player,449,base))as u8
}
#[unsafe(naked)]
pub unsafe extern "system" fn protect_detour(){
 core::arch::naked_asm!(
  "pushfq","sub rsp,0xc8",
  "mov [rsp+0x28],rax","mov [rsp+0x30],rcx","mov [rsp+0x38],rdx",
  "mov [rsp+0x40],r8","mov [rsp+0x48],r9","mov [rsp+0x50],r10","mov [rsp+0x58],r11",
  "movdqu [rsp+0x60],xmm0","movdqu [rsp+0x70],xmm1","movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3","movdqu [rsp+0xa0],xmm4","movdqu [rsp+0xb0],xmm5",
  "mov rcx,rbx","mov edx,ebp","call {callback}","mov [rsp+0x20],al",
  "movdqu xmm0,[rsp+0x60]","movdqu xmm1,[rsp+0x70]","movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]","movdqu xmm4,[rsp+0xa0]","movdqu xmm5,[rsp+0xb0]",
  "mov rax,[rsp+0x28]","mov rcx,[rsp+0x30]","mov rdx,[rsp+0x38]",
  "mov r8,[rsp+0x40]","mov r9,[rsp+0x48]","mov r10,[rsp+0x50]","mov r11,[rsp+0x58]",
  "cmp byte ptr [rsp+0x20],0","je 2f",
  "add rsp,0xc8","popfq","jmp qword ptr [rip+{resume}]",
  "2:","add rsp,0xc8","popfq","jmp qword ptr [rip+{original}]",
  callback=sym preserve_protection,resume=sym PROTECT_RESUME,original=sym PROTECT_ORIGINAL
 );
}

// Called after native authority + stateInfo449 checks, before any pool update.
// RBP is the pre-set HP, RBX the canonical stats object, XMM6 the regain ratio.
unsafe extern "system" fn capture(stats:usize,before:i32,rate:f32,window:f32,refresh:u8){
 if !ACTIVE.load(Ordering::Acquire)||SETTLING.with(|s|s.get())||!rate.is_finite()||rate<=0.||before<=0{return;}
 let base=BASE.load(Ordering::Acquire);let world=read_ptr(base+crate::WORLD_RVA);if world==0{return;}
 let player=read_ptr(world+0x174e8);
 if player==0||native_actor::stats(player)!=stats||!has_effect(player,OWNER,base)||matches!(ptr::read((player+0x710)as*const u8),1|2){return;}
 let after=read_i32(stats+0x140);let loss=before.saturating_sub(after);
 if loss<=0{return;}
 if let Ok(mut schedule)=SCHEDULE.lock(){
  schedule.sync((player,stats),true);
  schedule.add(loss,Terms{rate,window,refresh});
  // The setter already clamped this loss. Restore within its synchronous frame;
  // native code sees zero immediate loss and cannot grant the same budget twice.
  ptr::write_unaligned((stats+0x140)as*mut i32,before.min(read_i32(stats+0x144)));
 }
}
#[unsafe(naked)]
pub unsafe extern "system" fn detour(){
 core::arch::naked_asm!(
  "pushfq","sub rsp,0xc8",
  "mov [rsp+0x28],rax","mov [rsp+0x30],rcx","mov [rsp+0x38],rdx",
  "mov [rsp+0x40],r8","mov [rsp+0x48],r9","mov [rsp+0x50],r10","mov [rsp+0x58],r11",
  "movdqu [rsp+0x60],xmm0","movdqu [rsp+0x70],xmm1","movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3","movdqu [rsp+0xa0],xmm4","movdqu [rsp+0xb0],xmm5",
  "mov rcx,rbx","mov edx,ebp","movaps xmm2,xmm6","movss xmm3,[rsp+0x130]",
  "movzx eax,byte ptr [rsp+0x138]","mov [rsp+0x20],eax","call {callback}",
  "movdqu xmm0,[rsp+0x60]","movdqu xmm1,[rsp+0x70]","movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]","movdqu xmm4,[rsp+0xa0]","movdqu xmm5,[rsp+0xb0]",
  "mov rax,[rsp+0x28]","mov rcx,[rsp+0x30]","mov rdx,[rsp+0x38]",
  "mov r8,[rsp+0x40]","mov r9,[rsp+0x48]","mov r10,[rsp+0x50]","mov r11,[rsp+0x58]",
  "add rsp,0xc8","popfq","jmp qword ptr [rip+{original}]",
  callback=sym capture,original=sym ORIGINAL
 );
}
pub unsafe fn update(player:usize,base:usize,dt:f32){
 let stats=native_actor::stats(player);if stats==0{return;}
 let owned=has_effect(player,OWNER,base);let alive=!native_actor::incapacitated(player,stats);
 crate::maintain_status_effect(player,stats,FLASK,owned&&alive,base);
 let reset=has_effect(player,crate::reward_cycle_runtime::RESET,base);
 {let Ok(mut s)=SCHEDULE.lock()else{return;};s.sync((player,stats),owned&&alive&&!reset);}
 let module=read_ptr(read_ptr(player+0x1b8)+crate::DAMAGE_MODULE_SLOT);
 if module==0||read_ptr(module+8)!=player||read_ptr(module)!=base+crate::ominous_pact::PLAYER_DAMAGE_VTABLE_RVA{return;}
 let payments={let Ok(mut s)=SCHEDULE.lock()else{return;};s.tick(dt)};
 let setter:unsafe extern "system" fn(usize,i32,u8,f32,f32,u8)=std::mem::transmute(base+SET_RVA);
 let finish:unsafe extern "system" fn(usize,i32,usize,usize,i32,u8)=std::mem::transmute(base+FINISH_RVA);
 SETTLING.with(|s|s.set(true));let _scope=Settlement;
 for(amount,terms)in payments{
  if native_actor::stats(player)!=stats||native_actor::incapacitated(player,stats){break;}
  let before=read_i32(stats+0x140);
  setter(stats,before.saturating_sub(amount),0,terms.rate,terms.window,terms.refresh);
  let lost=before.saturating_sub(read_i32(stats+0x140)).max(0);
  if lost>0{
   // Same native completion used by 431090 HP costs: damage bookkeeping and
   // native notifications stay in the damage module. Do not synthesize a hit.
   let flags=(module+0x78)as*mut u8;ptr::write(flags,ptr::read(flags)|8);
   finish(module,lost,0,0,before,0);
  }
 }
}
#[cfg(test)]mod tests{
 use super::*;
 fn terms()->Terms{Terms{rate:1.,window:1.,refresh:1}}
 fn total(p:Vec<(i32,Terms)>)->i32{p.iter().map(|v|v.0).sum()}
 #[test]fn each_hit_finishes_two_seconds_after_its_own_start(){
  let mut s=Schedule::default();s.sync((1,2),true);s.add(200,terms());
  assert_eq!(total(s.tick(0.5)),50);s.add(400,terms());
  assert_eq!(total(s.tick(0.5)),150);assert_eq!(total(s.tick(1.)),300);
  assert_eq!(s.hits.len(),1);assert_eq!(total(s.tick(0.5)),100);assert!(s.hits.is_empty());
 }
 #[test]fn rounding_and_frame_rate_conserve_all_damage(){
  for amount in [1,2,3,119,1000,i32::MAX]{for dt in [1./144.,1./60.,0.37,2.5]{
   let mut s=Schedule::default();s.add(amount,terms());let mut paid=0i64;
   for _ in 0..400{paid+=i64::from(total(s.tick(dt)));}assert_eq!(paid,i64::from(amount));
  }}
 }
 #[test]fn death_loss_of_ownership_reset_and_object_replacement_discard_old_debts(){
  for(object,active)in [((1,2),false),((2,3),true)]{let mut s=Schedule::default();s.sync((1,2),true);s.add(500,terms());s.sync(object,active);assert_eq!(total(s.tick(3.)),0);}
 }
 #[test]fn invalid_time_does_not_consume_or_refresh_any_hit(){
  let mut s=Schedule::default();s.add(50,terms());for dt in [0.,-1.,f32::NAN,f32::INFINITY]{assert_eq!(total(s.tick(dt)),0);}assert_eq!(total(s.tick(2.)),50);
 }
 #[test]fn simultaneous_hits_keep_their_native_regain_terms(){
  let mut s=Schedule::default();s.add(100,terms());s.add(200,Terms{rate:0.5,window:0.25,refresh:0});
  let p=s.tick(1.);assert_eq!(p.len(),2);assert_eq!(p[0].0,50);assert_eq!(p[1].0,100);assert_eq!(p[1].1.rate,0.5);assert_eq!(p[1].1.window,0.25);assert_eq!(p[1].1.refresh,0);
 }
}

#[cfg(test)]
#[path="death_dance_native_tests.rs"]
mod native_tests;
