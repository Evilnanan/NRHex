//! Original weighted selectors, original float RNG and compiled scoped RNG
//! hook. Param row iteration, availability and temporary allocation use fixtures.
use super::*;
use crate::{win::*,pe,hook,EXE_HASH};
use sha2::{Digest,Sha256};
use std::cell::RefCell;
thread_local!{
 static RNG_OBJECT:Cell<usize>=const{Cell::new(0)};
 static GLOBAL_DRAWS:Cell<u32>=const{Cell::new(0)};
 static ALLOCATOR:Cell<usize>=const{Cell::new(0)};
 static BLOCKS:RefCell<Vec<Box<[u64]>>>=const{RefCell::new(Vec::new())};
}
// Native rows: enable mask, skill/spell ID, weight at +8.
static ARTS:[[i32;3];3]=[[1,101,10],[1,103,0],[1,200,30]];
static SPELLS:[[i32;3];5]=[[0,4000,100],[0,4070,30],[0,4140,10],[0,4450,0],[0,999999,100]];
unsafe extern "system" fn table(out:usize,id:i32)->usize{
 let (data,count)=match id{1=>(ARTS.as_ptr()as usize,ARTS.len()),2=>(SPELLS.as_ptr()as usize,SPELLS.len()),_=>(0,0)};
 ptr::write_unaligned(out as*mut usize,data);ptr::write_unaligned((out+8)as*mut u64,0);ptr::write_unaligned((out+0x10)as*mut i32,id);ptr::write_unaligned((out+0x14)as*mut u32,count as u32);out
}
unsafe extern "system" fn row(reference:usize,index:u32)->usize{if index<ptr::read_unaligned((reference+0x14)as*const u32){ptr::read_unaligned(reference as*const usize)+index as usize*12}else{0}}
unsafe extern "system" fn magic(out:usize,id:i32)->usize{ptr::write_unaligned(out as*mut i32,id);ptr::write_unaligned((out+8)as*mut usize,usize::from(matches!(id,4000|4070|4140|4450)));out}
unsafe extern "system" fn get_rng()->usize{RNG_OBJECT.with(Cell::get)}
unsafe extern "system" fn global_next(_:usize)->u32{GLOBAL_DRAWS.with(|c|c.set(c.get()+1));0x40000000}
static FLAGS:u32=32;
unsafe extern "system" fn flags(_:usize,_:usize,_:u32)->*const u32{&FLAGS}
unsafe extern "system" fn get_allocator()->usize{ALLOCATOR.with(Cell::get)}
unsafe extern "system" fn free(_:usize,_:usize){}
unsafe extern "system" fn reserve(vector:usize,count:u32){
 let mut block=vec![0u64;count as usize].into_boxed_slice();let p=block.as_mut_ptr()as usize;
 ptr::write_unaligned((vector+8)as*mut usize,p);ptr::write_unaligned((vector+0x10)as*mut usize,p);ptr::write_unaligned((vector+0x18)as*mut usize,p+count as usize*8);BLOCKS.with(|c|c.borrow_mut().push(block));
}
unsafe fn jump(base:usize,rva:usize,dest:usize){let mut bytes=vec![0x48,0xb8];bytes.extend_from_slice(&dest.to_le_bytes());bytes.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(bytes.as_ptr(),(base+rva)as*mut u8,bytes.len());}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; native image and global RNG hook must run alone"]
fn original_reward_art_and_magic_lotteries_replay_without_advancing_global_rng(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").expect("NRHEX_NATIVE_EXE")).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),EXE_HASH);
 let h=pe::Headers::parse(&raw,0).unwrap();let image=VirtualAlloc(ptr::null_mut(),h.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!image.is_null());let base=image as usize;
 for s in &h.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 // The protected selectors use absolute MOV r64,imm64 jump targets as well
 // as RIP-relative branches. Relocate only image-address immediates in this
 // private hash-guarded copy, never patch the on-disk game executable.
 for s in &h.sections{if s.characteristics&0x20000000==0{continue;}let p=(base+s.virtual_address as usize)as*mut u8;for i in 0..s.raw_size as usize-10{
  if matches!(ptr::read(p.add(i)),0x48|0x49)&&(0xb8..=0xbf).contains(&ptr::read(p.add(i+1))){let old=ptr::read_unaligned(p.add(i+2)as*const u64);if(0x140000000..0x140000000+h.image_size as u64).contains(&old){ptr::write_unaligned(p.add(i+2)as*mut u64,old-0x140000000+base as u64);}}
 }}
 for(rva,dest)in [(ART_TABLE,table as*const()as usize),(SPELL_TABLE,table as*const()as usize),(0xe86870,row as*const()as usize),(0xe7b1f0,row as*const()as usize),(0xe926c0,magic as*const()as usize),(0x6af4f0,get_rng as*const()as usize),(0x214f3e0,get_allocator as*const()as usize),(0xe7b290,reserve as*const()as usize)]{jump(base,rva,dest);}
 let rng_vtable=[0usize,global_next as*const()as usize];let rng=[rng_vtable.as_ptr()as usize];RNG_OBJECT.with(|c|c.set(rng.as_ptr()as usize));GLOBAL_DRAWS.with(|c|c.set(0));
 let mut allocator_vtable=[0usize;14];allocator_vtable[3]=flags as*const()as usize;allocator_vtable[13]=free as*const()as usize;let allocator=[allocator_vtable.as_ptr()as usize];ALLOCATOR.with(|c|c.set(allocator.as_ptr()as usize));
 let prepared=hook::prepare((base+RNG_RVA)as*mut u8,random as*const u8,64).unwrap();let original=RNG_ORIGINAL.swap(prepared.trampoline,Ordering::AcqRel);hook::write_code((base+RNG_RVA)as*mut u8,&prepared.patch).unwrap();let previous=ACTIVE.swap(true,Ordering::AcqRel);
 let float:unsafe extern "system" fn(usize)->f32=std::mem::transmute(base+RNG_RVA);assert_eq!(float(rng.as_ptr()as usize),0.25);assert_eq!(GLOBAL_DRAWS.with(Cell::get),1);
 let weapon=Weapon{id:33000000,kind:57,art:123,spell:4070,primary:4000,art_table:1,spell_table:2};let mut art_counts=[0;2];let mut spell_counts=[0;2];
 for seed in 0..2000{
  let a=generate_at(base,seed,false,&weapon).unwrap();let b=generate_at(base,seed,false,&weapon).unwrap();assert_eq!(a,b);assert_eq!(a.source,weapon.id);
  match a.art{101=>art_counts[0]+=1,200=>art_counts[1]+=1,id=>panic!("zero-weight or foreign art {id}")};
  match a.spell{4070=>spell_counts[0]+=1,4140=>spell_counts[1]+=1,id=>panic!("primary/unavailable/zero-weight spell {id}")};
  BLOCKS.with(|c|c.borrow_mut().clear());
 }
 assert!((400..600).contains(&art_counts[0]));assert!((400..600).contains(&spell_counts[1]));assert_eq!(GLOBAL_DRAWS.with(Cell::get),1);
 let unique=Weapon{art_table:-1,spell_table:-1,..weapon};assert_eq!(generate_at(base,42,true,&unique),Some(Hand{source:unique.id,art:unique.art,spell:unique.spell}));
 let missing=Weapon{art_table:99,..weapon};assert_eq!(generate_at(base,1,false,&missing),None);
 assert_eq!(float(rng.as_ptr()as usize),0.25);assert_eq!(GLOBAL_DRAWS.with(Cell::get),2);assert_eq!(DRAW.with(Cell::get),None);
 ACTIVE.store(previous,Ordering::Release);RNG_ORIGINAL.store(original,Ordering::Release);BLOCKS.with(|c|c.borrow_mut().clear());VirtualFree(image,0,MEM_RELEASE);
}}
