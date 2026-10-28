//! Raider Retaliate: two independent totals, committed before immortal HP clamp.
use crate::{has_effect,native_actor,read_i32,read_ptr,ptr,ACTIVE,BASE};
use std::{cell::RefCell,sync::{Mutex,atomic::{AtomicPtr,Ordering}}};
pub const OWNER:i32=95004800;
pub const QUERY:i32=990048;
pub const DISPATCH_RVA:usize=0x43d260;
pub const DISPATCH_PATTERN:&str="48 8B C4 48 89 58 20 55 56 57 41 54 41 55 41 56 41 57 48 8D 68 A9 48 81 EC 00 01 00 00 0F 29 70";
pub const COMMIT_RVA:usize=0x43bfb1;
pub const COMMIT_PATTERN:&str="41 8B 96 80 02 00 00 85 D2 0F 9F C0 45 0F B6 86 61 01 00 00 41 80 E0 01 F7 DA 88 44 24 30 F3 0F";
pub const OBJECT_RVA:usize=0x43a403;
pub const OBJECT_PATTERN:&str="0F 28 D8 0F C6 DB 00 0F 10 4D 88 0F 59 CB 0F 11 4D 88 F3 0F 10 4D 98 F3 0F 59 CB F3 0F 11 4D 98 F3 41 0F 10 86 F0 00 00 00 41 0F 2F C0 72 65 49 8B CF E8 E6";
pub const SCALE_RVA:usize=0x43a476;
pub const SCALE_PATTERN:&str="0F 10 45 88 0F 59 C2 0F 11 45 88 F3 0F 10 45 98 F3 0F 59 C2 F3 0F 11 45 98 41 80 8E C2 02 00 00 01 4D 85 E4 74 23 49 8B D6 48 8D 8D 80 03 00 00 E8 85 4D 0D";
pub static DISPATCH_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static COMMIT_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static OBJECT_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static SCALE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
#[derive(Clone,Copy,Default,PartialEq,Eq)]struct Identity{actor:usize,stats:usize,handle:u64}
#[derive(Default)]struct Cast{owner:Identity,serial:u64,phase:u8,cap:f64,before:f64,after:f64,frozen:Option<f32>}
impl Cast{
 fn clear(&mut self){let serial=self.serial.wrapping_add(1);*self=Self{serial,..Self::default()};}
 fn begin(&mut self,owner:Identity,hp:i32){self.clear();self.owner=owner;self.phase=1;self.cap=3.5*hp.max(0)as f64;}
 fn add(&mut self,before:f32,after:i32){
  if self.phase==0||self.frozen.is_some(){return;}
  if before.is_finite()&&before>=0.&&after>=0{self.before=(self.before+before as f64).min(self.cap);self.after=(self.after+after as f64).min(self.cap);}
 }
 fn freeze(&mut self)->f32{if self.phase!=2{return 0.;}*self.frozen.get_or_insert((self.before.max(self.after).min(self.cap)/2500.)as f32)}
}
static STATE:Mutex<Cast>=Mutex::new(Cast{owner:Identity{actor:0,stats:0,handle:0},serial:0,phase:0,cap:0.,before:0.,after:0.,frozen:None});
unsafe fn identity(actor:usize,base:usize)->Option<Identity>{
 if actor==0||!crate::is_local(actor,base)||crate::local_hero(base)!=Some(5)||!has_effect(actor,OWNER,base)||has_effect(actor,crate::reward_cycle_runtime::RESET,base){return None;}
 let stats=native_actor::stats(actor);
 if stats==0||native_actor::incapacitated(actor,stats){return None;}
 Some(Identity{actor,stats,handle:ptr::read_unaligned((actor+0x8)as*const u64)})
}
pub unsafe fn update(actor:usize,base:usize){
 let current=identity(actor,base);
 if let Ok(mut s)=STATE.lock(){if s.phase!=0&&current!=Some(s.owner){s.clear();}}
}
// Prefix only on ExecEventAllBody: additive/no-reset hurt overlays cannot cancel
// the cast. The actual empowered branch is latched after native 704011 consumption.
pub unsafe fn event(actor:usize,mode:i32,base:usize){
 if !crate::is_local(actor,base){return;}
 let owner=identity(actor,base);
 if let Ok(mut s)=STATE.lock(){
  let Some(owner)=owner else{s.clear();return;};
  match mode{
   1=>s.begin(owner,read_i32(owner.stats+0x144)),
   3 if s.owner==owner&&s.phase==1=>s.phase=2,
   _=>s.clear(),
  }
 }
}
fn empowered(attack:i32,table:i32)->bool{table==1&&matches!(attack,600000210|600000231)}
pub unsafe fn constructed(actor:usize,hit:usize,base:usize){
 if !empowered(read_i32(hit+0x40),read_i32(hit+0x44)){return;}
 let owner=identity(actor,base);
 if let Ok(mut s)=STATE.lock(){if owner==Some(s.owner){s.freeze();}}
}
pub unsafe fn bonus(actor:usize,hit:usize,base:usize)->f32{
 if !empowered(read_i32(hit+0x40),read_i32(hit+0x44)){return 0.;}
 let owner=identity(actor,base);
 STATE.lock().map(|mut s|if owner==Some(s.owner){s.freeze()}else{0.}).unwrap_or(0.)
}
#[derive(Default)]struct Impact{module:usize,hit:usize,serial:Option<u64>,before:Option<f32>}
#[derive(Default)]struct Calculation{eligible:bool,delta:[f32;5],sampled:bool}
thread_local!{
 static IMPACTS:RefCell<Vec<Impact>>=const{RefCell::new(Vec::new())};
 static CALCULATIONS:RefCell<Vec<Calculation>>=const{RefCell::new(Vec::new())};
}
type Dispatch=unsafe extern "system" fn(usize,usize,usize,u8,u8)->u64;
pub unsafe extern "system" fn dispatch(module:usize,source:usize,hit:usize,mode:u8,extra:u8)->u64{
 let mut serial=None;
 if ACTIVE.load(Ordering::Acquire)&&module!=0&&hit!=0{
  let owner=identity(read_ptr(module+8),BASE.load(Ordering::Acquire));
  serial=STATE.lock().ok().and_then(|s|(s.phase!=0&&s.frozen.is_none()&&owner==Some(s.owner)).then_some(s.serial));
 }
 IMPACTS.with(|s|s.borrow_mut().push(Impact{module,hit,serial,before:None}));
 let original:Dispatch=std::mem::transmute(DISPATCH_ORIGINAL.load(Ordering::Acquire));
 let result=original(module,source,hit,mode,extra);
 IMPACTS.with(|s|{s.borrow_mut().pop();});result
}
pub unsafe fn enter(module:usize,hit:usize){
 let eligible=IMPACTS.with(|s|s.borrow().last().is_some_and(|i|i.serial.is_some()&&i.module==module&&i.hit==hit))
  &&hit!=0&&ptr::read((hit+0x2c1)as*const u8)&8==0;
 CALCULATIONS.with(|s|s.borrow_mut().push(Calculation{eligible,..Default::default()}));
}
pub fn leave(){CALCULATIONS.with(|s|{s.borrow_mut().pop();});}
// Capture the difference before the object's defensive multiplier, then scale
// that difference alongside the optional HP-based input multiplier. Adding it
// to the combiner input preserves subsequent additive attacker contributions,
// including when object defense is zero. No division by a defensive rate.
unsafe extern "system" fn object_delta(attack:*const f32,rate:f32){
 CALCULATIONS.with(|s|{if let Some(c)=s.borrow_mut().last_mut().filter(|c|c.eligible){c.delta=std::array::from_fn(|i|*attack.add(i)*(1.-rate));}});
}
unsafe extern "system" fn scale_delta(rate:f32){CALCULATIONS.with(|s|{if let Some(c)=s.borrow_mut().last_mut().filter(|c|c.eligible){for d in &mut c.delta{*d*=rate;}}});}
pub unsafe fn raw_attack(attack:*const f32,holy:f32)->Option<[f32;5]>{
 if attack.is_null(){return None;}
 CALCULATIONS.with(|s|{let mut stack=s.borrow_mut();let c=stack.last_mut()?;if !c.eligible||c.sampled{return None;}c.sampled=true;Some(std::array::from_fn(|i|*attack.add(i)+c.delta[i]*if i==4{holy}else{1.}))})
}
pub fn candidate(value:f32){
 if value.is_finite()&&value>=0.{IMPACTS.with(|s|{if let Some(i)=s.borrow_mut().last_mut(){i.before=Some(value);}});}
}
unsafe extern "system" fn commit(module:usize,hit:usize){
 if !ACTIVE.load(Ordering::Acquire)||module==0||hit==0{return;}
 let base=BASE.load(Ordering::Acquire);let actor=read_ptr(module+8);
 let owner=identity(actor,base);
 if owner.is_none()||!has_effect(actor,704000,base)||ptr::read((hit+0x2c3)as*const u8)&2!=0{return;}
 let pair=IMPACTS.with(|stack|{let mut stack=stack.borrow_mut();let i=stack.last_mut()?;
  if i.module!=module||i.hit!=hit{return None;}Some((i.serial?,i.before.take()?))});
 if let Some((serial,before))=pair{if let Ok(mut s)=STATE.lock(){if s.serial==serial&&owner==Some(s.owner){s.add(before,read_i32(hit+0x280));}}}
}
#[unsafe(naked)]
pub unsafe extern "system" fn commit_detour(){
 core::arch::naked_asm!(
  "pushfq", "sub rsp,0xc8",
  "mov [rsp+0x20],rax", "mov [rsp+0x28],rcx", "mov [rsp+0x30],rdx", "mov [rsp+0x38],r8",
  "mov [rsp+0x40],r9", "mov [rsp+0x48],r10", "mov [rsp+0x50],r11",
  "movdqu [rsp+0x60],xmm0", "movdqu [rsp+0x70],xmm1", "movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3", "movdqu [rsp+0xa0],xmm4", "movdqu [rsp+0xb0],xmm5",
  "mov rcx,r12", "mov rdx,r14", "call {callback}",
  "mov rax,[rsp+0x20]", "mov rcx,[rsp+0x28]", "mov rdx,[rsp+0x30]", "mov r8,[rsp+0x38]",
  "mov r9,[rsp+0x40]", "mov r10,[rsp+0x48]", "mov r11,[rsp+0x50]",
  "movdqu xmm0,[rsp+0x60]", "movdqu xmm1,[rsp+0x70]", "movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]", "movdqu xmm4,[rsp+0xa0]", "movdqu xmm5,[rsp+0xb0]",
  "add rsp,0xc8", "popfq", "jmp qword ptr [rip+{original}]",
  callback=sym commit,original=sym COMMIT_ORIGINAL
 );
}
#[unsafe(naked)]
pub unsafe extern "system" fn object_detour(){
 core::arch::naked_asm!(
  "pushfq", "sub rsp,0xc8",
  "mov [rsp+0x20],rax", "mov [rsp+0x28],rcx", "mov [rsp+0x30],rdx", "mov [rsp+0x38],r8",
  "mov [rsp+0x40],r9", "mov [rsp+0x48],r10", "mov [rsp+0x50],r11",
  "movdqu [rsp+0x60],xmm0", "movdqu [rsp+0x70],xmm1", "movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3", "movdqu [rsp+0xa0],xmm4", "movdqu [rsp+0xb0],xmm5",
  "lea rcx,[rbp-0x78]", "movaps xmm1,xmm0", "call {callback}",
  "mov rax,[rsp+0x20]", "mov rcx,[rsp+0x28]", "mov rdx,[rsp+0x30]", "mov r8,[rsp+0x38]",
  "mov r9,[rsp+0x40]", "mov r10,[rsp+0x48]", "mov r11,[rsp+0x50]",
  "movdqu xmm0,[rsp+0x60]", "movdqu xmm1,[rsp+0x70]", "movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]", "movdqu xmm4,[rsp+0xa0]", "movdqu xmm5,[rsp+0xb0]",
  "add rsp,0xc8", "popfq", "jmp qword ptr [rip+{original}]",
  callback=sym object_delta,original=sym OBJECT_ORIGINAL
 );
}
#[unsafe(naked)]
pub unsafe extern "system" fn scale_detour(){
 core::arch::naked_asm!(
  "pushfq", "sub rsp,0xc8",
  "mov [rsp+0x20],rax", "mov [rsp+0x28],rcx", "mov [rsp+0x30],rdx", "mov [rsp+0x38],r8",
  "mov [rsp+0x40],r9", "mov [rsp+0x48],r10", "mov [rsp+0x50],r11",
  "movdqu [rsp+0x60],xmm0", "movdqu [rsp+0x70],xmm1", "movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3", "movdqu [rsp+0xa0],xmm4", "movdqu [rsp+0xb0],xmm5",
  "movaps xmm0,xmm2", "call {callback}",
  "mov rax,[rsp+0x20]", "mov rcx,[rsp+0x28]", "mov rdx,[rsp+0x30]", "mov r8,[rsp+0x38]",
  "mov r9,[rsp+0x40]", "mov r10,[rsp+0x48]", "mov r11,[rsp+0x50]",
  "movdqu xmm0,[rsp+0x60]", "movdqu xmm1,[rsp+0x70]", "movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]", "movdqu xmm4,[rsp+0xa0]", "movdqu xmm5,[rsp+0xb0]",
  "add rsp,0xc8", "popfq", "jmp qword ptr [rip+{original}]",
  callback=sym scale_delta,original=sym SCALE_ORIGINAL
 );
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn independent_sums_cap_and_freeze_for_all_recipients(){
  let mut c=Cast::default();c.begin(Identity::default(),1500);
  c.add(600.,150);c.add(100.,250); // max(sum)=700, not sum(max)=850.
  assert_eq!((c.before,c.after),(700.,400.));assert_eq!(c.freeze(),0.);
  c.phase=2;let bonus=c.freeze();assert!((bonus-700./2500.).abs()<1e-6);
  c.add(5000.,5000);assert_eq!(c.freeze(),bonus);
  let old=c.serial;c.clear();assert_ne!(old,c.serial);assert_eq!(c.freeze(),0.);
  c.begin(Identity::default(),1500);c.phase=2;c.add(1500.,6000);
  assert_eq!((c.before,c.after),(1500.,5250.));assert_eq!(c.freeze(),2.1);
  assert_eq!(crate::basics::multiplier(Some(5),600000210,1)+c.freeze(),4.6);
 }
 #[test]fn zero_post_mitigation_still_counts_pre_damage_and_invalid_values_do_not(){
  let mut c=Cast::default();c.begin(Identity::default(),1000);c.phase=2;
  c.add(150.,0);c.add(f32::NAN,999);c.add(f32::INFINITY,999);c.add(-1.,999);c.add(2.,-1);
  assert_eq!((c.before,c.after),(150.,0.));assert_eq!(c.freeze(),0.06);
  c.begin(Identity::default(),1);c.phase=2;c.add(f32::MAX,i32::MAX);assert_eq!(c.freeze(),3.5/2500.);
 }
 #[test]fn only_empowered_damage_components_match(){
  for id in [600000210,600000231]{assert!(empowered(id,1));assert!(!empowered(id,0));}
  for id in [600000200,600000220,600000230,600000240,600000250,0]{assert!(!empowered(id,1));}
 }
 #[test]fn shadow_preserves_additive_damage_with_zero_object_defense_and_nested_calculations(){unsafe{
  let raw=[100.,200.,300.,400.,500.];
  for rate in [0.,0.25,1.,2.]{
   CALCULATIONS.with(|s|s.borrow_mut().push(Calculation{eligible:true,..Default::default()}));
   object_delta(raw.as_ptr(),rate);scale_delta(2.);
   let mut actual=raw.map(|v|v*rate*2.);actual[0]+=33.;
   // A nested unrelated native evaluation cannot consume the outer candidate.
   enter(0,0);assert!(raw_attack(actual.as_ptr(),1.).is_none());leave();
   let expected=[233.,400.,600.,800.,1000.];assert_eq!(raw_attack(actual.as_ptr(),1.),Some(expected));
   assert!(raw_attack(actual.as_ptr(),1.).is_none());leave();
  }
 }}
 #[test]fn ordinary_pool_reuses_raider_exclusivity(){
  use crate::reward_choices::*;
  let index=crate::hex_pool::OWNER_IDS.iter().position(|&id|id==OWNER).unwrap();
  for hero in std::iter::once(None).chain((0..=255).map(Some)){
   assert_eq!(default_filters(HexSet::new(),hero).contains(&index),hero!=Some(5));
  }
  assert_eq!(crate::hex_pool::HISTORY_BITS[index],45);
 }
 #[test]fn dispatch_preserves_fifth_argument_return_and_nested_scope(){unsafe{
  unsafe extern "system" fn fixture(module:usize,source:usize,hit:usize,mode:u8,extra:u8)->u64{
   assert_eq!((module,hit),(0,0));
   IMPACTS.with(|s|assert_eq!(s.borrow().len(),if source==1{1}else{2}));
   if source==1{assert_eq!((mode,extra),(3,7));assert_eq!(dispatch(0,2,0,11,19),0x9876543210);}
   else{assert_eq!((mode,extra),(11,19));}
   0x9876543210
  }
  let old=DISPATCH_ORIGINAL.swap(fixture as*mut u8,Ordering::AcqRel);
  assert_eq!(dispatch(0,1,0,3,7),0x9876543210);IMPACTS.with(|s|assert!(s.borrow().is_empty()));
  DISPATCH_ORIGINAL.store(old,Ordering::Release);
 }}
}

#[cfg(test)]#[path="deliberate_punch_native_tests.rs"]mod native_tests;
