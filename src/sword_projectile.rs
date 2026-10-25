//! Sword limits independent spell flight from a fixed launch point. The native
//! update retires an over-range instance directly to state 4, then its manager
//! removes collision/VFX references and recycles it. Reviewed transition
//! projectiles retain native impact children. No damage-time filter is involved.
use crate::{ptr,read_i32,read_ptr,sword,ACTIVE,BASE};
use std::{cell::Cell,collections::BTreeMap,sync::{Mutex,atomic::{AtomicPtr,Ordering}}};
#[path="sword_projectile_stages.rs"]mod stages;
pub const RANGE:f32=6.;
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
pub const MOTION_RVA:usize=0x34ff00;
pub const MOTION_PATTERN:&str="48 8B C4 48 89 58 10 57 48 81 EC B0 00 00 00 0F 29 70 E8 48 8B FA 0F 29 78 D8 48 8B D9 44 0F 29 40 C8 45 0F 57 C0";
pub static BIND_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static CHILD_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static RESET_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static STEP_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static STOP:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static MOTION_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
type Point=[f32;4];
#[derive(Clone,Copy,Debug,Default,PartialEq)]
pub struct Flight{origin:Option<Point>,horizontal:bool}
#[derive(Clone,Copy,Debug)]struct Arc{launch:Point,elapsed:f32,gravity:f32}
#[derive(Clone,Copy)]struct Instance{handle:u64,bullet:i32,flight:Flight,previous:Point,arc:Option<Arc>,ending:bool}
#[derive(Default)]struct State{instances:BTreeMap<usize,Instance>,groups:BTreeMap<usize,(i32,Flight)>}
static STATE:Mutex<State>=Mutex::new(State{instances:BTreeMap::new(),groups:BTreeMap::new()});
// Some(None) explicitly marks an untracked parent. Never infer a fresh spell
// root from a descendant merely because it shares a root Bullet ID.
thread_local!{
 static CHILD:Cell<Option<Option<Flight>>>=const{Cell::new(None)};
 static FACTORY:Cell<Option<Flight>>=const{Cell::new(None)};
 static INITIALIZE:Cell<Option<Flight>>=const{Cell::new(None)};
}
pub struct FactoryGuard(Option<Flight>,Option<(usize,f32)>);
impl Drop for FactoryGuard{fn drop(&mut self){
 if let Some((address,value))=self.1{unsafe{ptr::write_unaligned(address as*mut f32,value);}}
 FACTORY.with(|s|s.set(self.0));
}}
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
fn quick_drop(bullet:i32)->bool{crate::sword_map::QUICK_DROP.binary_search(&bullet).is_ok()}
fn continues(parent:i32,child:i32)->bool{crate::sword_map::FLIGHT_CONTINUATIONS.binary_search(&(parent,child)).is_ok()}
fn range_end(bullet:i32)->Option<i32>{crate::sword_map::RANGE_ENDS.binary_search_by_key(&bullet,|r|r.0).ok().map(|i|crate::sword_map::RANGE_ENDS[i].1)}
fn arc_for(bullet:i32,launch:Point)->Option<Arc>{
 if !velocity(launch){return None;}
 let i=crate::sword_map::DROP_ARCS.binary_search_by_key(&bullet,|r|r.0).ok()?;
 Some(Arc{launch,elapsed:0.,gravity:crate::sword_map::DROP_ARCS[i].1})
}
fn arc_velocity(arc:&mut Arc,native:Point,dt:f32)->Option<Point>{
 if !dt.is_finite()||dt<=0.||!finite(native){return None;}
 let horizontal=arc.launch[0].hypot(arc.launch[2]);
 let heading=native[0].hypot(native[2]);
 let direction=if heading>0.0001{[native[0]/heading,native[2]/heading]}else if horizontal>0.0001{[arc.launch[0]/horizontal,arc.launch[2]/horizontal]}else{[0.,0.]};
 // Midpoint velocity keeps continuous gravity independent of frame rate.
 // Native yaw steering remains; player launch pitch is not cancelled.
 let vertical=arc.launch[1]-arc.gravity*(arc.elapsed+dt*0.5);
 arc.elapsed+=dt;
 Some([direction[0]*horizontal,vertical,direction[1]*horizontal,0.])
}
pub unsafe extern "system" fn motion(ins:usize,row_ref:usize,dt:f32){
 let original:unsafe extern "system" fn(usize,usize,f32)=std::mem::transmute(MOTION_ORIGINAL.load(Ordering::Acquire));
 original(ins,row_ref,dt);
 if !ACTIVE.load(Ordering::Acquire){return;}
 let Some(mut value)=instance(ins)else{return;};
 let Some(arc)=value.arc.as_mut()else{return;};
 let current=ptr::read_unaligned((ins+0x30)as*const Point);
 let Some(velocity)=arc_velocity(arc,current,dt)else{return;};
 // Apply before native motion/collision. Both integration modes see the same
 // midpoint; no position teleport, fabricated impact, or lifetime extension.
 for offset in [0x30,0x40]{ptr::write_unaligned((ins+offset)as*mut Point,velocity);}
 if let Ok(mut state)=STATE.lock(){state.instances.insert(ins,value);}
}
fn transition_distance(bullet:i32)->Option<f32>{
 if range_end(bullet).is_some()||matches!(bullet,10670000|10670100|10693000|210671000|210671050|10491000|210491000|210491001|210491002|210470000|210470050|210772000|210670000|210741000|210741050){Some(RANGE)}else{None}
}
fn deferred_transition(bullet:i32)->bool{range_end(bullet).is_some()||matches!(bullet,10670000|10670100|210671000|210671050)}
fn ground_projectile(bullet:i32)->bool{
 matches!(bullet,12407000..=12408400|10691000..=10691005|10691010..=10691016|10691020..=10691025|10691050..=10691055|10691060..=10691066|10691070..=10691075|10692000|10692002..=10692006|10692009..=10692016|10692100..=10692107|10692109..=10692112|10693000..=10693008|10491000|210491000|210491001|210491002|210470000|210470050|210772000|210670000|210741000|210741050)
}
fn native_return(parent:i32,child:i32)->bool{matches!((parent,child),(10670000,10670001)|(10670100,10670101)|(210671000,210671001)|(210671050,210671051)|(210772000,210772001)|(210670000,210670001)|(210741000,210741001)|(210741050,210741051)|(10491000,10491003)|(10491000,10491001)|(210491000,210491005)|(210491001,210491005)|(210491002,210491005)|(210491000,210491003)|(210491001,210491003)|(210491002,210491003)|(210781000,210781001)|(210781050,210781051)|(10631000,10631001)|(10631005,10631001))}
fn breath_stream(bullet:i32)->bool{matches!(bullet,10700000|10702000|10703000|10704000|210702002)}
fn synchronized_flight(bullet:i32)->bool{
 matches!(bullet,10406000|10406001|10603000|10603050|10700000|10700002|10700100|10700101|10702000|10702002|10702100|10702101|10703000|10703002|10703100|10703101|10704000|10704002|10704100|10704101|10705000|210702000|210702001|210730000|210730007|210774000)
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
 // Native payload+184 is copied to group+4B4, used by EmittePosType 6's
 // target-distance test. Keep native near-target / over-range self placement.
 let restore=if value.is_some()&&payload!=0&&matches!(read_i32(payload+4),210781000|210781050){
  let address=payload+0x184;let original=ptr::read_unaligned(address as*const f32);
  if original.is_finite()&&original>RANGE{ptr::write_unaligned(address as*mut f32,RANGE);Some((address,original))}else{None}
 }else{None};
 FactoryGuard(FACTORY.with(|s|s.replace(value)),restore)
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
 crate::echo_cast::held_bound(ins);
 let Some(mut flight)=INITIALIZE.with(Cell::get)else{return;};
 let p=point(ins);if !finite(p){return;}
 let row=param(ins,BASE.load(Ordering::Acquire));
 let bullet=read_i32(payload+4);
 flight.horizontal=ground_projectile(bullet)||quick_drop(bullet);
 if flight.origin.is_none()&&free_flight(row,bullet)&&velocity(ptr::read_unaligned((ins+0x30)as*const Point)){flight.origin=Some(p);}
 let arc=if free_flight(row,bullet){arc_for(bullet,ptr::read_unaligned((ins+0x30)as*const Point))}else{None};
 if let Ok(mut state)=STATE.lock(){state.instances.insert(ins,Instance{handle:ptr::read_unaligned((ins+8)as*const u64),bullet,flight,previous:p,arc,ending:false});}
}
pub unsafe extern "system" fn child(ins:usize,position:usize,orientation:usize,bullet:i32,mode:u8)->u8{
 let original:unsafe extern "system" fn(usize,usize,usize,i32,u8)->u8=std::mem::transmute(CHILD_ORIGINAL.load(Ordering::Acquire));
 let flight=if ACTIVE.load(Ordering::Acquire){instance(ins).and_then(|value|{
  if native_return(value.bullet,bullet)||(value.ending&&range_end(value.bullet)==Some(bullet)){None}else if continues(value.bullet,bullet){Some(value.flight)}else{Some(Flight::default())}
 })}else{None};
 if position!=0&&flight.is_some_and(|value|outside(value,ptr::read_unaligned(position as*const Point))){return 0;}
 let _guard=ChildGuard(CHILD.with(|s|s.replace(Some(flight))));
 let _echo=crate::echo_cast::child_scope(ins,bullet);
 original(ins,position,orientation,bullet,mode)
}
pub unsafe extern "system" fn reset(ins:usize){
 crate::echo_cast::held_reset(ins);
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
 // Four metres is the neutral flat-ground calibration, not a clipping plane.
 // Higher launches and terrain can carry the same smooth arc beyond it.
 if value.arc.is_some(){return 0;}
 if let Some(limit)=transition_distance(value.bullet){
  if let Some(origin)=value.flight.origin{
   if let Some(distance)=displacement(value.flight,p).filter(|distance|*distance>=limit){
    let ratio=limit/distance;
    let position=[origin[0]+(p[0]-origin[0])*ratio,if value.flight.horizontal{p[1]}else{origin[1]+(p[1]-origin[1])*ratio},origin[2]+(p[2]-origin[2])*ratio,p[3]];
    ptr::write_unaligned((ins+0x10)as*mut Point,position);
    if let Some(child)=range_end(value.bullet){
     // Target-validated native end effect. Most keep the existing child; the
     // reviewed Elden Stars stages share its terminal explosion, not another flight.
     value.ending=true;
     if let Ok(mut state)=STATE.lock(){state.instances.insert(ins,value);}
     ptr::write_unaligned((ins+0x600)as*mut i32,child);
    }
    let burst:unsafe extern "system" fn(usize,i32)=std::mem::transmute(base+MODE_RVA);burst(ins,3);
    return u8::from(!deferred_transition(value.bullet));
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
