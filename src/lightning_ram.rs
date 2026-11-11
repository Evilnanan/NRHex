//! Experimental Lightning Ram: private HKS control + native capsule velocity submission.
use crate::{ptr,read_ptr,read_i32,has_effect,native_actor,ACTIVE,BASE,ram_motion::{Motion,V,HIGH}};
use std::{collections::BTreeMap,sync::{Mutex,atomic::{AtomicPtr,AtomicU64,Ordering}}};
pub const OWNER:i32=95005600;
pub const FAST:i32=OWNER+1;
pub const QUERY:i32=990060;
pub const STOP_QUERY:i32=990061;
pub const BULLET:i32=OWNER+50;
pub const ATTACK:i32=OWNER+60;
pub const MOTION_RVA:usize=0x4702bb;
pub const MOTION_PATTERN:&str="48 8D 55 80 48 8B 8F 98 00 00 00 E8 25 75 92 00";
pub static MOTION_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
#[derive(Default)]struct State{actor:usize,handle:u64,stats:usize,motion:Motion,active:bool,interrupted:bool,heartbeat:f32,drive:bool,wish:V,window:bool,recoil:bool,rebound:f32,started:bool,trail:Option<V>,generation:u64}
static GENERATION:AtomicU64=AtomicU64::new(1);
static CONTACTS:Mutex<BTreeMap<(u64,u64,u64),V>>=Mutex::new(BTreeMap::new());
static STATE:Mutex<State>=Mutex::new(State{actor:0,handle:0,stats:0,motion:Motion{velocity:V(0.,0.,0.),locked:false,airborne:false},active:false,interrupted:false,heartbeat:0.,drive:false,wish:V(0.,0.,0.),window:false,recoil:false,rebound:0.,started:false,trail:None,generation:0});
#[repr(C,align(16))]struct Point([f32;4]);
pub fn reset(){if let Ok(mut s)=STATE.lock(){*s=State::default();}}
unsafe fn owns(actor:usize,base:usize)->bool{actor!=0&&has_effect(actor,OWNER,base)&&!has_effect(actor,crate::reward_cycle_runtime::RESET,base)&&native_actor::stats(actor)!=0&&!native_actor::incapacitated(actor,native_actor::stats(actor))}
unsafe fn eligible(actor:usize,base:usize)->bool{crate::is_local(actor,base)&&owns(actor,base)}
pub unsafe fn empty(actor:usize,source:i32)->bool{
 if actor==0||!matches!(source,-1|-2){return false;}
 let get:unsafe extern "system" fn(usize,i32)->i32=std::mem::transmute(read_ptr(read_ptr(actor)+0x250));
 get(actor,source)==110000
}
pub unsafe fn art(actor:usize,hand:i32,base:usize)->bool{matches!(hand,0|1)&&owns(actor,base)&&empty(actor,if hand==0{-2}else{-1})}
pub unsafe fn behavior(actor:usize,judge:i32,source:i32,base:usize)->Option<i32>{
 if !matches!(judge,3710|3711|3715|3716)||!owns(actor,base)||!empty(actor,source){return None;}
 let high=if crate::is_local(actor,base){STATE.lock().is_ok_and(|s|s.actor==actor&&s.active&&s.motion.velocity.len()>=HIGH)}else{has_effect(actor,FAST,base)};
 if high{Some(OWNER+90)}else{Some(300000000+judge-3000)}
}
unsafe fn fee(actor:usize,base:usize)->i32{
 let mut row=[0u8;0x18];row[0x14..0x16].copy_from_slice(&5i16.to_le_bytes());let r=[0usize,row.as_ptr()as usize];
 let f:unsafe extern "system" fn(usize,i32,usize)->i32=std::mem::transmute(base+crate::onyx_mastery::ARTS_FEE_RVA);f(r.as_ptr()as usize,0,read_ptr(actor+0x190)).max(0)
}
// One query per active action update. Return 0 native / 1 coast / 2 next roll / 3 recoil.
pub unsafe fn query(actor:usize,art_id:i32,hand:i32,held:bool,window:bool,angle:f32,base:usize)->i32{
 if art_id!=113||!eligible(actor,base)||!art(actor,hand,base){return 0;}
 let stats=native_actor::stats(actor);let cost=fee(actor,base);let can_drive=held&&read_i32(stats+0x150)>=cost&&read_i32(stats+0x15c)>=18;
 let mut rotation=Point([0.;4]);let get:unsafe extern "system" fn(usize,*mut f32)->*const f32=std::mem::transmute(base+crate::CHR_ROTATION_RVA);get(actor,rotation.0.as_mut_ptr());
 let yaw=rotation.0[1]-angle.to_radians();let wish=V(-yaw.sin(),0.,-yaw.cos());
 let (result,pay)={let Ok(mut s)=STATE.lock()else{return 0;};
  if s.actor!=actor||s.handle!=read_ptr(actor+8)as u64||s.stats!=stats{*s=State{actor,handle:read_ptr(actor+8)as u64,stats,..Default::default()};}
  if s.recoil{s.recoil=false;s.active=false;return 3;}
  s.heartbeat=0.06;if s.interrupted{return 0;}s.drive=can_drive;if wish.finite(){s.wish=wish;}
  // Native startup runs until real root motion starts. Observe it without
  // replacing the windup with immediate movement.
  let initial=!s.active;
  if initial{if !can_drive||s.rebound>0.{return 0;}s.active=true;s.started=false;s.window=false;s.generation=GENERATION.fetch_add(1,Ordering::Relaxed);}
  if !can_drive&&!s.motion.locked&&!s.motion.airborne&&s.started{s.active=false;return 0;}
  let cycle=window&&!s.window;s.window=window;
  (if cycle{2}else if s.started&&(s.motion.locked||s.motion.airborne){1}else{0},(cycle||initial)&&can_drive)
 };
 if pay&&cost>0{let add:unsafe extern "system" fn(usize,i32)=std::mem::transmute(base+crate::onyx_mastery::FP_ADD_RVA);add(stats,-cost);}
 result
}
struct Contact{normal:V,fraction:f32}
unsafe fn sweep(base:usize,position:V,motion:V)->Option<Contact>{
 let geometry=read_ptr(base+0x3c1e028);if geometry==0||read_ptr(geometry+0x98)==0||read_ptr(base+0x3c1d9a8)==0{return None;}
 let start=Point([position.0,position.1+0.45,position.2,1.]);let delta=Point(motion.array());
 let mut hit=u32::MAX;let mut flags=0;let mut fraction=1.;let mut point=Point([0.;4]);let mut normal=Point([0.;4]);
 let cast:crate::spirit_placement::Sweep=std::mem::transmute(base+crate::SPIRIT_SWEEP_RVA);
 let result=cast(read_ptr(geometry+0x98),&mut hit,0x79,start.0.as_ptr(),delta.0.as_ptr(),0.3,0,&mut flags,point.0.as_mut_ptr(),normal.0.as_mut_ptr(),&mut fraction,ptr::null_mut());
 let n=V::from(normal.0);(result==&mut hit as*mut u32&&hit&0xffffff!=0xffffff&&n.finite()&&n.len()>0.5&&fraction.is_finite()&&(0. ..=1.).contains(&fraction)).then_some(Contact{normal:n.unit(),fraction})
}
unsafe extern "system" fn velocity(physics:usize,vector:*mut f32,time:usize){
 if !ACTIVE.load(Ordering::Acquire)||physics==0||vector.is_null()||time==0{return;}
 let base=BASE.load(Ordering::Acquire);let actor=read_ptr(physics+8);
 if !eligible(actor,base)||read_ptr(read_ptr(actor+0x1b8)+0x68)!=physics{return;}
 let dt=ptr::read_unaligned((time+8)as*const f32);let position=V::from(ptr::read_unaligned((physics+0x70)as*const[f32;4]));
 if !position.finite(){return;}
 // Do not hold gameplay state across a native scene query.
 let (active,started,v,rebound,airborne)=match STATE.lock(){Ok(s)if s.actor==actor=>(s.active,s.started,s.motion.velocity,s.rebound,s.motion.airborne),_=>return};
 if !active&&rebound<=0.{return;}
 if !started{
  let native=V::from(ptr::read_unaligned(vector as*const[f32;4]));
  if native.0.hypot(native.2)<0.5{return;}
  if let Ok(mut s)=STATE.lock(){s.motion.velocity=native;s.started=true;}
 }
 let ground=if !airborne||(-3. ..=0.5).contains(&v.1){sweep(base,position,V(0.,-0.28,0.)).filter(|c|c.normal.1>0.45).map(|c|c.normal)}else{None};
 let candidate={let Ok(mut s)=STATE.lock()else{return;};let drive=s.drive;let wish=s.wish;
  if !s.motion.advance(dt,drive&&rebound<=0.,wish,ground){s.active=false;return;}s.motion.velocity};
 let contact=sweep(base,position,candidate.scale(dt));
 let desired={let Ok(mut s)=STATE.lock()else{return;};
  if let Some(c)=contact{if c.fraction<1.&&s.motion.velocity.dot(c.normal)< -0.1{if s.motion.collide(c.normal){s.active=false;s.recoil=true;s.rebound=0.45;}}}
  s.motion.velocity
 };
 // Only the stack-local native velocity is replaced. Both capsules, normal
 // collision resolution and native movement/network updates consume it.
 ptr::write_unaligned(vector as*mut[f32;4],desired.array());
 if active&&desired.0.hypot(desired.2)>0.5{
  let mut rotation=Point([0.;4]);let get:unsafe extern "system" fn(usize,*mut f32)->*const f32=std::mem::transmute(base+crate::CHR_ROTATION_RVA);
  get(actor,rotation.0.as_mut_ptr());rotation.0[1]=(-desired.0).atan2(-desired.2);
  let set:unsafe extern "system" fn(usize,*const f32)=std::mem::transmute(base+crate::parry_turn::SET_ROTATION_RVA);set(physics,rotation.0.as_ptr());
 }
}
unsafe fn fast_marker(actor:usize,base:usize,enabled:bool){
 if actor==0||!crate::is_local(actor,base){return;}
 let present=has_effect(actor,FAST,base);if present==enabled{return;}
 if enabled{let add:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);add(actor,FAST,0);}
 else{let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);remove(read_ptr(actor+0x190),FAST);}
}
pub unsafe fn update(actor:usize,base:usize,dt:f32){
 if !eligible(actor,base){fast_marker(actor,base,false);reset();return;}
 if !dt.is_finite()||dt<=0.{return;}
 if !STATE.lock().is_ok_and(|s|s.actor==actor){fast_marker(actor,base,false);return;}
 let position=V::from(crate::position(actor,base));
 let pulses={let Ok(mut s)=STATE.lock()else{return;};if s.actor!=actor{return;}
  s.heartbeat=(s.heartbeat-dt).max(0.);s.rebound=(s.rebound-dt).max(0.);
  if s.heartbeat==0.{s.active=false;if s.rebound==0.{s.started=false;s.interrupted=false;}s.trail=None;}
  if !s.active||!s.started||s.motion.velocity.len()<HIGH{s.trail=None;Vec::new()}else{
   let from=s.trail.unwrap_or(position);let displacement=position.sub(from);let length=displacement.len();
   if length>10.{s.active=false;Vec::new()}else if length<0.5&&s.trail.is_some(){Vec::new()}else{
    s.trail=Some(position);
    let count=(length/0.65).ceil().clamp(1.,16.)as usize;
    let damage=s.motion.velocity.len().powi(2)*1.5;
    (1..=count).map(|i|(from.add(displacement.scale(i as f32/count as f32)),damage,s.handle,s.generation)).collect()
   }
  }
 };
 let high=STATE.lock().is_ok_and(|s|s.active&&s.started&&s.motion.velocity.len()>=HIGH);fast_marker(actor,base,high);
 for (p,damage,handle,generation)in pulses{crate::static_charge::emit_ram(handle,generation,damage,[p.0,p.1+0.5,p.2,1.],base);}
}
// The serialized roll generation also deduplicates separate pulses on peers.
// Native ownership/authority still decides which contacts apply damage.
pub unsafe fn allow_contact(target:usize,bullet:usize,generation:Option<u64>,base:usize)->bool{
 let Some(generation)=generation else{return false;};
 let owner=read_ptr(bullet+0xb0)as u64;let handle=read_ptr(target+8)as u64;
 if handle==owner||!crate::spatial_ready(target)||crate::ambulance::party_member(target,base){return false;}
 if STATE.lock().is_ok_and(|s|s.handle==owner&&(!s.active||s.generation!=generation)){return false;}
 let actor=native_actor::resolve(base,owner);if !crate::spatial_ready(actor){return false;}
 let position=V::from(crate::position(actor,base));
 let Ok(mut contacts)=CONTACTS.lock()else{return false;};
 contacts.retain(|&(h,g,_),p|h!=owner||(g==generation&&p.sub(position).len()<3.));
 !contacts.contains_key(&(owner,generation,handle))
}
pub unsafe fn accepted_contact(target:usize,bullet:usize,generation:Option<u64>,result:u8,base:usize){
 if result==0{return;}let Some(generation)=generation else{return;};
 let owner=read_ptr(bullet+0xb0)as u64;let actor=native_actor::resolve(base,owner);
 if !crate::spatial_ready(actor){return;}
 let position=V::from(crate::position(actor,base));
 if let Ok(mut contacts)=CONTACTS.lock(){
  if contacts.len()>1024{contacts.clear();}
  contacts.insert((owner,generation,read_ptr(target+8)as u64),position);
 }
}
// Called after native 4381F0 has written the resolved impact into action+1C.
pub unsafe fn reaction(module:usize,hit:usize,kind:i32){
 if !ACTIVE.load(Ordering::Acquire)||module==0||hit==0||!STATE.lock().is_ok_and(|s|s.active){return;}
 if ptr::read((hit+0x2c1)as*const u8)&8!=0{return;}
 let target=read_ptr(module+8);if target==0{return;}let modules=read_ptr(target+0x1b8);if modules==0{return;}let action=read_ptr(modules+8);if action==0{return;}
 let impact=ptr::read((action+0x1c)as*const u8);let mut s=match STATE.lock(){Ok(s)=>s,_=>return};
 if target==s.actor&&kind!=3&&impact>=4{s.active=false;s.interrupted=true;s.started=false;s.motion=Motion::default();}
 if CONTACT.with(std::cell::Cell::get)&&read_i32(hit+0x40)==ATTACK&&read_i32(hit+0x44)==1&&s.active&&(kind==3||kind==99999||impact<7){
  s.motion.recoil();s.recoil=true;s.active=false;s.rebound=0.45;
 }
}
#[unsafe(naked)]
pub unsafe extern "system" fn motion_detour(){
 core::arch::naked_asm!(
  "pushfq", "sub rsp,0xc8",
  "mov [rsp+0x20],rax", "mov [rsp+0x28],rcx", "mov [rsp+0x30],rdx", "mov [rsp+0x38],r8",
  "mov [rsp+0x40],r9", "mov [rsp+0x48],r10", "mov [rsp+0x50],r11",
  "movdqu [rsp+0x60],xmm0", "movdqu [rsp+0x70],xmm1", "movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3", "movdqu [rsp+0xa0],xmm4", "movdqu [rsp+0xb0],xmm5",
  "mov rcx,rdi", "lea rdx,[rbp-0x80]", "mov r8,r14", "call {callback}",
  "mov rax,[rsp+0x20]", "mov rcx,[rsp+0x28]", "mov rdx,[rsp+0x30]", "mov r8,[rsp+0x38]",
  "mov r9,[rsp+0x40]", "mov r10,[rsp+0x48]", "mov r11,[rsp+0x50]",
  "movdqu xmm0,[rsp+0x60]", "movdqu xmm1,[rsp+0x70]", "movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]", "movdqu xmm4,[rsp+0xa0]", "movdqu xmm5,[rsp+0xb0]",
  "add rsp,0xc8", "popfq", "jmp qword ptr [rip+{original}]",
  callback=sym velocity,original=sym MOTION_ORIGINAL
 );
}

thread_local!{static CONTACT:std::cell::Cell<bool>=const{std::cell::Cell::new(false)};}
pub struct ContactScope(bool);
impl ContactScope{pub fn enter(local:bool)->Self{Self(CONTACT.with(|s|s.replace(local)))}}
impl Drop for ContactScope{fn drop(&mut self){CONTACT.with(|s|s.set(self.0));}}
pub unsafe fn local_bullet(bullet:usize)->bool{STATE.lock().is_ok_and(|s|s.handle!=0&&s.handle==read_ptr(bullet+0xb0)as u64)}

#[cfg(test)]#[path="lightning_ram_native_tests.rs"]mod native_tests;
