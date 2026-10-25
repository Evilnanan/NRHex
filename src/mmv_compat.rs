//! Narrow validation for the pinned MMV Legendary Stones item-copy detour.
//! NRHex calls the existing entry, so MMV still receives every copy operation.
use crate::{reward_reroll, win};
use sha2::{Digest, Sha256};
use core::ffi::c_void;

const DLL_HASH:&str="51bef3ddc321e6eeaa8da5c087b0cd9a1e71a8604f20a47e7113e18e9c67da59";
const BODY_HASH:&str="1ba00dba6e985ea85c8c2cc0ea4a67a944715079ae47078296a95669c53ff5c1";
const HOOK:usize=0x1750;
const ORIGINAL_SLOT:usize=0xfb028;
const ORIGINAL_TARGET:usize=0x217f781;

#[link(name="kernel32")]
extern "system" {
 fn ReadProcessMemory(process:*mut c_void,address:*const c_void,buffer:*mut c_void,length:usize,read:*mut usize)->i32;
}
unsafe fn read(address:usize,size:usize)->Result<Vec<u8>,String>{
 let mut bytes=vec![0u8;size];let mut amount=0;
 if ReadProcessMemory(win::GetCurrentProcess(),address as *const _,bytes.as_mut_ptr()as*mut _,size,&mut amount)==0||amount!=size {
  return Err(format!("MMV item-copy chain is unreadable at {address:#x}"));
 }
 Ok(bytes)
}
fn absolute_jump(bytes:&[u8])->Option<usize>{
 if bytes.len()!=14||bytes[..6]!=[0xff,0x25,0,0,0,0]{return None;}
 Some(u64::from_le_bytes(bytes[6..14].try_into().ok()?)as usize)
}
fn relative_jump(address:usize,bytes:&[u8])->Option<usize>{
 if bytes.len()<5||bytes[0]!=0xe9{return None;}
 address.checked_add(5)?.checked_add_signed(i32::from_le_bytes(bytes[1..5].try_into().ok()?)as isize)
}
fn validate_chain(base:usize,module:usize,head:&[u8],relay:&[u8],original:&[u8])->bool{
 let expected:Vec<u8>=reward_reroll::ITEM_COPY_PATTERN.split_whitespace().map(|v|u8::from_str_radix(v,16).unwrap()).collect();
 head.len()==expected.len()&&head[0]==0xe9&&head[5..]==expected[5..]&&
  absolute_jump(relay)==Some(module+HOOK)&&absolute_jump(original)==Some(base+ORIGINAL_TARGET)
}
pub unsafe fn accepts(base:usize,rva:u32)->Result<bool,String>{
 if rva as usize!=reward_reroll::ITEM_COPY_RVA{return Ok(false);}
 let module=win::GetModuleHandleW(win::wide("RestoredLegendaryStones.dll").as_ptr());
 if module.is_null(){return Ok(false);}
 let path=crate::module_path(module)?;
 let file=std::fs::read(path).map_err(|e|e.to_string())?;
 if format!("{:x}",Sha256::digest(file))!=DLL_HASH{return Err("Unreviewed MMV Legendary Stones DLL version".into());}
 let module=module as usize;
 if format!("{:x}",Sha256::digest(read(module+HOOK,0x106)?))!=BODY_HASH{return Err("MMV Legendary Stones hook body changed".into());}
 let head=read(base+rva as usize,reward_reroll::ITEM_COPY_PATTERN.split_whitespace().count())?;
 let Some(relay)=relative_jump(base+rva as usize,&head) else{return Ok(false);};
 let original=u64::from_le_bytes(read(module+ORIGINAL_SLOT,8)?.try_into().unwrap())as usize;
 Ok(validate_chain(base,module,&head,&read(relay,14)?,&read(original,14)?))
}

#[cfg(test)]mod tests{
 use super::*;
 fn jump(to:usize)->Vec<u8>{let mut v=vec![0xff,0x25,0,0,0,0];v.extend_from_slice(&(to as u64).to_le_bytes());v}
 #[test]fn only_reviewed_chain_and_unchanged_native_tail_are_accepted(){
  let base=0x140000000;let module=0x180000000;
  let mut head:Vec<u8>=reward_reroll::ITEM_COPY_PATTERN.split_whitespace().map(|v|u8::from_str_radix(v,16).unwrap()).collect();
  head[1..5].copy_from_slice(&(-0x1000i32).to_le_bytes());
  let relay=jump(module+HOOK);let original=jump(base+ORIGINAL_TARGET);
  assert_eq!(relative_jump(base+reward_reroll::ITEM_COPY_RVA,&head),Some(base+reward_reroll::ITEM_COPY_RVA+5-0x1000));
  assert!(validate_chain(base,module,&head,&relay,&original));
  assert!(!validate_chain(base,module,&head,&jump(module+HOOK+1),&original));
  assert!(!validate_chain(base,module,&head,&relay,&jump(base+ORIGINAL_TARGET+1)));
  assert!(!validate_chain(base,module,&head,&relay[..13],&original));
  head[8]^=1;assert!(!validate_chain(base,module,&head,&relay,&original));
  assert_eq!(relative_jump(0,&[0xe9,0,0,0,0x80]),None);
 }
}
