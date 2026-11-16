//! Execute native poise/rescue getters against speed-selected private rows.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
thread_local!{static ROWS:Cell<[usize;3]>=const{Cell::new([0;3])};}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn float(p:usize,v:f32){ptr::write_unaligned(p as*mut f32,v);}
unsafe extern "system" fn lookup(out:usize,kind:i32,id:i32)->usize{
 assert_eq!(kind,1);
 let index=match id{crate::lightning_ram::ATTACK=>0,crate::lightning_ram::FAST_ATTACK=>1,crate::lightning_ram::EXTREME_ATTACK=>2,_=>panic!("unexpected attack")};
 int(out,id);int(out+8,kind);ptr::write_unaligned((out+0x10)as*mut usize,ROWS.with(|r|r.get()[index]));out
}
#[test]#[ignore="requires NRHEX_NATIVE_EXE; run alone"]
fn native_ram_speed_selects_poise_and_rescue_before_construction(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let h=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),h.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for s in &h.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&(lookup as*const()as usize).to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(jump.as_ptr(),(base+0xe6d630)as*mut u8,jump.len());
 // Execute the actual native descriptor-to-network-payload coefficient copy.
 ptr::write((base+0x340599)as*mut u8,0xc3);
 let copy:unsafe extern "system" fn(usize,usize)=std::mem::transmute(base+0x34058e);
 let poise:unsafe extern "system" fn(usize,i32,usize,f32,usize,usize)->f32=std::mem::transmute(base+0x6ce2f0);
 let rescue:unsafe extern "system" fn(usize)->f32=std::mem::transmute(base+0x6cd640);
 let mut rows=[[0u8;0x200];3];let mut pointers=[0usize;3];
 for(i,(p,r))in[(12.,20.),(18.,30.),(30.,50.)].into_iter().enumerate(){let rp=rows[i].as_mut_ptr()as usize;pointers[i]=rp;float(rp+0x84,p);float(rp+0x1c0,r);}
 ROWS.with(|r|r.set(pointers));let old_active=ACTIVE.swap(true,Ordering::AcqRel);
 let mut descriptor=[0u8;0x110];let dp=descriptor.as_mut_ptr()as usize;
 let mut payload=[0u8;0x220];let pp=payload.as_mut_ptr()as usize;int(pp+4,crate::lightning_ram::BULLET);
 let mut group=[99i32;2];let gp=group.as_mut_ptr()as usize;
 let mut bullet=[0usize;0x300/8];let bp=bullet.as_mut_ptr()as usize;int(bp+0xa4,crate::lightning_ram::BULLET);ptr::write_unaligned((bp+8)as*mut u64,55);
 let mut hit=[0u8;0x300];let hp=hit.as_mut_ptr()as usize;
 let mut context=[0u8;0x120];let cp=context.as_mut_ptr()as usize;ptr::write_unaligned(cp as*mut usize,bp);int(cp+12,1);
 for level in [0,1,15]{for(speed,id,p,r)in[(6.,crate::lightning_ram::ATTACK,12.,20.),(9.99,crate::lightning_ram::ATTACK,12.,20.),(10.,crate::lightning_ram::FAST_ATTACK,18.,30.),(14.99,crate::lightning_ram::FAST_ATTACK,18.,30.),(15.,crate::lightning_ram::EXTREME_ATTACK,30.,50.),(30.,crate::lightning_ram::EXTREME_ATTACK,30.,50.)]{
  let damage=speed*speed*1.5;let limbs=if level==0{[1.;3]}else{crate::lightning_ram::encode_roll(0xdead_beef_abcd_1234,level)};
  let packet=[damage,limbs[0],limbs[1],limbs[2]];ptr::write_unaligned((dp+0x30)as*mut[f32;4],packet);copy(pp,dp);
  assert_eq!(ptr::read_unaligned((pp+0x124)as*const[f32;4]),packet);
  {let _f=factory(pp);assert_eq!(ptr::read_unaligned((pp+0x124)as*const[f32;4]),[1.;4]);factory_done(gp);let _i=initialize(gp);bound(bp);}
  int(cp+8,crate::lightning_ram::ATTACK);context[0xf8]=0;
  crate::ability_damage::before_construct(cp);assert_eq!(read_i32(cp+8),id);assert_eq!(context[0xf8],1);
  assert_eq!(poise(0,0,0,999.,cp,0),p);assert_eq!(rescue(cp),r);
  int(hp+0x40,id);int(hp+0x44,1);float(hp+0x148,p*1.5);float(hp+0x150,r*2.);
  let expected=damage+if speed>=15.{30.*level as f32}else{0.};
  assert_eq!(ram_base(cp),Some(expected));
  let native=[expected*1.5+7.,0.,0.,0.,0.];ptr::write_unaligned(hp as*mut[f32;5],native);
  constructed(hp,cp);assert_eq!(read_i32(hp+0x40),id);
  assert_eq!(ptr::read_unaligned(hp as*const[f32;5]),native);
  let stored=seed(bp).unwrap();assert_eq!(stored.damage,damage);assert_eq!(stored.ram_level,level);if level>0{assert_eq!(stored.victim,Some(0xdead_beef_abcd_1234));}
  assert_eq!(ptr::read_unaligned((hp+0x148)as*const f32),p*1.5);assert_eq!(ptr::read_unaligned((hp+0x150)as*const f32),r*2.);
 }}
 retired(bp);finished(gp);ROWS.with(|r|r.set([0;3]));ACTIVE.store(old_active,Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
 println!("PASS: native transport and poise/rescue getters at all speed boundaries; source modifiers retained; level bonus only changes extreme HP damage, never speed or tiers");
}}
