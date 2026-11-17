//! Optional, observe-only graph synchronization instrumentation. No wire writes.
//! All cached pointers are comparison keys. Dereferences use the current native
//! call's live module/snapshot or the game-thread roster, never a saved actor.
use std::{ptr,sync::{Mutex,OnceLock,atomic::{AtomicBool,AtomicPtr,AtomicUsize,Ordering}},time::Instant};
use crate::{hook,read_ptr};

static SERIALIZE:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static SEND:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static CLOCK:OnceLock<Instant>=OnceLock::new();
static TOTAL:AtomicUsize=AtomicUsize::new(0);
static DROPPED:AtomicUsize=AtomicUsize::new(0);
static LIMIT_REPORTED:AtomicBool=AtomicBool::new(false);
static QUEUE:Mutex<Vec<String>>=Mutex::new(Vec::new());
const LIMIT:usize=20000;
#[derive(Clone,Copy,Debug)]struct Actor{actor:usize,handle:u64,local:bool}
#[derive(Clone,Copy)]struct Binding{snapshot:usize,actor:Actor,seen:Instant}
struct State{scene:usize,seen:Option<Instant>,rolling:Option<Instant>,actors:Vec<Actor>,bindings:Vec<Binding>}
static STATE:Mutex<State>=Mutex::new(State{scene:0,seen:None,rolling:None,actors:Vec::new(),bindings:Vec::new()});
fn fresh(now:Instant,then:Instant)->bool{now.saturating_duration_since(then).as_secs_f32()<0.5}
fn recording(s:&State)->bool{s.rolling.is_some_and(|t|Instant::now().saturating_duration_since(t).as_secs_f32()<2.)}
fn record(message:impl FnOnce()->String){
 if TOTAL.fetch_update(Ordering::Relaxed,Ordering::Relaxed,|n|(n<LIMIT).then_some(n+1)).is_err(){return;}
 if let Ok(mut q)=QUEUE.try_lock(){if q.len()<1024{q.push(format!("RamSyncProbe t={:.6} {}",CLOCK.get_or_init(Instant::now).elapsed().as_secs_f64(),message()));return;}}
 DROPPED.fetch_add(1,Ordering::Relaxed);
}
fn watched(actor:usize,handle:u64)->Option<Actor>{
 let s=STATE.try_lock().ok()?;if !fresh(Instant::now(),s.seen?)||!recording(&s){return None;}
 s.actors.iter().find(|a|a.actor==actor&&a.handle==handle).copied()
}
unsafe fn live_actor(module:usize)->Option<Actor>{
 if module==0{return None;}let actor=read_ptr(module+8);if actor==0{return None;}
 watched(actor,read_ptr(actor+8)as u64)
}
fn hash(bytes:&[u8])->u64{bytes.iter().fold(0xcbf29ce484222325u64,|h,b|(h^u64::from(*b)).wrapping_mul(0x100000001b3))}

// Canonical native graph payload, without unspecified padding/unused entries.
// Mirrors DE58E0; identity/position fields are data, never treated as rate.
fn encode(s:&[u8])->Option<Vec<u8>>{
 if s.len()<0x17c{return None;}
 let counts=[s[0x14c]as usize,s[0x14d]as usize,s[0x14e]as usize];
 if counts[0]>16||counts[1]>4||counts[2]>32{return None;}
 let mut out=Vec::with_capacity(345);out.push(counts[0]as u8);
 for i in 0..counts[0]{let p=i*4;let v=(u16::from_le_bytes([s[p],s[p+1]])&0x7fff)|(u16::from(s[p+2])<<15);out.extend(v.to_le_bytes());}
 out.push(counts[1]as u8);out.extend_from_slice(&s[0x140..0x140+counts[1]*2]);
 out.push(counts[2]as u8);out.extend_from_slice(&s[0x40..0x40+counts[2]*8]);
 out.extend_from_slice(&s[0x148..0x14c]);out.extend_from_slice(&s[0x14f..0x160]);out.push(s[0x168]);
 if s[0x168]!=0{out.extend_from_slice(&s[0x160..0x168]);}
 out.extend_from_slice(&s[0x16c..0x17c]);Some(out)
}
fn wire_len(b:&[u8])->Option<usize>{
 let mut p=0;for (max,width)in [(16,2),(4,2),(32,8)]{let count=*b.get(p)? as usize;if count>max{return None;}p+=1+count*width;if p>b.len(){return None;}}
 let present=*b.get(p+21)?;p+=22+if present!=0{8}else{0};p+=16;
 (p<=b.len()).then_some(p)
}
fn entries(b:&[u8])->Option<(Vec<(u64,u64,usize)>,usize)>{
 let count=*b.first()? as usize;if !(1..=64).contains(&count){return None;}
 let mut p=1;let mut out=Vec::with_capacity(count);
 for _ in 0..count{let id=u64::from_le_bytes(b.get(p..p+8)?.try_into().ok()?);p+=8;let len=wire_len(b.get(p..)?)?;out.push((id,hash(&b[p..p+len]),len));p+=len;}
 Some((out,b.len()-p))
}
fn hex(b:&[u8])->String{b.iter().map(|v|format!("{v:02x}")).collect()}
unsafe fn snapshot(event:&str,actor:Actor,p:usize){
 if p==0||TOTAL.load(Ordering::Relaxed)>=LIMIT{return;}
 let Some(bytes)=encode(std::slice::from_raw_parts(p as*const u8,0x17c))else{return;};
 let local=if actor.local{crate::lightning_ram::sync_probe(actor.actor)}else{None};
 record(||format!("{event} actor={actor:x?} snapshot={p:x} motor={local:?} hash={:x} wire={}",hash(&bytes),hex(&bytes)));
}
pub(crate) unsafe extern "system" fn capture(module:usize,p:usize){
 if p==0{return;}let Some(actor)=live_actor(module)else{return;};
 // Pre-call: the destination has not been filled yet. Bind only, do not read it.
 if let Ok(mut s)=STATE.try_lock(){let now=Instant::now();s.bindings.retain(|b|fresh(now,b.seen)&&b.snapshot!=p);if s.bindings.len()>=32{s.bindings.remove(0);}s.bindings.push(Binding{snapshot:p,actor,seen:now});}
}
unsafe extern "system" fn serialize(p:usize,_writer:usize){
 let binding=STATE.try_lock().ok().and_then(|s|s.bindings.iter().find(|b|b.snapshot==p&&fresh(Instant::now(),b.seen)).copied());
 if let Some(b)=binding{snapshot("serialize",b.actor,p);}
}
pub(crate) unsafe extern "system" fn apply(module:usize,p:usize){if let Some(actor)=live_actor(module){snapshot("apply",actor,p);}}
unsafe extern "system" fn send(_network:usize,kind:u32,p:usize,len:u32){
 if kind!=0x46||p==0||len==0||len>65536||TOTAL.load(Ordering::Relaxed)>=LIMIT{return;}
 if !STATE.try_lock().is_ok_and(|s|recording(&s)&&!s.actors.is_empty()&&s.seen.is_some_and(|t|fresh(Instant::now(),t))){return;}
 let payload=std::slice::from_raw_parts(p as*const u8,len as usize);
 // Native entity references only, no platform/user identity or unrelated payload.
 record(||format!("send46 bytes={len} entries={:x?}",entries(payload)));
}
pub fn cycle(actor:usize){let motor=crate::lightning_ram::sync_probe(actor);record(||format!("next-roll actor={actor:x} motor={motor:?}"));}
pub unsafe fn update(local:usize,base:usize){
 let world=read_ptr(base+crate::WORLD_RVA);let scene=read_ptr(base+0x3c131c8);let mut actors=Vec::with_capacity(3);let mut rolling=false;
 if world!=0&&local!=0{
  let get:unsafe extern "system" fn(usize,i32)->usize=std::mem::transmute(base+0x5197e0);
  for actor in std::iter::once(local).chain((1..=3).map(|slot|get(world,slot))){
   if actor==0||actors.iter().any(|a:&Actor|a.actor==actor)||!crate::has_effect(actor,crate::lightning_ram::OWNER,base){continue;}
   actors.push(Actor{actor,handle:read_ptr(actor+8)as u64,local:actor==local});
   if let Some(graph)=crate::ram_stagger::graph(actor,base){
    let query:unsafe extern "system" fn(usize,*const u8)->u8=std::mem::transmute(base+crate::ram_stagger::NODE_RVA);
    rolling|=query(graph,b"a713_040010_hkx_AutoSet_00\0".as_ptr())!=0||query(graph,b"a713_040000_hkx_AutoSet_00\0".as_ptr())!=0;
   }
  }
 }
 if let Ok(mut s)=STATE.try_lock(){if s.scene!=scene{s.bindings.clear();s.rolling=None;}s.scene=scene;s.actors=actors;s.seen=Some(Instant::now());if rolling{s.rolling=s.seen;}}
 let messages=QUEUE.try_lock().map(|mut q|std::mem::take(&mut *q)).unwrap_or_default();
 for m in messages{crate::nlog_info!("{m}");}
 let dropped=DROPPED.swap(0,Ordering::Relaxed);if dropped!=0{crate::nlog_info!("RamSyncProbe dropped={dropped}");}
 if TOTAL.load(Ordering::Relaxed)>=LIMIT&&!LIMIT_REPORTED.swap(true,Ordering::Relaxed){crate::nlog_info!("RamSyncProbe capture limit reached; native behavior unchanged");}
}

// Tail trampoline retains native return type, all argument registers, flags,
// XMM0..5, shadow space, stack arguments, and the original return address.
macro_rules! observe{($name:ident,$callback:ident,$original:ident)=>{
 #[unsafe(naked)]unsafe extern "system" fn $name(){core::arch::naked_asm!(
  "pushfq","sub rsp,0xc0",
  "mov [rsp+0x20],rax","mov [rsp+0x28],rcx","mov [rsp+0x30],rdx","mov [rsp+0x38],r8","mov [rsp+0x40],r9","mov [rsp+0x48],r10","mov [rsp+0x50],r11",
  "movdqu [rsp+0x60],xmm0","movdqu [rsp+0x70],xmm1","movdqu [rsp+0x80],xmm2","movdqu [rsp+0x90],xmm3","movdqu [rsp+0xa0],xmm4","movdqu [rsp+0xb0],xmm5",
  "call {callback}",
  "movdqu xmm0,[rsp+0x60]","movdqu xmm1,[rsp+0x70]","movdqu xmm2,[rsp+0x80]","movdqu xmm3,[rsp+0x90]","movdqu xmm4,[rsp+0xa0]","movdqu xmm5,[rsp+0xb0]",
  "mov rax,[rsp+0x20]","mov rcx,[rsp+0x28]","mov rdx,[rsp+0x30]","mov r8,[rsp+0x38]","mov r9,[rsp+0x40]","mov r10,[rsp+0x48]","mov r11,[rsp+0x50]",
  "add rsp,0xc0","popfq","jmp qword ptr [rip+{original}]",callback=sym $callback,original=sym $original
 );}
};}
observe!(serialize_hook,serialize,SERIALIZE);
observe!(send_hook,send,SEND);
pub unsafe fn prepare(base:usize)->Result<Vec<(usize,hook::PreparedHook)>,String>{
 let targets:[(usize,*const u8,&AtomicPtr<u8>,&[u8]);2]=[
  (0xde58e0,serialize_hook as*const u8,&SERIALIZE,&[0x48,0x89,0x5c,0x24,0x08,0x48,0x89,0x6c,0x24,0x10,0x48,0x89,0x74,0x24,0x18,0x48]),
  (0xdf24d0,send_hook as*const u8,&SEND,&[0x48,0x89,0x5c,0x24,0x08,0x48,0x89,0x6c,0x24,0x10,0x48,0x89,0x74,0x24,0x18,0x57]),
 ];
 for(rva,_,_,sig)in targets{if std::slice::from_raw_parts((base+rva)as*const u8,sig.len())!=sig{return Err(format!("RamSyncProbe signature mismatch {rva:x}"));}}
 let mut result=Vec::new();for(rva,detour,original,_)in targets{let p=hook::prepare((base+rva)as*mut u8,detour,64)?;original.store(p.trampoline,Ordering::Release);result.push((rva,p));}
 crate::nlog_info!("RamSyncProbe prepared: shared capture/apply, serialize/send46; observe-only, no packet writes; limit={LIMIT}");Ok(result)
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn wire_handles_bounds_optional_reference_and_every_truncation(){
  for counts in [[0,0,0],[16,4,32],[3,2,5]]{for present in [0,1]{
   let mut s=[0u8;0x180];s[0x14c..0x14f].copy_from_slice(&counts);s[0x168]=present;
   let b=encode(&s).unwrap();assert_eq!(b.len(),41+counts[0]as usize*2+counts[1]as usize*2+counts[2]as usize*8+present as usize*8);
   assert_eq!(wire_len(&b),Some(b.len()));for n in 0..b.len(){assert_eq!(wire_len(&b[..n]),None);}
   for (at,max)in [(0x14c,16),(0x14d,4),(0x14e,32)]{let mut bad=s;bad[at]=max+1;assert!(encode(&bad).is_none());}
  }}
 }
 #[test]fn packet_parses_multiple_entities_and_reports_unclaimed_tail(){
  let b=encode(&[0u8;0x180]).unwrap();let mut packet=vec![2];
  for id in [123u64,456]{packet.extend(id.to_le_bytes());packet.extend(&b);}
  let (rows,tail)=entries(&packet).unwrap();assert_eq!(tail,0);assert_eq!(rows,vec![(123,hash(&b),41),(456,hash(&b),41)]);
  for n in 0..packet.len(){assert!(entries(&packet[..n]).is_none());}
  packet.extend([1,2,3]);assert_eq!(entries(&packet).unwrap().1,3);assert!(entries(&[0]).is_none());assert!(entries(&[65]).is_none());
 }
 #[test]fn canonical_payload_excludes_padding_but_retains_graph_values(){
  let mut a=[0u8;0x180];a[0x14c]=1;a[0x14e]=1;a[0]=3;a[2]=1;
  let original=encode(&a).unwrap();assert_eq!(&original[1..3],&0x8003u16.to_le_bytes());
  a[3]=255;a[0x17f]=255;a[0x100]=255;assert_eq!(encode(&a).unwrap(),original);
  a[0x40]=1;assert_ne!(encode(&a).unwrap(),original);
 }
 #[test]fn expired_roster_or_binding_is_not_reused(){let now=Instant::now();assert!(fresh(now,now));assert!(!fresh(now+std::time::Duration::from_millis(500),now));}
 static TEST_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
 #[unsafe(naked)]unsafe extern "system" fn clobber(){core::arch::naked_asm!(
  "xor eax,eax","xor ecx,ecx","xor edx,edx","xor r8d,r8d","xor r9d,r9d","xor r10d,r10d","xor r11d,r11d",
  "pxor xmm0,xmm0","pxor xmm1,xmm1","pxor xmm2,xmm2","pxor xmm3,xmm3","pxor xmm4,xmm4","pxor xmm5,xmm5","ret"
 );}
 observe!(test_observer,clobber,TEST_ORIGINAL);
 #[test]fn observer_preserves_mixed_arguments_stack_arguments_and_native_return(){unsafe{
  unsafe extern "system" fn original(a:usize,b:f64,c:usize,d:f32,e:u64,f:f64)->f64{
   assert_eq!((a,b,c,d,e,f),(0x123456789,1.75,0x987654321,2.5,0xfedcba9876543210,3.25));b+f
  }
  TEST_ORIGINAL.store(original as*mut u8,Ordering::Release);
  let call:unsafe extern "system" fn(usize,f64,usize,f32,u64,f64)->f64=std::mem::transmute(test_observer as*const());
  assert_eq!(call(0x123456789,1.75,0x987654321,2.5,0xfedcba9876543210,3.25),5.);
 }}
 #[test]#[ignore="requires NRHEX_NATIVE_EXE; execute alone"]
 fn native_serializer_matches_probe_and_observer_entry_trampolines_prepare(){unsafe{
  use crate::win::*;
  let path=std::path::PathBuf::from(std::env::var_os("NRHEX_NATIVE_EXE").unwrap());crate::verify_executable(&path).unwrap();
  let raw=std::fs::read(path).unwrap();let headers=crate::pe::Headers::parse(&raw,0).unwrap();
  let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
  for s in &headers.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
  let prepared=prepare(base).unwrap();assert_eq!(prepared.len(),2);
  type Serialize=unsafe extern "system" fn(*const u8,*const usize)->usize;
  let native:Serialize=std::mem::transmute(base+0xde58e0);let observed:Serialize=std::mem::transmute(serialize_hook as*const());
  for counts in [[0,0,0],[16,4,32],[1,1,1],[9,3,17]]{for present in [0,1]{
   let mut s=[0u8;0x180];for(i,b)in s.iter_mut().enumerate(){*b=(i*37)as u8;}s[0x14c..0x14f].copy_from_slice(&counts);s[0x168]=present;
   let expected=encode(&s).unwrap();let before=s;
   for call in [native,observed]{let mut out=[0xa5u8;512];let writer=[out.as_mut_ptr()as usize,out.len()];let n=call(s.as_ptr(),writer.as_ptr());
    assert_eq!(n,expected.len());assert_eq!(&out[..n],expected);assert!(out[n..].iter().all(|b|*b==0xa5));assert_eq!(s,before);
   }
  }}
  for(_,p)in prepared{assert_eq!(VirtualFree(p.trampoline as*mut _,0,MEM_RELEASE),1);if !p.relay.is_null(){assert_eq!(VirtualFree(p.relay as*mut _,0,MEM_RELEASE),1);}}
  SERIALIZE.store(ptr::null_mut(),Ordering::Release);SEND.store(ptr::null_mut(),Ordering::Release);
  assert_eq!(VirtualFree(memory,0,MEM_RELEASE),1);
 }}
}
