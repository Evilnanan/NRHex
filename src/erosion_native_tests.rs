//! Execute native immunity, attack-added selection and motion-value instructions
//! with the compiled Erosion hook in an isolated image. Never access a process.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
use std::sync::Mutex;
unsafe fn patch(base:usize,rva:usize,bytes:&[u8]){ptr::copy_nonoverlapping(bytes.as_ptr(),(base+rva)as*mut u8,bytes.len());}
unsafe fn pointer(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn integer(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn float(p:usize,v:f32){ptr::write_unaligned(p as*mut f32,v);}
static CALLS:Mutex<Vec<(usize,i32,usize,usize,usize)>>=Mutex::new(Vec::new());
unsafe extern "system" fn receiver(t:usize,e:i32,s:usize,b:u8,d:usize,c:usize,x:u8,y:u8,z:u8,j:u8,k:u8,l:u8,n:i32)->u8{
 assert_eq!((b,x,y,z,j,k,l,n),(0x81,0x82,0x83,0x84,0x85,0x86,0x87,-99));
 CALLS.lock().unwrap().push((t,e,s,d,c));0x91
}
unsafe extern "system" fn character(_:usize)->i32{1}
static MANAGER:std::sync::atomic::AtomicUsize=std::sync::atomic::AtomicUsize::new(0);
unsafe extern "system" fn contact_receiver(descriptor:usize,out:usize,_:usize)->u8{
 super::collect(MANAGER.load(Ordering::Acquire),out,descriptor);0x97
}

// Set up the native fragment's RBP/R14/XMM6 register contract while respecting
// the Win64 nonvolatile ABI. The fragment exits at the replaced next instruction.
core::arch::global_asm!(r#"
.global erosion_native_motion
erosion_native_motion:
 push rbp
 push r14
 sub rsp, 0x68
 movaps [rsp+0x20], xmm6
 lea rbp, [rsp+0x20]
 mov r14, rcx
 mov rax, rdx
 movaps xmm6, xmm2
 call r9
 movss xmm0, [rbp+0x38]
 movaps xmm6, [rsp+0x20]
 add rsp, 0x68
 pop r14
 pop rbp
 ret
"#);
unsafe extern "system"{fn erosion_native_motion(hit:usize,row:usize,generic:f32,code:usize)->f32;}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; run alone against the supported original EXE"]
fn compiled_erosion_and_native_ailment_paths_execute_together(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").expect("NRHEX_NATIVE_EXE")).unwrap();
 assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();
 let allocation=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!allocation.is_null());let base=allocation as usize;
 for s in &headers.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 // Only the security-cookie check is removed from the native immunity chain.
 patch(base,0x26909a0,&[0xc3]);
 let mut actor=vec![0u64;0x720/8];let target=actor.as_mut_ptr()as usize;
 let mut source=vec![0u64;0x720/8];let source=source.as_mut_ptr()as usize;
 let mut vtable=vec![0usize;8];vtable[2]=character as*const()as usize;pointer(source,vtable.as_ptr()as usize);
 let mut modules=vec![0u64;0x200/8];let modules=modules.as_mut_ptr()as usize;
 let mut stats=vec![0u64;0x200/8];let stats=stats.as_mut_ptr()as usize;
 let mut resist=vec![0u64;0x100/8];let resist=resist.as_mut_ptr()as usize;
 let mut manager=vec![0u64;0x30/8];let manager=manager.as_mut_ptr()as usize;
 let mut node=vec![0u64;0x80/8];let node=node.as_mut_ptr()as usize;
 let mut row=vec![0u64;0x400/8];let row=row.as_mut_ptr()as usize;
 let mut context=vec![0u64;0x48/8];let context=context.as_mut_ptr()as usize;
 let mut hit=vec![0u64;0x300/8];let hit=hit.as_mut_ptr()as usize;
 pointer(target+0x1b8,modules);pointer(modules,stats);pointer(modules+0x20,resist);pointer(resist,base+0x2c113c8);integer(stats+0x140,1000);
 pointer(target+0x190,manager);pointer(source+0x190,source);pointer(manager+8,node);pointer(node,row);float(context+0x38,1.);
 let immunity:unsafe extern "system" fn(usize,u32)->u8=std::mem::transmute(base+IMMUNE_RVA);
 let flags=[(0x163,4u8),(0x163,8),(0x163,16),(0x163,32),(0x259,8),(0x15f,128),(0x160,1)];
 for (i,(offset,mask))in flags.iter().enumerate(){
  ptr::write((row+offset)as*mut u8,*mask);
  for j in 0..7{assert_eq!(immunity(manager,j),u8::from(i==j as usize));}
  // Invalid/inactive effects in the native linked list must not confer immunity.
  integer(node+0x68,1);assert_eq!(immunity(manager,i as u32),0);integer(node+0x68,0);
  ptr::write((row+offset)as*mut u8,0);
 }
 // Execute the real native stateInfo152 selector and applicability filters;
 // provide existing vector capacity so the game's allocator is never entered.

 let collect:unsafe extern "system" fn(usize,usize,usize)=std::mem::transmute(base+0x506200);
 let mut values=[0i32;4];let start=values.as_mut_ptr()as usize;
 let mut vector=[0usize;4];vector[1]=start;vector[2]=start;vector[3]=start+16;
 ptr::write_unaligned((row+0x156)as*mut u16,152);integer(row+0x12c,CARRIER);
 collect(manager,vector.as_mut_ptr()as usize,context);assert_eq!(values[0],CARRIER);assert_eq!(vector[2],start+4);
 for (kind,offset,mask)in [(3u8,0x160,128u8),(4,0x161,1),(10,0x259,32)]{
  ptr::write(context as*mut u8,kind);vector[2]=start;collect(manager,vector.as_mut_ptr()as usize,context);assert_eq!(vector[2],start);
  ptr::write((row+offset)as*mut u8,mask);collect(manager,vector.as_mut_ptr()as usize,context);assert_eq!(vector[2],start+4);
  ptr::write((row+offset)as*mut u8,0);
 }
 ptr::write(context as*mut u8,0);

 // Reuse the installed shared 511010 detour, then execute the actual native
 // descriptor builder and state152 collector for roll/body and real attacks.
 let filter=crate::hook::prepare((base+crate::EFFECT_FILTER_RVA)as*mut u8,crate::sword_filter_detour as*const u8,64).unwrap();
 crate::EFFECT_FILTER_ORIG.store(filter.trampoline,Ordering::Release);
 crate::hook::write_code((base+crate::EFFECT_FILTER_RVA)as*mut u8,&filter.patch).unwrap();
 let collector=crate::hook::prepare((base+COLLECT_RVA)as*mut u8,super::collect as*const u8,64).unwrap();
 COLLECT_ORIGINAL.store(collector.trampoline,Ordering::Release);
 crate::hook::write_code((base+COLLECT_RVA)as*mut u8,&collector.patch).unwrap();
 let active_before=ACTIVE.swap(true,Ordering::AcqRel);
 let describe:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(base+crate::sword_regain::DESCRIPTOR_RVA);
 for (attack,table,blocked)in [(304,1,true),(307,1,true),(301,1,true),(550,1,true),(560,1,true),
   (304,0,false),(304,2,false),(10000,1,false),(300208910,1,false),(303401305,1,false),(40000,1,false)]{
  integer(hit+0x40,attack);integer(hit+0x44,table);describe(context,hit);
  vector[2]=start;collect(manager,vector.as_mut_ptr()as usize,context);
  assert_eq!(vector[2],if blocked{start}else{start+4},"attack {attack} table {table}");
 }
 // A native on-hit ailment owner using the same body contact remains intact.
 integer(context+0x34,10000);integer(context+0x38,1);
 let applies:unsafe extern "system" fn(usize,usize,usize,i32,i32)->u8=std::mem::transmute(base+crate::EFFECT_FILTER_RVA);
 for (kind,offset,mask)in [(3u8,0x160,128u8),(4,0x161,1),(10,0x259,32)]{
  ptr::write(context as*mut u8,kind);ptr::write((row+offset)as*mut u8,mask);
  vector[2]=start;collect(manager,vector.as_mut_ptr()as usize,context);assert_eq!(vector[2],start);
  assert_eq!(applies(node,context,0,-1,-1),1);
  integer(row+0x12c,8110002);collect(manager,vector.as_mut_ptr()as usize,context);assert_eq!(vector[2],start+4);assert_eq!(values[0],8110002);
  integer(row+0x12c,CARRIER);ACTIVE.store(false,Ordering::Release);vector[2]=start;
  collect(manager,vector.as_mut_ptr()as usize,context);assert_eq!(vector[2],start+4);
  ACTIVE.store(true,Ordering::Release);ptr::write((row+offset)as*mut u8,0);
 }

 // Execute the compiled BulletIns collision hook around the real native
 // collector. Only the continuation dispatch is a fixture; no world/AI runs.
 MANAGER.store(manager,Ordering::Release);
 let collision=crate::hook::prepare((base+CONTACT_RVA)as*mut u8,super::contact as*const u8,64).unwrap();
 CONTACT_ORIGINAL.store(contact_receiver as*mut u8,Ordering::Release);
 crate::hook::write_code((base+CONTACT_RVA)as*mut u8,&collision.patch).unwrap();
 let collide:unsafe extern "system" fn(usize,usize,usize)->u8=std::mem::transmute(base+CONTACT_RVA);
 let mut projectile=[0u64;0x300/8];let bullet=projectile.as_mut_ptr()as usize;
 integer(context+0x34,10308000);integer(context+0x38,1);
 ptr::write(context as*mut u8,0);
 for good in [-1,300,3080,3010,3011]{
  integer(bullet+0x2c0,good);values[0]=CARRIER;vector[2]=start+4;
  assert_eq!(collide(context,vector.as_mut_ptr()as usize,bullet),0x97);
  assert_eq!(vector[2],start+8);assert_eq!(values[0],CARRIER,"Existing vector entries stay intact");
  assert_eq!(values[1],if good>=0{ITEM_CARRIER}else{CARRIER});
  assert!(!ITEM_CONTACT.with(Cell::get));
 }
 integer(bullet+0x2c0,3080);
 for(kind,offset,mask)in [(3u8,0x160,128u8),(4,0x161,1),(10,0x259,32)]{
  ptr::write(context as*mut u8,kind);ptr::write((row+offset)as*mut u8,mask);vector[2]=start;
  collide(context,vector.as_mut_ptr()as usize,bullet);assert_eq!(vector[2],start,"Spell eligibility still precedes the item tier");ptr::write((row+offset)as*mut u8,0);
 }
 ptr::write(context as*mut u8,0);integer(row+0x12c,8110002);vector[2]=start;
 collide(context,vector.as_mut_ptr()as usize,bullet);assert_eq!(values[0],8110002,"Other native ailment additions stay intact");integer(row+0x12c,CARRIER);
 ACTIVE.store(false,Ordering::Release);vector[2]=start;collide(context,vector.as_mut_ptr()as usize,bullet);assert_eq!(values[0],CARRIER);ACTIVE.store(true,Ordering::Release);
 ptr::write(context as*mut u8,0);
 integer(hit+0x40,304);integer(hit+0x44,1);describe(context,hit);
 integer(row+0x12c,8110002);vector[2]=start;collect(manager,vector.as_mut_ptr()as usize,context);
 assert_eq!(vector[2],start+4);assert_eq!(values[0],8110002);integer(row+0x12c,CARRIER);
 ACTIVE.store(false,Ordering::Release);vector[2]=start;collect(manager,vector.as_mut_ptr()as usize,context);assert_eq!(vector[2],start+4);
 // Sword exception executes through the actual 506200 -> 511010 chain.
 // The collector must forward the source manager's owner, and only mapped
 // local sorcery/incantation contacts may bypass Erosion's spell exclusion.
 crate::SWORD_BONUSES.get_or_init(crate::sword::Bonuses::defaults);
 let mapped=crate::sword_map::ATTACKS[0].0;
 let mut world=vec![0usize;0x17500/8];let world=world.as_mut_ptr()as usize;
 let previous_world=read_ptr(base+crate::WORLD_RVA);
 pointer(base+crate::WORLD_RVA,world);pointer(world+0x174e8,source);pointer(manager+0x10,source);
 let saved_base=BASE.swap(base,Ordering::AcqRel);
 let lookup=std::slice::from_raw_parts((base+crate::HAS_EFFECT_RVA)as*const u8,10).to_vec();
 let mut sword_owned=vec![0x81,0xfa];sword_owned.extend_from_slice(&crate::sword::OWNER.to_le_bytes());sword_owned.extend_from_slice(&[0x0f,0x94,0xc0,0xc3]);
 ACTIVE.store(true,Ordering::Release);patch(base,crate::HAS_EFFECT_RVA,&sword_owned);
 for (kind,offset,mask)in [(3u8,0x160,128u8),(4,0x161,1)]{
  ptr::write(context as*mut u8,kind);integer(context+0x34,mapped);integer(context+0x38,1);
  ptr::write((row+offset)as*mut u8,mask);vector[2]=start;
  let descriptor=std::slice::from_raw_parts(context as*const u8,0x3c).to_vec();
  for count in 1..=3{
   collect(manager,vector.as_mut_ptr()as usize,context);
   assert_eq!(vector[2],start+count*4);assert_eq!(values[count-1],SPELL_CARRIER,"One 10-point spell carrier per contact");
  }
  assert_eq!(std::slice::from_raw_parts(context as*const u8,0x3c),descriptor,"Shared spell descriptor unchanged");
  integer(context+0x34,-1);vector[2]=start;collect(manager,vector.as_mut_ptr()as usize,context);assert_eq!(vector[2],start,"Unmapped spell excluded");integer(context+0x34,mapped);
  integer(context+0x38,0);collect(manager,vector.as_mut_ptr()as usize,context);assert_eq!(vector[2],start,"Attack table must match Sword");integer(context+0x38,1);
  pointer(manager+0x10,target);collect(manager,vector.as_mut_ptr()as usize,context);assert_eq!(vector[2],start,"Remote owner is not the local Sword owner");pointer(manager+0x10,source);
  patch(base,crate::HAS_EFFECT_RVA,&[0x31,0xc0,0xc3]);collect(manager,vector.as_mut_ptr()as usize,context);assert_eq!(vector[2],start,"No Sword: ordinary spell still excluded");
  integer(row+0x12c,8110002);collect(manager,vector.as_mut_ptr()as usize,context);assert_eq!(vector[2],start+4,"Other native added ailments remain unchanged");integer(row+0x12c,CARRIER);
  patch(base,crate::HAS_EFFECT_RVA,&sword_owned);vector[2]=start;
  ptr::write((row+offset)as*mut u8,0);collect(manager,vector.as_mut_ptr()as usize,context);assert_eq!(vector[2],start,"Remaining native applicability is still enforced");
 }
 ptr::write(context as*mut u8,10);ptr::write((row+0x259)as*mut u8,32);vector[2]=start;
 collect(manager,vector.as_mut_ptr()as usize,context);assert_eq!(vector[2],start,"Other spell types do not acquire Sword's exception");ptr::write((row+0x259)as*mut u8,0);
 patch(base,crate::HAS_EFFECT_RVA,&lookup);BASE.store(saved_base,Ordering::Release);pointer(base+crate::WORLD_RVA,previous_world);pointer(manager+0x10,0);
 ACTIVE.store(true,Ordering::Release);pointer(manager+0x10,0);ptr::write(context as*mut u8,0);integer(bullet+0x2c0,-1);
 for id in [12421005,12421006,12301020,12301120,98802001]{
  assert!(source_data::PASSIVE_BULLETS.binary_search(&id).is_ok());integer(bullet+0xa4,id);vector[2]=start;
  collide(context,vector.as_mut_ptr()as usize,bullet);assert_eq!(values[0],PASSIVE_CARRIER);
  assert!(!PASSIVE_CONTACT.with(Cell::get));
  vector[2]=start;collect(manager,vector.as_mut_ptr()as usize,context);assert_eq!(values[0],CARRIER,"Projectile scope cannot leak to weapon hit");
 }
 // Execute original payload ID copy instructions to validate BulletIns+A4.
 patch(base,0x33e799,&[0xc3]);let copy:unsafe extern "system" fn(usize,usize)=std::mem::transmute(base+0x33e789);
 let payload=[-1i32,12421005,-1];copy(bullet+0xa0,payload.as_ptr()as usize);assert_eq!(read_i32(bullet+0xa4),12421005);
 ACTIVE.store(active_before,Ordering::Release);
 ptr::write_bytes(context as*mut u8,0,0x48);float(context+0x38,1.);
 vector[2]=start;ptr::write_unaligned((row+0x156)as*mut u16,2103);collect(manager,vector.as_mut_ptr()as usize,context);assert_eq!(vector[2],start);
 // Real native additional-ailment correction, including both correction fields.
 patch(base,0x43c899,&[0xc3]);ptr::write((row+0x259)as*mut u8,1);
 for (a,b,scale,want)in [(1.,1.,1.,1.),(0.5,0.5,1.,0.25),(0.,0.,1.,0.),(0.,1.,1.,0.),(1.,0.,1.,0.),(0.5,0.5,0.4,0.1)]{
  float(hit+0x188,a);float(hit+0x18c,b);assert_eq!(erosion_native_motion(hit,row,scale,base+0x43c869),want);
 }
 ptr::write((row+0x259)as*mut u8,0);assert_eq!(erosion_native_motion(hit,row,0.4,base+0x43c869),0.4);
 // Install the actual compiled hook at the real native entry. Its continuation
 // is a recording fixture: network, proc and damage execution are not claimed.
 let prepared=crate::hook::prepare((base+RVA)as*mut u8,detour as*const u8,64).unwrap();
 ORIGINAL.store(receiver as*mut u8,Ordering::Release);crate::hook::write_code((base+RVA)as*mut u8,&prepared.patch).unwrap();
 patch(base,crate::HAS_EFFECT_RVA,&[0xb0,1,0xc3]);patch(base,crate::HAS_STATE_RVA,&[0x31,0xc0,0xc3]);
 let previous_base=BASE.swap(base,Ordering::AcqRel);let previous_active=ACTIVE.swap(true,Ordering::AcqRel);
 let apply:Apply=std::mem::transmute(base+RVA);
 let call=|id|{apply(target,id,source,0x81,0x12345678,context,0x82,0x83,0x84,0x85,0x86,0x87,-99)};
 let expected=|id|{assert_eq!(call(CARRIER),0x91);assert_eq!(CALLS.lock().unwrap().pop(),Some((target,id,source,0x12345678,context)));};
 for selected in 0..7{
  for i in 0..7{integer(resist+0x2c+i*4,if i==selected{84}else{400});integer(resist+0x10+i*4,400);}
  for (carrier,first) in [(ITEM_CARRIER,ITEM_FIRST),(SPELL_CARRIER,SPELL_FIRST),(PASSIVE_CARRIER,PASSIVE_FIRST)]{
   assert_eq!(call(carrier),0x91);assert_eq!(CALLS.lock().unwrap().pop().unwrap().1,first+selected as i32);
  }
  expected(FIRST+selected as i32);
 }
 for i in 0..7{integer(resist+0x2c+i*4,400);integer(resist+0x10+i*4,400);}
 integer(resist+0x2c,154);integer(resist+0x10,1);integer(resist+0x2c+4,84);expected(FIRST+1); // full threshold, not remaining
 assert_eq!(call(ITEM_CARRIER),0x91);assert_eq!(CALLS.lock().unwrap().pop().unwrap().1,ITEM_FIRST+1);
 integer(resist+0x2c+4,252);expected(FIRST); // native escalation takes effect immediately
 integer(resist+0x2c+8,154);integer(resist+0x10+8,0);expected(FIRST+2); // tie prefers progress
 integer(resist+0x10,0);expected(FIRST); // stable exact tie
 ptr::write((row+0x163)as*mut u8,4);expected(FIRST+2); // native immunity
 integer(resist+0x2c+12,10);expected(FIRST+3); // death not hard excluded
 ptr::write((row+0x163)as*mut u8,0x3c);ptr::write((row+0x259)as*mut u8,8);ptr::write((row+0x15f)as*mut u8,128);ptr::write((row+0x160)as*mut u8,1);
 assert_eq!(call(CARRIER),0);assert_eq!(call(ITEM_CARRIER),0);assert!(CALLS.lock().unwrap().is_empty()); // all immune
 ptr::write_bytes(row as*mut u8,0,0x400);
 // In-progress debuffs excluded via native state presence; no immune case is
 // disguised as a magic 999 threshold.
 patch(base,crate::HAS_STATE_RVA,&[0x66,0x83,0xfa,0x74,0x0f,0x94,0xc0,0xc3]);expected(FIRST);
 patch(base,crate::HAS_STATE_RVA,&[0x31,0xc0,0xc3]);float(context+0x38,0.);assert_eq!(call(CARRIER),0);
 float(context+0x38,f32::NAN);assert_eq!(call(CARRIER),0);float(context+0x38,1.);
 integer(stats+0x140,0);assert_eq!(call(CARRIER),0);integer(stats+0x140,1000);
 patch(base,crate::HAS_EFFECT_RVA,&[0x31,0xc0,0xc3]);assert_eq!(call(CARRIER),0);patch(base,crate::HAS_EFFECT_RVA,&[0xb0,1,0xc3]);
 for id in [FIRST,FIRST+6,ITEM_FIRST,ITEM_FIRST+6,8110402]{assert_eq!(call(id),0x91);assert_eq!(CALLS.lock().unwrap().pop().unwrap().1,id);} // selected network payload never reselects
 ACTIVE.store(false,Ordering::Release);assert_eq!(call(CARRIER),0x91);assert_eq!(CALLS.lock().unwrap().pop().unwrap().1,CARRIER);
 ACTIVE.store(previous_active,Ordering::Release);BASE.store(previous_base,Ordering::Release);
 VirtualFree(allocation,0,MEM_RELEASE);
}}
