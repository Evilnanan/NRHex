//! Native category filters, source aggregation, physical construction, rate
//! serialization and recipient combiner. PARAM/weapon/stat lookups are fixtures.
use super::*;
use crate::{static_charge,lightning_ram,win::*};
use sha2::{Digest,Sha256};
use std::{cell::Cell,collections::BTreeMap};
thread_local!{static ROW:Cell<usize>=const{Cell::new(0)};}
unsafe fn put<T:Copy>(p:usize,v:T){ptr::write_unaligned(p as*mut T,v);}
unsafe fn float(p:usize)->f32{ptr::read_unaligned(p as*const f32)}
unsafe fn bytes(base:usize,rva:usize,b:&[u8]){ptr::copy_nonoverlapping(b.as_ptr(),(base+rva)as*mut u8,b.len());}
unsafe fn jump(base:usize,rva:usize,f:usize){let mut b=vec![0x48,0xb8];b.extend_from_slice(&f.to_le_bytes());b.extend_from_slice(&[0xff,0xe0]);bytes(base,rva,&b);}
unsafe extern "system" fn row(out:usize,_:i32,_:i32)->usize{put(out+16,ROW.with(Cell::get));out}
unsafe extern "system" fn empty(out:usize,_:i32)->usize{put(out+8,0usize);out}
unsafe extern "system" fn zero_ref(_:usize,out:usize,_:i32){ptr::write_bytes(out as*mut u8,0,12);}
unsafe extern "system" fn scale(_:usize,_:usize,_:usize,_:i32)->f32{1.}
unsafe extern "system" fn bullet_kind(_:usize)->i32{3}
unsafe extern "system" fn owner(handle:u64)->usize{if handle==77{123}else{0}}
fn rows(path:&str,size:usize)->BTreeMap<i32,Vec<u8>>{
 let raw=std::fs::read(std::env::var_os(path).expect(path)).unwrap();let count=u16::from_le_bytes(raw[10..12].try_into().unwrap())as usize;
 (0..count).map(|i|{let a=64+i*24;let id=i32::from_le_bytes(raw[a..a+4].try_into().unwrap());let off=u64::from_le_bytes(raw[a+8..a+16].try_into().unwrap())as usize;(id,raw[off..off+size].to_vec())}).collect()
}
#[test]#[ignore="requires NRHEX_NATIVE_EXE, NRHEX_RAM_ATTACKS, NRHEX_RAM_EFFECTS; run alone"]
fn ram_native_melee_skill_bonuses_and_velocity_base_apply_once(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let attacks=rows("NRHEX_RAM_ATTACKS",0x1d0);let effects=rows("NRHEX_RAM_EFFECTS",0x400);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let mem=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!mem.is_null());let image=mem as usize;
 for s in &headers.sections{bytes(image,s.virtual_address as usize,&raw[s.raw_offset as usize..(s.raw_offset+s.raw_size)as usize]);}
 // Whole routines use relative calls and constants; these dependencies replace
 // only game PARAM stores, empty equipment references and attribute scaling.
 jump(image,0xe6d630,row as*const()as usize);
 for r in [0xea9a60,0xea7fc0,0xe6f150]{jump(image,r,empty as*const()as usize);}
 jump(image,0xe6f8e0,zero_ref as*const()as usize);
 for r in [0x6d1cc0,0x6d2370]{jump(image,r,scale as*const()as usize);}
 bytes(image,0x26909a0,&[0xc3]); // fixture stack-cookie verifier, no loader imports
 let p=crate::hook::prepare((image+RVA)as*mut u8,detour as*const u8,64).unwrap();assert_eq!(p.stolen,6);
 let old_original=ORIGINAL.swap(p.trampoline,Ordering::AcqRel);crate::hook::write_code((image+RVA)as*mut u8,&p.patch).unwrap();
 let old_base=crate::BASE.swap(image,Ordering::AcqRel);let old_active=ACTIVE.swap(true,Ordering::AcqRel);
 let old_calc=crate::glass_cannon::ORIGINAL.swap((image+0x6c5fc0)as*mut u8,Ordering::AcqRel);
 let init:unsafe extern "system" fn(usize)->usize=std::mem::transmute(image+0x33e360);
 let calc:unsafe extern "system" fn(usize,usize,i32,i32,f32,usize,u8,usize,u8)=std::mem::transmute(image+0x6c3200);
 let aggregate:unsafe extern "system" fn(usize,usize,i32,i32)=std::mem::transmute(image+0x502830);
 let cached:unsafe extern "system" fn(usize,usize)=std::mem::transmute(image+0xe70270);
 let mut context=[0u8;0x180];let c=context.as_mut_ptr()as usize;
 let mut hit=[0u8;0x300];let h=hit.as_mut_ptr()as usize;
 let mut stats=[0u64;0x1000/8];let st=stats.as_mut_ptr()as usize;put(st+0x84c,1f32);put(st+0x850,1f32);
 let mut bullet=[0u64;0x300/8];let b=bullet.as_mut_ptr()as usize;put(b+0xa4,lightning_ram::BULLET);put(b+8,55u64);
 let mut vt=[0usize;4];vt[2]=bullet_kind as*const()as usize;put(b,vt.as_ptr()as usize);put(b+0xb0,77u64);
 let mut payload=[0u8;0x220];let pp=payload.as_mut_ptr()as usize;put(pp+4,lightning_ram::BULLET);
 let mut group=[1i32;2];let gp=group.as_mut_ptr()as usize;
 for level in [1,15]{for speed in [6f32,10.,15.,19.25]{
  let velocity=speed*speed*1.5;let id=lightning_ram::attack_for_damage(velocity);let base_damage=lightning_ram::contact_damage(velocity,level);
  let limbs=lightning_ram::encode_roll(19,level);put(pp+0x124,[velocity,limbs[0],limbs[1],limbs[2]]);
  {let _g=static_charge::factory(pp);static_charge::factory_done(gp);let _i=static_charge::initialize(gp);static_charge::bound(b);}
  ROW.with(|r|r.set(attacks[&id].as_ptr()as usize));
  for (power,flat) in [(1f32,0f32),(1.5,0.),(0.5,50.),(0.,7.)]{
   init(c);put(c,b);put(c+8,id);put(c+12,1i32);crate::ability_damage::before_construct(c);assert_eq!(context[0xf8],1);
   put(c+0x6c,power);put(c+0x48,flat);let mut out=[0f32;16];calc(out.as_mut_ptr()as usize,st,-1,-1,1.,c,0,0,1);
   assert_eq!(&out[..5],&[base_damage*power+flat,0.,0.,0.,0.]);
   put(h,[out[0],0.,0.,0.,0.]);put(h+0x40,id);put(h+0x44,1i32);put(h+0x148,45f32);put(h+0x150,60f32);
   static_charge::constructed(h,c);assert_eq!(float(h),out[0]);assert_eq!(float(h+0x148),45.);assert_eq!(float(h+0x150),60.);
  }
  // Read native melee/skill and charged-attack-only relic rows from the staged
  // package. Actual 502830 filtering/multiplication is left unchanged.
  for (ids,wanted) in [(vec![],1f32),(vec![8320500],1.09),(vec![8350000],1.15),(vec![8320500,8350000],1.09*1.15),(vec![7006700],1.15),(vec![8320100],1.)]{
   init(c);put(c,b);put(c+8,id);put(c+12,1i32);context[0x10]=1;categories(c);
   let mut nodes:Vec<Vec<usize>>=ids.iter().map(|i|{let mut n=vec![0usize;0x80/8];n[0]=effects[i].as_ptr()as usize;n}).collect();
   for i in 0..nodes.len().saturating_sub(1){nodes[i][0x38/8]=nodes[i+1].as_ptr()as usize;}
   let mut manager=[0usize;16];manager[1]=nodes.first().map_or(0,|n|n.as_ptr()as usize);aggregate(manager.as_ptr()as usize,c,-1,-1);
   assert!((float(c+0x90)-wanted).abs()<0.0001,"effects {ids:?}: {}",float(c+0x90));
   hit[0x25]=0;cached(h,c);assert_eq!(ptr::read_unaligned((h+0x14)as*const i16),(wanted*100.)as i16);
  }
  // Receiver retains offensive multipliers once, instead of overriding them
  // with the raw network seed. A physical cut still halves final HP damage.
  put(h,[base_damage,0.,0.,0.,0.]);put(h+0x40,id);put(h+0x44,1i32);
  let target=[0usize;4];let module=[0usize,target.as_ptr()as usize];static_charge::enter(module.as_ptr()as usize,0,h);assert!(static_charge::fixed_components().is_none());
  let one=[1f32;5];let rates=[1.5f32;5];let cuts=[0.5f32;5];let mut out=[0f32;5];
  let result=crate::glass_cannon::detour(out.as_mut_ptr(),hit.as_ptr()as*const f32,rates.as_ptr(),cuts.as_ptr(),one.as_ptr(),one.as_ptr(),one.as_ptr(),one.as_ptr(),1.,1.,1.,1.,1.,1.,1.);
  assert_eq!(result,base_damage*1.5*0.5);static_charge::leave();
  // Ironjar is a source-side addition to a freshly built hit, before the
  // unchanged native recipient calculation. Identity/status are fixtures;
  // this does not establish live multiplayer state replication.
  put(c,b);put(c+8,id);put(c+12,1i32);put(h+0x148,45f32);put(h+0x150,60f32);
  ironjar_hit(h,c,owner,|actor|actor==123);
  assert_eq!(float(h+0x148),90.);assert_eq!(float(h+0x150),60.);
  static_charge::enter(module.as_ptr()as usize,0,h);
  let doubled=crate::glass_cannon::detour(out.as_mut_ptr(),hit.as_ptr()as*const f32,rates.as_ptr(),cuts.as_ptr(),one.as_ptr(),one.as_ptr(),one.as_ptr(),one.as_ptr(),1.,1.,1.,1.,1.,1.,1.);
  assert_eq!(doubled,base_damage*2.*1.5*0.5);static_charge::leave();
 }}
 // Without a Ram row, this new mid-hook leaves the native base untouched.
 ROW.with(|r|r.set(attacks[&300000711].as_ptr()as usize));init(c);put(c+8,300000711i32);put(c+12,1i32);context[0xf8]=1;
 let mut ordinary=[0f32;16];calc(ordinary.as_mut_ptr()as usize,st,-1,-1,1.,c,0,0,1);assert_eq!(ordinary[0],40.);
 ACTIVE.store(false,Ordering::Release);let mut baseline=[0f32;16];calc(baseline.as_mut_ptr()as usize,st,-1,-1,1.,c,0,0,1);assert_eq!(ordinary.map(f32::to_bits),baseline.map(f32::to_bits));ACTIVE.store(true,Ordering::Release);
 static_charge::retired(b);static_charge::finished(gp);crate::glass_cannon::ORIGINAL.store(old_calc,Ordering::Release);ORIGINAL.store(old_original,Ordering::Release);crate::BASE.store(old_base,Ordering::Release);ACTIVE.store(old_active,Ordering::Release);
 VirtualFree(p.trampoline as*mut _,0,MEM_RELEASE);if !p.relay.is_null(){VirtualFree(p.relay as*mut _,0,MEM_RELEASE);}VirtualFree(mem,0,MEM_RELEASE);
 println!("PASS: fractional velocity/level base, native power/flat/rate/category calculation once, preserved defenses and original low-speed attacks");
}}
