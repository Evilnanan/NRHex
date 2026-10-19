use std::{cell::Cell,ptr,sync::{Mutex,OnceLock,atomic::{AtomicPtr,AtomicUsize,Ordering}}};
use crate::{hook,read_ptr,read_i32};

static BATCH:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static GATE:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static SCENARIO:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static CREATE:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static GENERATOR:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static TRANSFER:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static FREE_SLOT:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static LINES:AtomicUsize=AtomicUsize::new(0);
static QUEUE:OnceLock<Mutex<Vec<String>>>=OnceLock::new();

#[derive(Clone,Copy,Default,Debug)]
struct Scope {
 active:bool,
 gate_calls:u32,
 gate_rejected:u32,
 scenario_rejected:u32,
 creates:u32,
 null_creates:u32,
 first_rejected:u64,
 slot_calls:u32,
 slot_misses:u32,
}
thread_local!{static SCOPE:Cell<Scope>=Cell::new(Scope::default());}

fn record_gate(handle:u64,result:u8,scenario:bool){
 SCOPE.with(|cell|{
  let mut scope=cell.get();
  if !scope.active{return;}
  if scenario{
   scope.scenario_rejected+=u32::from(result==0);
  }else{
   scope.gate_calls+=1;
   scope.gate_rejected+=u32::from(result==0);
  }
  if result==0&&scope.first_rejected==0{scope.first_rejected=handle;}
  cell.set(scope);
 });
}

fn relevant(map:u32)->bool{matches!(map>>24,46|52)}
fn log_allowed()->bool{LINES.fetch_add(1,Ordering::Relaxed)<3000}
fn enqueue(message:String){if let Ok(mut queue)=QUEUE.get_or_init(Default::default).lock(){queue.push(message);}}

unsafe fn occupancy(collection:usize)->(i32,usize){
 let capacity=read_i32(collection+0x10);
 let entries=read_ptr(collection+0x18);
 if !(0..=4096).contains(&capacity)||entries==0{return(capacity,0);}
 (capacity,(0..capacity as usize).filter(|index|read_ptr(entries+index*16)!=0).count())
}

unsafe extern "system" fn batch(collection:usize,context:usize,budget:usize,state:usize)->u8{
 let original:unsafe extern "system" fn(usize,usize,usize,usize)->u8=std::mem::transmute(BATCH.load(Ordering::Acquire));
 let map=read_i32(context+0x34)as u32;
 let watched=relevant(map);
 let previous=SCOPE.with(|cell|cell.replace(Scope{active:watched,..Default::default()}));
 let before=if watched{occupancy(collection)}else{(0,0)};
 let result=original(collection,context,budget,state);
 let observed=SCOPE.with(|cell|cell.replace(previous));
 if watched&&log_allowed(){
  enqueue(format!("CageProbe batch map={map:#x} context={context:#x} collection={collection:#x} result={result} before={before:?} after={:?} scope={observed:?}",occupancy(collection)));
 }
 result
}

unsafe extern "system" fn gate(world:usize,handle:u64)->u8{
 let original:unsafe extern "system" fn(usize,u64)->u8=std::mem::transmute(GATE.load(Ordering::Acquire));
 let result=original(world,handle);
 record_gate(handle,result,false);
 result
}

unsafe extern "system" fn scenario(world:usize,handle:u64)->u8{
 let original:unsafe extern "system" fn(usize,u64)->u8=std::mem::transmute(SCENARIO.load(Ordering::Acquire));
 let result=original(world,handle);
 record_gate(handle,result,true);
 result
}

type Create=unsafe extern "system" fn(usize,usize,usize,usize,usize,u8,u8,u8)->usize;
unsafe extern "system" fn create(factory:usize,existing:usize,entry:usize,map:usize,part:usize,mode:u8,flag:u8,other:u8)->usize{
 let original:Create=std::mem::transmute(CREATE.load(Ordering::Acquire));
 let result=original(factory,existing,entry,map,part,mode,flag,other);
 SCOPE.with(|cell|{
  let mut scope=cell.get();
  if scope.active{scope.creates+=1;scope.null_creates+=u32::from(result==0);cell.set(scope);}
 });
 if map>0x10000{
  let identity=read_i32(map)as u32;
  if relevant(identity)&&log_allowed(){
   let handle=if result>0x10000{read_ptr(result+8)}else{0};
   enqueue(format!("CageProbe create map={identity:#x} existing={existing:#x} result={result:#x} handle={handle:#x} part={part:#x} entry={entry:#x} mode={mode}/{flag}/{other}"));
  }
 }
 result
}

unsafe extern "system" fn generator(generator:usize,map_data:usize,index:i32,context:usize){
 let original:unsafe extern "system" fn(usize,usize,i32,usize)=std::mem::transmute(GENERATOR.load(Ordering::Acquire));
 original(generator,map_data,index,context);
 let map=read_i32(context+8)as u32;
 if relevant(map)&&log_allowed(){
  let begin=read_ptr(generator+0xd0);
  let end=read_ptr(generator+0xd8);
  let count=if end>=begin{(end-begin)/8}else{usize::MAX};
  enqueue(format!("CageProbe generator map={map:#x} index={index} bound={count} entity={} generator={generator:#x}",read_i32(generator+0x210)));
 }
}

unsafe fn pending(entries:usize,count:i32)->Vec<u64>{
 if entries==0||!(0..=4096).contains(&count){return Vec::new();}
 (0..count as usize).filter_map(|index|{
  let actor=read_ptr(entries+index*16);
  if actor==0{None}else{Some(read_ptr(actor+8)as u64)}
 }).collect()
}

unsafe extern "system" fn transfer(collection:usize,entries:usize,count:i32,context:usize,state:usize){
 let original:unsafe extern "system" fn(usize,usize,i32,usize,usize)=std::mem::transmute(TRANSFER.load(Ordering::Acquire));
 let map=read_i32(context+0x34)as u32;
 let watched=relevant(map);
 let previous=SCOPE.with(|cell|cell.replace(Scope{active:watched,..Default::default()}));
 let before=if watched{occupancy(collection)}else{(0,0)};
 let incoming=if watched{pending(entries,count)}else{Vec::new()};
 original(collection,entries,count,context,state);
 let observed=SCOPE.with(|cell|cell.replace(previous));
 if watched&&log_allowed(){
  let remaining=pending(entries,count);
  enqueue(format!("CageProbe transfer map={map:#x} context={context:#x} collection={collection:#x} count={count} before={before:?} after={:?} incoming={} remaining={} scope={observed:?} remaining_handles={remaining:x?}",occupancy(collection),incoming.len(),remaining.len()));
 }
}

unsafe extern "system" fn free_slot(collection:usize,output:usize)->usize{
 let original:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(FREE_SLOT.load(Ordering::Acquire));
 let result=original(collection,output);
 SCOPE.with(|cell|{
  let mut scope=cell.get();
  if scope.active{
   scope.slot_calls+=1;
   scope.slot_misses+=u32::from(read_ptr(output)==0);
   cell.set(scope);
  }
 });
 result
}

pub unsafe fn prepare(base:usize)->Result<Vec<(usize,hook::PreparedHook)>,String>{
 let targets:[(usize,*const u8,&AtomicPtr<u8>,&[u8]);7]=[
  (0x4ad480,batch as*const u8,&BATCH,&[0x40,0x55,0x53,0x56,0x57,0x41,0x54,0x41,0x55,0x41,0x56,0x41,0x57]),
  (0x519d20,gate as*const u8,&GATE,&[0x48,0x89,0x5c,0x24,0x08,0x57,0x48,0x83,0xec,0x30]),
  (0x519e40,scenario as*const u8,&SCENARIO,&[0x48,0x8b,0x0d,0x51,0x9c,0xf1,0x03]),
  (0x3e5ba0,create as*const u8,&CREATE,&[0x48,0x8b,0xc4,0x48,0x89,0x68,0x18,0x48,0x89,0x70,0x20]),
  (0x4df850,generator as*const u8,&GENERATOR,&[0x40,0x55,0x53,0x56,0x57,0x41,0x54,0x41,0x55,0x41,0x56,0x41,0x57]),
  (0x4f86f0,transfer as*const u8,&TRANSFER,&[0x48,0x89,0x5c,0x24,0x08,0x48,0x89,0x54,0x24,0x10]),
  (0x4af970,free_slot as*const u8,&FREE_SLOT,&[0x44,0x8b,0x51,0x10,0x4c,0x8b,0xc2,0x4c,0x8b,0xc9])
 ];
 for (rva,_,_,signature) in targets{
  if std::slice::from_raw_parts((base+rva)as*const u8,signature.len())!=signature{return Err(format!("Cage probe signature mismatch at {rva:#x}"));}
 }
 let mut prepared=Vec::new();
 for(rva,detour,original,_)in targets{
  let value=hook::prepare((base+rva)as*mut u8,detour,64)?;
  original.store(value.trampoline,Ordering::Release);
  prepared.push((rva,value));
 }
 std::thread::Builder::new().name("cage-probe-log".into()).spawn(||loop{
  std::thread::sleep(std::time::Duration::from_millis(250));
  let messages=QUEUE.get_or_init(Default::default).lock().map(|mut queue|std::mem::take(&mut *queue)).unwrap_or_default();
  for message in messages{crate::nlog_info!("{message}");}
 }).map_err(|error|format!("Cage probe logger: {error}"))?;
 crate::nlog_info!("CageProbe prepared: observe-only batch/gates/create/generator/transfer/free-slot; bounded asynchronous logging; no spawn-rule changes");
 Ok(prepared)
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn scope_records_rejections_without_modifying_unwatched_context(){
  SCOPE.with(|cell|cell.set(Scope::default()));
  record_gate(4,0,false);
  assert_eq!(SCOPE.with(|cell|cell.get().gate_calls),0);
  SCOPE.with(|cell|cell.set(Scope{active:true,..Default::default()}));
  record_gate(5,1,false);record_gate(6,0,false);record_gate(7,0,true);
  let scope=SCOPE.with(|cell|cell.replace(Scope::default()));
  assert_eq!((scope.gate_calls,scope.gate_rejected,scope.scenario_rejected,scope.first_rejected),(2,1,1,6));
  assert!(relevant(0x2e321a03));assert!(relevant(0x340a1909));assert!(!relevant(0x3c2b2600));
 }
 #[test]fn create_forwards_all_eight_arguments_and_return(){unsafe{
  unsafe extern "system" fn fixture(factory:usize,existing:usize,entry:usize,map:usize,part:usize,mode:u8,flag:u8,other:u8)->usize{
   assert_eq!((factory,existing,entry,map,part,mode,flag,other),(11,22,33,44,55,66,77,88));99
  }
  let previous=CREATE.swap(fixture as*mut u8,Ordering::AcqRel);
  assert_eq!(create(11,22,33,44,55,66,77,88),99);
  CREATE.store(previous,Ordering::Release);
 }}
 #[test]fn transfer_and_slot_forward_arguments_and_preserve_scope(){unsafe{
  unsafe extern "system" fn fixture(collection:usize,entries:usize,count:i32,context:usize,state:usize){
   assert_ne!(collection,0);assert_eq!((entries,count,state),(0,0,77));
   assert_eq!(read_i32(context+0x34)as u32,0x340a1909);
  }
  unsafe extern "system" fn slot_fixture(collection:usize,output:usize)->usize{
   assert_eq!(collection,19);(output as*mut usize).write(0);output
  }
  let mut context=[0u64;16];
  let collection=[0u64;8];
  let context_ptr=context.as_mut_ptr()as usize;
  ((context_ptr+0x34)as*mut u32).write(0x340a1909);
  let previous=TRANSFER.swap(fixture as*mut u8,Ordering::AcqRel);
  SCOPE.with(|cell|cell.set(Scope{active:true,slot_calls:3,..Default::default()}));
  transfer(collection.as_ptr()as usize,0,0,context_ptr,77);
  assert_eq!(SCOPE.with(|cell|cell.get().slot_calls),3);
  TRANSFER.store(previous,Ordering::Release);
  let previous=FREE_SLOT.swap(slot_fixture as*mut u8,Ordering::AcqRel);
  let mut output=[usize::MAX;2];
  let output_ptr=output.as_mut_ptr()as usize;
  assert_eq!(free_slot(19,output_ptr),output_ptr);
  let observed=SCOPE.with(|cell|cell.replace(Scope::default()));
  assert_eq!((observed.slot_calls,observed.slot_misses),(4,1));
  assert_eq!(output[0],0);
  FREE_SLOT.store(previous,Ordering::Release);
 }}
}
