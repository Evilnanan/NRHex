//! Execute both original parameter constructors against a small native
//! repository fixture, then execute the compiled hooks against the same data.
//! This does not create actors, load game resources or test multiplayer.
use super::*;
use crate::{win::*,pe,hook,EXE_HASH};
use sha2::{Digest,Sha256};
use std::cell::RefCell;
thread_local!{static TABLES:RefCell<Vec<(u32,usize)>>=const{RefCell::new(Vec::new())};}
unsafe extern "system" fn query(manager:usize,kind:u32,_:u32)->usize{assert_eq!(manager,1);TABLES.with(|v|v.borrow().iter().find(|x|x.0==kind).map_or(0,|x|x.1))}
struct Table{_bytes:Box<[u64]>,_file:Box<[u64;17]>,wrapper:Box<[u64;17]>,rows:Vec<(i32,usize)>}
impl Table{
 unsafe fn new(ids:&[i32],size:usize,weapon:bool)->Self{
  let start=0x40+ids.len()*12;let map=(start+ids.len()*size+15)&!15;
  let mut bytes=vec![0u64;(16+map+ids.len()*8).div_ceil(8)].into_boxed_slice();let data=bytes.as_mut_ptr()as usize+16;
  ptr::write_unaligned((data-16)as*mut i32,map as i32);ptr::write_unaligned((data-12)as*mut i32,ids.len()as i32);
  ptr::write_unaligned((data+8)as*mut u16,1234);ptr::write_unaligned((data+10)as*mut u16,ids.len()as u16);ptr::write((data+0x2d)as*mut u8,2);
  let mut rows=Vec::new();
  for (index,&id) in ids.iter().enumerate(){
   let offset=start+index*size;let row=data+offset;
   ptr::write_unaligned((data+0x30+index*12)as*mut i32,id);ptr::write_unaligned((data+0x34+index*12)as*mut u32,offset as u32);
   ptr::write_unaligned((data+map+index*8)as*mut i32,id);ptr::write_unaligned((data+map+index*8+4)as*mut i32,index as i32);
   ptr::write_bytes(row as*mut u8,index as u8,size);
   if weapon{ptr::write_unaligned((row+0xda)as*mut i16,0);}
   rows.push((id,row));
  }
  let mut file=Box::new([0u64;17]);file[16]=data as u64;let mut wrapper=Box::new([0u64;17]);wrapper[16]=file.as_ptr()as u64;
  Self{_bytes:bytes,_file:file,wrapper,rows}
 }
 fn address(&self)->usize{self.wrapper.as_ptr()as usize}
 fn row(&self,id:i32)->usize{self.rows.iter().find(|x|x.0==id).unwrap().1}
}
unsafe fn jump(base:usize,rva:usize,dest:usize){let mut bytes=vec![0x48,0xb8];bytes.extend_from_slice(&dest.to_le_bytes());bytes.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(bytes.as_ptr(),(base+rva)as*mut u8,bytes.len());}
unsafe extern "system" fn lottery_table(out:usize,_:i32)->usize{ptr::write_unaligned(out as*mut u64,1);out}
unsafe extern "system" fn art_pick(_:usize,excluded:i32)->i32{assert_eq!(excluded,-1);0}
unsafe extern "system" fn spell_pick(_:usize,_:*const i32)->i32{0}
unsafe extern "system" fn art_get(_:usize,_:i32)->i32{1234}
unsafe extern "system" fn spell_get(_:usize,_:i32)->i32{5678}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; private PE and global hooks must run alone"]
fn compiled_virtual_rows_execute_with_original_ci_and_weapon_constructors(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").expect("NRHEX_NATIVE_EXE")).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),EXE_HASH);
 let h=pe::Headers::parse(&raw,0).unwrap();let image=VirtualAlloc(ptr::null_mut(),h.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!image.is_null());let base=image as usize;
 for s in &h.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 jump(base,0xe9e680,query as*const()as usize);ptr::write_unaligned((base+0x3c28880)as*mut usize,1);
 // Lottery behavior is executed separately by demons_lottery/native_tests.
 // This fixture checks constructors, row storage and their integration only.
 for(rva,dest)in [(crate::demons_lottery::ART_TABLE,lottery_table as*const()as usize),(crate::demons_lottery::SPELL_TABLE,lottery_table as*const()as usize),(crate::demons_lottery::ART_PICK,art_pick as*const()as usize),(crate::demons_lottery::SPELL_PICK,spell_pick as*const()as usize),(crate::demons_lottery::ART_GET,art_get as*const()as usize),(crate::demons_lottery::SPELL_GET,spell_get as*const()as usize)]{jump(base,rva,dest);}
 let previous_base=crate::BASE.swap(base,Ordering::AcqRel);
 let ci_ids:Vec<_>=(0..10).flat_map(|hero|(0..6).map(move|skin|35000+hero*100+skin)).collect();
 let weapon_ids:Vec<_>=crate::demons_data::WEAPONS.iter().map(|w|w.id).collect();
 let cis=Table::new(&ci_ids,data::CI_SIZE,false);let weapons=Table::new(&weapon_ids,data::WEAPON_SIZE,true);let reinforcements=Table::new(&[0,12],256,false);
 TABLES.with(|v|*v.borrow_mut()=vec![(0x18,cis.address()),(0,weapons.address()),(4,reinforcements.address())]);
 let ci:Lookup=std::mem::transmute(base+CI_RVA);let w:Lookup=std::mem::transmute(base+WEAPON_RVA);
 let mut normal=[0u64;6];let np=normal.as_mut_ptr()as usize;ci(np,35000);assert_eq!(crate::read_ptr(np+8),cis.row(35000));
 w(np,weapon_ids[0]+12);assert_eq!(crate::read_ptr(np+8),weapons.row(weapon_ids[0]));assert_eq!(crate::read_ptr(np+0x20),reinforcements.row(12));
 let mut saved=Vec::new();
 for (rva,detour,original) in [(CI_RVA,chara as*const u8,&CI_ORIGINAL),(WEAPON_RVA,weapon as*const u8,&WEAPON_ORIGINAL)]{
  let prepared=hook::prepare((base+rva)as*mut u8,detour,64).unwrap();let old=original.swap(prepared.trampoline,Ordering::AcqRel);saved.push((original,old));hook::write_code((base+rva)as*mut u8,&prepared.patch).unwrap();
 }
 let active=ACTIVE.swap(true,Ordering::AcqRel);
 for seed in [0,1,2,19,42,512,701,77777,demons::SEEDS-1]{
  assert!(prepare(seed,base));
  let roll=demons::roll(seed).unwrap();let id=demons::chara_id(seed).unwrap();let mut out=[0u64;6];let p=out.as_mut_ptr()as usize;
  ci(p,id);let data_ptr=crate::read_ptr(p+8);assert_ne!(data_ptr,0);assert_ne!(data_ptr,cis.row(roll.source_chara()));assert_eq!(crate::read_i32(p),id);
  ci(np,roll.source_chara());assert_eq!(crate::read_ptr(np+8),cis.row(roll.source_chara()));
  for (left,hand) in std::iter::once((false,roll.right)).chain(roll.left.map(|h|(true,h))){
   let meta=crate::demons_data::WEAPONS.iter().find(|w|w.id==hand.source).unwrap();let art=if meta.art_table>=0{1234}else{hand.art};let spell=if meta.spell_table>=0{5678}else{hand.spell};
   let id=demons::weapon_id(seed,left).unwrap();let original:Lookup=std::mem::transmute(WEAPON_ORIGINAL.load(Ordering::Acquire));
   for level in [0,12]{
    original(np,hand.source+level);let expected=normal;w(p,id+level);assert_eq!(crate::read_i32(p),id+level);assert_eq!(crate::read_i32(p+0x10),id);assert_eq!(out[3..],expected[3..]);
    let row=crate::read_ptr(p+8);assert_eq!(crate::read_i32(row+data::WEAPON_ART),art);assert_eq!(crate::read_i32(row+data::WEAPON_SPELL),spell);
    let mut bytes=std::slice::from_raw_parts(weapons.row(hand.source)as*const u8,data::WEAPON_SIZE).to_vec();bytes[data::WEAPON_ART..data::WEAPON_ART+4].copy_from_slice(&art.to_le_bytes());bytes[data::WEAPON_SPELL..data::WEAPON_SPELL+4].copy_from_slice(&spell.to_le_bytes());assert_eq!(std::slice::from_raw_parts(row as*const u8,data::WEAPON_SIZE),bytes);
   }
  }
 }
 ACTIVE.store(false,Ordering::Release);let id=demons::chara_id(42).unwrap();ci(np,id);assert_eq!(crate::read_ptr(np+8),0);w(np,demons::weapon_id(42,false).unwrap());assert_eq!(crate::read_ptr(np+8),0);
 ACTIVE.store(active,Ordering::Release);crate::BASE.store(previous_base,Ordering::Release);for(original,old)in saved{original.store(old,Ordering::Release);}TABLES.with(|v|v.borrow_mut().clear());VirtualFree(image,0,MEM_RELEASE);
}}
