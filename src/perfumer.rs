//! Free locomotion requests a native aromatic use. Native successful use, not
//! the request or animation start, consumes the 30-second opportunity.
use crate::{has_effect,is_local,native_actor,read_ptr,read_i32,ptr,ACTIVE,BASE};
use std::sync::{Mutex,atomic::{AtomicPtr,Ordering}};
pub const OWNER:i32=95004100;
pub const BEGIN:i32=990041;pub const CLIP_ACTIVE:i32=990043;pub const CLIP_INACTIVE:i32=990044;pub const MANUAL:i32=990045;
pub const GOODS:[i32;3]=[OWNER+20,OWNER+21,OWNER+22];
pub const INVENTORY_RVA:usize=0x1f5610;
pub const INVENTORY_PATTERN:&str="48 89 5C 24 18 48 89 7C 24 20 41 56 48 83 EC 20 83 3A FF 41 8B D8 4C 8B F2 48 8B F9 0F 84 B3 00";
pub const USE_RVA:usize=0x3979b0;
pub const USE_PATTERN:&str="48 89 5C 24 10 44 88 4C 24 20 55 56 57 41 54 41 55 41 56 41 57 48 81 EC 10 01 00 00 4D 8B E0 44";
pub const TARGET_RVA:usize=0x602900;
pub const TARGET_PATTERN:&str="48 89 5C 24 10 48 89 74 24 18 57 48 83 EC 20 41 8B F8 8B F2 48 8B D9 E8 F4 C0 CF FF 48 8B 0D B5";
pub const CAN_USE_RVA:usize=0x3d49a0;
pub const CAN_USE_PATTERN:&str="48 89 5C 24 08 48 89 6C 24 20 56 57 41 56 48 83 EC 50 48 8B FA 48 8B D9 45 32 F6 83 3A FF";
pub static INVENTORY_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static CAN_USE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
thread_local!{static VIRTUAL_PRESENCE:std::cell::Cell<bool>=const{std::cell::Cell::new(false)};}
pub static USE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
const TARGET_MANAGER:usize=0x3c0abd8;
fn choose(previous:Option<usize>,draw:usize)->Option<usize>{(0..3).filter(|&kind|previous!=Some(0)||kind!=0).nth(draw)}
#[derive(Clone,Copy)]struct Pending{kind:usize,saved:[i32;3],age:f64,seen_clip:bool,finished:bool}
struct Cycle{elapsed:f64,safe:f64,last:Option<usize>,pending:Option<Pending>}
impl Cycle{
 const fn new()->Self{Self{elapsed:0.,safe:0.,last:None,pending:None}}
 fn advance(&mut self,dt:f32,safe:bool){if dt.is_finite()&&dt>0.&&dt<=1.{let dt=f64::from(dt);self.elapsed=(self.elapsed+dt).min(30.);self.safe=if safe{self.safe+dt}else{0.};if let Some(p)=&mut self.pending{p.age+=dt;}}else if !safe{self.safe=0.;}}
 fn due(&self)->bool{self.elapsed>=30.&&self.safe>=0.5&&self.pending.is_none()}
 fn used(&mut self,kind:usize)->bool{let Some(p)=&mut self.pending else{return false;};if p.kind!=kind||p.finished{return false;}p.finished=true;self.last=Some(kind);self.elapsed=0.;true}
}
static CYCLE:Mutex<Cycle>=Mutex::new(Cycle::new());
fn encoded(kind:usize)->i32{GOODS[kind]|0x40000000}
unsafe fn valid(player:usize,base:usize)->bool{if player==0||!is_local(player,base)||!has_effect(player,OWNER,base)||has_effect(player,crate::reward_cycle_runtime::RESET,base){return false;}let stats=native_actor::stats(player);stats!=0&&!native_actor::incapacitated(player,stats)}
unsafe fn restore(player:usize,p:Pending){if player!=0&&read_i32(player+0x174)==encoded(p.kind){for (offset,value) in [0x174,0x178,0x18c].into_iter().zip(p.saved){ptr::write_unaligned((player+offset)as*mut i32,value);}}}
pub unsafe fn cancel(player:usize){let p=CYCLE.lock().ok().and_then(|mut c|c.pending.take());if let Some(p)=p{restore(player,p);}}
pub unsafe fn reset(player:usize,_base:usize){cancel(player);if let Ok(mut c)=CYCLE.lock(){*c=Cycle::new();}}
unsafe fn unobserved(base:usize)->bool{
 // Native owns exactly 32 threat classes. A missing manager is unknown.
 if read_ptr(base+TARGET_MANAGER)==0{return false;}
 let get:unsafe extern "system" fn(*mut i32,u32,u32)->*const i32=std::mem::transmute(base+TARGET_RVA);
 let mut state=-1;get(&mut state,0,31);state==0
}
pub unsafe fn update(player:usize,base:usize,dt:f32){
 if player==0||!is_local(player,base){return;}
 if has_effect(player,crate::reward_cycle_runtime::RESET,base){reset(player,base);return;}
 if !valid(player,base){cancel(player);let owned=has_effect(player,OWNER,base);if let Ok(mut c)=CYCLE.lock(){if owned{c.advance(dt,false);}else{c.safe=0.;}}return;}
 let safe=unobserved(base);
 let stale=if let Ok(mut c)=CYCLE.lock(){c.advance(dt,safe);c.pending.is_some_and(|p|p.age>=10.)}else{false};
 if stale{cancel(player);}
}
pub unsafe fn query(player:usize,code:i32,base:usize)->i32{
 if player==0||!is_local(player,base){return 0;}
 if code==MANUAL{cancel(player);return 0;}
 if code==CLIP_ACTIVE{if let Ok(mut c)=CYCLE.lock(){if let Some(p)=&mut c.pending{p.seen_clip=true;}}return 0;}
 if code==CLIP_INACTIVE{let ended=CYCLE.lock().ok().is_some_and(|c|c.pending.is_some_and(|p|p.seen_clip||p.age>=0.5));if ended{cancel(player);}return 0;}
 if code!=BEGIN||!valid(player,base)||!unobserved(base){return 0;}
 let previous=match CYCLE.lock(){Ok(c)if c.due()=>c.last,_=>return 0};
 let Some(draw)=crate::random_index(if previous==Some(0){2}else{3})else{return 0;};let Some(kind)=choose(previous,draw)else{return 0;};
 let saved=[read_i32(player+0x174),read_i32(player+0x178),read_i32(player+0x18c)];
 if let Ok(mut c)=CYCLE.lock(){if !c.due(){return 0;}c.pending=Some(Pending{kind,saved,age:0.,seen_clip:false,finished:false});}else{return 0;}
 for(offset,value)in [(0x174,encoded(kind)),(0x178,1),(0x18c,-1)]{ptr::write_unaligned((player+offset)as*mut i32,value);}
 let can:unsafe extern "system" fn(usize,*const i32)->u8=std::mem::transmute(base+CAN_USE_RVA);let item=encoded(kind);
 if can(player,&item)==0{cancel(player);return 0;}
 (kind+1)as i32
}
pub unsafe extern "system" fn can_use(player:usize,item:*const i32)->u8{
 let original:unsafe extern "system" fn(usize,*const i32)->u8=std::mem::transmute(CAN_USE_ORIGINAL.load(Ordering::Acquire));
 let base=BASE.load(Ordering::Acquire);
 let authorized=!item.is_null()&&ACTIVE.load(Ordering::Acquire)&&valid(player,base)&&CYCLE.lock().ok().is_some_and(|c|c.pending.is_some_and(|p|ptr::read_unaligned(item)==encoded(p.kind)&&read_i32(player+0x174)==encoded(p.kind)));
 let previous=VIRTUAL_PRESENCE.with(|s|s.replace(authorized));let result=original(player,item);VIRTUAL_PRESENCE.with(|s|s.set(previous));result
}
// 1F1B80 tail-calls this lookup. Its only bypassed consumer is the boolean
// "item exists" test at6D06B9 for our current private non-consuming good.
// No synthetic index reaches inventory readers or inventory mutation APIs.
#[unsafe(naked)]pub unsafe extern "system" fn inventory(_root:usize,_item:*const i32,_filter:i32)->i32{
 core::arch::naked_asm!("mov r9,[rsp]","jmp {callback}",callback=sym inventory_lookup);
}
unsafe extern "system" fn inventory_lookup(root:usize,item:*const i32,filter:i32,caller:usize)->i32{
 let original:unsafe extern "system" fn(usize,*const i32,i32)->i32=std::mem::transmute(INVENTORY_ORIGINAL.load(Ordering::Acquire));let result=original(root,item,filter);
 let base=BASE.load(Ordering::Acquire);if result>=0||!ACTIVE.load(Ordering::Acquire)||caller!=base+0x6d06b9||item.is_null()||!VIRTUAL_PRESENCE.with(|s|s.get()){return result;}
 let id=ptr::read_unaligned(item);let matches=CYCLE.lock().ok().is_some_and(|c|c.pending.is_some_and(|p|id==encoded(p.kind)));
 if matches{0}else{result}
}
type Use=unsafe extern "system" fn(usize,i32,usize,u8,u8,f32)->u8;
pub unsafe extern "system" fn used(context:usize,good:i32,data:usize,flag:u8,entry:u8,elapsed:f32)->u8{
 let original:Use=std::mem::transmute(USE_ORIGINAL.load(Ordering::Acquire));let result=original(context,good,data,flag,entry,elapsed);
 if result==0||context==0||!ACTIVE.load(Ordering::Acquire){return result;}
 let base=BASE.load(Ordering::Acquire);let player=read_ptr(context);
 if !valid(player,base){return result;}if let Some(kind)=GOODS.iter().position(|&g|g==good){if let Ok(mut c)=CYCLE.lock(){c.used(kind);}}
 result
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn only_ironjar_is_excluded_after_itself(){for previous in [None,Some(0),Some(1),Some(2)]{let count=if previous==Some(0){2}else{3};let kinds=(0..count).map(|n|choose(previous,n).unwrap()).collect::<Vec<_>>();for kind in 0..3{assert_eq!(kinds.contains(&kind),previous!=Some(0)||kind!=0);}assert_eq!(choose(previous,count),None);}}
 #[test]fn thirty_seconds_and_half_second_of_safety_are_both_required(){let mut c=Cycle::new();for _ in 0..30{c.advance(1.,false);}assert!(!c.due());c.advance(0.25,true);assert!(!c.due());c.advance(0.25,true);assert!(c.due());c.advance(0.01,false);assert!(!c.due());}
 #[test]fn requests_failures_and_interruptions_do_not_spend_or_reroll_the_last_success(){let mut c=Cycle::new();for _ in 0..30{c.advance(1.,true);}c.pending=Some(Pending{kind:1,saved:[-1,0,0],age:0.,seen_clip:false,finished:false});assert!(!c.due());assert_eq!(c.last,None);assert!(!c.used(2));c.pending=None;assert!(c.due());c.pending=Some(Pending{kind:1,saved:[-1,0,0],age:0.,seen_clip:false,finished:false});assert!(c.used(1));assert_eq!(c.elapsed,0.);assert_eq!(c.last,Some(1));assert!(!c.used(1));for dt in [f32::NAN,f32::INFINITY,-1.,0.,2.]{c.advance(dt,true);}assert_eq!(c.elapsed,0.);}
}
#[cfg(test)]#[path="perfumer_native_tests.rs"]mod native_tests;
