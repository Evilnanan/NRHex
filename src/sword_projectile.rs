//! Sword limits independent spell flight from a fixed launch point. The native
//! update retires an over-range instance directly to state 4, then its manager
//! removes collision/VFX references and recycles it. Reviewed transition
//! projectiles retain native impact children. No damage-time filter is involved.
use crate::{ptr,read_i32,read_ptr,sword,ACTIVE,BASE};
use std::{cell::Cell,collections::BTreeMap,sync::{Mutex,atomic::{AtomicPtr,Ordering}}};
#[path="sword_projectile_stages.rs"]mod stages;
pub const RANGE:f32=6.;
const CRYSTAL_BURST_RANGE:f32=3.;
pub const BIND_RVA:usize=0x34e750;
pub const BIND_PATTERN:&str="48 8B C4 48 89 58 08 48 89 68 10 48 89 70 18 57 48 83 EC 50 48 8B DA 48 C7 40 E8 00 00 00 00 8B 52 04";
pub const CHILD_RVA:usize=0x350fb0;
pub const CHILD_PATTERN:&str="48 8B C4 48 89 58 10 55 56 57 41 54 41 55 41 56 41 57 48 8D A8 D8 FC FF FF 48 81 EC F0 03 00 00";
pub const RESET_RVA:usize=0x34a6f0;
pub const RESET_PATTERN:&str="40 53 48 81 EC B0 02 00 00 48 8B 05 68 B8 79 03 48 33 C4 48 89 84 24 A0 02 00 00 48 8B D9";
pub const STEP_RVA:usize=0x34f593;
pub const STEP_PATTERN:&str="8B 97 90 00 00 00 48 8B 8F 88 00 00 00 44 8B 04 91 48 8D 35 D5 AC 63 03";
pub const MODE_RVA:usize=0x355270;
pub const MODE_PATTERN:&str="4C 8B DC 49 89 5B 20 56 48 81 EC 80 00 00 00 48 8B 05 E2 0C 79 03 48 33 C4 48 89 44 24 50";
pub const PARAM_RVA:usize=0x354aa0;
pub const PARAM_PATTERN:&str="44 8B 81 90 00 00 00 48 8B 91 88 00 00 00 46 8B 0C 82 41 83 E9 01 74 24 41 83 E9 01 74 16 41 83 F9 01 74 08 48 8D 05 ? ? ? ? C3 48 8D 81 F8 05 00 00 C3";
pub const STOP_RVA:usize=0x34f50a;
pub static BIND_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static CHILD_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static RESET_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static STEP_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static STOP:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
type Point=[f32;4];
#[derive(Clone,Copy,Debug,Default,PartialEq)]
pub struct Flight{origin:Option<Point>,horizontal:bool}
#[derive(Clone,Copy)]struct Instance{handle:u64,bullet:i32,flight:Flight,previous:Point}
#[derive(Default)]struct State{instances:BTreeMap<usize,Instance>,groups:BTreeMap<usize,(i32,Flight)>}
static STATE:Mutex<State>=Mutex::new(State{instances:BTreeMap::new(),groups:BTreeMap::new()});
// Some(None) explicitly marks an untracked parent. Never infer a fresh spell
// root from a descendant merely because it shares a root Bullet ID.
thread_local!{
 static CHILD:Cell<Option<Option<Flight>>>=const{Cell::new(None)};
 static FACTORY:Cell<Option<Flight>>=const{Cell::new(None)};
 static INITIALIZE:Cell<Option<Flight>>=const{Cell::new(None)};
}
pub struct FactoryGuard(Option<Flight>);
impl Drop for FactoryGuard{fn drop(&mut self){FACTORY.with(|s|s.set(self.0));}}
pub struct InitializeGuard(Option<Flight>);
impl Drop for InitializeGuard{fn drop(&mut self){INITIALIZE.with(|s|s.set(self.0));}}
struct ChildGuard(Option<Option<Flight>>);
impl Drop for ChildGuard{fn drop(&mut self){CHILD.with(|s|s.set(self.0));}}
fn finite(p:Point)->bool{p[..3].iter().all(|v|v.is_finite())}
fn displacement(flight:Flight,p:Point)->Option<f32>{
 let mut origin=flight.origin?;
 if flight.horizontal{origin[1]=p[1];}
 sword::distance(origin,p)
}
fn outside(flight:Flight,p:Point)->bool{
 displacement(flight,p).is_some_and(|distance|distance>RANGE)
}
fn root(magic:i32,bullet:i32)->bool{
 crate::sword_map::ROOTS.binary_search_by_key(&bullet,|r|r.0).ok().is_some_and(|i|{
  let magics=crate::sword_map::ROOTS[i].1;
  if magic==-1{magics.iter().any(|&id|id!=4210)}else{magic!=4210&&magics.contains(&magic)}
 })
}
fn crystal_carrier(bullet:i32)->bool{matches!(bullet,10450000|10450001|10450002|10450050|10450051|10450052)}
fn transition_distance(bullet:i32)->Option<f32>{
 if crystal_carrier(bullet){Some(CRYSTAL_BURST_RANGE)}else if matches!(bullet,10670000|10670100|10693000){Some(RANGE)}else{None}
}
fn ground_projectile(bullet:i32)->bool{
 matches!(bullet,10691000..=10691005|10691010..=10691016|10691020..=10691025|10691050..=10691055|10691060..=10691066|10691070..=10691075|10692000|10692002..=10692006|10692009..=10692016|10692100..=10692107|10692109..=10692112|10693000..=10693008)
}
fn native_return(parent:i32,child:i32)->bool{matches!((parent,child),(10670000,10670001)|(10670100,10670101))}
fn breath_stream(bullet:i32)->bool{matches!(bullet,10700000|10702000|10703000|10704000)}
fn synchronized_flight(bullet:i32)->bool{
 matches!(bullet,10406000|10406001|10603000|10603050|10700000|10700002|10700100|10700101|10702000|10702002|10702100|10702101|10703000|10703002|10703100|10703101|10704000|10704002|10704100|10704101|10705000)
}
unsafe fn instance(ins:usize)->Option<Instance>{
 if ins==0{return None;}
 let handle=ptr::read_unaligned((ins+8)as*const u64);
 STATE.lock().ok()?.instances.get(&ins).copied().filter(|v|v.handle==handle)
}
unsafe fn param(ins:usize,base:usize)->usize{
 let get:unsafe extern "system" fn(usize)->usize=std::mem::transmute(base+PARAM_RVA);
 read_ptr(get(ins))
}
unsafe fn free_flight(row:usize,bullet:i32)->bool{
 if row==0{return false;}
 let follow=ptr::read((row+0x9a)as*const u8)&7;
 (follow==0||(follow==4&&breath_stream(bullet)))&&(ptr::read((row+0x9c)as*const u8)&0x20==0||synchronized_flight(bullet))
}
fn velocity(v:Point)->bool{finite(v)&&v[..3].iter().any(|v|v.abs()>0.0001)}
unsafe fn moving(ins:usize)->bool{
 [0x30,0x40].into_iter().any(|offset|velocity(ptr::read_unaligned((ins+offset)as*const Point)))
}
unsafe fn point(ins:usize)->Point{ptr::read_unaligned((ins+0x10)as*const Point)}
unsafe fn launch(transform:usize)->Point{
 let m=ptr::read_unaligned(transform as*const[f32;12]);[m[3],m[7],m[11],1.]
}
pub unsafe fn factory_scope(payload:usize)->FactoryGuard{
 let value=if !ACTIVE.load(Ordering::Acquire)||payload==0{None}
 else if let Some(inherited)=CHILD.with(Cell::get){inherited}
 else if read_i32(payload)<0&&root(read_i32(payload+0x24),read_i32(payload+4)){
  let base=BASE.load(Ordering::Acquire);
  let resolve:unsafe extern "system" fn(u64)->usize=std::mem::transmute(base+crate::CHR_HANDLE_RVA);
  let actor=resolve(ptr::read_unaligned((payload+0x10)as*const u64));
  // Evaluate source ownership on each peer, rather than only for its local
  // player. The same replicated source/trajectory yields the same retirement.
  (actor!=0&&crate::has_effect(actor,sword::OWNER,base)).then_some(Flight::default())
 }else{None};
 FactoryGuard(FACTORY.with(|s|s.replace(value)))
}
pub unsafe fn factory_done(group:usize){
 if group==0{return;}
 if let Ok(mut state)=STATE.lock(){
  state.groups.remove(&group);
  if let Some(flight)=FACTORY.with(Cell::get){state.groups.insert(group,(read_i32(group),flight));}
 }
}
pub unsafe fn initialize_scope(group:usize,transform:usize)->Option<InitializeGuard>{
 let value=if ACTIVE.load(Ordering::Acquire)&&group!=0{
  STATE.lock().ok().and_then(|state|state.groups.get(&group).copied())
   .filter(|(id,_)|*id==read_i32(group)).map(|(_,flight)|flight).or_else(||FACTORY.with(Cell::get))
 }else{None};
 if transform!=0&&value.is_some_and(|flight|outside(flight,launch(transform))){return None;}
 Some(InitializeGuard(INITIALIZE.with(|s|s.replace(value))))
}
pub fn finish_group(group:usize){if let Ok(mut state)=STATE.lock(){state.groups.remove(&group);}}
pub unsafe extern "system" fn bind(ins:usize,payload:usize,target:usize,transform:usize,mode:i32,index:u32){
 let original:unsafe extern "system" fn(usize,usize,usize,usize,i32,u32)=std::mem::transmute(BIND_ORIGINAL.load(Ordering::Acquire));
 original(ins,payload,target,transform,mode,index);
 let Some(mut flight)=INITIALIZE.with(Cell::get)else{return;};
 let p=point(ins);if !finite(p){return;}
 let row=param(ins,BASE.load(Ordering::Acquire));
 let bullet=read_i32(payload+4);
 flight.horizontal=ground_projectile(bullet);
 if flight.origin.is_none()&&free_flight(row,bullet)&&velocity(ptr::read_unaligned((ins+0x30)as*const Point)){flight.origin=Some(p);}
 if crystal_carrier(bullet)&&flight.origin.is_some(){
  let speed=sword::distance([0.;4],ptr::read_unaligned((ins+0x30)as*const Point)).unwrap_or(0.);
  if speed>0.0001{
   let remaining=ptr::read_unaligned((ins+0x2d4)as*const f32);
   if remaining.is_finite()&&remaining>=0.{ptr::write_unaligned((ins+0x2d4)as*mut f32,remaining.max(CRYSTAL_BURST_RANGE/speed+0.1));}
  }
 }
 if let Ok(mut state)=STATE.lock(){state.instances.insert(ins,Instance{handle:ptr::read_unaligned((ins+8)as*const u64),bullet,flight,previous:p});}
}
pub unsafe extern "system" fn child(ins:usize,position:usize,orientation:usize,bullet:i32,mode:u8)->u8{
 let original:unsafe extern "system" fn(usize,usize,usize,i32,u8)->u8=std::mem::transmute(CHILD_ORIGINAL.load(Ordering::Acquire));
 let flight=if ACTIVE.load(Ordering::Acquire){instance(ins).and_then(|value|{
  if native_return(value.bullet,bullet){None}else if stages::continues(value.bullet,bullet){Some(value.flight)}else{Some(Flight::default())}
 })}else{None};
 if position!=0&&flight.is_some_and(|value|outside(value,ptr::read_unaligned(position as*const Point))){return 0;}
 let _guard=ChildGuard(CHILD.with(|s|s.replace(Some(flight))));
 original(ins,position,orientation,bullet,mode)
}
pub unsafe extern "system" fn reset(ins:usize){
 if let Ok(mut state)=STATE.lock(){state.instances.remove(&ins);}
 let original:unsafe extern "system" fn(usize)=std::mem::transmute(RESET_ORIGINAL.load(Ordering::Acquire));original(ins);
}
unsafe extern "system" fn after_move(ins:usize)->u8{
 if !ACTIVE.load(Ordering::Acquire){return 0;}
 let Some(mut value)=instance(ins)else{return 0;};
 let p=point(ins);if !finite(p){return 0;}
 let base=BASE.load(Ordering::Acquire);let row=param(ins,base);
 // Zero-speed generators/attached weapons retain native range. If they
 // release into flight later (including gravity), anchor at that release.
 // A stationary controller teleporting to its target is not free flight.
 if value.flight.origin.is_none()&&free_flight(row,value.bullet)&&moving(ins)&&sword::distance(value.previous,p).is_some_and(|d|d>0.0001){value.flight.origin=Some(value.previous);}
 value.previous=p;
 if let Ok(mut state)=STATE.lock(){state.instances.insert(ins,value);}
 if let Some(limit)=transition_distance(value.bullet){
  if let Some(origin)=value.flight.origin{
   if let Some(distance)=displacement(value.flight,p).filter(|distance|*distance>=limit){
    let ratio=limit/distance;
    let position=[origin[0]+(p[0]-origin[0])*ratio,if value.flight.horizontal{p[1]}else{origin[1]+(p[1]-origin[1])*ratio},origin[2]+(p[2]-origin[2])*ratio,p[3]];
    ptr::write_unaligned((ins+0x10)as*mut Point,position);
    let burst:unsafe extern "system" fn(usize,i32)=std::mem::transmute(base+MODE_RVA);burst(ins,3);
    return u8::from(!matches!(value.bullet,10670000|10670100));
   }
  }
  return 0;
 }
 if !outside(value.flight,p){return 0;}
 // Use the native forced-erasure VFX policy and skip impact state 3. The
 // caller returns false so BulletMan performs normal complete cleanup.
 if row!=0{ptr::write((ins+0x583)as*mut u8,ptr::read((row+0xf8)as*const u8));}
 let retire:unsafe extern "system" fn(usize,i32)=std::mem::transmute(base+MODE_RVA);retire(ins,4);
 1
}
#[unsafe(naked)]pub unsafe extern "system" fn step(){
 core::arch::naked_asm!(
  "pushfq","sub rsp,0xc8",
  "mov [rsp+0x20],rax","mov [rsp+0x28],rcx","mov [rsp+0x30],rdx","mov [rsp+0x38],r8",
  "mov [rsp+0x40],r9","mov [rsp+0x48],r10","mov [rsp+0x50],r11",
  "movdqu [rsp+0x60],xmm0","movdqu [rsp+0x70],xmm1","movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3","movdqu [rsp+0xa0],xmm4","movdqu [rsp+0xb0],xmm5",
  "mov rcx,rdi","call {callback}","mov [rsp+0x58],al",
  "mov rax,[rsp+0x20]","mov rcx,[rsp+0x28]","mov rdx,[rsp+0x30]","mov r8,[rsp+0x38]",
  "mov r9,[rsp+0x40]","mov r10,[rsp+0x48]","mov r11,[rsp+0x50]",
  "movdqu xmm0,[rsp+0x60]","movdqu xmm1,[rsp+0x70]","movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]","movdqu xmm4,[rsp+0xa0]","movdqu xmm5,[rsp+0xb0]",
  "cmp byte ptr [rsp+0x58],0","jne 2f",
  "add rsp,0xc8","popfq","jmp qword ptr [rip+{original}]",
  "2:","add rsp,0xc8","popfq","jmp qword ptr [rip+{stop}]",
  callback=sym after_move,original=sym STEP_ORIGINAL,stop=sym STOP
 );
}
#[cfg(test)]#[path="sword_projectile_native_tests.rs"]mod native_tests;
