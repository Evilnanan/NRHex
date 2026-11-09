//! Submit the wave's original victim through native explicit-target contact.
use crate::{ptr,read_ptr,read_i32,ACTIVE,BASE};
use std::sync::atomic::{AtomicPtr,Ordering};
pub const RVA:usize=0x535260;
pub const PATTERN:&str="48 8B C4 48 89 58 10 48 89 70 18 48 89 78 20 55 41 54 41 55 41 56 41 57 48 8D A8 F8 FC FF FF 48 81 EC E0 03 00 00 0F 29 70 C8 0F 29 78 B8 44 0F 29 40 A8";
pub const CONTACT_RVA:usize=0x5363a0;
pub const CONTACT_PATTERN:&str="4D 85 C0 0F 84 B7 04 00 00 48 8B C4 55 53 56 57 41 54 41 55 41 56 41 57";
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
#[repr(C,align(16))]struct Point([f32;4]);
unsafe fn submit(attack:usize,base:usize){
 if attack==0||read_i32(attack+0x78)!=crate::soul_critical::ATTACK||read_i32(attack+0x7c)!=1
  ||read_ptr(attack+8)==0||read_i32(attack+0x1ec)>=0{return;}
 let resolve:unsafe extern "system" fn(u64)->usize=std::mem::transmute(base+crate::CHR_HANDLE_RVA);
 let bullet=resolve(read_ptr(attack+0x30)as u64);
 let pending=crate::static_charge::pending_soul_victim(bullet);
 let Some(victim)=pending else{return;};
 let target=resolve(victim);
 if target==0||read_ptr(target+8)as u64!=victim||!crate::spatial_ready(target){return;}
 let point=Point(crate::position(target,base));
 if !point.0[..3].iter().all(|v|v.is_finite()){return;}
 let normal=Point([0.;4]);let mut part=-1i8;
 // Same arguments as native 535260's +1E0 bound-target branch. In particular
 // part=-1 avoids dependence on a body-part contact produced by spatial search.
 // 5363A0 builds/owns the complete hit and routes the bullet through 33C120.
 let contact:unsafe extern "system" fn(usize,usize,usize,usize,usize,i32,i32,*mut i8)=std::mem::transmute(base+CONTACT_RVA);
 contact(attack,bullet,target,&point as*const Point as usize,&normal as*const Point as usize,0,-1,&mut part);
}
pub unsafe extern "system" fn update(attack:usize){
 if ACTIVE.load(Ordering::Acquire){submit(attack,BASE.load(Ordering::Acquire));}
 // Preserve normal area contacts and native lifetime/bookkeeping unchanged.
 let original:unsafe extern "system" fn(usize)=std::mem::transmute(ORIGINAL.load(Ordering::Acquire));
 original(attack);
}

#[cfg(test)]pub unsafe fn native_fixture(base:usize){
 use std::cell::{Cell,RefCell};
 thread_local!{
  static OBJECTS:Cell<(usize,usize)>=const{Cell::new((0,0))};
  static EVENTS:RefCell<Vec<(u8,u8,f32,u8)>>=const{RefCell::new(Vec::new())};
  static UPDATES:Cell<usize>=const{Cell::new(0)};
 }
 unsafe fn patch(base:usize,rva:usize,fun:usize){let mut b=vec![0x48,0xb8];b.extend_from_slice(&fun.to_le_bytes());b.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(b.as_ptr(),(base+rva)as*mut u8,b.len());}
 unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
 unsafe extern "system" fn resolve(h:u64)->usize{OBJECTS.with(|s|{let(a,b)=s.get();match h{111=>b,123=>a,_=>0}})}
 unsafe extern "system" fn owner(m:usize)->usize{read_ptr(m+8)}
 unsafe extern "system" fn kind(_:usize)->i32{1}
 unsafe extern "system" fn yes(_:usize)->u8{1}
 unsafe extern "system" fn zero(_:usize,_:usize)->usize{0}
 unsafe extern "system" fn copy(out:usize,src:usize)->usize{ptr::copy_nonoverlapping(src as*const u8,out as*mut u8,0x2d0);out}
 unsafe extern "system" fn nop(_:usize){}
 #[unsafe(naked)]unsafe extern "system" fn cookie(){core::arch::naked_asm!("ret");}
 unsafe extern "system" fn reaction(_:usize,_:usize,_:usize,_:u8,_:u8){}
 unsafe extern "system" fn dispatch(_:usize,_:usize,hit:usize,_:u8,extra:u8){
  EVENTS.with(|s|s.borrow_mut().push((ptr::read((hit+0x121)as*const u8),extra,ptr::read(hit as*const f32),ptr::read((hit+0x24)as*const u8))));
 }
 unsafe extern "system" fn gate_contact(hit:usize,target:usize,_:usize)->u8{
  let module=read_ptr(read_ptr(target+0x1b8)+0x98);
  let f:unsafe extern "system" fn(usize,usize,usize,u8)=std::mem::transmute(BASE.load(Ordering::Acquire)+0x436530);
  let before=EVENTS.with(|s|s.borrow().len());f(module,0,hit,0);
  u8::from(EVENTS.with(|s|s.borrow().len())>before)
 }
 unsafe extern "system" fn route(hit:usize)->u8{
  assert_eq!(read_i32(hit+0x40),crate::soul_critical::ATTACK);
  assert_eq!(read_i32(hit+0x44),1);assert_eq!(ptr::read((hit+0x121)as*const u8),0);
  assert_eq!(read_i32(hit+0x250),0);assert_eq!(read_i32(hit+0x260),-1);
  let (target,bullet)=OBJECTS.with(Cell::get);
  assert_eq!(read_ptr(hit+0x228),bullet);assert_eq!(read_ptr(hit+0x230),target);
  // Exercise the production shared-entry chain, not static_charge directly.
  crate::devils_gaze_hit::detour(hit,target,bullet)
 }
 unsafe extern "system" fn normal_update(_:usize){UPDATES.with(|s|s.set(s.get()+1));}
 crate::static_charge::reset();EVENTS.with(|s|s.borrow_mut().clear());UPDATES.with(|s|s.set(0));
 let old_contact=crate::erosion::CONTACT_ORIGINAL.load(Ordering::Acquire);
 let old_gaze=crate::devils_gaze_hit::ORIGINAL.load(Ordering::Acquire);
 crate::link_contact_hooks(gate_contact as*const()as*mut u8);
 for(rva,f)in [(crate::CHR_HANDLE_RVA,resolve as*const()as usize),(0x42c120,owner as*const()as usize),
  (0x427260,yes as*const()as usize),(0x39bd20,zero as*const()as usize),
  (0x33ba30,copy as*const()as usize),(0x490910,zero as*const()as usize),
  (0x438610,reaction as*const()as usize),(0x43d260,dispatch as*const()as usize),
  (0x26909a0,cookie as*const()as usize),(0x52cef0,route as*const()as usize),(0x5361f0,nop as*const()as usize)]{patch(base,rva,f);}
 let prepared=crate::hook::prepare((base+crate::soul_immunity::GATE_RVA)as*mut u8,crate::soul_immunity::gate_detour as*const u8,64).unwrap();
 assert_eq!(prepared.stolen,7);
 let previous_resume=crate::soul_immunity::GATE_RESUME.swap((base+crate::soul_immunity::GATE_RVA+7)as*mut u8,Ordering::AcqRel);
 crate::hook::write_code((base+crate::soul_immunity::GATE_RVA)as*mut u8,&prepared.patch).unwrap();
 let previous=ORIGINAL.swap(normal_update as*const()as*mut u8,Ordering::AcqRel);
 let mut actor=[0usize;0x800/8];let a=actor.as_mut_ptr()as usize;
 let mut modules=[0usize;0x200/8];let mut behavior=[0u8;0x300];let mut physics=[0f32;0x90/4];
 let module=[0usize,a];let m=module.as_ptr()as usize;let mut vt=[0usize;0x120/8];vt[2]=kind as*const()as usize;vt[0x118/8]=zero as*const()as usize;
 actor[0]=vt.as_ptr()as usize;actor[1]=123;actor[0x60/8]=a;actor[0x1b8/8]=modules.as_ptr()as usize;
 modules[1]=behavior.as_mut_ptr()as usize;modules[0x68/8]=physics.as_mut_ptr()as usize;modules[0x98/8]=m;
 behavior[0x58]=8;physics[0x70/4]=2.;
 #[repr(C,align(16))]struct Aligned([u8;0x400]);
 let mut attack=Aligned([0;0x400]);let p=attack.0.as_mut_ptr()as usize;
 ptr::write_unaligned((p+8)as*mut usize,1);ptr::write_unaligned((p+0x30)as*mut u64,111);
 int(p+0x78,crate::soul_critical::ATTACK);int(p+0x7c,1);int(p+0x1ec,-1);
 ptr::write_unaligned((p+0x38)as*mut[f32;5],crate::soul_critical::components(900.));
 let mut bullet=[0usize;0x300/8];let b=bullet.as_mut_ptr()as usize;bullet[1]=111;int(b+0xa4,crate::soul_critical::BULLET);
 OBJECTS.with(|s|s.set((a,b)));
 let mut payload=[0u8;0x220];let pp=payload.as_mut_ptr()as usize;int(pp+4,crate::soul_critical::BULLET);
 let v=crate::soul_immunity::encode(123);ptr::write_unaligned((pp+0x124)as*mut[f32;4],[900.,v[0],v[1],v[2]]);
 {let _f=crate::static_charge::factory(pp);crate::static_charge::bound(b);}
 // Unscoped native damage still rejects a normal attack during throw-only protection.
 let mut hit=[0u8;0x2d0];let h=hit.as_mut_ptr()as usize;int(h+0x40,crate::soul_critical::ATTACK);int(h+0x44,1);
 assert_eq!(gate_contact(h,a,b),0);
 {let _outer=crate::soul_immunity::contact(Some((a,h)));
  {let _nested=crate::soul_immunity::contact(None);assert_eq!(gate_contact(h,a,b),0);}
  assert_eq!(gate_contact(h,a,b),1);
 }
 assert_eq!(hit[0x121],0);assert_eq!(behavior[0x58],8);EVENTS.with(|s|s.borrow_mut().clear());
 // Real 5363A0 creates the full hit; real 436530 and the mid-hook select the
 // native protected continuation, without changing damage classification.
 update(p);assert_eq!(EVENTS.with(|s|s.borrow().clone()),[(0,1,562.5,7)]);
 assert_eq!(crate::static_charge::pending_soul_victim(b),None);
 update(p);assert_eq!(EVENTS.with(|s|s.borrow().len()),1);assert_eq!(UPDATES.with(Cell::get),2);
 assert_eq!(crate::static_charge::contact(gate_contact,h,a,b),0); // area contact cannot hit twice
 assert_eq!(behavior[0x58],8);assert_eq!(attack.0[0x38+0x121],0);
 // A new wave is independent; stale targets and non-private attacks are not submitted.
 {let _f=crate::static_charge::factory(pp);crate::static_charge::bound(b);}
 actor[1]=124;update(p);assert_eq!(EVENTS.with(|s|s.borrow().len()),1);actor[1]=123;
 int(p+0x78,crate::static_charge::ATTACK);update(p);assert_eq!(EVENTS.with(|s|s.borrow().len()),1);
 physics[0x70/4]=6.;
 int(p+0x78,crate::soul_critical::ATTACK);update(p);assert_eq!(EVENTS.with(|s|s.borrow().len()),2);
 assert_eq!(EVENTS.with(|s|s.borrow().last().copied()),Some((0,1,562.5,5)));
 let override_level:unsafe extern "system" fn(u8)->i32=std::mem::transmute(base+0xe6d510);
 assert_eq!(override_level(6),7);assert_eq!(override_level(10),5);
 ORIGINAL.store(previous,Ordering::Release);crate::soul_immunity::GATE_RESUME.store(previous_resume,Ordering::Release);
 crate::erosion::CONTACT_ORIGINAL.store(old_contact,Ordering::Release);
 crate::devils_gaze_hit::ORIGINAL.store(old_gaze,Ordering::Release);
 crate::static_charge::reset();
}
