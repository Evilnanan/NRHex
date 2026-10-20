use super::*;
use crate::{ptr,win::*};
use sha2::{Digest,Sha256};
use std::{cell::Cell,sync::atomic::Ordering};
thread_local!{
 static OWNED:Cell<bool>=const{Cell::new(true)};
 static RESET:Cell<bool>=const{Cell::new(false)};
 static ACCEPT:Cell<u8>=const{Cell::new(1)};
 static FAIL:Cell<bool>=const{Cell::new(false)};
 static DEFER:Cell<bool>=const{Cell::new(false)};
 static IMAGE:Cell<usize>=const{Cell::new(0)};
 static NODE:Cell<usize>=const{Cell::new(0)};
 static ROW:Cell<usize>=const{Cell::new(0)};
 static APPLICATIONS:Cell<usize>=const{Cell::new(0)};
 static BASE_HP:Cell<i32>=const{Cell::new(1000)};
}
unsafe fn pointer(address:usize,value:usize){ptr::write_unaligned(address as*mut usize,value);}
unsafe fn integer(address:usize,value:i32){ptr::write_unaligned(address as*mut i32,value);}
unsafe fn float(address:usize,value:f32){ptr::write_unaligned(address as*mut f32,value);}
unsafe fn value(address:usize)->f32{ptr::read_unaligned(address as*const f32)}
unsafe fn patch(base:usize,address:usize,bytes:&[u8]){ptr::copy_nonoverlapping(bytes.as_ptr(),(base+address)as*mut u8,bytes.len());}
unsafe fn jump(base:usize,address:usize,target:usize){
 let mut bytes=vec![0x48,0xb8];bytes.extend_from_slice(&target.to_le_bytes());bytes.extend_from_slice(&[0xff,0xe0]);patch(base,address,&bytes);
}
unsafe extern "system" fn lookup(output:usize,identity:i32){assert_eq!(identity,BUFF);pointer(output,ROW.with(Cell::get));integer(output+8,identity);}
unsafe extern "system" fn has(manager:usize,identity:i32)->u8{
 if identity==OWNER{return u8::from(OWNED.with(Cell::get));}
 if identity==crate::reward_cycle_runtime::RESET{return u8::from(RESET.with(Cell::get));}
 let mut node=read_ptr(manager+8);
 while node!=0{if read_i32(node+8)==identity&&(read_i32(node+0x68)as u32&0x800c0003)==0{return 1;}node=read_ptr(node+0x38);}0
}
unsafe extern "system" fn accepted(_:usize,identity:i32,_:usize,_:usize,_:usize,_:u8,_:u8,_:u8,_:u8,_:u8,_:i32)->u8{
 if identity==crate::runtime::ULTIMATE_TRIGGER{ACCEPT.with(Cell::get)}else{1}
}
unsafe extern "system" fn apply(player:usize,identity:i32,flag:u8)->u8{
 assert_eq!((identity,flag),(BUFF,0));
 if FAIL.with(Cell::get){return 0;}
 APPLICATIONS.with(|count|count.set(count.get()+1));
 if DEFER.with(Cell::get){return 1;}
 commit(player,identity)
}
unsafe fn commit(player:usize,identity:i32)->u8{
 let base=IMAGE.with(Cell::get);let node=NODE.with(Cell::get);let manager=read_ptr(player+0x190);
 assert_eq!(crate::effect_detour(player,identity,player,0,0,0,0,0,0,0,0),1);
 if has(manager,BUFF)==0{
  pointer(node,ROW.with(Cell::get));integer(node+8,BUFF);integer(node+0x68,0);
  pointer(node+0x38,0);pointer(node+0x40,0);pointer(manager+8,node);
 }
 let matching:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+0x510b20);
 let category:unsafe extern "system" fn(usize,u16,u8,i32)->u8=std::mem::transmute(base+0x511450);
 assert_eq!(matching(node,BUFF),1);assert_eq!(category(node,20,0,BUFF),1);
 let refresh:unsafe extern "system" fn(usize,i32,f32,u8)->u8=std::mem::transmute(base+0x511700);
 refresh(node,identity,1.,0)
}
unsafe extern "system" fn max_hp(_:usize)->i32{BASE_HP.with(Cell::get)}
unsafe extern "system" fn hundred(_:usize)->i32{100}
unsafe extern "system" fn false_flag(_:usize)->u8{0}
unsafe extern "system" fn change_state(player:usize,state:u8,_:f32){ptr::write((player+0x710)as*mut u8,state);}
unsafe extern "system" fn died(player:usize){integer(native_actor::stats(player)+0x140,0);}
fn row(bytes:&[u8],identity:i32)->[u8;1024]{
 let count=u16::from_le_bytes(bytes[10..12].try_into().unwrap())as usize;
 for index in 0..count{
  let offset=64+index*24;
  if i32::from_le_bytes(bytes[offset..offset+4].try_into().unwrap())==identity{
   let start=u64::from_le_bytes(bytes[offset+8..offset+16].try_into().unwrap())as usize;return bytes[start..start+1024].try_into().unwrap();
  }
 }
 panic!("Missing effect {identity}");
}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE and NRHEX_CHALLENGE_FIXTURES; run alone"]
fn native_ultimate_challenge_hp_refresh_expiry_and_death(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();
 assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let params=std::fs::read(std::env::var_os("NRHEX_CHALLENGE_FIXTURES").unwrap()).unwrap();let buff=row(&params,BUFF);
 ROW.with(|slot|slot.set(buff.as_ptr()as usize));
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();
 let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 for(address,target)in [(0xea17e0,lookup as*const()as usize),(crate::HAS_EFFECT_RVA,has as*const()as usize),(crate::APPLY_SELF_RVA,apply as*const()as usize)]{jump(base,address,target);}
 patch(base,0x6ada30,&[0xb0,1,0xc3]);
 patch(base,0x5090b0,&[0x31,0xc0,0xc3]);patch(base,0x509000,&[0x31,0xc0,0xc3]);
 let mut splice=vec![0x31,0xc0,0xe9];splice.extend_from_slice(&((0x50b806isize-0x50b63e-7)as i32).to_le_bytes());patch(base,0x50b63e,&splice);
 let saved_base=crate::BASE.swap(base,Ordering::AcqRel);let saved_active=crate::ACTIVE.swap(true,Ordering::AcqRel);
 let saved_original=crate::APPLY_ORIG.swap(accepted as*mut u8,Ordering::AcqRel);IMAGE.with(|slot|slot.set(base));
 let mut world=vec![0usize;0x17500/8];let world=world.as_mut_ptr()as usize;pointer(base+crate::WORLD_RVA,world);
 let mut actor=vec![0usize;0x740/8];let player=actor.as_mut_ptr()as usize;pointer(world+0x174e8,player);
 let mut actor_vtable=vec![0usize;0x220/8];actor_vtable[0x120/8]=false_flag as*const()as usize;pointer(player,actor_vtable.as_ptr()as usize);
 let mut modules=vec![0usize;0x200/8];let modules=modules.as_mut_ptr()as usize;pointer(player+0x1b8,modules);
 let mut attributes=vec![0usize;0x200/8];let stats=attributes.as_mut_ptr()as usize;pointer(modules,stats);pointer(stats+8,player);
 let mut stats_vtable=vec![0usize;0x28/8];stats_vtable[0x10/8]=max_hp as*const()as usize;stats_vtable[0x18/8]=hundred as*const()as usize;stats_vtable[0x20/8]=hundred as*const()as usize;pointer(stats,stats_vtable.as_ptr()as usize);
 let mut manager=vec![0usize;8];let manager=manager.as_mut_ptr()as usize;pointer(player+0x190,manager);
 let mut effect_node=vec![0usize;0x88/8];let node=effect_node.as_mut_ptr()as usize;NODE.with(|slot|slot.set(node));
 integer(stats+0x140,600);integer(stats+0x144,1000);integer(stats+0x178,1000);
 let notify=|identity,actor|crate::effect_detour(actor,identity,actor,0,0,0,0,0,0,0,0);
 let cast=|identity,actor|{
  let result=notify(identity,actor);
  if actor==player&&identity==crate::runtime::ULTIMATE_TRIGGER{update(player,base,1./60.);}
  result
 };
 DEFER.with(|slot|slot.set(true));notify(crate::runtime::ULTIMATE_TRIGGER,player);
 assert_eq!((read_i32(stats+0x140),read_i32(stats+0x144)),(600,1000));
 for _ in 0..3{update(player,base,1./60.);assert_eq!(read_i32(stats+0x140),600);}
 DEFER.with(|slot|slot.set(false));commit(player,BUFF);
 update(player,base,1./60.);assert_eq!((read_i32(stats+0x140),read_i32(stats+0x144)),(1100,1500));
 update(player,base,1./60.);assert_eq!(read_i32(stats+0x140),1100);clear(player,base);
 integer(stats+0x140,1000);cast(crate::runtime::ULTIMATE_TRIGGER,player);
 assert_eq!((read_i32(stats+0x140),read_i32(stats+0x144)),(1500,1500));clear(player,base);
 DEFER.with(|slot|slot.set(true));notify(crate::runtime::ULTIMATE_TRIGGER,player);
 ptr::write((player+0x710)as*mut u8,1);update(player,base,1./60.);
 DEFER.with(|slot|slot.set(false));commit(player,BUFF);update(player,base,1./60.);
 assert_eq!(has(manager,BUFF),0);assert_eq!(read_i32(stats+0x140),1000);
 ptr::write((player+0x710)as*mut u8,0);update(player,base,1./60.);assert_eq!(read_i32(stats+0x140),1000);
 DEFER.with(|slot|slot.set(true));notify(crate::runtime::ULTIMATE_TRIGGER,player);
 update(player,base,0.5);update(player,base,0.5);
 DEFER.with(|slot|slot.set(false));commit(player,BUFF);update(player,base,1./60.);
 assert_eq!(read_i32(stats+0x140),1000);clear(player,base);
 integer(stats+0x140,600);APPLICATIONS.with(|slot|slot.set(0));
 cast(crate::runtime::ULTIMATE_ACTION,player);assert_eq!(APPLICATIONS.with(Cell::get),0);
 cast(12345,player);ACCEPT.with(|slot|slot.set(0));cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(APPLICATIONS.with(Cell::get),0);
 ACCEPT.with(|slot|slot.set(1));cast(crate::runtime::ULTIMATE_TRIGGER,player);
 assert_eq!((read_i32(stats+0x140),read_i32(stats+0x144)),(1100,1500));assert_eq!(value(node+0x4c),15.);
 let aggregate:unsafe extern "system" fn(usize,u8,u8)->f32=std::mem::transmute(base+0x50c1b0);
 assert_eq!(aggregate(manager,0,0),1.5);assert_eq!(aggregate(manager,1,0),1.);
 for remaining in [10.,1.,0.001]{
  integer(stats+0x140,600);float(node+0x4c,remaining);cast(crate::runtime::ULTIMATE_TRIGGER,player);
  assert_eq!((read_i32(stats+0x140),read_i32(stats+0x144)),(1100,1500));assert_eq!(value(node+0x4c),15.);
  assert_eq!(read_ptr(manager+8),node);assert_eq!(read_ptr(node+0x38),0);
 }
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);
 integer(stats+0x140,900);assert_eq!(remove(manager,BUFF),1);recalculate(stats,base);
 assert_eq!((read_i32(stats+0x140),read_i32(stats+0x144)),(900,1000));
 cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(read_i32(stats+0x140),1400);clear(player,base);
 assert_eq!((read_i32(stats+0x140),read_i32(stats+0x144)),(1000,1000));
 for down in [1u8,2]{
  cast(crate::runtime::ULTIMATE_TRIGGER,player);ptr::write((player+0x710)as*mut u8,down);update(player,base,1./60.);
  assert_eq!(has(manager,BUFF),0);assert_eq!(read_i32(stats+0x144),1000);
  let count=APPLICATIONS.with(Cell::get);cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(APPLICATIONS.with(Cell::get),count);
  ptr::write((player+0x710)as*mut u8,0);update(player,base,1./60.);assert_eq!(has(manager,BUFF),0);
 }
 cast(crate::runtime::ULTIMATE_TRIGGER,player);integer(stats+0x140,0);update(player,base,1./60.);
 assert_eq!(has(manager,BUFF),0);assert_eq!(read_i32(stats+0x140),0);
 integer(stats+0x140,300);recalculate(stats,base);update(player,base,1./60.);
 assert_eq!((read_i32(stats+0x140),read_i32(stats+0x144)),(300,1000));assert_eq!(has(manager,BUFF),0);
 cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!((read_i32(stats+0x140),read_i32(stats+0x144)),(800,1500));
 OWNED.with(|slot|slot.set(false));update(player,base,1./60.);assert_eq!(has(manager,BUFF),0);assert_eq!(read_i32(stats+0x140),800);
 let count=APPLICATIONS.with(Cell::get);cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(APPLICATIONS.with(Cell::get),count);
 OWNED.with(|slot|slot.set(true));RESET.with(|slot|slot.set(true));cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(APPLICATIONS.with(Cell::get),count);RESET.with(|slot|slot.set(false));
 FAIL.with(|slot|slot.set(true));cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(read_i32(stats+0x140),800);assert_eq!(has(manager,BUFF),0);FAIL.with(|slot|slot.set(false));
 cast(crate::runtime::ULTIMATE_TRIGGER,player+8);crate::ACTIVE.store(false,Ordering::Release);cast(crate::runtime::ULTIMATE_TRIGGER,player);
 assert_eq!(APPLICATIONS.with(Cell::get),count);crate::ACTIVE.store(true,Ordering::Release);
 BASE_HP.with(|slot|slot.set(1800));integer(stats+0x140,600);cast(crate::runtime::ULTIMATE_TRIGGER,player);
 assert_eq!((read_i32(stats+0x140),read_i32(stats+0x144)),(1980,2700));
 RESET.with(|slot|slot.set(true));update(player,base,1./60.);assert_eq!(has(manager,BUFF),0);assert_eq!(read_i32(stats+0x144),1800);RESET.with(|slot|slot.set(false));
 for notification in [102100,crate::balloon::DOWN]{
  cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(has(manager,BUFF),1);
  cast(notification,player);assert_eq!(has(manager,BUFF),0);assert_eq!(read_i32(stats+0x144),1800);
 }
 let saved_state=crate::balloon_runtime::STATE_ORIGINAL.swap(change_state as*mut u8,Ordering::AcqRel);
 cast(crate::runtime::ULTIMATE_TRIGGER,player);crate::balloon_runtime::state_detour(player,1,0.);
 assert_eq!(has(manager,BUFF),0);assert_eq!(read_i32(stats+0x144),1800);
 crate::balloon_runtime::state_detour(player,0,0.);update(player,base,1./60.);assert_eq!(has(manager,BUFF),0);
 crate::balloon_runtime::STATE_ORIGINAL.store(saved_state,Ordering::Release);
 let saved_death=crate::DEATH_ORIG.swap(died as*mut u8,Ordering::AcqRel);
 cast(crate::runtime::ULTIMATE_TRIGGER,player);crate::actor_lifecycle::death_detour(player);
 assert_eq!(has(manager,BUFF),0);assert_eq!(read_i32(stats+0x140),0);
 crate::DEATH_ORIG.store(saved_death,Ordering::Release);
 integer(stats+0x140,300);
 pointer(modules,0);let count=APPLICATIONS.with(Cell::get);cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(APPLICATIONS.with(Cell::get),count);
 crate::BASE.store(saved_base,Ordering::Release);crate::ACTIVE.store(saved_active,Ordering::Release);crate::APPLY_ORIG.store(saved_original,Ordering::Release);
 assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
