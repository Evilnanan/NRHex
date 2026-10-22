//! Delayed spell-root replay through native BulletMan, with retained native
//! target/snapshot references. Descendants and replay dispatch are not echoed.
use crate::{ptr,read_ptr,read_i32,has_effect,is_local,native_actor,ACTIVE,BASE};
use std::{cell::Cell,collections::BTreeMap,sync::{Arc,Mutex,atomic::{AtomicPtr,Ordering}}};
#[path="echo_cast_held.rs"]mod held;
#[path="echo_cast_stream.rs"]mod stream;
pub(crate) use held::{bound as held_bound,child_scope,reset as held_reset};
pub const OWNER:i32=95003500;
pub const EMIT_RVA:usize=0x35a460;
pub const FACTORY_RVA:usize=0x35da70;
pub const FACTORY_PATTERN:&str="48 89 5C 24 08 55 56 57 41 54 41 55 41 56 41 57 48 83 EC 40 4D 8B F0 48 8B EA 48 8B F1";
pub const TARGET_COPY_RVA:usize=0x3482e0;
pub const TARGET_COPY_PATTERN:&str="48 89 5C 24 08 57 48 83 EC 20 0F B6 02 48 8B D9 88 01 48 8B FA 48 8B 42 08";
pub const RELEASE_RVA:usize=0x33e650;
pub const RELEASE_PATTERN:&str="48 89 4C 24 08 53 48 83 EC 20 48 8B D9 48 8B 89 98 00 00 00 48 85 C9 74 20 8B 41 18 8D 50 FF 33 D0 81 E2 FF FF FF 3F 33 D0 89 51 18 F7 C2 FF FF FF 3F 75 05 E8 37 02 1F 00 48 8B 8B A0 00 00 00 48 85 C9 74 1F 8B 41 28 8D 50 FF 33 D0 0F BA F2 1F 33 D0 89";
pub const TARGET_VALID_RVA:usize=0x3494c0;
pub const TARGET_VALID_PATTERN:&str="48 89 5C 24 08 57 48 83 EC 30 48 8B 51 10 48 8B F9 48 85 D2";
pub static EMIT_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static FACTORY_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub const INITIALIZE_RVA:usize=0x343cb0;
pub const INITIALIZE_PATTERN:&str="40 53 55 56 57 41 56 41 57 48 81 EC 98 00 00 00 48 8B 05 A1 22 7A 03";
pub const FINISH_RVA:usize=0x342590;
pub const FINISH_PATTERN:&str="48 89 5C 24 10 48 89 6C 24 18 48 89 74 24 20 57 41 56 41 57 48 81 EC A0 00 00 00 48 8B 05 B6 39 7A 03";
pub static INITIALIZE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static FINISH_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub const VOLLEY_CREATE_RVA:usize=0x52ef20;
pub const VOLLEY_MANAGER_RVA:usize=0x3c0f428;
pub const VOLLEY_CREATE_PATTERN:&str="48 89 5C 24 10 55 56 57 48 83 EC 20 48 8B EA 48 8B F9 BA FF FF FF FF 48 83 C1 40 E8 ? ? ? ? 48 8B 5F 30";
type Emit=unsafe extern "system" fn(usize,usize,usize,usize)->usize;
type Factory=unsafe extern "system" fn(usize,usize,usize)->usize;
type Initialize=unsafe extern "system" fn(usize,usize,usize,u32,u32)->usize;
type Finish=unsafe extern "system" fn(usize);
#[repr(C,align(16))]struct Descriptor([u8;0x110]);
#[repr(C,align(16))]#[derive(Clone,Copy)]struct Transform([u8;0x30]);
type Shots=Arc<Mutex<BTreeMap<u32,Transform>>>;
struct Binding{group:usize,id:i32,replay:bool,shots:Shots}
struct Volley{source:usize,replay:usize,base:usize}
impl Drop for Volley{fn drop(&mut self){unsafe{
 let release:unsafe extern "system" fn(usize)=std::mem::transmute(self.base+RELEASE_RVA);
 for reference in [self.source,self.replay]{
  let mut descriptor=Descriptor([0;0x110]);
  ptr::write_unaligned(descriptor.0.as_mut_ptr().add(0xa0)as*mut usize,reference);
  release(descriptor.0.as_mut_ptr()as usize);
 }
}}}
struct Ticket{descriptor:Box<Descriptor>,pose:[u8;0x40],base:usize,wait:f64,area:bool,shots:Shots,owner:Owner,volley:Option<Arc<Volley>>,shared:Option<Arc<stream::Shared>>}
impl Drop for Ticket{fn drop(&mut self){unsafe{
 let release:unsafe extern "system" fn(usize)=std::mem::transmute(self.base+RELEASE_RVA);
 release(self.descriptor.0.as_mut_ptr()as usize);
}}}
#[derive(Clone,Copy,Default,PartialEq,Eq)]struct Owner{actor:usize,handle:u64,stats:usize,context:usize}
#[derive(Default)]struct Queue{owner:Owner,pending:Vec<Ticket>,groups:Vec<Binding>}
static QUEUE:Mutex<Queue>=Mutex::new(Queue{owner:Owner{actor:0,handle:0,stats:0,context:0},pending:Vec::new(),groups:Vec::new()});
#[derive(Clone,Copy,Default)]struct Scope{descriptor:usize,owner:Owner,consumed:bool}
#[derive(Clone,Copy,Default)]struct Replay{ticket:usize,consumed:bool}
thread_local!{static SCOPE:Cell<Scope>=Cell::new(Scope::default());static REPLAY:Cell<Replay>=Cell::new(Replay::default());}
struct ScopeGuard(Scope);
impl Drop for ScopeGuard{fn drop(&mut self){SCOPE.with(|s|s.set(self.0));}}
struct ReplayGuard(Replay);
impl Drop for ReplayGuard{fn drop(&mut self){REPLAY.with(|s|s.set(self.0));}}
fn eligible(magic:i32,bullet:i32)->bool{
 if crate::echo_cast_data::EXCLUDED.binary_search(&magic).is_ok(){return false;}
 area_replay(magic,bullet).is_some()||crate::echo_cast_data::ROOTS.binary_search_by_key(&magic,|r|r.0).ok().is_some_and(|i|crate::echo_cast_data::ROOTS[i].1.binary_search(&bullet).is_ok())
}
fn root_magic(magic:i32,bullet:i32)->Option<i32>{
 if magic!=-1{return eligible(magic,bullet).then_some(magic);}
 crate::echo_cast_data::ROOTS.iter().find(|(_,roots)|roots.binary_search(&bullet).is_ok()).map(|(magic,_)|*magic)
  .or_else(||crate::echo_cast_data::AREAS.iter().find(|(_,root,_)|*root==bullet).map(|(magic,_,_)|*magic))
}
fn area_replay(magic:i32,bullet:i32)->Option<i32>{
 if crate::echo_cast_data::WORLD_AREAS.binary_search(&magic).is_ok(){
  return crate::echo_cast_data::ROOTS.binary_search_by_key(&magic,|row|row.0).ok().and_then(|index|crate::echo_cast_data::ROOTS[index].1.binary_search(&bullet).ok()).map(|_|bullet);
 }
 crate::echo_cast_data::AREAS.binary_search_by_key(&magic,|r|r.0).ok().and_then(|i|{
  let (_,root,replay)=crate::echo_cast_data::AREAS[i];(root==bullet).then_some(replay)
 })
}
fn replay_delay(_:i32)->f64{0.5}
unsafe fn owner(actor:usize,base:usize)->Option<Owner>{
 if !is_local(actor,base)||!has_effect(actor,OWNER,base){return None;}
 let stats=native_actor::stats(actor);if stats==0||native_actor::incapacitated(actor,stats){return None;}
 Some(Owner{actor,handle:ptr::read_unaligned((actor+8)as*const u64),stats,context:read_ptr(base+0x3c131c8)})
}
unsafe fn scope(descriptor:usize,base:usize)->Option<Owner>{
 if descriptor==0||read_i32(descriptor+8)>=0{return None;}
 root_magic(read_i32(descriptor+0x18),read_i32(descriptor+0x14)).or_else(||held::magic(read_i32(descriptor+0x18),read_i32(descriptor+0x14)))?;
 let world=read_ptr(base+crate::WORLD_RVA);if world==0{return None;}
 let actor=read_ptr(world+0x174e8);let value=owner(actor,base)?;
 (ptr::read_unaligned(descriptor as*const u64)==value.handle).then_some(value)
}
pub unsafe extern "system" fn emit(manager:usize,out:usize,descriptor:usize,error:usize)->usize{
 let original:Emit=std::mem::transmute(EMIT_ORIGINAL.load(Ordering::Acquire));
 let value=if ACTIVE.load(Ordering::Acquire)&&REPLAY.with(Cell::get).ticket==0{scope(descriptor,BASE.load(Ordering::Acquire))}else{None};
 let _guard=ScopeGuard(SCOPE.with(|s|s.replace(value.map_or(Scope::default(),|owner|Scope{descriptor,owner,consumed:false}))));
 let result=original(manager,out,descriptor,error);
 crate::wraith::emitted(descriptor,out,BASE.load(Ordering::Acquire));
 result
}
unsafe fn retain_counter(pointer:usize,offset:usize,mask:u32){
 if pointer==0{return;}
 let address=(pointer+offset)as*mut u32;let old=ptr::read_unaligned(address);
 ptr::write_unaligned(address,(old&!mask)|(old.wrapping_add(1)&mask));
}
unsafe fn replay_volley(scope:Scope,magic:i32,base:usize)->Result<Option<Arc<Volley>>,()>{
 let source=read_ptr(scope.descriptor+0xa0);
 if source==0||!matches!(magic,4300..=4302|6001|6710|6820|6830)&&crate::echo_cast_data::EXPANDED.binary_search(&magic).is_err(){return Ok(None);}
 let existing=QUEUE.lock().ok().and_then(|queue|queue.pending.iter()
  .filter(|ticket|ticket.owner==scope.owner)
  .filter_map(|ticket|ticket.volley.as_ref()).find(|volley|volley.source==source).cloned()).or_else(||stream::volley(source,scope.owner));
 if let Some(volley)=existing{return Ok(Some(volley));}
 let manager=read_ptr(base+VOLLEY_MANAGER_RVA);if manager==0{return Err(());}
 let create:unsafe extern "system" fn(usize,*const u16)->usize=std::mem::transmute(base+VOLLEY_CREATE_RVA);
 let label=[78u16,82,72,101,120,32,69,99,104,111,0];
 let replay=create(manager,label.as_ptr());if replay==0{return Err(());}
 retain_counter(source,0x28,0x7fffffff);
 Ok(Some(Arc::new(Volley{source,replay,base})))
}
unsafe fn snapshot(scope:Scope,payload:usize,base:usize)->Option<Ticket>{
 if payload==0||ptr::read_unaligned((payload+0x10)as*const u64)!=scope.owner.handle{return None;}
 let bullet=read_i32(scope.descriptor+0x14);
 let magic=root_magic(read_i32(scope.descriptor+0x18),bullet).or_else(||held::released(read_i32(scope.descriptor+0x18),bullet))?;
 let replay=area_replay(magic,bullet);
 let targets=read_ptr(payload+0x148);let end=read_ptr(payload+0x150);
 // Independent root shots use one native target descriptor. Multi-target
 // selection needs a separate replay representation rather than choosing one.
 if replay.is_none()&&(targets==0||end.wrapping_sub(targets)!=0x50){return None;}
 let volley=replay_volley(scope,magic,base).ok()?;
 let mut d=Box::new(Descriptor(ptr::read_unaligned(scope.descriptor as*const[u8;0x110])));
 if matches!(magic,6710|7520){d.0[0x98..0xa0].fill(0);}
 if let Some(volley)=volley.as_ref(){ptr::write_unaligned(d.0.as_mut_ptr().add(0xa0)as*mut usize,volley.replay);}
 d.0[0x90..0x98].fill(0);
 // The copied target is replaced with the target already resolved by native
 // calculation, not the player's lock-on at replay time.
 if let Some(bullet)=replay{
  d.0[0x14..0x18].copy_from_slice(&bullet.to_le_bytes());
  // These stationary radial areas use the captured world pose. Clear entity
  // targeting and its reference, with the native empty handle/index sentinels.
  d.0[0xb0..0x100].fill(0);
  d.0[0xb8..0xc0].fill(0xff);d.0[0xe0..0xe4].fill(0xff);d.0[0xf0..0xf4].fill(0xff);
 }else{
  ptr::write_unaligned(d.0.as_mut_ptr().add(0xc0)as*mut usize,0);
  let copy:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(base+TARGET_COPY_RVA);
  copy(d.0.as_mut_ptr().add(0xb0)as usize,targets);
 }
 retain_counter(read_ptr(d.0.as_ptr()as usize+0x98),0x18,0x3fffffff);
 retain_counter(read_ptr(d.0.as_ptr()as usize+0xa0),0x28,0x7fffffff);
 // Native flag0 requests an independent shot; bit3 retains native networking.
 let flags=read_i32(scope.descriptor+0x44)as u32|3;
 d.0[0x44..0x48].copy_from_slice(&flags.to_le_bytes());
 Some(Ticket{descriptor:d,pose:ptr::read_unaligned((payload+0x190)as*const[u8;0x40]),base,wait:replay_delay(magic),area:replay.is_some(),shots:Arc::default(),owner:scope.owner,volley,shared:None})
}
pub unsafe extern "system" fn factory(manager:usize,payload:usize,error:usize)->usize{
 let original:Factory=std::mem::transmute(FACTORY_ORIGINAL.load(Ordering::Acquire));
 let scope=SCOPE.with(|s|{let v=s.get();s.set(Scope{consumed:true,..v});v});
 let base=BASE.load(Ordering::Acquire);
 let replay=REPLAY.with(|r|{let v=r.get();r.set(Replay{consumed:true,..v});v});
 if replay.ticket!=0&&!replay.consumed{
  let ticket=&*(replay.ticket as*const Ticket);
  if !ticket.area{
   let targets=read_ptr(payload+0x148);let end=read_ptr(payload+0x150);
   if targets==0||end.wrapping_sub(targets)!=0x50{return 0;}
   // Native calculation may reselect a current lock for a forward-only
   // descriptor. Restore its original resolved target immediately before
   // allocation; native networking then serializes the restored result too.
   let copy:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(base+TARGET_COPY_RVA);
   copy(targets,ticket.descriptor.0.as_ptr().add(0xb0)as usize);
  }
  ptr::copy_nonoverlapping(ticket.pose.as_ptr(),(payload+0x190)as*mut u8,0x40);
 }
 let seed=if replay.ticket==0{held::capture(scope,payload,base)}else{None};
 let mut ticket=if replay.ticket==0{if scope.descriptor!=0&&!scope.consumed{snapshot(scope,payload,base)}else{held::snapshot(payload,base)}}else{None};
 let continuous=if ticket.as_ref().is_some_and(|ticket|stream::eligible(ticket,payload)){ticket.take().map(|ticket|stream::seed(ticket))}else{None};
 let _stream=stream::factory(continuous.clone());
 let _held=held::factory(seed.clone());
 let _flight=crate::sword_projectile::factory_scope(payload);
 let result=original(manager,payload,error);
 held::group(result,seed);
 stream::group(result,continuous);
 crate::sword_projectile::factory_done(result);
 if result!=0&&replay.ticket!=0&&!replay.consumed{
  let ticket=&*(replay.ticket as*const Ticket);
  stream::created(ticket,result);
  if !ticket.area{if let Ok(mut queue)=QUEUE.lock(){if queue.owner==ticket.owner{bind(&mut queue,result,true,Arc::clone(&ticket.shots));}}}
 }
 if result!=0{if let Some(ticket)=ticket{enqueue(ticket,Some(result));}}
 result
}
fn enqueue(ticket:Ticket,group:Option<usize>){
 let stale=if let Ok(mut queue)=QUEUE.lock(){let old=if queue.owner!=ticket.owner{queue.owner=ticket.owner;queue.groups.clear();std::mem::take(&mut queue.pending)}else{Vec::new()};if !ticket.area{if let Some(group)=group{unsafe{bind(&mut queue,group,false,Arc::clone(&ticket.shots));}}}queue.pending.push(ticket);old}else{Vec::new()};drop(stale);
}
unsafe fn bind(queue:&mut Queue,group:usize,replay:bool,shots:Shots){
 queue.groups.retain(|binding|binding.group!=group);
 queue.groups.push(Binding{group,id:read_i32(group),replay,shots});
}
pub unsafe extern "system" fn initialize(group:usize,out:usize,transform:usize,target:u32,index:u32)->usize{
 let _held=held::initialize(group);
 let continuous=stream::source(group);
 let original:Initialize=std::mem::transmute(INITIALIZE_ORIGINAL.load(Ordering::Acquire));
 let binding=if ACTIVE.load(Ordering::Acquire)&&group!=0&&transform!=0{
  let id=read_i32(group);QUEUE.lock().ok().and_then(|queue|queue.groups.iter().find(|binding|binding.group==group&&binding.id==id).map(|binding|(binding.replay,Arc::clone(&binding.shots))))
 }else{None};
 let frozen=binding.as_ref().filter(|(replay,_)|*replay).and_then(|(_,shots)|shots.lock().ok()?.get(&index).copied());
 let source=frozen.as_ref().map_or(transform,|pose|pose.0.as_ptr()as usize);
 let Some(_flight)=crate::sword_projectile::initialize_scope(group,source)else{
  if out!=0{ptr::write_unaligned(out as*mut u64,u64::MAX);}
  return out;
 };
 let result=original(group,out,source,target,index);
 if out!=0&&read_ptr(out)!=usize::MAX{
  stream::initialized(group,continuous,transform,index);
  if let Some((false,shots))=binding{if let Ok(mut shots)=shots.lock(){shots.entry(index).or_insert_with(||Transform(ptr::read_unaligned(transform as*const[u8;0x30])));}}
 }
 result
}
pub unsafe extern "system" fn finish(group:usize){
 held::finish(group);
 stream::finish(group);
 crate::sword_projectile::finish_group(group);
 if let Ok(mut queue)=QUEUE.lock(){queue.groups.retain(|binding|binding.group!=group);}
 let original:Finish=std::mem::transmute(FINISH_ORIGINAL.load(Ordering::Acquire));original(group);
}
pub unsafe fn update(actor:usize,base:usize,dt:f32){
 let current=owner(actor,base);
 held::update(current);
 stream::update(current);
 let (mut ready,discard)=if let Ok(mut q)=QUEUE.lock(){
  if current!=Some(q.owner){q.groups.clear();(Vec::new(),std::mem::take(&mut q.pending))}else{
   let mut ready=Vec::new();let mut waiting=Vec::new();
   for mut ticket in std::mem::take(&mut q.pending){if dt.is_finite()&&dt>0.{ticket.wait-=f64::from(dt);}if ticket.wait<=1e-7{ready.push(ticket);}else{waiting.push(ticket);}}
   q.pending=waiting;(ready,Vec::new())
  }
 }else{return;};drop(discard);
 let manager=read_ptr(base+0x3c0ae30);if manager==0{return;}
 let emit:Emit=std::mem::transmute(EMIT_ORIGINAL.load(Ordering::Acquire));
 for ticket in &mut ready{
  stream::prepare(ticket);
  let descriptor=ticket.descriptor.0.as_mut_ptr()as usize;
  let _replay=ReplayGuard(REPLAY.with(|r|r.replace(Replay{ticket:ticket as*const Ticket as usize,consumed:false})));
  let mut out=-1i32;let mut error=-1i32;emit(manager,&mut out as*mut i32 as usize,descriptor,&mut error as*mut i32 as usize);
  if out>=0{emit_visual(ticket,manager,emit);}
 }
}
unsafe fn emit_visual(ticket:&Ticket,manager:usize,emit:Emit){
 let root=read_i32(ticket.descriptor.0.as_ptr()as usize+0x14);
 let Some(&(_,bullet))=crate::echo_cast_data::VISUALS.iter().find(|row|row.0==root)else{return;};
 let mut descriptor=Box::new(Descriptor(ticket.descriptor.0));
 descriptor.0[0x14..0x18].copy_from_slice(&bullet.to_le_bytes());
 descriptor.0[0x18..0x1c].copy_from_slice(&(-1i32).to_le_bytes());
 descriptor.0[0x90..0x100].fill(0);
 descriptor.0[0xb8..0xc0].fill(0xff);descriptor.0[0xe0..0xe4].fill(0xff);descriptor.0[0xf0..0xf4].fill(0xff);
 descriptor.0[0x50..0x90].copy_from_slice(&ticket.pose);
 let mut visual=Ticket{descriptor,pose:ticket.pose,base:ticket.base,wait:0.,area:true,shots:Arc::default(),owner:ticket.owner,volley:None,shared:None};
 let _replay=ReplayGuard(REPLAY.with(|state|state.replace(Replay{ticket:&visual as*const Ticket as usize,consumed:false})));
 let mut out=-1i32;let mut error=-1i32;
 emit(manager,&mut out as*mut i32 as usize,visual.descriptor.0.as_mut_ptr()as usize,&mut error as*mut i32 as usize);
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn all_roots_use_the_standard_replay_delay(){
  for &(magic,roots)in crate::echo_cast_data::ROOTS{
   for &bullet in roots{
    let expected=0.5;
    assert_eq!(replay_delay(root_magic(magic,bullet).unwrap()),expected);
    assert_eq!(replay_delay(root_magic(-1,bullet).unwrap()),expected);
   }
  }
  for &(magic,_,_)in crate::echo_cast_data::AREAS{assert_eq!(replay_delay(magic),0.5);}
  assert_eq!(replay_delay(6830),0.5);
 }
 #[test]fn root_filter_preserves_requested_exclusions_and_rejects_children(){
  assert!(eligible(4021,10402100));assert!(eligible(4021,10402150));assert!(!eligible(4021,10402151));assert!(!eligible(4200,10420000));assert!(eligible(7000,10700000));assert!(!eligible(4021,10462000));
  for (magic,roots)in [(4110,&[10411000,10411010,10411012,10411014][..]),(4200,&[10420000,10420001,10420002,10420003,10420004][..]),(4520,&[10452000,10452001,10452002][..]),(4700,&[10470000,10470005,10470010,10470020,10470030,10470040][..]),(4701,&[10470100,10470110,10470120,10470150,10470151,10470152][..]),(4130,&[10413000][..]),(7900,&[10790001,10790051,10790099][..])]{
   for &root in roots{assert!(!eligible(magic,root));}
  }
 }
 #[test]fn direct_native_spell_roots_allow_missing_magic_without_guessing_ids(){
  let mut roots=std::collections::BTreeSet::new();
  for &(magic,bullets)in crate::echo_cast_data::ROOTS{
   for &bullet in bullets{assert!(roots.insert(bullet));assert_eq!(root_magic(-1,bullet),Some(magic));assert_eq!(root_magic(magic,bullet),Some(magic));}
  }
  for &(magic,bullet,_)in crate::echo_cast_data::AREAS{
   assert!(roots.insert(bullet));assert_eq!(root_magic(-1,bullet),Some(magic));
  }
  for bullet in [10400099,10402151,10422015,10413000,10470000,10470100,10411010,10452000,10642000,10420000,10700001,10790001,95003512]{assert_eq!(root_magic(-1,bullet),None);}
  assert_eq!(root_magic(4021,10400000),None);assert_eq!(root_magic(-2,10400000),None);
 }
 #[test]fn retain_preserves_native_high_flag_bits(){unsafe{
  let mut a=[0u32;16];a[6]=0xc0000002;a[10]=0x80000005;let p=a.as_mut_ptr()as usize;retain_counter(p,0x18,0x3fffffff);retain_counter(p,0x28,0x7fffffff);assert_eq!(a[6],0xc0000003);assert_eq!(a[10],0x80000006);
 }}
 #[test]fn group_profiles_replace_only_the_native_cast_root(){
  assert_eq!(crate::echo_cast_data::AREAS.len(),11);
  for &(magic,root,replay) in crate::echo_cast_data::AREAS{
   assert!(eligible(magic,root));assert_eq!(area_replay(magic,root),Some(replay));
   assert!(!eligible(magic,replay));assert!(!eligible(magic,root+1));
  }
  for magic in [6420,6760,6971]{assert!(!eligible(magic,10000000+magic*100));}
 }
 #[test]fn expanded_world_areas_keep_native_roots_without_echoing_personal_effects(){
  assert_eq!(crate::echo_cast_data::EXPANDED.len(),26);
  assert_eq!(crate::echo_cast_data::EXCLUDED,&[4110,4200,4520,4700,4701]);
  for &magic in crate::echo_cast_data::EXPANDED{
   let roots=crate::echo_cast_data::ROOTS.iter().find(|row|row.0==magic).unwrap().1;
   for &root in roots{assert!(eligible(magic,root));assert_eq!(area_replay(magic,root),crate::echo_cast_data::WORLD_AREAS.contains(&magic).then_some(root));}
  }
  for magic in [4130,4300,4301,4302,4430,4440,4450,4460,4470,6420,7080,7500,7510,7900]{assert!(!crate::echo_cast_data::ROOTS.iter().any(|row|row.0==magic));}
 }
 #[test]fn volley_roots_include_no_cost_sides_but_not_helpers_or_descendants(){
  for (magic,held,flight)in crate::echo_cast_data::RELEASES{
   assert_eq!(held::magic(*magic,*held),Some(*magic));assert_eq!(held::magic(-1,*held),Some(*magic));
   assert_eq!(held::released(*magic,*flight),Some(*magic));assert_eq!(held::released(-1,*flight),Some(*magic));
   assert_eq!(held::released(*magic,*held),None);assert_eq!(held::released(4021,*flight),None);
   assert!(!eligible(*magic,*held));assert!(!eligible(*magic,*flight));
  }
  for (magic,visuals)in [(5030,&[10503010][..]),(6270,&[10624000,10624050][..])]{for &visual in visuals{assert!(eligible(magic,visual));}}
  for root in [10409000,10409001,10409002,10409050,10409051,10409052]{assert!(eligible(4090,root));}
  for root in [10450010,10450011,10450012,10450060,10450061,10450062]{assert!(eligible(4500,root));}
  for root in [10682010,10682011,10682012,10682070,10682071,10682072]{assert!(eligible(6820,root));}
  assert!(eligible(4220,10422000));assert!(eligible(4220,10422050));assert!(!eligible(4220,10422015));
  assert!(!eligible(4220,10422098));assert!(!eligible(4220,10422099));assert!(!eligible(4050,10404099));
  assert!(eligible(4060,10406000));assert!(!eligible(4520,10452000));assert!(!eligible(7900,10790001));
 }
}
#[cfg(test)]
#[path="echo_cast_native_tests.rs"]mod native_tests;
