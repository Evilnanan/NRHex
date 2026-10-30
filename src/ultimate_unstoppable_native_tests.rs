//! Execute installed Endure reactions and real native same-ID candidate search,
//! list removal and duration initialization through the compiled ultimate hook.
use super::*;use crate::ptr;use crate::win::*;use sha2::{Digest,Sha256};
use std::{cell::Cell,sync::atomic::Ordering};
thread_local!{
 static OWNED:Cell<bool>=const{Cell::new(true)};static RESET:Cell<bool>=const{Cell::new(false)};
 static AWAKENED:Cell<bool>=const{Cell::new(false)};static ACCEPT:Cell<u8>=const{Cell::new(1)};static FAIL:Cell<bool>=const{Cell::new(false)};
 static IMAGE:Cell<usize>=const{Cell::new(0)};static NODE:Cell<usize>=const{Cell::new(0)};static FP_NODE:Cell<usize>=const{Cell::new(0)};
 static ROWS:Cell<usize>=const{Cell::new(0)};static APPLICATIONS:Cell<usize>=const{Cell::new(0)};
}
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn float(p:usize,v:f32){ptr::write_unaligned(p as*mut f32,v);}
unsafe fn value(p:usize)->f32{ptr::read_unaligned(p as*const f32)}
unsafe fn patch(base:usize,rva:usize,b:&[u8]){ptr::copy_nonoverlapping(b.as_ptr(),(base+rva)as*mut u8,b.len());}
unsafe fn jump(base:usize,rva:usize,f:usize){let mut b=vec![0x48,0xb8];b.extend_from_slice(&f.to_le_bytes());b.extend_from_slice(&[0xff,0xe0]);patch(base,rva,&b);}
unsafe fn branch(base:usize,at:usize,to:usize,prefix:&[u8]){let mut b=prefix.to_vec();b.push(0xe9);b.extend_from_slice(&((to as isize-at as isize-b.len()as isize-4)as i32).to_le_bytes());patch(base,at,&b);}
unsafe fn lookup(id:i32)->usize{let rows=std::slice::from_raw_parts(ROWS.with(Cell::get)as*const(i32,usize),3);rows.iter().find(|r|r.0==id).unwrap().1}
unsafe extern "system" fn row_lookup(out:usize,id:i32){put(out,lookup(id));int(out+8,id);}
unsafe extern "system" fn has(manager:usize,id:i32)->u8{
 if id==OWNER{return u8::from(OWNED.with(Cell::get));}
 if id==crate::ultimate_awakening::OWNER{return u8::from(AWAKENED.with(Cell::get));}
 if id==crate::reward_cycle_runtime::RESET{return u8::from(RESET.with(Cell::get));}
 let mut n=read_ptr(manager+8);while n!=0{if crate::read_i32(n+8)==id&&(crate::read_i32(n+0x68)as u32&0x800c0003)==0{return 1;}n=read_ptr(n+0x38);}0
}
unsafe extern "system" fn original(_:usize,id:i32,_:usize,_:usize,_:usize,_:u8,_:u8,_:u8,_:u8,_:u8,_:i32)->u8{if id==crate::runtime::ULTIMATE_TRIGGER{ACCEPT.with(Cell::get)}else{1}}
unsafe fn candidate(base:usize,manager:usize,id:i32)->i32{
 let f:unsafe extern "system" fn(usize,i32,usize,usize,u8,f32,u8,u8,i32)->i32=std::mem::transmute(base+0x50d060);f(manager,id,0,0,0,1.,0,0,-1)
}
unsafe extern "system" fn apply(player:usize,id:i32,flag:u8)->u8{
 assert_eq!(flag,0);if id==crate::ultimate_awakening::REFILL{return 1;}assert!(id==BUFF||id==crate::ultimate_awakening::BUFF);
 if FAIL.with(Cell::get){return 0;}assert_eq!(crate::effect_detour(player,id,player,0,0,0,0,0,0,0,0),1);
 let base=IMAGE.with(Cell::get);let manager=read_ptr(player+0x190);let node=if id==BUFF{APPLICATIONS.with(|s|s.set(s.get()+1));NODE.with(Cell::get)}else{FP_NODE.with(Cell::get)};
 if has(manager,id)==0{
  // Execute native collision selection before the allocation fixture. A
  // repeated category1001 ID must be removed; unrelated Endure stays separate.
  assert_eq!(candidate(base,manager,id),0);put(node,lookup(id));int(node+8,id);int(node+0x68,0);put(node+0x38,0);put(node+0x40,0);
  let mut last=read_ptr(manager+8);if last==0{put(manager+8,node);}else{while read_ptr(last+0x38)!=0{last=read_ptr(last+0x38);}put(last+0x38,node);put(node+0x40,last);}
 }
 let initialize:unsafe extern "system" fn(usize,i32,f32,u8)->u8=std::mem::transmute(base+0x511700);initialize(node,id,1.,0)
}
fn row(bytes:&[u8],id:i32)->[u8;1024]{
 let count=u16::from_le_bytes(bytes[10..12].try_into().unwrap())as usize;
 for i in 0..count{let at=64+i*24;if i32::from_le_bytes(bytes[at..at+4].try_into().unwrap())==id{let off=u64::from_le_bytes(bytes[at+8..at+16].try_into().unwrap())as usize;return bytes[off..off+1024].try_into().unwrap();}}panic!("Missing effect {id}")
}
#[test]fn unstoppable_is_available_to_every_hero_weapon_affinity_and_party(){
 let index=crate::hex_pool::OWNER_IDS.iter().position(|&id|id==OWNER).unwrap();
 for hero in std::iter::once(None).chain((0..=10).map(Some)){for affinity in [None,Some(1100),Some(1600),Some(1700),Some(57),Some(61)]{for party in [None,Some(1),Some(2),Some(3)]{
  let filtered=crate::reward_choices::pool_exclusions(crate::reward_choices::party_exclusions(crate::reward_choices::weapon_exclusions(crate::reward_choices::role_exclusions(Default::default(),hero),affinity),party));
  assert!(!filtered.contains(&index));let round=crate::reward_cycle::Round::new(&filtered,&Default::default());assert!(round.candidates.contains(&index));
  assert!(round.preferred([index,0,1]).unwrap().contains(&index));assert!(crate::reward_cycle::Single::new(&filtered,&Default::default(),&crate::reward_choices::mask([0,1])).candidates.contains(&index));
 }}}
 assert_eq!(crate::hex_pool::HISTORY_BITS[index],36);
}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE and NRHEX_UNSTOPPABLE_FIXTURES; run alone"]
fn native_ultimate_unstoppable_buildup_immunity(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let params=std::fs::read(std::env::var_os("NRHEX_UNSTOPPABLE_FIXTURES").unwrap()).unwrap();let buff=row(&params,BUFF);let endure=row(&params,1650);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 patch(base,0x26909a0,&[0xc3]);branch(base,0x42d801,0x42d988,&[]);
 let mut actor=[0usize;0x200/8];let actor=actor.as_mut_ptr()as usize;
 let mut manager=[0usize;8];let manager=manager.as_mut_ptr()as usize;put(actor+0x190,manager);
 let mut node=[0usize;0x88/8];let node=node.as_mut_ptr()as usize;put(manager+8,node);put(node,buff.as_ptr()as usize);int(node+8,BUFF);
 let mut resist=[0usize;0x100/8];let resist=resist.as_mut_ptr()as usize;put(resist+8,actor);
 let mut context=[0usize;0x48/8];let context=context.as_mut_ptr()as usize;
 let mut incoming=[0u8;1024];let incoming_ptr=incoming.as_mut_ptr()as usize;
 let wrapper=[incoming_ptr,0usize];
 let immunity:unsafe extern "system" fn(usize,u32)->u8=std::mem::transmute(base+0x509790);
 let accumulate:unsafe extern "system" fn(usize,usize,usize)->u8=std::mem::transmute(base+0x42d110);
 let states=[2u16,5,6,116,260,436,437];let offsets=[0xcc,0xd0,0xd4,0xd8,0x1a8,0x338,0x33c];
 let boluses=[3061,3071,3051,3090,3093,3056,3066];
 for index in 0..7{
  assert_eq!(immunity(manager,index as u32),1);
  ptr::write_unaligned((incoming_ptr+0x156)as*mut u16,states[index]);
  for amount in [1,20,100000,-99999]{for attack_context in [0u8,1]{
   ptr::write_bytes(context as*mut u8,0,0x48);float(context+0x38,1.);ptr::write((context+0x40)as*mut u8,attack_context);
   for multiplier in 0..7{float(context+multiplier*4,1.);}
   for meter in 0..7{int(resist+0x10+meter*4,1);int(resist+0x2c+meter*4,100);int(resist+0x64+meter*4,7);}
   int(resist+0xb8,0);int(incoming_ptr+offsets[index],amount);
   let before=std::slice::from_raw_parts(resist as*const u8,0x100).to_vec();let context_before=std::slice::from_raw_parts(context as*const u8,0x48).to_vec();
   assert_eq!(accumulate(resist,wrapper.as_ptr()as usize,context),1);
   assert_eq!(std::slice::from_raw_parts(resist as*const u8,0x100),before);
   assert_eq!(std::slice::from_raw_parts(context as*const u8,0x48),context_before);
  }}
  let bolus=row(&params,boluses[index]);let bolus_wrapper=[bolus.as_ptr()as usize,0usize];
  assert_eq!(accumulate(resist,bolus_wrapper.as_ptr()as usize,context),1);assert_eq!(crate::read_i32(resist+0x10+index*4),1);
  for mode in 0..3{
   put(node,if mode==0{endure.as_ptr()as usize}else{buff.as_ptr()as usize});int(node+0x68,if mode==1{2}else{0});put(manager+8,if mode==2{0}else{node});
   assert_eq!(immunity(manager,index as u32),0);
   ptr::write_bytes(context as*mut u8,0,0x48);float(context+0x38,1.);
   for multiplier in 0..7{float(context+multiplier*4,1.);}
   int(resist+0x10+index*4,50);int(incoming_ptr+offsets[index],10);
   assert_eq!(accumulate(resist,wrapper.as_ptr()as usize,context),1);
   assert_eq!(crate::read_i32(resist+0x10+index*4),40,"native meter write, status {index}, mode {mode}");
   accumulate(resist,bolus_wrapper.as_ptr()as usize,context);
   assert_eq!(crate::read_i32(resist+0x10+index*4),100,"native bolus removal, status {index}, mode {mode}");
  }
  put(node,buff.as_ptr()as usize);int(node+0x68,0);put(manager+8,node);
 }
 // Native reset is independent of the negative-buildup/bolus path. Preserve
 // every byte outside the seven meters, including proc counts/active flags.
 let clear:unsafe extern "system" fn(usize)=std::mem::transmute(base+CLEAR_RVA);
 for threshold in [-1,0,1,100,9999,i32::MAX]{for remaining in [0,1,50,9999]{
  for index in 0..7{int(resist+0x10+index*4,remaining);int(resist+0x2c+index*4,threshold);int(resist+0x64+index*4,7);}
  int(resist+0xb8,0x7f);
  let mut expected=std::slice::from_raw_parts(resist as*const u8,0x100).to_vec();
  for index in 0..7{expected[0x10+index*4..0x14+index*4].copy_from_slice(&threshold.max(0).to_le_bytes());}
  clear(resist);assert_eq!(std::slice::from_raw_parts(resist as*const u8,0x100),expected);
 }}
 assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE and NRHEX_UNSTOPPABLE_FIXTURES; run alone"]
fn native_ultimate_unstoppable_endure_reaction_and_private_replacement(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let params=std::fs::read(std::env::var_os("NRHEX_UNSTOPPABLE_FIXTURES").unwrap()).unwrap();let buff=row(&params,BUFF);let endure=row(&params,1650);let sword=row(&params,crate::sword::POISE);let fp=row(&params,crate::ultimate_awakening::BUFF);
 let rows=[(BUFF,buff.as_ptr()as usize),(1650,endure.as_ptr()as usize),(crate::ultimate_awakening::BUFF,fp.as_ptr()as usize)];ROWS.with(|s|s.set(rows.as_ptr()as usize));
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 jump(base,0xea17e0,row_lookup as*const()as usize);jump(base,crate::HAS_EFFECT_RVA,has as*const()as usize);jump(base,crate::APPLY_SELF_RVA,apply as*const()as usize);
 // Return collision/no-collision at the native add routine's original exits;
 // allocator/effect storage is a fixture, but all candidate predicates execute.
 branch(base,0x50d12b,0x50d557,&[0xbe,1,0,0,0]);branch(base,0x50d368,0x50d557,&[0x31,0xf6]);
 // Execute the native remove lookup and linked-list splice. Skip destruction
 // and category bookkeeping for stack-backed synthetic nodes.
 branch(base,0x50b63e,0x50b806,&[0x31,0xc0]);
 let saved_base=crate::BASE.swap(base,Ordering::AcqRel);let saved_active=crate::ACTIVE.swap(true,Ordering::AcqRel);let saved_original=crate::APPLY_ORIG.swap(original as*mut u8,Ordering::AcqRel);IMAGE.with(|s|s.set(base));
 let mut world=vec![0usize;0x17500/8];let world=world.as_mut_ptr()as usize;put(base+crate::WORLD_RVA,world);
 let mut a=[0usize;0x720/8];let player=a.as_mut_ptr()as usize;put(world+0x174e8,player);
 let mut modules=[0usize;0x180/8];let modules=modules.as_mut_ptr()as usize;put(player+0x1b8,modules);let mut stats=[0usize;0x180/8];let stats=stats.as_mut_ptr()as usize;put(modules,stats);int(stats+0x140,100);
 let mut resist=[0usize;0x100/8];let resist=resist.as_mut_ptr()as usize;
 put(resist,base+0x2c113c8);put(resist+8,player);put(modules+0x20,resist);
 let seed=||{for i in 0..7{int(resist+0x10+i*4,1);int(resist+0x2c+i*4,100+i as i32);int(resist+0x64+i*4,7);}int(resist+0xb8,0x7f);};
 let check=|cleared|{for i in 0..7{assert_eq!(crate::read_i32(resist+0x10+i*4),if cleared{100+i as i32}else{1});assert_eq!(crate::read_i32(resist+0x2c+i*4),100+i as i32);assert_eq!(crate::read_i32(resist+0x64+i*4),7);}assert_eq!(crate::read_i32(resist+0xb8),0x7f);};
 seed();
 let mut manager=[0usize;8];let manager=manager.as_mut_ptr()as usize;put(player+0x190,manager);
 let mut n=[0usize;0x88/8];let node=n.as_mut_ptr()as usize;NODE.with(|s|s.set(node));let mut fpn=[0usize;0x88/8];let fp_node=fpn.as_mut_ptr()as usize;FP_NODE.with(|s|s.set(fp_node));
 let mut en=[0usize;0x88/8];let end_node=en.as_mut_ptr()as usize;put(end_node,endure.as_ptr()as usize);int(end_node+8,1650);float(end_node+0x4c,1.25);
 let mut sn=[0usize;0x88/8];let sword_node=sn.as_mut_ptr()as usize;put(sword_node,sword.as_ptr()as usize);int(sword_node+8,crate::sword::POISE);float(sword_node+0x4c,-1.);
 let cast=|id,actor|crate::effect_detour(actor,id,actor,0,0,0,0,0,0,0,0);
 cast(12345,player);cast(crate::runtime::ULTIMATE_ACTION,player);assert_eq!(APPLICATIONS.with(Cell::get),0);ACCEPT.with(|s|s.set(0));assert_eq!(cast(crate::runtime::ULTIMATE_TRIGGER,player),0);assert_eq!(APPLICATIONS.with(Cell::get),0);ACCEPT.with(|s|s.set(1));check(false);
 cast(crate::runtime::ULTIMATE_TRIGGER,player);check(true);assert_eq!(APPLICATIONS.with(Cell::get),1);assert_eq!(value(node+0x4c),15.);assert_eq!(candidate(base,manager,BUFF),1);
 let reaction:unsafe extern "system" fn(usize,u8,u8)->u8=std::mem::transmute(base+0x507c40);
 let mut original_manager=[0usize;8];let reference=original_manager.as_mut_ptr()as usize;put(reference+8,end_node);
 for level in 0..=12{for flag in [0,1]{assert_eq!(reaction(manager,level,flag),reaction(reference,level,flag));}}
 let mut sm=[0usize;8];let sword_manager=sm.as_mut_ptr()as usize;put(sword_manager+8,sword_node);
 for level in 0..=12{for flag in [0,1]{assert_eq!(reaction(sword_manager,level,flag),reaction(reference,level,flag),"Sword/Endure mismatch at reaction {level}, flag {flag}");}}
 assert_eq!(reaction(sword_manager,3,0),1);assert_eq!(reaction(sword_manager,4,0),1);assert_eq!(reaction(reference,4,0),1);
 // A naive category20 conversion loses the real no-stagger reaction entirely.
 let mut wrong=buff;wrong[0x13e..0x140].copy_from_slice(&20u16.to_le_bytes());put(node,wrong.as_ptr()as usize);assert_eq!(reaction(manager,1,0),0);put(node,buff.as_ptr()as usize);assert_eq!(reaction(manager,1,0),1);
 // Add independent vanilla Endure and Sword. Repeated ultimates replace only
 // our one node; their row IDs, durations and eligibility are preserved.
 put(node+0x38,end_node);put(end_node+0x40,node);put(end_node+0x38,sword_node);put(sword_node+0x40,end_node);
 for remaining in [14.,3.,0.001]{seed();float(node+0x4c,remaining);cast(crate::runtime::ULTIMATE_TRIGGER,player);check(true);assert_eq!(value(node+0x4c),15.);assert_eq!(value(end_node+0x4c),1.25);assert_eq!(value(sword_node+0x4c),-1.);
  let mut p=read_ptr(manager+8);let mut count=0;let mut ours=0;while p!=0{count+=1;ours+=usize::from(crate::read_i32(p+8)==BUFF);p=read_ptr(p+0x38);assert!(count<5);}assert_eq!((count,ours),(3,1));
 }
 // An expired node is invisible to has_effect but still blocks native add.
 int(node+0x68,2);assert_eq!(has(manager,BUFF),0);assert_eq!(candidate(base,manager,BUFF),1);cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(has(manager,BUFF),1);assert_eq!(value(node+0x4c),15.);
 // Coexistence with Ultimate Awakening keeps native FP protection independent.
 AWAKENED.with(|s|s.set(true));cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(value(fp_node+0x4c),7.);assert_eq!(value(node+0x4c),15.);let fp_rate:unsafe extern "system" fn(usize)->f32=std::mem::transmute(base+0x507310);assert_eq!(fp_rate(manager),0.);AWAKENED.with(|s|s.set(false));
 for(owned,reset,hp,down)in [(false,false,100,0),(true,true,100,0),(true,false,0,0),(true,false,100,1),(true,false,100,2)]{
  seed();OWNED.with(|s|s.set(owned));RESET.with(|s|s.set(reset));int(stats+0x140,hp);ptr::write((player+0x710)as*mut u8,down);let count=APPLICATIONS.with(Cell::get);float(node+0x4c,7.);cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(APPLICATIONS.with(Cell::get),count);assert_eq!(value(node+0x4c),7.);check(false);
 }
 OWNED.with(|s|s.set(true));RESET.with(|s|s.set(false));int(stats+0x140,100);ptr::write((player+0x710)as*mut u8,0);let count=APPLICATIONS.with(Cell::get);
 seed();cast(crate::runtime::ULTIMATE_TRIGGER,player+8);crate::ACTIVE.store(false,Ordering::Release);cast(crate::runtime::ULTIMATE_TRIGGER,player);crate::ACTIVE.store(true,Ordering::Release);assert_eq!(APPLICATIONS.with(Cell::get),count);check(false);
 FAIL.with(|s|s.set(true));cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(has(manager,BUFF),0);FAIL.with(|s|s.set(false));cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(value(node+0x4c),15.);
 for invalid in [0usize,1,2]{
  seed();put(modules+0x20,if invalid==0{0}else{resist});
  put(resist,if invalid==1{0}else{base+0x2c113c8});put(resist+8,if invalid==2{0}else{player});
  cast(crate::runtime::ULTIMATE_TRIGGER,player);check(false);
 }
 put(modules+0x20,resist);put(resist,base+0x2c113c8);put(resist+8,player);
 // Native expiry removes the finite effect; list storage/removal is exercised
 // above, while elapsed game-time updates themselves are outside this fixture.
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);assert_eq!(remove(manager,BUFF),1);assert_eq!(has(manager,1650),1);assert_eq!(has(manager,crate::sword::POISE),1);
 for offset in [0x1c,0x20,0x24,0x28,0x2c,0x30,0x34]{assert_eq!(value(buff.as_ptr()as usize+offset),1.);}
 put(modules,0);let count=APPLICATIONS.with(Cell::get);cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(APPLICATIONS.with(Cell::get),count);put(modules,stats);put(player+0x190,0);cast(crate::runtime::ULTIMATE_TRIGGER,player);assert_eq!(APPLICATIONS.with(Cell::get),count);
 crate::BASE.store(saved_base,Ordering::Release);crate::ACTIVE.store(saved_active,Ordering::Release);crate::APPLY_ORIG.store(saved_original,Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
