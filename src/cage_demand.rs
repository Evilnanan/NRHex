use std::{cell::Cell,collections::HashMap,ptr,sync::{Mutex,OnceLock,atomic::{AtomicPtr,Ordering}}};
use crate::{hook,read_i32,read_ptr,BASE,WORLD_RVA};

static PRELOAD:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static BATCH:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static GATE:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static INIT:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static DESTROY:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static INVOKE:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static SELECT:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static REQUEST:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static RECORDS:OnceLock<Mutex<HashMap<usize,Record>>>=OnceLock::new();
thread_local!{static CONTEXT:Cell<usize>=const{Cell::new(0)};}

#[derive(Clone,Copy,Debug)]
struct Record{context:usize,map:u32,index:i32,npc:i32,requested:bool,reported:bool}
fn records()->&'static Mutex<HashMap<usize,Record>>{RECORDS.get_or_init(Default::default)}
fn cage_map(map:u32)->bool{map>>24==46&&matches!((map>>16)&255,50|60|70|80)}
fn cage_npc(npc:i32)->bool{(95300000..95300084).contains(&npc)}
fn belongs(map:u32,handle:u64)->bool{(handle>>32)as u32==map&&(handle as u32&0xff000000)==0x11000000}

struct ContextScope(usize);
impl ContextScope{fn enter(context:usize)->Self{Self(CONTEXT.with(|value|value.replace(context)))}}
impl Drop for ContextScope{fn drop(&mut self){CONTEXT.with(|value|value.set(self.0));}}

struct Part{storage:[usize;10],base:usize,enemy:bool}
#[repr(C,align(16))]
struct NativeVector([f32;4]);
impl Part{
 unsafe fn enemy(base:usize,data:usize,extra:usize,index:i32)->Self{
  let mut part=Self{storage:[0;10],base,enemy:true};
  let get:unsafe extern "system" fn(usize,usize,usize,i32)->usize=std::mem::transmute(base+0xe35620);
  get(part.address(),data,extra,index);part
 }
 unsafe fn generator_part(base:usize,data:usize,index:i32)->Self{
  let mut part=Self{storage:[0;10],base,enemy:false};
  let get:unsafe extern "system" fn(usize,usize,usize,i32)->usize=std::mem::transmute(base+0xe338a0);
  get(part.address(),data,0,index);part
 }
 fn address(&mut self)->usize{self.storage.as_mut_ptr()as usize}
 fn present(&self)->bool{self.storage[3]!=0}
 unsafe fn is_enemy(&self)->bool{self.present()&&read_i32(self.storage[3]+0xc)==2}
 unsafe fn npc(&mut self)->i32{
  let get:unsafe extern "system" fn(usize)->i32=std::mem::transmute(self.base+0xe35940);get(self.address())
 }
 unsafe fn index(&self)->i32{if self.present(){read_i32(self.storage[3]+0x10)}else{-1}}
}
impl Drop for Part{fn drop(&mut self){unsafe{
 let release:unsafe extern "system" fn(usize)=std::mem::transmute(self.base+if self.enemy{0xe35690}else{0xe33bd0});
 release(self.address());
}}}

unsafe extern "system" fn preload(task:usize){
 let original:unsafe extern "system" fn(usize)=std::mem::transmute(PRELOAD.load(Ordering::Acquire));
 let _scope=ContextScope::enter(read_ptr(task+8));original(task);
}
unsafe extern "system" fn batch(collection:usize,context:usize,budget:usize,state:usize)->u8{
 let original:unsafe extern "system" fn(usize,usize,usize,usize)->u8=std::mem::transmute(BATCH.load(Ordering::Acquire));
 let _scope=ContextScope::enter(context);original(collection,context,budget,state)
}
unsafe extern "system" fn gate(world:usize,handle:u64)->u8{
 let original:unsafe extern "system" fn(usize,u64)->u8=std::mem::transmute(GATE.load(Ordering::Acquire));
 let result=original(world,handle);
 let context=CONTEXT.with(Cell::get);
 if result==0||context==0{return result;}
 let map=read_i32(context+0x34)as u32;
 if !cage_map(map)||!belongs(map,handle){return result;}
 let data=read_ptr(context+0x58);
 if data==0{return result;}
 let base=BASE.load(Ordering::Acquire);
 let mut part=Part::enemy(base,data,read_ptr(context+0x68),(handle as u32&0xffffff)as i32);
 if part.present()&&cage_npc(part.npc()){0}else{result}
}

unsafe extern "system" fn init(generator:usize,data:usize,index:i32,context:usize){
 let original:unsafe extern "system" fn(usize,usize,i32,usize)=std::mem::transmute(INIT.load(Ordering::Acquire));
 original(generator,data,index,context);
 let map=read_i32(context+0x34)as u32;
 if !cage_map(map)||map!=read_i32(context+8)as u32{return;}
 if ptr::read((generator+0xf1)as*const u8)!=3{return;}
 let indices=std::slice::from_raw_parts((generator+0x150)as*const i32,32);
 if indices[0]<0||indices[1..].iter().any(|&value|value>=0){return;}
 let base=BASE.load(Ordering::Acquire);
 let mut part=Part::generator_part(base,data,indices[0]);
 if !part.is_enemy(){return;}
 let npc=part.npc();let index=part.index();
 if !cage_npc(npc)||index<0{return;}
 if let Ok(mut entries)=records().lock(){entries.insert(generator,Record{context,map,index,npc,requested:false,reported:false});}
}
unsafe extern "system" fn destroy(generator:usize,flags:u32)->usize{
 if let Ok(mut entries)=records().lock(){entries.remove(&generator);}
 let original:unsafe extern "system" fn(usize,u32)->usize=std::mem::transmute(DESTROY.load(Ordering::Acquire));
 original(generator,flags)
}

unsafe fn bind(generator:usize,actor:usize)->bool{
 let begin=read_ptr(generator+0xd0);let end=read_ptr(generator+0xd8);let capacity=read_ptr(generator+0xe0);
 if begin==0||end!=begin||capacity<begin+8{return false;}
 ptr::write(begin as*mut usize,actor);
 ptr::write((generator+0xd8)as*mut usize,begin+8);true
}

unsafe fn reward_records(manager:usize,map:u32)->Vec<usize>{
 let mut found=Vec::new();if manager==0||!cage_map(map){return found;}
 let head=read_ptr(manager+8);let count=read_ptr(manager+0x10);
 if head==0||count==0||count>4096{return found;}
 let mut pending=vec![read_ptr(head+8)];let mut visited=0;
 while let Some(node)=pending.pop(){
  if node==0||node==head||ptr::read((node+0x19)as*const u8)!=0{continue;}
  visited+=1;if visited>count{return Vec::new();}
  pending.push(read_ptr(node));pending.push(read_ptr(node+0x10));
  if read_i32(node+0x20)as u32!=map{continue;}
  let reward=read_ptr(node+0x28);if reward==0||read_i32(reward+0x18)as u32!=map{continue;}
  let group=read_i32(reward+0x20)as u32;
  if (5937..5950).contains(&(group%10000)){found.push(reward);}
 }
 found
}

unsafe fn restore_rewards(record:Record,base:usize){
 let attach:unsafe extern "system" fn(usize)=std::mem::transmute(base+0x37f980);
 for reward in reward_records(read_ptr(base+0x3c0b408),record.map){attach(reward);}
}

unsafe fn resolve_map_actor(base:usize,handle:u64)->usize{
 let world=read_ptr(base+WORLD_RVA);
 if world==0||handle==u64::MAX||!belongs((handle>>32)as u32,handle){return 0;}
 let resolve:unsafe extern "system" fn(usize,*const u64)->usize=std::mem::transmute(base+0x518f30);
 resolve(world,&handle)
}

unsafe fn restore_riding(record:Record,base:usize){
 let data=read_ptr(record.context+0x58);if data==0{return;}
 let count:unsafe extern "system" fn(usize,i32)->i32=std::mem::transmute(base+0xe3d530);
 let get:unsafe extern "system" fn(usize,i32,i32)->usize=std::mem::transmute(base+0xe3d550);
 let length=count(data,0x15);if !(0..=4096).contains(&length){return;}
 for index in 0..length{
  let event=get(data,0x15,index);if event==0{continue;}
  let pair=read_ptr(event+0x20);if pair==0||read_i32(pair)<0||read_i32(pair+4)<0{continue;}
  let mut rider_part=Part::generator_part(base,data,read_i32(pair));
  let mut mount_part=Part::generator_part(base,data,read_i32(pair+4));
  if !rider_part.is_enemy()||!mount_part.is_enemy(){continue;}
  let rider_index=rider_part.index();let mount_index=mount_part.index();
  if rider_index<0||mount_index<0||rider_index==mount_index||record.index!=rider_index&&record.index!=mount_index{continue;}
  let rider_npc=rider_part.npc();let mount_npc=mount_part.npc();
  if !cage_npc(rider_npc)||!cage_npc(mount_npc){continue;}
  let handle=|part_index:i32|((record.map as u64)<<32)|0x11000000|part_index as u64;
  let rider=resolve_map_actor(base,handle(rider_index));
  let mount=resolve_map_actor(base,handle(mount_index));
  if rider==0||mount==0||crate::native_actor::npc_id(rider,base)!=Some(rider_npc)||crate::native_actor::npc_id(mount,base)!=Some(mount_npc){continue;}
  if read_ptr(rider+0x20)==0||read_ptr(mount+0x20)==0||read_ptr(rider+0x60)==0||read_ptr(mount+0x60)==0{continue;}
  let rider_modules=read_ptr(rider+0x1b8);let mount_modules=read_ptr(mount+0x1b8);
  if rider_modules==0||mount_modules==0{continue;}
  let rider_action=read_ptr(rider_modules+0xe8);let mount_action=read_ptr(mount_modules+0xe8);
  if rider_action==0||mount_action==0{continue;}
  let link:unsafe extern "system" fn(usize,usize,u8)=std::mem::transmute(base+0x3d7660);
  link(rider,mount,1);
  let get_state:unsafe extern "system" fn(usize,*mut u64)->*const u64=std::mem::transmute(base+0x3d1280);
  let set_state:unsafe extern "system" fn(usize,u64)=std::mem::transmute(base+0x3d75f0);
  let mut state=0;set_state(rider,*get_state(mount,&mut state));
  let get_position:unsafe extern "system" fn(usize,*mut [f32;4])->*const [f32;4]=std::mem::transmute(base+0x3d0e60);
  let set_position:unsafe extern "system" fn(usize,*const [f32;4])=std::mem::transmute(base+0x3d75b0);
  let get_rotation:unsafe extern "system" fn(usize,*mut [f32;4])->*const [f32;4]=std::mem::transmute(base+0x3ceb20);
  let set_rotation:unsafe extern "system" fn(usize,*const [f32;4])=std::mem::transmute(base+0x3d5ed0);
  let mut position=NativeVector([0f32;4]);let mut rotation=NativeVector([0f32;4]);
  set_position(rider,get_position(mount,&mut position.0));set_rotation(rider,get_rotation(mount,&mut rotation.0));
  let activate:unsafe extern "system" fn(usize,u8)=std::mem::transmute(base+0x4819b0);
  activate(rider_action,1);activate(mount_action,1);
 }
}

unsafe fn create_selected(generator:usize,record:Record,base:usize)->bool{
 if read_i32(record.context+0x34)as u32!=record.map{return false;}
 let data=read_ptr(record.context+0x58);
 if data==0||ptr::read((record.context+0xe0)as*const u8)==0{return false;}
 let world=read_ptr(base+WORLD_RVA);if world==0{return false;}
 let handle=((record.map as u64)<<32)|0x11000000|record.index as u64;
 let existing=resolve_map_actor(base,handle);
 if existing!=0{return crate::native_actor::npc_id(existing,base)==Some(record.npc)&&bind(generator,existing);}
 let collection=world+0x9ba0;
 let mut slot=[0usize;2];
 let free:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(base+0x4af970);
 free(collection,slot.as_mut_ptr()as usize);
 if slot[0]==0{return false;}
 let begin=read_ptr(generator+0xd0);
 if begin==0||read_ptr(generator+0xd8)!=begin||read_ptr(generator+0xe0)<begin+8{return false;}
 let mut part=Part::enemy(base,data,read_ptr(record.context+0x68),record.index);
 if !part.present()||part.npc()!=record.npc{return false;}
 let mut factory=[0usize;2];
 let factory_init:unsafe extern "system" fn(usize)->usize=std::mem::transmute(base+0x3e4a80);
 let factory_address=factory_init(factory.as_mut_ptr()as usize);
 let create:unsafe extern "system" fn(usize,usize,usize,*const u32,usize,u8,u8,u8)->usize=std::mem::transmute(base+0x3e5ba0);
 let actor=create(factory_address,0,slot[0],&record.map,part.address(),ptr::read((record.context+0x4e)as*const u8),ptr::read((record.context+0x4b)as*const u8),u8::from(ptr::read((record.context+0x4f)as*const u8)==2));
 let factory_release:unsafe extern "system" fn(usize)=std::mem::transmute(base+0x3e4a90);
 if actor==0{factory_release(factory_address);return false;}
 let post:unsafe extern "system" fn(usize,usize,*const u32)=std::mem::transmute(base+0x3e5cc0);
 post(factory_address,actor,&record.map);
 factory_release(factory_address);
 ptr::write((actor+0x48)as*mut u32,ptr::read((actor+0x48)as*const u32)|1);
 let apply_state:unsafe extern "system" fn(usize,usize)=std::mem::transmute(read_ptr(read_ptr(actor)+0x610));
 let state:unsafe extern "system" fn(usize)->usize=std::mem::transmute(base+0x4f8610);
 apply_state(actor,state(collection));
 let register:unsafe extern "system" fn(usize,usize,usize,i32,u8)=std::mem::transmute(base+0x4ab700);
 register(collection,actor,slot[0],slot[1]as i32,ptr::read((record.context+0x4e)as*const u8));
 let dormant:unsafe extern "system" fn(usize)=std::mem::transmute(base+0x3d7930);
 let disable:unsafe extern "system" fn(usize,u8)=std::mem::transmute(base+0x3d63d0);
 dormant(actor);disable(actor,1);
 restore_rewards(record,base);restore_riding(record,base);
 let bound=bind(generator,actor);
 crate::nlog_info!("CageDemand created selected map={:#x} index={} npc={} handle={handle:#x} bound={bound}",record.map,record.index,record.npc);
 bound
}

unsafe fn ensure(generator:usize){
 let record=records().lock().ok().and_then(|entries|entries.get(&generator).copied());
 let Some(record)=record else{return;};if !record.requested{return;}
 if read_ptr(generator+0xd8)>read_ptr(generator+0xd0){
  if let Ok(mut entries)=records().lock(){if let Some(value)=entries.get_mut(&generator){value.requested=false;}}return;
 }
 let success=create_selected(generator,record,BASE.load(Ordering::Acquire));
 if let Ok(mut entries)=records().lock(){if let Some(value)=entries.get_mut(&generator){value.requested=!success;
  if !success&&!value.reported{value.reported=true;crate::nlog_warn!("CageDemand waiting for selected actor map={:#x} npc={}",record.map,record.npc);}
 }}
}
unsafe fn request_actor(generator:usize){
 if let Ok(mut entries)=records().lock(){if let Some(value)=entries.get_mut(&generator){value.requested=true;value.reported=false;}}
 ensure(generator);
}
unsafe fn requested_generator(collection:usize,entity:u32)->Option<usize>{
 if collection==0||!cage_map(read_i32(collection+0x10)as u32){return None;}
 let count=read_i32(collection+0xd0);let begin=read_ptr(collection+0xd8);
 if !(1..=4096).contains(&count)||begin==0{return None;}
 for index in 0..count as usize{
  let generator=read_ptr(begin+index*8);
  if generator!=0&&read_i32(generator+0x210)as u32==entity{return Some(generator);}
 }
 None
}
unsafe extern "system" fn request(collection:usize,entity:*const u32)->u8{
 if !entity.is_null(){if let Some(generator)=requested_generator(collection,ptr::read_unaligned(entity)){request_actor(generator);}}
 let original:unsafe extern "system" fn(usize,*const u32)->u8=std::mem::transmute(REQUEST.load(Ordering::Acquire));
 original(collection,entity)
}
unsafe extern "system" fn invoke(generator:usize){
 request_actor(generator);
 let original:unsafe extern "system" fn(usize)=std::mem::transmute(INVOKE.load(Ordering::Acquire));original(generator);
}
unsafe extern "system" fn select(generator:usize,frame:usize){
 ensure(generator);
 let original:unsafe extern "system" fn(usize,usize)=std::mem::transmute(SELECT.load(Ordering::Acquire));original(generator,frame);
}

pub unsafe fn prepare(base:usize)->Result<Vec<(usize,hook::PreparedHook)>,String>{
 let calls:&[(usize,&[u8])]=&[
  (0x4ddc90,&[0x8b,0x81,0x10,0x02,0x00,0x00,0x89,0x02,0x48,0x8b,0xc2,0xc3]),
  (0x518f30,&[0x48,0x89,0x5c,0x24,0x18,0x56,0x48,0x83,0xec,0x20]),
  (0xe35620,&[0x48,0x89,0x4c,0x24,0x08,0x53,0x48,0x83,0xec,0x30]),
  (0xe35690,&[0x48,0x89,0x4c,0x24,0x08,0x48,0x83,0xec,0x28]),
  (0xe338a0,&[0x48,0x89,0x4c,0x24,0x08,0x53,0x48,0x83,0xec,0x20]),
  (0xe33bd0,&[0x48,0x89,0x4c,0x24,0x08,0x53,0x48,0x83,0xec,0x20]),
  (0xe35940,&[0x48,0x83,0xec,0x28,0x48,0x83,0x79,0x18,0x00]),
  (0x4af970,&[0x44,0x8b,0x51,0x10,0x4c,0x8b,0xc2,0x4c,0x8b,0xc9]),
  (0x3e4a80,&[0x48,0x8d,0x05,0xe1,0x7a,0x82,0x02,0x48,0x89,0x01]),
  (0x3e4a90,&[0x48,0x8d,0x05,0xd1,0x7a,0x82,0x02,0x48,0x89,0x01,0xc3]),
  (0x3e5ba0,&[0x48,0x8b,0xc4,0x48,0x89,0x68,0x18,0x48,0x89,0x70,0x20]),
  (0x3e5cc0,&[0x48,0x89,0x5c,0x24,0x08,0x55,0x56,0x57]),
  (0x4f8610,&[0xe9,0x42,0x95,0x60,0x00]),
  (0x4ab700,&[0x48,0x85,0xd2,0x0f,0x84,0x5a,0x02,0x00,0x00]),
  (0x3d7930,&[0x40,0x53,0x48,0x83,0xec,0x20,0x48,0x8b,0x41,0x18]),
  (0x3d63d0,&[0x48,0x89,0x5c,0x24,0x08,0x57,0x48,0x83,0xec,0x20]),
  (0x37f980,&[0x48,0x89,0x5c,0x24,0x10,0x57,0x48,0x83,0xec,0x40]),
  (0xe3d530,&[0x83,0xfa,0x1b,0x77,0x0e,0x48,0x63,0xc2,0x48,0x03]),
  (0xe3d550,&[0x4c,0x8b,0xc9,0x85,0xd2,0x78,0x2a,0x83,0xfa,0x1c]),
  (0x3d7660,&[0x4c,0x8b,0x51,0x20,0x33,0xc0,0x48,0x85,0xd2,0x74]),
  (0x3d1280,&[0x48,0x8b,0x81,0x08,0x03,0x00,0x00,0x48,0x89,0x02]),
  (0x3d75f0,&[0x48,0x89,0x91,0x08,0x03,0x00,0x00,0xc3]),
  (0x3d0e60,&[0x40,0x53,0x48,0x83,0xec,0x20,0x48,0x8b,0x49,0x60]),
  (0x3d75b0,&[0x40,0x53,0x48,0x83,0xec,0x20,0x48,0x8b,0xd9,0x48]),
  (0x3ceb20,&[0x40,0x53,0x48,0x83,0xec,0x20,0x48,0x8b,0x49,0x60]),
  (0x3d5ed0,&[0x48,0x8b,0x49,0x60,0xe9,0xe7,0x6d,0xfc,0xff]),
  (0x4819b0,&[0x88,0x91,0xa1,0x01,0x00,0x00,0xc3])
 ];
 for &(rva,signature)in calls{
  if std::slice::from_raw_parts((base+rva)as*const u8,signature.len())!=signature{return Err(format!("Cage demand native ABI signature mismatch at {rva:#x}"));}
 }
 let targets:[(usize,*const u8,&AtomicPtr<u8>,&[u8]);8]=[
  (0x528330,request as*const u8,&REQUEST,&[0x48,0x89,0x5c,0x24,0x10,0x48,0x89,0x6c,0x24,0x20]),
  (0x529530,preload as*const u8,&PRELOAD,&[0x48,0x89,0x5c,0x24,0x10,0x48,0x89,0x6c,0x24,0x18]),
  (0x4ad480,batch as*const u8,&BATCH,&[0x40,0x55,0x53,0x56,0x57,0x41,0x54]),
  (0x519d20,gate as*const u8,&GATE,&[0x48,0x89,0x5c,0x24,0x08,0x57,0x48,0x83,0xec,0x30]),
  (0x4df850,init as*const u8,&INIT,&[0x40,0x55,0x53,0x56,0x57,0x41,0x54]),
  (0x4dcef0,destroy as*const u8,&DESTROY,&[0x48,0x89,0x5c,0x24,0x10,0x48,0x89,0x4c,0x24,0x08]),
  (0x4de4d0,invoke as*const u8,&INVOKE,&[0x40,0x53,0x48,0x83,0xec,0x20]),
  (0x4ded70,select as*const u8,&SELECT,&[0x48,0x89,0x5c,0x24,0x08,0x48,0x89,0x6c,0x24,0x10])
 ];
 for(rva,_,_,signature)in targets{
  if std::slice::from_raw_parts((base+rva)as*const u8,signature.len())!=signature{return Err(format!("Cage demand signature mismatch at {rva:#x}"));}
 }
 let mut prepared=Vec::new();
 for(rva,detour,original,_)in targets{
  let value=hook::prepare((base+rva)as*mut u8,detour,64)?;original.store(value.trampoline,Ordering::Release);prepared.push((rva,value));
 }
 crate::nlog_info!("CageDemand prepared: selected-only native actors; native map-unload ownership; experimental validation build");
 Ok(prepared)
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn demand_request_resolves_only_exact_generator_in_cage_map(){unsafe{
  let mut collection=[0usize;28];let address=collection.as_mut_ptr()as usize;
  let mut first=[0usize;67];let mut second=[0usize;67];
  let first_address=first.as_mut_ptr()as usize;let second_address=second.as_mut_ptr()as usize;
  let generators=[0,first_address,second_address];
  ptr::write((first_address+0x210)as*mut u32,146603800);
  ptr::write((second_address+0x210)as*mut u32,146603801);
  ptr::write((address+0x10)as*mut u32,0x2e3c0605);
  ptr::write((address+0xd0)as*mut i32,3);collection[0xd8/8]=generators.as_ptr()as usize;
  assert_eq!(requested_generator(address,146603800),Some(first_address));
  assert_eq!(requested_generator(address,146603801),Some(second_address));
  assert_eq!(requested_generator(address,146603802),None);
  ptr::write((address+0x10)as*mut u32,0x340a0605);assert_eq!(requested_generator(address,146603800),None);
  ptr::write((address+0x10)as*mut u32,0x2e3c0605);
  for count in [-1,0,4097]{ptr::write((address+0xd0)as*mut i32,count);assert_eq!(requested_generator(address,146603800),None);}
  assert_eq!(requested_generator(0,146603800),None);
 }}
 #[test]fn reward_tree_filters_instance_and_custom_group_without_mutation(){unsafe{
  let map=0x2e461a07u32;
  let mut manager=[0usize;3];let mut head=[0usize;6];
  let mut nodes=[[0usize;6];4];let mut rewards=[[0usize;5];4];
  let head_address=head.as_mut_ptr()as usize;
  manager[1]=head_address;manager[2]=4;head[1]=nodes.as_ptr()as usize;
  ptr::write((head_address+0x19)as*mut u8,1);
  for index in 0..4{
   let node=nodes[index].as_mut_ptr()as usize;let reward=rewards[index].as_mut_ptr()as usize;
   nodes[index][0]=head_address;nodes[index][2]=if index<3{nodes[index+1].as_ptr()as usize}else{head_address};nodes[index][5]=reward;
   let instance=if index==1{map+1}else{map};
   ptr::write((node+0x20)as*mut u32,instance);ptr::write((reward+0x18)as*mut u32,instance);
   ptr::write((reward+0x20)as*mut u32,if index==2{146705936}else{146705937+index as u32});
  }
  let before=(manager,head,nodes,rewards);
  let mut selected=reward_records(manager.as_ptr()as usize,map);selected.sort_unstable();
  let mut expected=vec![rewards[0].as_ptr()as usize,rewards[3].as_ptr()as usize];expected.sort_unstable();
  assert_eq!(selected,expected);assert_eq!((manager,head,nodes,rewards),before);
  assert!(reward_records(manager.as_ptr()as usize,map+2).is_empty());
  nodes[3][2]=nodes[0].as_ptr()as usize;assert!(reward_records(manager.as_ptr()as usize,map).is_empty());
  manager[2]=4097;assert!(reward_records(manager.as_ptr()as usize,map).is_empty());
 }}
 #[test]fn filter_only_custom_gaol_actors(){
  for block in [50,60,70,80]{assert!(cage_map(0x2e001a03|(block<<16)));}
  for map in [0x340a1909,0x2e421b32,0x3c321a03]{assert!(!cage_map(map));}
  for npc in [95300000,95300083]{assert!(cage_npc(npc));}
  for npc in [48000010,95300084,95299999]{assert!(!cage_npc(npc));}
  assert!(belongs(0x2e321a03,0x2e321a0311000010));
  assert!(!belongs(0x2e321a03,0x2e321a0411000010));
  assert!(!belongs(0x2e321a03,0x2e321a0310000010));
 }
 #[test]fn nested_creation_context_restores_parent(){
  assert_eq!(CONTEXT.with(Cell::get),0);
  {let _outer=ContextScope::enter(123);{let _inner=ContextScope::enter(456);assert_eq!(CONTEXT.with(Cell::get),456);}assert_eq!(CONTEXT.with(Cell::get),123);}
  assert_eq!(CONTEXT.with(Cell::get),0);
 }
 #[test]fn binding_is_single_and_capacity_checked(){unsafe{
  let mut generator=[0usize;0x260/8];let mut vector=[0usize;32];let address=generator.as_mut_ptr()as usize;
  generator[0xd0/8]=vector.as_mut_ptr()as usize;generator[0xd8/8]=generator[0xd0/8];generator[0xe0/8]=generator[0xd0/8]+256;
  assert!(bind(address,123));assert_eq!(vector[0],123);assert!(!bind(address,456));assert_eq!(vector[0],123);
  ptr::write((address+0xd8)as*mut usize,read_ptr(address+0xd0));ptr::write((address+0xe0)as*mut usize,read_ptr(address+0xd0));assert!(!bind(address,456));
 }}
}
