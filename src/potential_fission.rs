//! Split fresh local native potentials; saved counters are expedition state.
//! Reuse native lottery and private routing, and keep separate native groups.
use crate::{*, fission_catalog::ROOTS};
use std::{collections::{BTreeMap,VecDeque},sync::Mutex};
pub const OWNER:i32=95003200;
pub const CREATE_RVA:usize=0x57d0a0;
pub const CREATE_PATTERN:&str="40 56 57 41 56 48 81 EC 80 00 00 00 48 8B 05 ? ? ? ? 48 33 C4 48 89 44 24 50";
pub const LOOKUP_RVA:usize=0x582680;
pub const LOOKUP_PATTERN:&str="48 89 5C 24 08 89 54 24 10 55 56 57 48 8B EC 48 83 EC 20 48 8B F1 48 8D 4D 28 E8 ? ? ? ? 83 E8 01 0F 84";
pub const GROUP_RVA:usize=0x57a8c5;
pub const GROUP_PATTERN:&str="0F B7 8A CA 00 00 00 66 0F BA E1 0E 73 0E 66 C1 E9 07 66 83 E1 03";
pub static CREATE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static GROUP_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static GROUP_SKIP:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
type Key=(u64,i32,u8);
#[derive(Clone,Copy,Debug)]
struct Job{key:Key,lots:[i32;2],count:u8,map:u32,position:[f32;3],masks:[u8;2],wanted:[u8;2],uids:[[u32;3];2]}
struct State{spent:Option<u8>,seen:BTreeMap<Key,bool>,queue:VecDeque<Job>,resetting:bool,wait:f32}
impl State{
 const fn new()->Self{Self{spent:None,seen:BTreeMap::new(),queue:VecDeque::new(),resetting:false,wait:0.}}
 fn observe(&mut self,key:Key,eligible:bool,native:u8,job:Job)->bool{
  if let Some(&split)=self.seen.get(&key){return split;}
  if self.resetting{self.seen.insert(key,false);return false;}
  let spent=self.spent.get_or_insert(native);*spent=(*spent).max(native);
  let split=eligible&&!self.resetting&&*spent<3;
  self.seen.insert(key,split);
  if split{*spent+=1;self.queue.push_back(job);self.wait=0.;}split
 }
 fn sync(&mut self,native:u8,reset:bool)->(u8,bool){
  if reset{self.reset();}else if self.resetting&&native==0{self.resetting=false;}
  if self.resetting{return (0,true);}
  let n=self.spent.get_or_insert(native);*n=(*n).max(native);(*n,false)
 }
 fn reset(&mut self){if !self.resetting{*self=Self::new();self.spent=Some(0);self.resetting=true;}}
}
static STATE:Mutex<State>=Mutex::new(State::new());
#[derive(Clone,Copy,Default)]
struct Generated{manager:usize,lot:i32,slot:u8,source:u64,map:u32,position:[f32;3],mask:u8,attempted:u8,count:u8,limit:u8,uids:[u32;3]}
thread_local!{static GENERATED:Cell<Generated>=const{Cell::new(Generated{manager:0,lot:0,slot:0,source:0,map:0,position:[0.;3],mask:0,attempted:0,count:0,limit:0,uids:[0;3]})};}
fn root(lot:i32)->Option<(i32,i32,u8)>{ROOTS.binary_search_by_key(&lot,|v|v.0).ok().map(|i|{let r=ROOTS[i];(r.1,r.2,r.3)})}
fn generated(lot:i32)->bool{
 let offset=lot-98_000_000;offset>=0&&((offset/10)as usize)<ROOTS.len()&&matches!(offset%10,0|5)
}
unsafe fn local(base:usize)->Option<(usize,u8)>{
 let world=read_ptr(base+WORLD_RVA);if world==0{return None;}
 let player=read_ptr(world+0x174e8);if player==0{return None;}
 let game=read_ptr(base+GAME_DATA_RVA);if game==0{return None;}
 let data=read_ptr(game+8);if data==0{return None;}
 let slot=read_i32(data+8).max(1);(1..=3).contains(&slot).then_some((player,slot as u8))
}
unsafe fn spent_native(player:usize,base:usize)->u8{
 (1..=3).rev().find(|&n|has_effect(player,OWNER+n,base)).unwrap_or(0)as u8
}
// A node may predate hook readiness or have been restored while the player
// singleton was absent. Check the actual existing list before reserving it.
unsafe fn already_existing(manager:usize,key:Key)->bool{
 let mut node=read_ptr(manager+0x58);
 for _ in 0..65536{
  if node==0{return false;}
  if ptr::read_unaligned((node+0x80)as*const u64)==key.0&&read_i32(node+0xd8)==key.1&&
   ((ptr::read_unaligned((node+0xca)as*const u16)>>7)&3)as u8==key.2{return true;}
  node=read_ptr(node);
 }true // A malformed/overlong list must not turn an old ball into a new one.
}
fn fresh_caller(rva:usize)->bool{matches!(rva,0x575a53|0x575b39|0x573fe6|0x57ee94)}
#[unsafe(naked)]
pub unsafe extern "system" fn create(){core::arch::naked_asm!("mov r8,[rsp]","jmp {callback}",callback=sym capture);}
unsafe extern "system" fn capture(manager:usize,descriptor:usize,caller:usize)->usize{
 let original:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(CREATE_ORIGINAL.load(Ordering::Acquire));
 if !ACTIVE.load(Ordering::Acquire)||descriptor==0{return original(manager,descriptor);}
 let lot=read_i32(descriptor+0xb8);let Some((a,b,count))=root(lot)else{return original(manager,descriptor);};
 let base=BASE.load(Ordering::Acquire);let Some((player,slot))=local(base)else{return original(manager,descriptor);};
 let flags=ptr::read_unaligned((descriptor+0xaa)as*const u16);
 let source=ptr::read_unaligned((descriptor+0x60)as*const u64);
 let position=ptr::read_unaligned((descriptor+0x78)as*const [f32;3]);
 if manager!=read_ptr(base+ITEMS_RVA)||((flags>>7)&3)as u8!=slot||flags&0x4000==0||
  source as u32==u32::MAX||source==0||!position.iter().all(|p|p.is_finite())||read_i32(descriptor+0x58)&15!=2{return original(manager,descriptor);}
 let key=(source,lot,slot);
 // Restoration/late-join packets establish an old group, never spend a charge.
 // Also remember pre-acquisition groups so subsequent option packets stay old.
 let eligible=fresh_caller(caller.wrapping_sub(base))&&has_effect(player,OWNER,base)&&!has_effect(player,reward_cycle_runtime::RESET,base)&&!already_existing(manager,key);
 let native=spent_native(player,base);
 let job=Job{key,lots:[a,b],count,map:ptr::read_unaligned((descriptor+0x74)as*const u32),position,masks:[0;2],wanted:[0;2],uids:[[0;3];2]};
 let split=STATE.lock().map(|mut s|s.observe(key,eligible,native,job)).unwrap_or(false);
 if split{0}else{original(manager,descriptor)}
}
// Called at the original dispatch only within our synchronous native redraw.
// Always copy the descriptor: the caller owns its buffer and network metadata.
pub unsafe fn route(manager:usize,descriptor:usize,original:unsafe extern "system" fn(usize,usize)->usize)->Option<usize>{
 let scope=GENERATED.with(Cell::get);
 if scope.manager==0||scope.manager!=manager||descriptor==0||read_i32(descriptor+0xb8)!=scope.lot{return None;}
 let owner=ptr::read_unaligned((descriptor+0xaa)as*const u16);
 let index=ptr::read((descriptor+0xa9)as*const u8);
 if ((owner>>7)&3)as u8!=scope.slot||index>=scope.count||scope.limit&(1<<index)==0||scope.mask&(1<<index)!=0||scope.attempted&(1<<index)!=0{return Some(0);}
 GENERATED.with(|c|{let mut s=c.get();s.attempted|=1<<index;c.set(s);});
 let mut copy=ptr::read_unaligned(descriptor as*const [u64;24]);let p=copy.as_mut_ptr()as usize;
 ptr::write_unaligned((p+0x58)as*mut u32,ptr::read_unaligned((p+0x58)as*const u32)&!0x10);
 ptr::write_unaligned((p+0x60)as*mut u64,scope.source);
 ptr::write_unaligned((p+0x74)as*mut u32,scope.map);
 ptr::write_unaligned((p+0x78)as*mut [f32;3],scope.position);
 let node=original(manager,p);
 if node!=0{GENERATED.with(|c|{let mut s=c.get();s.mask|=1<<index;s.uids[index as usize]=ptr::read_unaligned((p+0x5c)as*const u32);c.set(s);});}
 Some(node)
}
fn same_partition(a:i32,b:i32)->bool{if generated(a)||generated(b){a==b}else{true}}
unsafe extern "system" fn group_allowed(incoming:usize,candidate:usize)->u8{
 let incoming_lot=read_i32(incoming+0xd8);let candidate_lot=read_i32(candidate+0xd8);
 let incoming_snack=snack::bonus_lot(incoming_lot);let candidate_snack=snack::bonus_lot(candidate_lot);
 if incoming_snack!=candidate_snack{return 0;}
 if incoming_snack{
  let scope=PERSONAL_REWARD.with(Cell::get);
  if scope.manager!=0&&snack::bonus_lot(scope.lot){return scope.nodes.contains(&candidate)as u8;}
 }
 same_partition(incoming_lot,candidate_lot)as u8
}
#[unsafe(naked)]
pub unsafe extern "system" fn group(){core::arch::naked_asm!(
 "pushfq","sub rsp,0xc8",
 "mov [rsp+0x28],rax","mov [rsp+0x30],rcx","mov [rsp+0x38],rdx",
 "mov [rsp+0x40],r8","mov [rsp+0x48],r9","mov [rsp+0x50],r10","mov [rsp+0x58],r11",
 "movdqu [rsp+0x60],xmm0","movdqu [rsp+0x70],xmm1","movdqu [rsp+0x80],xmm2",
 "movdqu [rsp+0x90],xmm3","movdqu [rsp+0xa0],xmm4","movdqu [rsp+0xb0],xmm5",
 "mov rcx,r8","call {callback}","mov [rsp+0x20],al",
 "movdqu xmm0,[rsp+0x60]","movdqu xmm1,[rsp+0x70]","movdqu xmm2,[rsp+0x80]",
 "movdqu xmm3,[rsp+0x90]","movdqu xmm4,[rsp+0xa0]","movdqu xmm5,[rsp+0xb0]",
 "mov rax,[rsp+0x28]","mov rcx,[rsp+0x30]","mov rdx,[rsp+0x38]",
 "mov r8,[rsp+0x40]","mov r9,[rsp+0x48]","mov r10,[rsp+0x50]","mov r11,[rsp+0x58]",
 "cmp byte ptr [rsp+0x20],0","je 2f",
 "add rsp,0xc8","popfq","jmp qword ptr [rip+{original}]",
 "2:","add rsp,0xc8","popfq","jmp qword ptr [rip+{skip}]",
 callback=sym group_allowed,original=sym GROUP_ORIGINAL,skip=sym GROUP_SKIP
 );}
// The packed-history updater can consume RESET via menu/selection callbacks
// before our next character Update. Notify here too, then retain the reset
// until native removal of every saved fission marker is acknowledged.
pub unsafe fn reset(player:usize,base:usize){
 if let Ok(mut s)=STATE.lock(){s.reset();}
 let manager=read_ptr(player+0x190);if manager==0{return;}
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);
 for n in 1..=3{if has_effect(player,OWNER+n,base){remove(manager,OWNER+n);}}
}
pub unsafe fn update(player:usize,base:usize,dt:f32){
 let manager=read_ptr(player+0x190);if manager==0{return;}
 let reset=has_effect(player,reward_cycle_runtime::RESET,base);
 let native=spent_native(player,base);
 let (desired,due,clearing)={let Ok(mut s)=STATE.lock()else{return;};
  let (desired,clearing)=s.sync(native,reset);
  if dt.is_finite()&&dt>0.{s.wait=(s.wait-dt).max(0.);}
  let due=s.wait<=0.||clearing;if due{s.wait=0.25;}(desired,due,clearing)
 };
 if due{
  let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);
  let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);
  // Acknowledged newer marker before deleting an older one: delayed native
  // effect operations cannot briefly erase the saved counter.
  if desired>0&&!has_effect(player,OWNER+desired as i32,base){apply(player,OWNER+desired as i32,0);}
  if desired==0||has_effect(player,OWNER+desired as i32,base){for n in 1..=3{if n!=desired as i32&&has_effect(player,OWNER+n,base){remove(manager,OWNER+n);}}}
 }
 let stats=native_actor::stats(player);
 if clearing||!has_effect(player,OWNER,base)||stats==0||native_actor::incapacitated(player,stats){return;}
 let Some((_,slot))=local(base)else{return;};let items=read_ptr(base+ITEMS_RVA);if items==0{return;}
 let job=STATE.lock().ok().and_then(|mut s|if due{s.queue.pop_front()}else{None});let Some(mut job)=job else{return;};
 let previous=GENERATED.with(Cell::get);if previous.manager!=0{STATE.lock().unwrap().queue.push_front(job);return;}
 let spawn:unsafe extern "system" fn(usize,usize,i32,u8,u8,u8,u8,*const f32)->u8=std::mem::transmute(base+SPAWN_RVA);
 for i in 0..2{if job.wanted[i]!=0&&job.masks[i]&job.wanted[i]==job.wanted[i]{continue;}
  // A partially created ball may already have been claimed before retry.
  // Never replenish that ball after its successfully created options vanish.
  if job.masks[i]!=0{
   let lookup:unsafe extern "system" fn(usize,u32)->usize=std::mem::transmute(base+LOOKUP_RVA);
   if (0..3).any(|n|job.masks[i]&(1<<n)!=0&&lookup(items,job.uids[i][n])==0){job.masks[i]=job.wanted[i];continue;}
  }
  let mut position=job.position;position[0]+=if i==0{-0.5}else{0.5};
  GENERATED.with(|c|c.set(Generated{manager:items,lot:job.lots[i],slot,source:job.key.0,map:job.map,position,mask:job.masks[i],attempted:0,count:job.count,limit:if job.wanted[i]==0{(1<<job.count)-1}else{job.wanted[i]},uids:job.uids[i]}));
  let p=[position[0],position[1],position[2],0.];spawn(items,player,job.lots[i],0,1,1,0,p.as_ptr());
  let result=GENERATED.with(|c|{let result=c.get();c.set(previous);result});
  job.masks[i]=result.mask;job.uids[i]=result.uids;if job.wanted[i]==0{job.wanted[i]=result.attempted;}
 }
 if (0..2).any(|i|job.wanted[i]==0||job.masks[i]&job.wanted[i]!=job.wanted[i]){if let Ok(mut s)=STATE.lock(){s.queue.push_front(job);}}
}
#[cfg(test)]mod tests{
 use super::*;
 fn job(n:u64)->Job{let r=ROOTS[0];Job{key:(n,r.0,1),lots:[r.1,r.2],count:r.3,map:0,position:[0.;3],masks:[0;2],wanted:[0;2],uids:[[0;3];2]}}
 #[test]fn only_three_new_groups_old_groups_stay_old_and_duplicate_options_share_one_charge(){
  let mut s=State::new();let old=job(1);assert!(!s.observe(old.key,false,0,old));assert!(!s.observe(old.key,true,0,old));
  for n in 2..=4{let j=job(n);for _ in 0..3{assert!(s.observe(j.key,true,0,j));}}
  assert_eq!(s.spent,Some(3));assert_eq!(s.queue.len(),3);
  let fourth=job(5);assert!(!s.observe(fourth.key,true,0,fourth));assert!(s.observe(job(2).key,false,3,job(2)));
  // Character replacement, death or stale native state does not reset anything.
  assert_eq!(s.spent,Some(3));s.reset();s.reset();assert_eq!(s.spent,Some(0));assert!(s.queue.is_empty());
  assert_eq!(s.sync(3,false),(0,true));assert!(!s.observe(job(9).key,true,3,job(9)));assert_eq!(s.spent,Some(0));
  assert_eq!(s.sync(0,false),(0,false));assert!(s.observe(job(10).key,true,0,job(10)));
 }
 #[test]fn restored_spent_count_and_group_partitions(){
  let mut s=State::new();let j=job(1);assert!(s.observe(j.key,true,2,j));assert_eq!(s.spent,Some(3));assert!(!s.observe(job(2).key,true,1,job(2)));
  for &(source,a,b,_)in ROOTS{assert!(root(source).is_some());assert!(generated(a)&&generated(b));assert!(same_partition(a,a));assert!(!same_partition(a,b));assert!(!same_partition(source,a));assert!(same_partition(source,source+10));}
  for caller in [0x573955,0x57f046,0x57f42c,0]{assert!(!fresh_caller(caller));}
 }
}

#[cfg(test)]#[path="potential_fission_native_tests.rs"]mod native_tests;
