//! Reuse PrivateItemChoiceDialog's native Leave key, without choosing/dropping
//! an item or closing the menu. Only our three local private goods qualify.
use crate::{read_ptr,read_i32,hex_pool,reward_choices,reward_cycle,reward_cycle_runtime};
use std::{cell::Cell,ptr,sync::{Mutex,atomic::{AtomicPtr,Ordering}}};

pub const FIRST:i32=95002826;
pub const LABEL:i32=95002850;
pub const CTOR_RVA:usize=0x9c6050;
pub const TEXT_RVA:usize=0x7ac4e0;
pub const ENABLE_RVA:usize=0x9c7fc0;
pub const PRESS_RVA:usize=0x9c87b0;
pub const SELECT_RVA:usize=0x9c8450;
pub const ITEM_CTOR_RVA:usize=0x8b9780;
pub const ITEM_COPY_RVA:usize=0x8b7fc0;
pub const ROW_DTOR_RVA:usize=0x9c6b90;
pub const BIND_RVA:usize=0x9c5cd0;
pub static CTOR_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static TEXT_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static ENABLE_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static PRESS_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
thread_local!{static CONSTRUCTING:Cell<usize>=const{Cell::new(0)};}
static USES:Mutex<Uses>=Mutex::new(Uses{mask:None,wait:0.,dirty:false});

struct Uses{mask:Option<u8>,wait:f32,dirty:bool}
impl Uses{
 fn observe(&mut self,native:u8)->u8{let mask=*self.mask.get_or_insert(native&7);self.dirty=mask!=native;mask}
 fn clear(&mut self){self.mask=Some(0);self.dirty=true;self.wait=0.;}
 fn spend(&mut self,slot:usize)->bool{
  let Some(mask)=self.mask else{return false;};if slot>=3||mask&(1<<slot)!=0{return false;}
  self.mask=Some(mask|(1<<slot));self.dirty=true;self.wait=0.;true
 }
}
pub unsafe fn update(player:usize,base:usize,dt:f32){
 let due=USES.lock().map(|mut s|{if dt.is_finite()&&dt>0.{s.wait=(s.wait-dt).max(0.);}s.mask.is_none()||(s.dirty&&s.wait<=0.)}).unwrap_or(false);if !due{return;}
 let native=(0..3).fold(0,|m,i|m|if crate::has_effect(player,FIRST+i,base){1<<i}else{0});
 let wanted=USES.lock().ok().map(|mut s|{s.wait=0.25;s.observe(native)});let Some(wanted)=wanted else{return;};
 let manager=read_ptr(player+0x190);if manager==0{return;}
 let add:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);
 for i in 0..3{if (wanted^native)&(1<<i)!=0{if wanted&(1<<i)!=0{add(player,FIRST+i,0);}else{remove(manager,FIRST+i);}}}
}
pub fn reset(){if let Ok(mut s)=USES.lock(){s.clear();}}

unsafe fn local(base:usize)->Option<(usize,u8)>{
 let world=read_ptr(base+crate::WORLD_RVA);if world==0{return None;}
 let player=read_ptr(world+0x174e8);if player==0{return None;}
 let game=read_ptr(base+crate::GAME_DATA_RVA);if game==0{return None;}
 let data=read_ptr(game+8);if data==0{return None;}
 let slot=read_i32(data+8).max(1);if !(1..=3).contains(&slot){return None;}
 Some((player,slot as u8))
}
unsafe fn first(dialog:usize,base:usize)->Option<usize>{
 if dialog==0||read_ptr(dialog)!=base+0x2c5b020{return None;}
 let list=dialog+0x1770;
 // The embedded array begins at list+8; its count is array+0x350.
 // list+0x358 is a count, not a change counter.
 if read_ptr(list)!=base+0x2c5afe8||read_ptr(list+0x358)!=3{return None;}
 Some((list+15)&!7)
}
unsafe fn hex_dialog(dialog:usize,base:usize)->bool{
 first(dialog,base).is_some_and(|row|(0..3).all(|i|reward_choices::is_hex_good(ptr::read_unaligned((row+i*0x118+0x64)as *const u32))))
}
struct Choice{player:usize,row:usize,node:usize,index:usize,visible:u32,old:u32}
unsafe fn choice(dialog:usize,base:usize)->Option<Choice>{
 let first=first(dialog,base)?;
 if !hex_dialog(dialog,base){return None;}
 let selected:unsafe extern "system" fn(usize)->usize=std::mem::transmute(base+SELECT_RVA);
 let row=selected(dialog);if row<first||(row-first)%0x118!=0{return None;}
 let index=(row-first)/0x118;if index>=3{return None;}
 let (player,slot)=local(base)?;let manager=read_ptr(base+crate::ITEMS_RVA);if manager==0{return None;}
 let mut uids=[0u32;3];let mut goods=[0u32;3];
 for i in 0..3{uids[i]=ptr::read_unaligned((first+i*0x118+8)as *const u32);goods[i]=ptr::read_unaligned((first+i*0x118+0x64)as *const u32);}
 let mut found=[0usize;3];let mut node=read_ptr(manager+0x58);
 for _ in 0..4096{
  if node==0{break;}
  let uid=ptr::read_unaligned((node+0x78)as *const u32);
  if let Some(i)=uids.iter().position(|&u|u==uid){
   let good=ptr::read_unaligned((node+0x28)as *const u32);
   if found[i]!=0||good!=goods[i]||!crate::reward_recovery::eligible(uid,ptr::read_unaligned((node+0xca)as *const u16),read_i32(node+0xd8),good,slot){return None;}
   found[i]=node;
  }
  node=read_ptr(node);
 }
 if node!=0||found.contains(&0)||uids[0]==uids[1]||uids[0]==uids[2]||uids[1]==uids[2]{return None;}
 let mut visible=0;
 for g in goods{visible|=1<<hex_pool::OWNER_IDS.iter().position(|&id|g==(0x40000000|id as u32))?;}
 Some(Choice{player,row,node:found[index],index,visible,old:goods[index]})
}
unsafe fn plan(c:&Choice,base:usize)->Option<reward_cycle::Single>{
 update(c.player,base,0.);
 if !USES.lock().ok()?.mask.is_some_and(|m|m&(1<<c.index)==0){return None;}
 let seen=reward_cycle_runtime::seen(c.player,base)?;
 let r=reward_cycle::Single::new(crate::unavailable_hexes(c.player,base),seen,c.visible);
 (!r.candidates.is_empty()).then_some(r)
}

pub unsafe extern "system" fn constructor(dialog:usize,b:usize,c:usize,d:usize,e:usize,f:usize,g:usize)->usize{
 let original:unsafe extern "system" fn(usize,usize,usize,usize,usize,usize,usize)->usize=std::mem::transmute(CTOR_ORIGINAL.load(Ordering::Acquire));
 let previous=CONSTRUCTING.with(|v|v.replace(dialog));let result=original(dialog,b,c,d,e,f,g);CONSTRUCTING.with(|v|v.set(previous));result
}
pub unsafe extern "system" fn text(out:usize,id:i32)->usize{
 let base=crate::BASE.load(Ordering::Acquire);
 let id=if id==120600&&crate::ACTIVE.load(Ordering::Acquire)&&CONSTRUCTING.with(|v|hex_dialog(v.get(),base)){LABEL}else{id};
 let original:unsafe extern "system" fn(usize,i32)->usize=std::mem::transmute(TEXT_ORIGINAL.load(Ordering::Acquire));original(out,id)
}
pub unsafe extern "system" fn enabled(functor:usize)->u8{
 let base=crate::BASE.load(Ordering::Acquire);let dialog=read_ptr(functor+8);
 if crate::ACTIVE.load(Ordering::Acquire)&&hex_dialog(dialog,base){return choice(dialog,base).and_then(|c|plan(&c,base)).is_some() as u8;}
 let original:unsafe extern "system" fn(usize)->u8=std::mem::transmute(ENABLE_ORIGINAL.load(Ordering::Acquire));original(functor)
}
struct Busy;
impl Drop for Busy{fn drop(&mut self){crate::CALLBACK_LOCK.store(false,Ordering::Release);}}
pub unsafe extern "system" fn press(dialog:usize){
 let base=crate::BASE.load(Ordering::Acquire);
 if !crate::ACTIVE.load(Ordering::Acquire)||!hex_dialog(dialog,base){
  let original:unsafe extern "system" fn(usize)=std::mem::transmute(PRESS_ORIGINAL.load(Ordering::Acquire));original(dialog);return;
 }
 if crate::CALLBACK_LOCK.swap(true,Ordering::AcqRel){return;}let _busy=Busy;
 let Some(c)=choice(dialog,base)else{return;};let Some(r)=plan(&c,base)else{return;};
 let Some(next)=crate::random_order(&r.candidates)else{return;};let Some(seen)=r.next_seen(next)else{return;};
 let good=0x40000000|hex_pool::OWNER_IDS[next] as u32;
 // Build a genuine native MenuGaitem. Do not shallow-copy its strings or
 // mutate only the displayed ID: its cached names/icons/details also matter.
 #[repr(C,align(16))]struct Row([u8;0x118]);let mut tmp=Row([0;0x118]);let temp=tmp.0.as_mut_ptr()as usize;
 ptr::write(temp as *mut usize,base+0x2c5af90);
 let uid=ptr::read_unaligned((c.row+8)as *const u32);ptr::write((temp+8)as *mut u32,uid);
 let ctor:unsafe extern "system" fn(usize,u32)=std::mem::transmute(base+ITEM_CTOR_RVA);
 let copy:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(base+ITEM_COPY_RVA);
 let destroy:unsafe extern "system" fn(usize)=std::mem::transmute(base+ROW_DTOR_RVA);
 ptr::write_unaligned((c.node+0x28)as *mut u32,good);ctor(temp+0x10,uid);
 if ptr::read_unaligned((temp+0x64)as *const u32)!=good{
  ptr::write_unaligned((c.node+0x28)as *mut u32,c.old);destroy(temp);return;
 }
 let spent=USES.lock().map(|mut s|s.spend(c.index)).unwrap_or(false);
 if !spent{ptr::write_unaligned((c.node+0x28)as *mut u32,c.old);destroy(temp);return;}
 copy(c.row+0x10,temp+0x10);destroy(temp);
 reward_cycle_runtime::commit(c.player,base,seen);update(c.player,base,0.);
 // Rebind the native provider, preserving focus. Its own renderer updates
 // name, icon and caption; confirmation still grants the same node UID.
 let bind:unsafe extern "system" fn(usize,usize,i32,u8)=std::mem::transmute(base+BIND_RVA);bind(dialog+0xe20,dialog+0x1770,c.index as i32,0);
 crate::nlog_info!("Hex reroll: slot={} old={} new={} uid={uid:#x}",c.index+1,c.old&0xfffffff,good&0xfffffff);
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn each_slot_has_one_use_and_native_write_delay_cannot_restore_it(){
  let mut u=Uses{mask:None,wait:0.,dirty:false};assert_eq!(u.observe(0),0);
  for slot in [2,0,1]{assert!(u.spend(slot));assert!(!u.spend(slot));let wanted=u.mask.unwrap();assert_eq!(u.observe(0),wanted);assert_eq!(u.observe(wanted),wanted);assert!(!u.dirty);}
  assert_eq!(u.mask,Some(7));assert!(!u.spend(3));
 }
 #[test]fn saved_slot_uses_survive_reopening_revival_and_runtime_restart(){
  let mut u=Uses{mask:None,wait:0.,dirty:false};assert_eq!(u.observe(5),5);assert!(!u.spend(0));assert!(!u.spend(2));assert!(u.spend(1));
  u.clear();assert_eq!(u.observe(7),0);assert_eq!(u.observe(0),0);assert!(u.spend(0));
 }
 #[test]fn label_override_is_scoped_to_our_dialog_and_constructor_forwards_seven_arguments(){unsafe{
  // Exercise the actual wrappers: the native constructor creates the footer
  // while the TLS scope is live. Ordinary weapon menus retain Leave.
  use std::sync::atomic::AtomicUsize;
  static CALLS:AtomicUsize=AtomicUsize::new(0);
  unsafe extern "system" fn lookup(out:usize,id:i32)->usize{out+id as usize}
  unsafe extern "system" fn ctor(a:usize,b:usize,c:usize,d:usize,e:usize,f:usize,g:usize)->usize{
   assert_eq!([b,c,d,e,f,g],[11,22,33,44,55,66]);assert_eq!(CONSTRUCTING.with(Cell::get),a);
   CALLS.store(text(10,120600),Ordering::Relaxed);assert_eq!(text(10,123),133);0x12345678
  }
  let old_base=crate::BASE.swap(0x10000000,Ordering::AcqRel);let old_active=crate::ACTIVE.swap(true,Ordering::AcqRel);
  let old_ctor=CTOR_ORIGINAL.swap(ctor as *mut u8,Ordering::AcqRel);let old_text=TEXT_ORIGINAL.swap(lookup as *mut u8,Ordering::AcqRel);
  let base=crate::BASE.load(Ordering::Acquire);let mut dialog=vec![0u64;0x1d28/8+1];let p=dialog.as_mut_ptr()as usize;
  ptr::write(p as *mut usize,base+0x2c5b020);ptr::write((p+0x1770)as *mut usize,base+0x2c5afe8);ptr::write((p+0x1770+0x358)as *mut u64,3);
  let row=(p+0x1770+15)&!7;
  for i in 0..3{ptr::write_unaligned((row+i*0x118+0x64)as *mut u32,0x40000000|hex_pool::OWNER_IDS[i]as u32);}
  assert_eq!(constructor(p,11,22,33,44,55,66),0x12345678);assert_eq!(CALLS.load(Ordering::Relaxed),10+LABEL as usize);assert_eq!(CONSTRUCTING.with(Cell::get),0);
  assert_eq!(text(10,120600),120610);
  ptr::write_unaligned((row+0x64)as *mut u32,100);
  assert_eq!(constructor(p,11,22,33,44,55,66),0x12345678);assert_eq!(CALLS.load(Ordering::Relaxed),120610);
  CTOR_ORIGINAL.store(old_ctor,Ordering::Release);TEXT_ORIGINAL.store(old_text,Ordering::Release);crate::BASE.store(old_base,Ordering::Release);crate::ACTIVE.store(old_active,Ordering::Release);
 }}
}
pub const CTOR_PATTERN:&str="48 89 5C 24 18 55 56 57 41 54 41 55 41 56 41 57 48 8D AC 24 00 FA FF FF 48 81 EC 00 07 00 00 48";
pub const TEXT_PATTERN:&str="48 89 4C 24 08 53 48 83 EC 30 48 8B D9 C7 44 24 20 00 00 00 00 4C 8D 0D 74 D5 48 02 44 8B C2 BA";
pub const ENABLE_PATTERN:&str="48 89 5C 24 08 57 48 83 EC 30 48 8B 59 08 48 8D 8B 20 0E 00 00 E8 76 9A DB FF 8B F8 85 C0 0F 88";
pub const PRESS_PATTERN:&str="40 55 53 56 57 48 8D 6C 24 C1 48 81 EC B8 00 00 00 48 8B F9 48 81 C1 20 0E 00 00 E8 80 92 DB FF";
pub const SELECT_PATTERN:&str="48 89 5C 24 08 57 48 83 EC 20 48 8B F9 48 81 C1 20 0E 00 00 E8 E7 95 DB FF 8B D8 85 C0 78 33 48";
pub const ITEM_CTOR_PATTERN:&str="48 89 5C 24 10 48 89 74 24 18 48 89 7C 24 20 55 48 8D AC 24 20 FF FF FF 48 81 EC E0 01 00 00 48 8B 05 C2 C7 22 03 48 33 C4 48 89 85 D0 00 00 00";
pub const ITEM_COPY_PATTERN:&str="E9 BC 77 8C 01 48 8B 05 9C DF 22 03 48 33 C4 48 89 45 C0 48 8B DA 48 8B F9 C7 44 24 40 00 00 00";
pub const ROW_DTOR_PATTERN:&str="48 89 4C 24 08 53 56 57 48 83 EC 30 48 8B F1 48 8D 79 10 48 89 7C 24 58 48 8D 4F 78 E8 2F CA F2";
pub const BIND_PATTERN:&str="40 53 55 56 57 41 56 41 57 48 81 EC B8 00 00 00 48 8B 05 81 02 12 03 48 33 C4 48 89 84 24 A0 00";
