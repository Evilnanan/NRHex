//! Native wire-field packing and constructor roundtrip of compiled modifiers.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
use std::sync::atomic::{AtomicUsize,Ordering};
static LOCAL:AtomicUsize=AtomicUsize::new(0);
unsafe extern "system" fn has(bits:usize,id:i32)->u8{u8::from(match id{crate::strength::OWNER=>bits&1!=0,crate::sword::OWNER=>bits&2!=0,crate::marking::OWNER=>bits&4!=0,_=>false})}
unsafe extern "system" fn bullet_kind(_:usize)->i32{3}
unsafe extern "system" fn resolve(handle:u64)->usize{if handle==77{LOCAL.load(Ordering::Relaxed)}else{0}}
unsafe fn put(base:usize,rva:usize,bytes:&[u8]){ptr::copy_nonoverlapping(bytes.as_ptr(),(base+rva)as*mut u8,bytes.len());}
fn jump(address:usize)->Vec<u8>{let mut b=vec![0x48,0xb8];b.extend_from_slice(&address.to_le_bytes());b.extend_from_slice(&[0xff,0xe0]);b}
#[test]#[ignore="requires NRHEX_NATIVE_EXE; global runtime/catalog, run alone"]
fn compiled_modifiers_survive_native_wire_rebuild_without_reapplying(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let pe=crate::pe::Headers::parse(&raw,0).unwrap();let mem=VirtualAlloc(ptr::null_mut(),pe.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!mem.is_null());let base=mem as usize;
 for s in &pe.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 let pack=std::slice::from_raw_parts((base+0x4358d2)as*const u8,11).to_vec();assert_eq!(pack,[0x41,0x0f,0xb6,0x40,0x1e,0x88,0x87,0x8c,0,0,0]);
 let mut code=vec![0x57,0x49,0x89,0xc8,0x48,0x89,0xd7];code.extend_from_slice(&pack);code.extend_from_slice(&[0x5f,0xc3]);put(base,0x1000,&code);
 let decode=std::slice::from_raw_parts((base+0x4350e6)as*const u8,10).to_vec();assert_eq!(decode,[0x0f,0xb6,0x83,0x8c,0,0,0,0x88,0x45,8]);
 let mut code=vec![0x53,0x55,0x48,0x89,0xcb,0x48,0x8d,0xaa,0xc0,0,0,0];code.extend_from_slice(&decode);
 // Include the actual received-context flag store immediately after decode.
 let flag=std::slice::from_raw_parts((base+0x4350f0)as*const u8,4);assert_eq!(flag,[0xc6,0x45,0x4e,1]);code.extend_from_slice(flag);code.extend_from_slice(&[0x5d,0x5b,0xc3]);put(base,0x1100,&code);
 let mut code=vec![0x41,0x54,0x56,0x48,0x83,0xec,0x58,0x48,0x89,0xce,0x49,0x89,0xd4,0x4c,0x89,0x44,0x24,0x48];code.extend_from_slice(&jump(base+0xe709bd));put(base,0x1200,&code);put(base,0xe709f7,&[0x48,0x83,0xc4,0x58,0x5e,0x41,0x5c,0xc3]);
 put(base,crate::HAS_EFFECT_RVA,&jump(has as*const()as usize));put(base,crate::CHR_HANDLE_RVA,&jump(resolve as*const()as usize));
 let pack:unsafe extern "system" fn(usize,usize)=std::mem::transmute(base+0x1000);
 let decode:unsafe extern "system" fn(usize,usize)=std::mem::transmute(base+0x1100);
 let build:unsafe extern "system" fn(usize,usize,usize)=std::mem::transmute(base+0x1200);
 let lookup:unsafe extern "system" fn(u8)->i32=std::mem::transmute(base+0xe6d510);
 for level in 0..=11{assert_eq!(lookup(override_id(level).unwrap()),level as i32);}
 crate::SWORD_BONUSES.get_or_init(crate::sword::Bonuses::defaults);let spell=crate::sword_map::ATTACKS[0].0;
 let mut actor=[0usize;0x720/8];let a=actor.as_mut_ptr()as usize;actor[1]=77;LOCAL.store(a,Ordering::Relaxed);
 let mut modules=[0usize;0x200/8];let mut stats=[0usize;0x200/8];modules[0]=stats.as_mut_ptr()as usize;actor[0x1b8/8]=modules.as_mut_ptr()as usize;ptr::write_unaligned((modules[0]+0x140)as*mut i32,100);
 let vt=[0usize,0,bullet_kind as*const()as usize];let mut bullet=[0usize;0xc0/8];bullet[0]=vt.as_ptr()as usize;bullet[0xb0/8]=77;let b=bullet.as_mut_ptr()as usize;
 let mut world=vec![0usize;0x17500/8];world[0x174e8/8]=a;ptr::write_unaligned((base+crate::WORLD_RVA)as*mut usize,world.as_ptr()as usize);
 let old_base=crate::BASE.swap(base,Ordering::AcqRel);let old_active=crate::ACTIVE.swap(true,Ordering::AcqRel);
 for effects in 0..8{actor[0x190/8]=effects;for (attack,source,kind,melee)in [(100,a,1u8,true),(100,b,1,true),(spell,a,3,false),(spell,b,3,false),(600000120,a,1,true)]{for native in 0..=11{for prior_override in [false,true]{
  let mut context=[0usize;0x120/8];let cp=context.as_mut_ptr()as usize;context[0]=source;ptr::write_unaligned((cp+8)as*mut i32,attack);ptr::write_unaligned((cp+12)as*mut i32,1);
  if prior_override{ptr::write((cp+0xc8)as*mut u8,override_id(native).unwrap());}
  let mut row=[0u8;0x200];row[0x72]=native;let mut hit=[0u8;0x2c8];let hp=hit.as_mut_ptr()as usize;hit[0x122]=kind;if melee{hit[0x124+130/8]|=1<<(130%8);}
  ptr::write_unaligned((hp+0x40)as*mut i32,attack);ptr::write_unaligned((hp+0x44)as*mut i32,1);
  build(hp,cp,row.as_ptr()as usize);crate::on_strength_constructed(hp,cp);
  let enabled=effects&1!=0&&attack!=600000120&&if attack==spell{effects&2!=0}else{source==a};
  let expected=if enabled{crate::strength::remap(native)}else{native};assert_eq!(hit[0x24],expected);
  let mut message=[0u8;0x150];pack(hp,message.as_mut_ptr()as usize);
  for received_source in [a,b,0]{
   let mut received_context=context;received_context[0]=received_source;let rp=received_context.as_mut_ptr()as usize;decode(message.as_ptr()as usize,rp);
   let mut received=hit;let rh=received.as_mut_ptr()as usize;build(rh,rp,row.as_ptr()as usize);let before=received;
   crate::on_strength_constructed(rh,rp);assert_eq!(received[0x24],expected);assert_eq!(received,before);
  }
 }}}}
 // Slide overrides Mighty, including its distinct air-start tier; receiver
 // must not replace a transmitted air tier with its own default ground state.
 actor[0x190/8]=7;
 for air in [false,true]{crate::marking::start(a,if air{2}else{1},base);
  let mut context=[0usize;0x120/8];context[0]=a;let cp=context.as_mut_ptr()as usize;ptr::write_unaligned((cp+8)as*mut i32,crate::marking::ATTACK);ptr::write_unaligned((cp+12)as*mut i32,1);
  let mut row=[0u8;0x200];row[0x72]=1;let mut hit=[0u8;0x2c8];hit[0x124+130/8]|=1<<(130%8);let hp=hit.as_mut_ptr()as usize;
  build(hp,cp,row.as_ptr()as usize);crate::on_strength_constructed(hp,cp);assert_eq!(hit[0x24],if air{3}else{9});
  let mut message=[0u8;0x150];pack(hp,message.as_mut_ptr()as usize);decode(message.as_ptr()as usize,cp);crate::marking::reset();
  build(hp,cp,row.as_ptr()as usize);crate::on_strength_constructed(hp,cp);assert_eq!(hit[0x24],if air{3}else{9});
 }
 crate::marking::reset();crate::BASE.store(old_base,Ordering::Release);crate::ACTIVE.store(old_active,Ordering::Release);assert_ne!(VirtualFree(mem,0,MEM_RELEASE),0);
}}
