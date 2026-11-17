//! Ram visual metadata travels inside the native typed graph-variable list.
//! No new message type, tail bytes, peer physics, or damage simulation.
use std::{ptr,sync::{Mutex,atomic::{AtomicPtr,AtomicU32,AtomicUsize,Ordering}},time::Instant};
use crate::{hook,read_ptr,read_i32};

const SIZE:usize=0x17c;
const MAGIC:u32=0x4d415201;
const TTL:f32=0.5;
const NAMES:[&[u8];6]=[b"MARamSyncV1\0",b"MARamGeneration\0",b"MARamCycle\0",b"MARamSample\0",b"MARamRate\0",b"MARamPhase\0"];
const ROLL:&[u8]=b"a713_040010_hkx_AutoSet_00\0";
const ROLL_ALT:&[u8]=b"a713_040010_hkx_AutoSet_01\0";
const ACTION:&[u8]=b"SwordArtsOneshotComboEnd_CMSG\0";
static BASE:AtomicUsize=AtomicUsize::new(0);
static CAPTURE:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static APPLY:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static SAMPLE:AtomicU32=AtomicU32::new(1);
static REPORTED:AtomicUsize=AtomicUsize::new(0);

#[derive(Clone,Copy,Debug,PartialEq)]struct Identity{actor:usize,handle:u64,graph:usize,instance:usize,context:usize}
#[derive(Clone,Copy)]struct Binding{id:Identity,ids:[i32;6],local:bool}
#[derive(Clone,Copy,Debug,PartialEq)]struct Frame{generation:u32,cycle:u32,sample:u32,rate:f32,time:f32}
impl Frame{
 fn rolling(self)->bool{self.time>=0.}
 fn words(self)->[u32;6]{[MAGIC,self.generation,self.cycle,self.sample,self.rate.to_bits(),self.time.to_bits()]}
 fn from_words(w:[u32;6])->Option<Self>{
  let f=Self{generation:w[1],cycle:w[2],sample:w[3],rate:f32::from_bits(w[4]),time:f32::from_bits(w[5])};
  (w[0]==MAGIC&&f.rate.is_finite()&&(0.25..=8.).contains(&f.rate)&&f.time.is_finite()
   &&(f.time== -1.||(0. ..=4.).contains(&f.time))&&(!f.rolling()||f.generation!=0)).then_some(f)
 }
}
fn counts(s:&[u8])->Option<usize>{(s.len()>=SIZE&&s[0x14c]<=16&&s[0x14d]<=4&&s[0x14e]<=32).then(||s[0x14e]as usize)}
fn word(s:&[u8],p:usize)->u32{u32::from_le_bytes(s[p..p+4].try_into().unwrap())}
fn put(s:&mut[u8],p:usize,v:u32){s[p..p+4].copy_from_slice(&v.to_le_bytes());}
fn slots(s:&[u8],ids:[i32;6])->Option<([Option<usize>;6],usize)>{
 let n=counts(s)?;if ids.iter().any(|v|*v<0)||(0..6).any(|i|ids[..i].contains(&ids[i])){return None;}
 let mut found=[None;6];
 for i in 0..n{if let Some(k)=ids.iter().position(|id|*id as u32==word(s,0x40+i*8)){
  if found[k].replace(i).is_some(){return None;}
 }}Some((found,n))
}
fn insert(s:&mut[u8],ids:[i32;6],frame:Frame)->bool{
 let Some((found,mut n))=slots(s,ids)else{return false;};
 if n+found.iter().filter(|v|v.is_none()).count()>32{return false;}
 for (k,value)in frame.words().into_iter().enumerate(){let i=found[k].unwrap_or_else(||{let i=n;n+=1;i});put(s,0x40+i*8,ids[k]as u32);put(s,0x44+i*8,value);}
 s[0x14e]=n as u8;true
}
fn extract(s:&[u8],ids:[i32;6])->Option<Frame>{
 let (found,_)=slots(s,ids)?;let mut w=[0;6];for k in 0..6{w[k]=word(s,0x44+found[k]?*8);}Frame::from_words(w)
}
#[derive(Clone,Copy,Debug,PartialEq)]enum Policy{Native,NoRepeat,DropRoll}
fn newer(a:u32,b:u32)->bool{let d=a.wrapping_sub(b);d!=0&&d<0x80000000}
#[derive(Clone,Copy)]struct Peer{id:Identity,frame:Frame,seen:Instant}
impl Peer{
 fn receive(&mut self,f:Frame,now:Instant,rolling:bool)->Policy{
  if !newer(f.sample,self.frame.sample){return Policy::DropRoll;}
  let same=f.rolling()&&self.frame.rolling()&&f.generation==self.frame.generation&&f.cycle==self.frame.cycle;
  self.frame=f;self.seen=now;
  if same&&rolling{Policy::NoRepeat}else{Policy::Native}
 }
 fn rate(&self,now:Instant,time:f32)->Option<f32>{
  let age=now.saturating_duration_since(self.seen).as_secs_f32();
  if age>TTL||!self.frame.rolling(){return None;}
  // Only small same-cycle corrections. Never jump the clip/TAE time, run
  // peer motor code, or wait for a backlog of old animation cycles.
  let error=self.frame.time+age*self.frame.rate-time;
  let correction=if age<0.15&&error.is_finite()&&error.abs()<0.12{(error*0.6).clamp(-0.08,0.08)}else{0.};
  Some((self.frame.rate*(1.+correction)).clamp(0.25,8.))
 }
}
struct State{scene:usize,seen:Option<Instant>,bindings:Vec<Binding>,peers:Vec<Peer>}
static STATE:Mutex<State>=Mutex::new(State{scene:0,seen:None,bindings:Vec::new(),peers:Vec::new()});

unsafe fn equal_ascii(p:usize,name:&[u8])->bool{p!=0&&name.iter().enumerate().all(|(i,v)|ptr::read((p+i)as*const u8)==*v)}
unsafe fn equal_wide(p:usize,name:&str)->bool{
 p!=0&&name.encode_utf16().chain(std::iter::once(0)).enumerate().all(|(i,v)|ptr::read_unaligned((p+i*2)as*const u16)==v)
}
// Match only the native all-body loop event. Never filter a whole snapshot,
// a generic attack state, recovery, or interruption.
unsafe fn roll_event(id:u16,base:usize)->bool{
 let registry=read_ptr(base+0x3c1d768);if registry==0{return false;}
 let table=read_ptr(registry+0x10);if table==0{return false;}
 let count=read_i32(table+0x120);let array=read_ptr(table+0x110);
 if !(1..=32768).contains(&count)||id as i32>=count||array==0{return false;}
 let s=array+id as usize*0x130+0x108;
 let p=if read_ptr(s+24)>7{read_ptr(s)}else{s};
 equal_wide(p,"SwordArtsOneShotComboEnd")||equal_wide(p,"SwordArtsOneshotComboEnd")
}
fn filter_events(s:&mut[u8],policy:Policy,mut is_roll:impl FnMut(u16)->bool){
 if counts(s).is_none()||policy==Policy::Native{return;}
 let n=s[0x14c]as usize;let mut out=0;
 for i in 0..n{let p=i*4;let id=u16::from_le_bytes([s[p],s[p+1]])&0x7fff;
  let matched=is_roll(id);if matched&&policy==Policy::DropRoll{continue;}
  let mut entry:[u8;4]=s[p..p+4].try_into().unwrap();if matched{entry[2]=0;}
  s[out*4..out*4+4].copy_from_slice(&entry);out+=1;
 }s[0x14c]=out as u8;
}
unsafe fn identity(actor:usize,base:usize)->Option<Identity>{
 let graph=crate::ram_stagger::graph(actor,base)?;let anim=read_ptr(read_ptr(actor+0x1b8)+0x28);
 Some(Identity{actor,handle:read_ptr(actor+8)as u64,graph,instance:read_ptr(anim+0x10),context:anim+0x1800})
}
unsafe fn ready(actor:usize,base:usize)->bool{
 let stats=crate::native_actor::stats(actor);
 stats!=0&&!crate::native_actor::incapacitated(actor,stats)&&crate::has_effect(actor,crate::lightning_ram::OWNER,base)
 &&!crate::has_effect(actor,crate::reward_cycle_runtime::RESET,base)
}
unsafe fn variable_ids(id:Identity,base:usize)->Option<[i32;6]>{
 // D57410 resolves the global name and verifies this graph has a local map.
 // D570A0 then verifies the mapped type; local array indices are not wire IDs.
 if read_ptr(base+0x3c1d4a8)==0{return None;}
 let character=read_ptr(id.instance+0x30);let mut ids=[-1;6];
 let lookup:unsafe extern "system" fn(*mut i32,*const u8,usize)->*mut i32=std::mem::transmute(base+0xd57410);
 let kind:unsafe extern "system" fn(usize,i32)->i32=std::mem::transmute(base+0xd570a0);
 for k in 0..6{lookup(&mut ids[k],NAMES[k].as_ptr(),character);if ids[k]<0||kind(character,ids[k])!=if k<4{3}else{4}{return None;}}
 Some(ids)
}
unsafe fn clip_time(graph:usize,base:usize)->Option<f32>{
 let query:unsafe extern "system" fn(usize,*const u8)->u8=std::mem::transmute(base+crate::ram_stagger::NODE_RVA);
 if query(graph,ACTION.as_ptr())==0{return None;}
 // Same active-node traversal as native hkbIsNodeActive (16B03C0).
 let list=read_ptr(graph+0xe0);if list==0{return None;}let array=read_ptr(list);let count=read_i32(list+8);
 if array==0||!(1..=2048).contains(&count){return None;}
 for i in 0..count as usize{let info=read_ptr(array+i*8);if info==0{continue;}let node=read_ptr(info+8);if node==0{continue;}
  let name=read_ptr(node+0x48)&!1;
  if equal_ascii(name,ROLL)||equal_ascii(name,ROLL_ALT){
   // hkbClipGenerator localTime, from the target TAG0 TYPE schema.
   let time=ptr::read_unaligned((node+0x140)as*const f32);
   if time.is_finite()&&(0. ..=4.).contains(&time){return Some(time);}
  }
 }None
}
fn binding(actor:usize,handle:u64)->Option<Binding>{
 let s=STATE.try_lock().ok()?;
 if Instant::now().saturating_duration_since(s.seen?).as_secs_f32()>0.25{return None;}
 s.bindings.iter().find(|b|b.id.actor==actor&&b.id.handle==handle).copied()
}
unsafe fn module_binding(module:usize,base:usize)->Option<Binding>{
 if module==0{return None;}let actor=read_ptr(module+8);if actor==0{return None;}
 let b=binding(actor,read_ptr(actor+8)as u64)?;
 (identity(actor,base)==Some(b.id)).then_some(b)
}
pub unsafe fn update(local:usize,base:usize){
 let scene=read_ptr(base+0x3c131c8);let world=read_ptr(base+crate::WORLD_RVA);let now=Instant::now();
 let previous=STATE.lock().map(|s|if s.scene==scene{s.bindings.clone()}else{Vec::new()}).unwrap_or_default();
 let mut bindings=Vec::with_capacity(3);
 if local!=0&&world!=0{
  let get:unsafe extern "system" fn(usize,i32)->usize=std::mem::transmute(base+0x5197e0);
  for actor in std::iter::once(local).chain((1..=3).map(|n|get(world,n))){
   if actor==0||bindings.iter().any(|b:&Binding|b.id.actor==actor){continue;}
   let Some(id)=identity(actor,base)else{continue;};
   if actor==local&&REPORTED.load(Ordering::Relaxed)!=id.graph{
    let ids=variable_ids(id,base);
    crate::nlog_info!("RamSync graph={:x} typed variable IDs={ids:?}",id.graph);
    REPORTED.store(id.graph,Ordering::Relaxed);
   }
   if !ready(actor,base){continue;}
   let ids=previous.iter().find(|b|b.id==id).map(|b|b.ids).or_else(||variable_ids(id,base));
   if let Some(ids)=ids{bindings.push(Binding{id,ids,local:actor==local});}
  }
 }
 if let Ok(mut s)=STATE.lock(){
  if s.scene!=scene{s.peers.clear();}s.scene=scene;s.seen=Some(now);
  s.peers.retain(|p|bindings.iter().any(|b|!b.local&&b.id==p.id));s.bindings=bindings;
 }
}
pub unsafe fn speed(context:usize,base:usize)->Option<f32>{
 if context==0{return None;}let actor=read_ptr(context+8);if actor==0{return None;}
 let b=binding(actor,read_ptr(actor+8)as u64)?;
 if b.local||b.id.context!=context||identity(actor,base)!=Some(b.id)||!ready(actor,base){return None;}
 let time=clip_time(b.id.graph,base)?;
 let s=STATE.try_lock().ok()?;s.peers.iter().find(|p|p.id==b.id)?.rate(Instant::now(),time)
}
unsafe extern "system" fn capture(module:usize,p:usize){
 let original:unsafe extern "system" fn(usize,usize)=std::mem::transmute(CAPTURE.load(Ordering::Acquire));original(module,p);
 if p==0{return;}let base=BASE.load(Ordering::Acquire);let Some(b)=module_binding(module,base).filter(|b|b.local)else{return;};
 let Some((generation,cycle,rate,rolling))=crate::lightning_ram::sync_frame(b.id.actor)else{return;};
 let time=if rolling{clip_time(b.id.graph,base).unwrap_or(-1.)}else{-1.};
 let frame=Frame{generation:generation as u32,cycle,sample:SAMPLE.fetch_add(1,Ordering::Relaxed),rate,time};
 if Frame::from_words(frame.words()).is_some(){insert(std::slice::from_raw_parts_mut(p as*mut u8,SIZE),b.ids,frame);}
 #[cfg(feature="ram-sync-probe")]crate::ram_sync_probe::capture(module,p);
}
unsafe extern "system" fn apply(module:usize,p:usize){
 let original:unsafe extern "system" fn(usize,usize)=std::mem::transmute(APPLY.load(Ordering::Acquire));
 #[cfg(feature="ram-sync-probe")]crate::ram_sync_probe::apply(module,p);
 let base=BASE.load(Ordering::Acquire);
 let Some(b)=module_binding(module,base).filter(|b|!b.local&&p!=0)else{original(module,p);return;};
 let bytes=std::slice::from_raw_parts(p as*const u8,SIZE);
 let Some(frame)=extract(bytes,b.ids)else{original(module,p);return;};
 let rolling=clip_time(b.id.graph,base).is_some();let now=Instant::now();
 let policy=if let Ok(mut s)=STATE.try_lock(){
  if let Some(peer)=s.peers.iter_mut().find(|peer|peer.id==b.id){peer.receive(frame,now,rolling)}
  else if s.peers.len()<3{s.peers.push(Peer{id:b.id,frame,seen:now});Policy::Native}else{Policy::Native}
 }else{Policy::Native};
 // Publish the multiplier before native state events, which are applied
 // before graph variables. The snapshot copy is consumed synchronously.
 #[repr(align(16))]struct Copy([u8;SIZE]);let mut copy=Copy([0;SIZE]);copy.0.copy_from_slice(bytes);
 filter_events(&mut copy.0,policy,|id|roll_event(id,base));original(module,copy.0.as_ptr()as usize);
}
pub unsafe fn prepare(base:usize)->Result<Vec<(usize,hook::PreparedHook)>,String>{
 let targets:[(usize,*const u8,&AtomicPtr<u8>,&[u8]);2]=[
  (0x40a130,capture as*const u8,&CAPTURE,&[0x48,0x89,0x5c,0x24,0x10,0x48,0x89,0x6c,0x24,0x18,0x48,0x89,0x74,0x24,0x20,0x57]),
  (0x408db0,apply as*const u8,&APPLY,&[0x48,0x89,0x5c,0x24,0x18,0x55,0x56,0x57,0x41,0x56,0x41,0x57,0x48,0x83,0xec,0x50]),
 ];
 for(rva,_,_,sig)in targets{if std::slice::from_raw_parts((base+rva)as*const u8,sig.len())!=sig{return Err(format!("RamSync signature mismatch {rva:x}"));}}
 let mut result=Vec::new();for(rva,detour,original,_)in targets{let p=hook::prepare((base+rva)as*mut u8,detour,64)?;original.store(p.trampoline,Ordering::Release);result.push((rva,p));}
 BASE.store(base,Ordering::Release);
 crate::nlog_info!("RamSync prepared: native graph variables; requires matching named behavior variables; peer visual only");Ok(result)
}

#[cfg(test)]mod tests{
 use super::*;
 const IDS:[i32;6]=[701,702,703,704,705,706];
 fn frame(sample:u32,cycle:u32)->Frame{Frame{generation:1,cycle,sample,rate:4.,time:0.1}}
 fn id()->Identity{Identity{actor:1,handle:2,graph:3,instance:4,context:5}}
 #[test]fn roundtrip_preserves_native_fields_and_rejects_full_or_duplicate_lists(){
  for count in 0..=32{let mut s=[0xa5;SIZE];s[0x14c]=3;s[0x14d]=2;s[0x14e]=count;
   for i in 0..count as usize{put(&mut s,0x40+i*8,i as u32);put(&mut s,0x44+i*8,0x1234);}
   let before=s;let ok=insert(&mut s,IDS,frame(1,1));assert_eq!(ok,count<=26);
   if ok{assert_eq!(extract(&s,IDS),Some(frame(1,1)));assert_eq!(&s[..0x40+count as usize*8],&before[..0x40+count as usize*8]);
    assert_eq!(&s[0x140..0x14e],&before[0x140..0x14e]);assert_eq!(&s[0x14f..],&before[0x14f..]);
    assert!(insert(&mut s,IDS,frame(2,2)));assert_eq!(s[0x14e],count+6);assert_eq!(extract(&s,IDS),Some(frame(2,2)));
   }else{assert_eq!(s,before);}
  }
  let mut s=[0;SIZE];insert(&mut s,IDS,frame(1,1));s[0x14e]=7;put(&mut s,0x40+6*8,IDS[0]as u32);let before=s;
  assert!(extract(&s,IDS).is_none());assert!(!insert(&mut s,IDS,frame(2,1)));assert_eq!(s,before);
 }
 #[test]fn malformed_nan_old_schema_and_partial_metadata_fall_back(){
  let mut s=[0;SIZE];assert!(extract(&s,IDS).is_none());insert(&mut s,IDS,frame(1,1));
  for (slot,value)in [(0,0),(1,0),(4,f32::NAN.to_bits()),(4,9f32.to_bits()),(5,f32::INFINITY.to_bits()),(5,(-2f32).to_bits())]{let mut b=s;put(&mut b,0x44+slot*8,value);assert!(extract(&b,IDS).is_none());}
  for n in 0..SIZE{assert!(extract(&s[..n],IDS).is_none());}
 }
 #[test]fn same_cycle_updates_do_not_replay_but_next_cycle_and_interruptions_survive(){
  let now=Instant::now();let mut p=Peer{id:id(),frame:frame(1,1),seen:now};
  assert_eq!(p.receive(frame(2,1),now,true),Policy::NoRepeat);
  assert_eq!(p.receive(frame(3,2),now,true),Policy::Native);
  assert_eq!(p.receive(frame(2,1),now,true),Policy::DropRoll);assert_eq!(p.frame.sample,3);
  assert_eq!(p.receive(frame(4,2),now,false),Policy::Native);
  assert_eq!(p.receive(Frame{time:-1.,..frame(5,2)},now,true),Policy::Native);assert!(p.rate(now,0.).is_none());
  assert_eq!(p.receive(Frame{generation:2,..frame(6,0)},now,true),Policy::Native);
 }
 #[test]fn wraparound_and_peer_rates_are_bounded_without_position_input(){
  assert!(newer(0,u32::MAX));assert!(!newer(u32::MAX,0));assert!(!newer(2,2));
  let now=Instant::now();for rate in [0.25,1.,4.,8.]{let p=Peer{id:id(),frame:Frame{rate,..frame(1,1)},seen:now};
   assert_eq!(p.rate(now,0.1),Some(rate));assert!((0.25..=8.).contains(&p.rate(now,0.05).unwrap()));
   assert!(p.rate(now+std::time::Duration::from_secs(1),0.1).is_none());}
 }
 #[test]fn filtering_changes_only_exact_roll_entries_not_other_states_or_variables(){
  let mut s=[0;SIZE];s[0x14c]=3;for (i,id)in [123u32,456,789].iter().enumerate(){put(&mut s,i*4,*id|0x10000);}
  insert(&mut s,IDS,frame(1,1));let original=s;
  filter_events(&mut s,Policy::NoRepeat,|id|id==456);assert_eq!(s[6],0);s[6]=1;assert_eq!(s,original);
  filter_events(&mut s,Policy::DropRoll,|id|id==456);assert_eq!(s[0x14c],2);assert_eq!(word(&s,0),123|0x10000);assert_eq!(word(&s,4),789|0x10000);
  assert_eq!(&s[0x40..0x14c],&original[0x40..0x14c]);assert_eq!(&s[0x14d..],&original[0x14d..]);
 }
 #[test]#[ignore="requires NRHEX_NATIVE_EXE; execute alone"]
 fn native_transport_roundtrips_all_metadata_without_a_private_tail(){unsafe{
  use crate::win::*;
  let path=std::path::PathBuf::from(std::env::var_os("NRHEX_NATIVE_EXE").unwrap());
  let raw=std::fs::read(path).unwrap();
  use sha2::{Digest,Sha256};assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);let headers=crate::pe::Headers::parse(&raw,0).unwrap();
  let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
  for s in &headers.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
  let hooks=prepare(base).unwrap();assert_eq!(hooks.len(),2);
  let encode:unsafe extern "system" fn(*const u8,*const usize)->usize=std::mem::transmute(base+0xde58e0);
  let decode:unsafe extern "system" fn(*mut u8,*mut usize)->u8=std::mem::transmute(base+0xde42e0);
  for count in [0,1,26]{for states in [0,16]{for shorts in [0,4]{for reference in [0,1]{for rate in [0.25,1.,8.]{
   let mut s=[0u8;SIZE];s[0x14c]=states;s[0x14d]=shorts;s[0x14e]=count;s[0x168]=reference;
   for i in 0..count as usize{put(&mut s,0x40+i*8,i as u32);put(&mut s,0x44+i*8,0x12345678);}
   for i in 0..states as usize{put(&mut s,i*4,123+i as u32|0x10000);}
   let f=Frame{rate,..frame(u32::MAX,8)};assert!(insert(&mut s,IDS,f));let before=s;
   let mut wire=[0xa5u8;512];let writer=[wire.as_mut_ptr()as usize,wire.len()];let len=encode(s.as_ptr(),writer.as_ptr());
   assert_eq!(len,41+states as usize*2+shorts as usize*2+(count as usize+6)*8+reference as usize*8);
   assert!(wire[len..].iter().all(|b|*b==0xa5));assert_eq!(s,before);
   // D40890's nonempty state decoder is Arxan-split in the disk EXE.
   // Encoding covers both state counts; native decoding below deliberately
   // tests variable transport with no state entries, without emulating it.
   if states!=0{continue;}
   let mut out=[0xa5u8;SIZE+16];let mut reader=[wire.as_ptr()as usize,len,0];
   assert_eq!(decode(out.as_mut_ptr(),reader.as_mut_ptr()),1);assert_eq!(reader[0],wire.as_ptr()as usize+len);assert_eq!(reader[1],0);
   assert!(out[SIZE..].iter().all(|b|*b==0xa5));assert_eq!(extract(&out,IDS),Some(f));
   assert_eq!(&out[..states as usize*4],&s[..states as usize*4]);
   // Missing final byte is rejected by native decoding, not consumed as
   // an unvalidated private extension.
   let mut reader=[wire.as_ptr()as usize,len-1,0];assert_eq!(decode(out.as_mut_ptr(),reader.as_mut_ptr()),0);
  }}}}}
  for(_,p)in hooks{VirtualFree(p.trampoline as*mut _,0,MEM_RELEASE);if !p.relay.is_null(){VirtualFree(p.relay as*mut _,0,MEM_RELEASE);}}
  CAPTURE.store(ptr::null_mut(),Ordering::Release);APPLY.store(ptr::null_mut(),Ordering::Release);BASE.store(0,Ordering::Release);
  VirtualFree(memory,0,MEM_RELEASE);
 }}
}
