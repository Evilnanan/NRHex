use super::*;
thread_local!{
 static MARKER:Cell<usize>=const{Cell::new(0)};
 static APPLIED:Cell<usize>=const{Cell::new(0)};
}
unsafe extern "system" fn apply(player:usize,effect:i32,_:u8)->u8{
 assert_eq!(effect,FLASK);
 let manager=read_ptr(player+0x190);let marker=MARKER.with(Cell::get);
 integer(marker+0x68,4);pointer(marker+0x38,read_ptr(manager+8));pointer(manager+8,marker);
 APPLIED.with(|count|count.set(count.get()+1));1
}
unsafe extern "system" fn remove(manager:usize,effect:i32)->u8{
 assert_eq!(effect,FLASK);let mut link=manager+8;
 loop{let node=read_ptr(link);if node==0{return 0;}
  if read_i32(node+8)==effect{pointer(link,read_ptr(node+0x38));return 1;}
  link=node+0x38;
 }
}
pub unsafe fn run(base:usize,raw:&[u8]){
 let headers=crate::pe::Headers::parse(raw,0).unwrap();
 for(rva,length)in [(crate::HAS_EFFECT_RVA,0x21),(crate::HAS_STATE_RVA,0x94)]{
  let section=headers.sections.iter().find(|section|rva>=section.virtual_address as usize&&rva+length<=section.virtual_address as usize+section.raw_size as usize).unwrap();
  let start=section.raw_offset as usize+rva-section.virtual_address as usize;patch(base,rva,&raw[start..start+length]);
 }
 for(rva,target)in [(crate::APPLY_SELF_RVA,apply as*const()as usize),(crate::REMOVE_EFFECT_RVA,remove as*const()as usize)]{
  let mut code=vec![0x48,0xb8];code.extend_from_slice(&target.to_le_bytes());code.extend_from_slice(&[0xff,0xe0]);patch(base,rva,&code);
 }
 let mut row=vec![0u64;0x180/8];let row=row.as_mut_ptr()as usize;
 ptr::write_unaligned((row+0x156)as*mut u16,2015);
 let mut marker=vec![0u64;0x80/8];let marker=marker.as_mut_ptr()as usize;
 pointer(marker,row);integer(marker+8,FLASK);MARKER.with(|state|state.set(marker));APPLIED.with(|count|count.set(0));
 let mut native=vec![0u64;0x80/8];let native=native.as_mut_ptr()as usize;
 pointer(native,row);integer(native+8,8440200);integer(native+0x68,4);
 let mut manager=vec![0u64;0x40/8];let manager=manager.as_mut_ptr()as usize;
 pointer(manager+0x28,usize::MAX);pointer(manager+8,native);
 let mut player=vec![0u64;0x198/8];let player=player.as_mut_ptr()as usize;pointer(player+0x190,manager);
 update_flask(player,base,false);assert!(has_effect(player,8440200,base));assert!(crate::has_state(player,2015,base));
 update_flask(player,base,true);update_flask(player,base,true);
 assert_eq!(APPLIED.with(Cell::get),1);assert!(has_effect(player,FLASK,base));
 pointer(manager+8,0);assert!(!crate::has_state(player,2015,base));
 update_flask(player,base,false);update_flask(player,base,true);
 assert_eq!(APPLIED.with(Cell::get),2);assert!(crate::has_state(player,2015,base));
 pointer(manager+8,0);update_flask(player,base,true);
 assert_eq!(APPLIED.with(Cell::get),3);assert!(crate::has_state(player,2015,base));
 pointer(native+0x38,marker);pointer(manager+8,native);pointer(marker+0x38,0);
 update_flask(player,base,false);
 assert!(!has_effect(player,FLASK,base));assert!(has_effect(player,8440200,base));assert!(crate::has_state(player,2015,base));
 let mut replacement=vec![0u64;0x40/8];let replacement=replacement.as_mut_ptr()as usize;pointer(replacement+0x28,usize::MAX);
 pointer(player+0x190,replacement);update_flask(player,base,true);
 assert_eq!(APPLIED.with(Cell::get),4);assert!(crate::has_state(player,2015,base));
 update_flask(player,base,false);assert!(!crate::has_state(player,2015,base));
 MARKER.with(|state|state.set(0));
}
