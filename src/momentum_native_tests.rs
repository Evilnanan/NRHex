//! Actual native on-hit selection, applicability, AttackRate getter and
//! multiplication instructions. Effect storage/replication is an isolated fixture.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
use std::{cell::RefCell,collections::BTreeSet};
thread_local!{static EFFECTS:RefCell<BTreeSet<i32>>=RefCell::new(BTreeSet::new());}
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn patch(base:usize,rva:usize,code:&[u8]){ptr::copy_nonoverlapping(code.as_ptr(),(base+rva)as*mut u8,code.len());}
unsafe fn jump(base:usize,rva:usize,target:usize){let mut code=vec![0x48,0xb8];code.extend_from_slice(&target.to_le_bytes());code.extend_from_slice(&[0xff,0xe0]);patch(base,rva,&code);}
unsafe extern "system" fn has(_:usize,id:i32)->u8{EFFECTS.with(|s|u8::from(s.borrow().contains(&id)))}
unsafe extern "system" fn apply(_:usize,id:i32,flag:u8)->u8{assert_eq!(flag,0);EFFECTS.with(|s|s.borrow_mut().insert(id));1}
unsafe extern "system" fn remove(_:usize,id:i32)->u8{EFFECTS.with(|s|s.borrow_mut().remove(&id));1}
unsafe extern "system" fn unexpected(_:usize,_:i32,_:usize,_:usize,_:usize,_:u8,_:u8,_:u8,_:u8,_:u8,_:i32)->u8{panic!("Private notification was installed instead of consumed")}
fn row(bytes:&[u8],id:i32)->[u8;1024]{
 assert_eq!(&bytes[0x2c..0x30],&[0,0x85,7,0]);let count=u16::from_le_bytes(bytes[10..12].try_into().unwrap())as usize;
 for i in 0..count{let at=64+i*24;if i32::from_le_bytes(bytes[at..at+4].try_into().unwrap())==id{let off=u64::from_le_bytes(bytes[at+8..at+16].try_into().unwrap())as usize;return bytes[off..off+1024].try_into().unwrap();}}
 panic!("Missing installed effect {id}")
}
// Re-enter exactly the native physical AttackRate multiplication fragment.
// Its correction factor is one, as for an unconditional passive effect.
core::arch::global_asm!(r#"
.global momentum_native_rate
momentum_native_rate:
 push rdi
 sub rsp,0x40
 movaps [rsp+0x20],xmm6
 movaps [rsp+0x30],xmm8
 mov rdi,rcx
 movaps xmm0,xmm1
 mov eax,0x3f800000
 movd xmm6,eax
 movaps xmm8,xmm6
 call r8
 movss xmm0,[rdi]
 movaps xmm6,[rsp+0x20]
 movaps xmm8,[rsp+0x30]
 add rsp,0x40
 pop rdi
 ret
"#);
unsafe extern "system"{fn momentum_native_rate(out:*mut f32,rate:f32,fragment:usize)->f32;}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE and NRHEX_MOMENTUM_FIXTURES; run alone"]
fn native_momentum_first_hit_and_unrefreshable_cycle_use_original_attack_rates(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let params=std::fs::read(std::env::var_os("NRHEX_MOMENTUM_FIXTURES").unwrap()).unwrap();
 let owner=row(&params,OWNER);let weak=row(&params,WEAK);let reference=row(&params,320500);
 assert_eq!(owner[0x3c4],0);assert_eq!(u16::from_le_bytes(owner[0x156..0x158].try_into().unwrap()),2103);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 // The native on-hit predicate uses a small absolute-RVA jump table.
 // Its LEA obtains this private image base without relying on PE relocation.
 patch(base,0x50314c,&[0xc3]);patch(base,0x26909a0,&[0xc3]);
 for rva in [0xea1b00,0xea1d40]{patch(base,rva,&[0xb0,1,0xc3]);}
 for(rva,target)in [(crate::HAS_EFFECT_RVA,has as*const()as usize),(crate::APPLY_SELF_RVA,apply as*const()as usize),(crate::REMOVE_EFFECT_RVA,remove as*const()as usize)]{jump(base,rva,target);}
 let saved_base=crate::BASE.swap(base,std::sync::atomic::Ordering::AcqRel);let saved_active=crate::ACTIVE.swap(true,std::sync::atomic::Ordering::AcqRel);
 let saved_apply=crate::APPLY_ORIG.swap(unexpected as*mut u8,std::sync::atomic::Ordering::AcqRel);
 let saved_cycle={let mut c=CYCLE.lock().unwrap();std::mem::replace(&mut *c,Cycle::new())};
 let filter=crate::hook::prepare((base+crate::EFFECT_FILTER_RVA)as*mut u8,crate::sword_filter_detour as*const u8,64).unwrap();let saved_filter=crate::EFFECT_FILTER_ORIG.swap(filter.trampoline,std::sync::atomic::Ordering::AcqRel);crate::hook::write_code((base+crate::EFFECT_FILTER_RVA)as*mut u8,&filter.patch).unwrap();
 let mut actor=[0usize;0x720/8];let player=actor.as_mut_ptr()as usize;let mut modules=[0usize;4];let modules=modules.as_mut_ptr()as usize;put(player+0x1b8,modules);
 let mut stats=[0usize;0x150/8];let stats=stats.as_mut_ptr()as usize;put(modules,stats);int(stats+0x140,1000);
 let mut world=vec![0usize;0x17500/8];let world=world.as_mut_ptr()as usize;put(base+crate::WORLD_RVA,world);put(world+0x174e8,player);
 let mut manager=[0usize;4];let manager=manager.as_mut_ptr()as usize;put(player+0x190,manager);put(manager+0x10,player);
 let mut node=[0usize;0x70/8];let node=node.as_mut_ptr()as usize;put(node,owner.as_ptr()as usize);put(manager+8,node);
 EFFECTS.with(|s|{s.borrow_mut().clear();s.borrow_mut().insert(OWNER);});
 let get:unsafe extern "system" fn(usize)->f32=std::mem::transmute(base+0x510750);
 let calc=|row:&[u8;1024],initial:f32|{let n=[row.as_ptr()as usize];let rate=get(n.as_ptr()as usize);let mut out=initial;momentum_native_rate(&mut out,rate,base+0x503137)};
 // The full-HP reference and our passive use precisely the same multiplier.
 assert!((calc(&reference,1.)-1.1).abs()<0.00001);
 let initial=calc(&owner,1.);assert!((initial-1.3).abs()<0.00001);assert_eq!(CYCLE.lock().unwrap().elapsed,None);
 let mut hit=[0u8;0x2c8];let h=hit.as_mut_ptr()as usize;int(h+0x280,130);hit[0x122]=1;int(h+0x40,10000);int(h+0x44,1);
 let mut ids=[0i32;8];let mut out=[0usize;4];let outp=out.as_mut_ptr()as usize;
 let select:unsafe extern "system" fn(usize,usize,usize)=std::mem::transmute(base+0x505e90);
 let mut notify=||{put(outp+8,ids.as_mut_ptr()as usize);put(outp+0x10,ids.as_mut_ptr()as usize);put(outp+0x18,ids.as_mut_ptr()as usize+32);select(manager,outp,h);let count=(read_ptr(outp+0x10)-read_ptr(outp+8))/4;for &id in &ids[..count]{assert_eq!(id,PULSE);assert_eq!(crate::effect_detour(player,id,player,0,0,0,0,0,0,0,0),1);}count};
 // Roll collisions never consume the first hit. All damage elements and
 // sorcery/incantation/shaman descriptors use the actual applicability switch.
 int(h+0x40,304);assert_eq!(notify(),0);assert_eq!(CYCLE.lock().unwrap().elapsed,None);int(h+0x40,10000);
 assert_eq!(notify(),1);assert_eq!(CYCLE.lock().unwrap().elapsed,Some(0.));assert!(EFFECTS.with(|s|s.borrow().contains(&VISUAL)));
 for _ in 0..4{update(player,base,1.);for kind in [1u8,2,3,4,10]{ptr::write((h+0x122)as*mut u8,kind);assert_eq!(notify(),1);}}
 assert_eq!(CYCLE.lock().unwrap().elapsed,Some(4.));assert!(!EFFECTS.with(|s|s.borrow().contains(&WEAK)));
 update(player,base,1.);assert!(EFFECTS.with(|s|s.borrow().contains(&WEAK)));assert!(!EFFECTS.with(|s|s.borrow().contains(&VISUAL)));
 let negative=calc(&weak,initial);assert!((negative-0.5).abs()<0.00001);
 for offset in [0x1c,0x20,0x24,0x28,0x2c,0x30,0x34]{assert_eq!(f32::from_le_bytes(weak[offset..offset+4].try_into().unwrap()),1.5);}
 for _ in 0..4{update(player,base,1.);assert_eq!(notify(),1);}assert_eq!(CYCLE.lock().unwrap().elapsed,Some(9.));
 update(player,base,1.);assert_eq!(CYCLE.lock().unwrap().elapsed,None);assert!(!EFFECTS.with(|s|s.borrow().contains(&WEAK)));
 assert_eq!(calc(&owner,1.),initial);assert_eq!(notify(),1);assert_eq!(CYCLE.lock().unwrap().elapsed,Some(0.));
 // Missing ownership, replicas and downed actors cannot start local cycles.
 reset(player,base);EFFECTS.with(|s|{s.borrow_mut().remove(&OWNER);});pulse(player,PULSE,base);assert_eq!(CYCLE.lock().unwrap().elapsed,None);
 EFFECTS.with(|s|{s.borrow_mut().insert(OWNER);});pulse(player+8,PULSE,base);int(stats+0x140,0);pulse(player,PULSE,base);assert_eq!(CYCLE.lock().unwrap().elapsed,None);int(stats+0x140,1000);
 assert!(pulse(player,PULSE,base));update(player,base,1.);reset(player,base);assert_eq!(CYCLE.lock().unwrap().elapsed,None);assert!(!EFFECTS.with(|s|s.borrow().contains(&WEAK)||s.borrow().contains(&VISUAL)));
 *CYCLE.lock().unwrap()=saved_cycle;crate::BASE.store(saved_base,std::sync::atomic::Ordering::Release);crate::ACTIVE.store(saved_active,std::sync::atomic::Ordering::Release);crate::APPLY_ORIG.store(saved_apply,std::sync::atomic::Ordering::Release);crate::EFFECT_FILTER_ORIG.store(saved_filter,std::sync::atomic::Ordering::Release);
 assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
