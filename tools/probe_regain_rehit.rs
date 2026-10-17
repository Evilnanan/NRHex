use std::{ffi::c_void,ptr};
#[link(name="kernel32")]
extern "system"{fn VirtualAlloc(p:*mut c_void,n:usize,t:u32,protect:u32)->*mut c_void;fn VirtualFree(p:*mut c_void,n:usize,t:u32)->i32;}
fn u16le(b:&[u8],p:usize)->usize{u16::from_le_bytes(b[p..p+2].try_into().unwrap())as usize}
fn u32le(b:&[u8],p:usize)->usize{u32::from_le_bytes(b[p..p+4].try_into().unwrap())as usize}
unsafe fn patch(base:usize,rva:usize,b:&[u8]){ptr::copy_nonoverlapping(b.as_ptr(),(base+rva)as*mut u8,b.len());}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn number(p:usize)->f32{ptr::read_unaligned(p as*const f32)}
unsafe fn state(label:&str,s:usize){println!("{{\"step\":\"{label}\",\"hp\":{},\"pool\":{},\"protected\":{},\"window\":{},\"pending\":{}}}",ptr::read_unaligned((s+0x140)as*const i32),number(s+0x168),number(s+0x16c),number(s+0x170),number(s+0x174));}
fn main(){unsafe{
 let raw=std::fs::read(std::env::args_os().nth(1).unwrap()).unwrap();let nt=u32le(&raw,0x3c);let optional=nt+24;let size=u32le(&raw,optional+56);
 let memory=VirtualAlloc(ptr::null_mut(),size,0x3000,0x40);assert!(!memory.is_null());let base=memory as usize;let sections=optional+u16le(&raw,nt+20);
 for i in 0..u16le(&raw,nt+6){let h=sections+i*40;let count=u32le(&raw,h+16);let start=u32le(&raw,h+20);patch(base,u32le(&raw,h+12),&raw[start..start+count]);}
 for(rva,code)in [(0x6ada30,&[0x31,0xc0,0xc3][..]),(0x42c120,&[0x48,0x8b,0xc1,0xc3]),(0x42c130,&[0x48,0x8b,0xc1,0xc3]),(0x3d4180,&[0xb0,1,0xc3]),(0x508e20,&[0xb0,1,0xc3])]{patch(base,rva,code);}
 let set:unsafe extern "system" fn(usize,i32,u8,f32,f32,u8)=std::mem::transmute(base+0x426b00);
 let queue:unsafe extern "system" fn(usize,f32)=std::mem::transmute(base+0x4275a0);
 let tick:unsafe extern "system" fn(usize,f32)=std::mem::transmute(base+0x4289c0);
 let mut stats=vec![0u64;0x200/8];let s=stats.as_mut_ptr()as usize;
 for clear in [1,0]{
  ptr::write_bytes(s as*mut u8,0,0x200);int(s+0x140,1000);int(s+0x144,1000);
  set(s,800,1,1.,1.,1);state("first_hit_200",s);tick(s,1.);assert_eq!(number(s+0x168),200.);
  set(s,700,clear,1.,1.,1);state(&format!("second_hit_100_clear_{clear}"),s);
  assert_eq!(number(s+0x168),300.);assert_eq!(number(s+0x16c),if clear==1{100.}else{300.});
  tick(s,0.1);state("after_0.1_seconds",s);assert!((number(s+0x168)-if clear==1{200.}else{300.}).abs()<0.01);
  tick(s,0.1);state("after_0.2_seconds",s);assert!((number(s+0x168)-if clear==1{100.}else{300.}).abs()<0.01);
  queue(s,999.);tick(s,0.016);state("attack_recovery",s);assert_eq!(ptr::read_unaligned((s+0x140)as*const i32),if clear==1{800}else{1000});
 }
 VirtualFree(memory,0,0x8000);
 println!("PASS: original native setter and settlement confirm only the new hit remains protected when clear=1; clear=0 preserves protection of both");
}}
