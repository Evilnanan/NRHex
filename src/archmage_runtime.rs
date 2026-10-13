//! Native spell-slot getter and local catalyst overlays.
use crate::{archmage,archmage::Catalyst,ptr,read_ptr,read_i32,ACTIVE,BASE,MAGE};
use std::sync::atomic::{AtomicPtr,Ordering};
pub const RVA:usize=0x6a1e10;
pub const PATTERN:&str="48 89 6C 24 10 48 89 74 24 18 57 48 83 EC 50 48 8B 71 08 48 8B E9 8B FA 48 85 F6 0F 84 C5 00 00 00";
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
type Get=unsafe extern "system" fn(usize,u32)->u64;
#[derive(Clone,Copy)]
pub struct Drop {pub catalyst:Catalyst,pub payload:[u8;0x48]}

#[unsafe(naked)]
pub unsafe extern "system" fn wrapper(_reference:usize,_slot:u32)->u64{
 core::arch::naked_asm!("mov r8, [rsp]","jmp {handler}",handler=sym detour);
}
unsafe extern "system" fn detour(reference:usize,slot:u32,caller:usize)->u64{
 let original:Get=std::mem::transmute(ORIGINAL.load(Ordering::Acquire));
 let result=original(reference,slot);
 // Serialization remains original; only the exact public equipment exports
 // are patched after native serialization. No inventory/drop/save contamination.
 if slot!=1||!ACTIVE.load(Ordering::Acquire)||caller.wrapping_sub(BASE.load(Ordering::Acquire))==0x6c180b{return result;}
 let base=BASE.load(Ordering::Acquire);
 let Some(player)=local_owner(base) else{return result;};
 let Some(c)=catalyst_reference(reference) else{return result;};
 if !equipped(player,c.handle){return result;}
 MAGE.get().and_then(|s|s.lock().ok()?.selected(c)).map(|spells|spells[slot as usize] as u32 as u64).unwrap_or(result)
}
pub unsafe fn local_owner(base:usize)->Option<usize>{
 let world=read_ptr(base+crate::WORLD_RVA);if world==0{return None;}
 let player=read_ptr(world+0x174e8);
 (player!=0&&crate::has_effect(player,archmage::OWNER,base)).then_some(player)
}
pub unsafe fn equipped(player:usize,handle:u32)->bool{
 let equip=read_ptr(player+0x670);
 equip!=0&&(0..6).any(|i|read_i32(equip+0x1c+4*i) as u32==handle)
}
pub unsafe fn catalyst_reference(reference:usize)->Option<Catalyst>{
 if reference==0{return None;}
 let entry=read_ptr(reference+8);if entry==0{return None;}
 let handle=read_i32(reference) as u32;
 if handle==0||(handle>>28)&7!=0||read_i32(entry+8) as u32!=handle{return None;}
 let original:Get=std::mem::transmute(ORIGINAL.load(Ordering::Acquire));
 let spells=[original(reference,0) as i32,original(reference,1) as i32];
 if !spells.iter().any(|&i|archmage::kind(i).is_some()){return None;}
 Some(Catalyst{handle,base:read_i32(reference+0x10),original:spells})
}
pub unsafe fn catalyst_handle(handle:*const u32,base:usize)->Option<Catalyst>{
 if handle.is_null()||ptr::read_unaligned(handle)==0{return None;}
 let init:unsafe extern "system" fn(*mut u64,*const u32)->usize=std::mem::transmute(base+crate::ARTS_REF_RVA);
 let release:unsafe extern "system" fn(*mut u64)=std::mem::transmute(base+0x6a1d90);
 let mut reference=[0u64;3];init(reference.as_mut_ptr(),handle);
 let c=catalyst_reference(reference.as_ptr() as usize);release(reference.as_mut_ptr());c
}
unsafe fn hand(player:usize,hand:i32,base:usize)->Option<Catalyst>{
 let get:unsafe extern "system" fn(usize,*mut u32,i32)->*const u32=std::mem::transmute(read_ptr(read_ptr(player)+0x258));
 let mut handle=0;let p=get(player,&mut handle,if hand==0{-2}else{-1});catalyst_handle(p,base)
}
pub unsafe fn pulse(player:usize,effect:i32,base:usize){
 if !matches!(effect,archmage::BUSY|archmage::LEFT_0|archmage::LEFT_1|archmage::RIGHT_0|archmage::RIGHT_1)
  ||!crate::is_local(player,base)||!crate::has_effect(player,archmage::OWNER,base){return;}
 let used=archmage::pulse(effect).and_then(|(h,slot)|hand(player,h,base).map(|c|(c,slot)));
 if let Some(state)=MAGE.get(){if let Ok(mut s)=state.lock(){s.notify(used);}}
}
pub unsafe fn update(player:usize,base:usize){
 let Some(state)=MAGE.get() else{return;};
 let modules=read_ptr(player+0x1b8);if modules==0{return;}
 let stats=read_ptr(modules);if stats==0{return;}
 let owned=crate::has_effect(player,archmage::OWNER,base);let alive=read_i32(stats+0x140)>0;
 let context=read_ptr(base+0x3c131c8);
 if !owned||!alive{crate::archmage_cost::clear();}
 let pending={let Ok(mut s)=state.lock() else{return;};s.synchronize(context,owned,alive);s.tick()};
 reroll(pending);
 let marker=crate::has_effect(player,archmage::MARKER,base);
 if owned&&alive&&!marker{
  let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);apply(player,archmage::MARKER,0);
 }else if (!owned||!alive)&&marker{
  let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);remove(read_ptr(player+0x190),archmage::MARKER);
 }
}
unsafe fn reroll(pending:Vec<(Catalyst,usize)>){
 let Some(state)=MAGE.get() else{return;};
 for (c,slot) in pending{
  let old=state.lock().ok().and_then(|s|s.selected(c)).unwrap_or(c.original)[slot];
  let choices:Vec<_>=archmage::pool(c.original[slot]).into_iter().filter(|&id|id!=old).collect();
  if choices.is_empty(){continue;}
  let mut random=0u32;let count=choices.len() as u32;
  loop{
   if crate::BCryptGenRandom(ptr::null_mut(),&mut random as *mut u32 as *mut u8,4,2)<0{break;}
   if random>=u32::MAX-u32::MAX%count{continue;}
   let spell=choices[(random%count) as usize];
   if let Ok(mut s)=state.lock(){s.reroll(c,slot,spell);}
   crate::nlog_info!("Archmage: handle={} slot={slot} spell={old}->{spell}",c.handle);break;
  }
 }
}
pub unsafe fn serialized(payload:usize,handle:usize,caller:usize,transfer:bool)->Option<Drop>{
 if payload==0||handle==0{return None;}
 let base=BASE.load(Ordering::Acquire);let player=local_owner(base)?;
 let c=catalyst_handle(handle as *const u32,base)?;
 if transfer{
  let pending=MAGE.get().and_then(|state|state.lock().ok().map(|mut s|s.drop_boundary(c))).unwrap_or_default();
  reroll(pending);
 }
 // A used catalyst can be unequipped before being dropped. Capture its
 // overlay by instance identity even though it is no longer in the six slots.
 if !transfer&&!equipped(player,c.handle){return None;}
 let spells=MAGE.get()?.lock().ok()?.selected(c)?;
 let original_payload=ptr::read_unaligned(payload as *const [u8;0x48]);
 if crate::arts::network_export(caller.wrapping_sub(base),transfer){
  let mut data=original_payload;archmage::snapshot(&mut data,c,spells);ptr::write_unaligned(payload as *mut [u8;0x48],data);
 }
 transfer.then_some(Drop{catalyst:c,payload:original_payload})
}
pub unsafe fn bind_drop(descriptor:usize){
 let scope=crate::ARTS_TRANSFER.with(|c|c.get());
 let Some(drop)=scope.mage_drop.filter(|_|scope.active) else{return;};
 if descriptor==0||ptr::read_unaligned(descriptor as *const u8)!=1||read_i32(descriptor+0x58)&0xf!=1{return;}
 if ptr::read_unaligned((descriptor+8) as *const [u8;0x48])!=drop.payload{return;}
 let uid=read_i32(descriptor+0x5c) as u32;if uid==0{return;}
 if let Some(state)=MAGE.get(){if let Ok(mut s)=state.lock(){s.dropped(uid,drop.catalyst);}}
 crate::ARTS_TRANSFER.with(|c|{let mut s=c.get();s.mage_drop=None;c.set(s);});
}
pub unsafe fn picked_up(out:usize,uid:u32,base:usize){
 if let Some(c)=catalyst_handle(out as *const u32,base){
  if let Some(state)=MAGE.get(){if let Ok(mut s)=state.lock(){s.picked_up(uid,c);}}
 }
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn getter_wrapper_forwards_both_arguments_and_entire_native_return(){unsafe{
  unsafe extern "system" fn native(reference:usize,slot:u32)->u64{
   assert_eq!(reference,0x123456789abcdef0);assert_eq!(slot,1);0xfedcba9876543210
  }
  ORIGINAL.store(native as *const () as *mut u8,Ordering::Release);
  assert_eq!(wrapper(0x123456789abcdef0,1),0xfedcba9876543210);
 }}
}
