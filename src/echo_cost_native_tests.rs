//! Execute the original Magic stamina getter, actor modifier dispatch, cached
//! cost store and native stamina consumer, with the compiled mid-function hook.
use super::*;
use crate::{ptr,win::*};
use sha2::{Digest,Sha256};
use std::sync::atomic::{AtomicBool,AtomicI32};
static OWNED:AtomicBool=AtomicBool::new(true);
static SCALE:AtomicI32=AtomicI32::new(100);
static FLAT:AtomicI32=AtomicI32::new(0);
static START:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn patch(base:usize,rva:usize,b:&[u8]){ptr::copy_nonoverlapping(b.as_ptr(),(base+rva)as*mut u8,b.len());}
unsafe fn jump(base:usize,rva:usize,f:usize){let mut b=vec![0xff,0x25,0,0,0,0];b.extend_from_slice(&f.to_le_bytes());patch(base,rva,&b);}
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from(id==crate::echo_cast::OWNER&&OWNED.load(Ordering::Relaxed))}
unsafe extern "system" fn modified(actor:usize,raw:i32,weapon:i32,flag:u8,spell:u32)->i32{
 assert_ne!(actor,0);assert_eq!((weapon,flag,spell),(-1,0,1));
 raw*SCALE.load(Ordering::Relaxed)/100+FLAT.load(Ordering::Relaxed)
}
// The original virtual dispatch uses the native shadow-space fifth argument.
// Deliberately fill every volatile register to detect detour ABI corruption.
#[unsafe(naked)]unsafe extern "system" fn modifier(){core::arch::naked_asm!(
 "sub rsp,0x38", "mov eax,[rsp+0x60]", "mov [rsp+0x20],eax", "call {modified}", "add rsp,0x38",
 "mov r8,0x88888888", "mov r9,0x99999999", "mov r10,0xaaaaaaaa", "mov r11,0xbbbbbbbb",
 "pcmpeqd xmm0,xmm0", "pcmpeqd xmm1,xmm1", "pcmpeqd xmm2,xmm2",
 "pcmpeqd xmm3,xmm3", "pcmpeqd xmm4,xmm4", "pcmpeqd xmm5,xmm5", "ret",modified=sym modified
);}
// Recreate the constructor's live nonvolatile registers and frame at 3849AD.
// Five pushes leave RSP aligned to 16, as in the actual constructor interior.
#[unsafe(naked)]unsafe extern "system" fn calculate(_:usize,_:usize,_:u8,_:usize)->i32{core::arch::naked_asm!(
 "push rbx", "push rbp", "push rsi", "push rdi", "push r12", "sub rsp,0xa0",
 "mov r12,r9", "mov rdi,rcx", "lea rbp,[rsp+0x80]", "mov [rbp-0x31],rdx",
 "mov dword ptr [rbp-0x39],4021", "movzx ebx,r8b", "mov esi,-1",
 "lea rcx,[rbp-0x39]", "movzx edx,bl", "jmp qword ptr [rip+{start}]",start=sym START
);}
#[unsafe(naked)]unsafe extern "system" fn finish(){core::arch::naked_asm!(
 "mov [r12],r8", "mov [r12+8],r9", "mov [r12+16],r10", "mov [r12+24],r11",
 "movdqu [r12+32],xmm0", "movdqu [r12+48],xmm1", "movdqu [r12+64],xmm2",
 "movdqu [r12+80],xmm3", "movdqu [r12+96],xmm4", "movdqu [r12+112],xmm5",
 "mov [r12+128],rcx", "mov [r12+136],rdx", "lea rax,[rbp-0x39]", "mov [r12+144],rax",
 "mov [r12+152],rsi", "mov [r12+160],rbx", "mov [r12+168],rdi",
 "mov eax,[rdi+0xc0]", "add rsp,0xa0", "pop r12", "pop rdi", "pop rsi", "pop rbp", "pop rbx", "ret"
);}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE and NRHEX_ECHO_MAGIC_FIXTURE; run alone"]
fn native_echo_spell_stamina_cost_and_abi(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let fixture=std::fs::read(std::env::var_os("NRHEX_ECHO_MAGIC_FIXTURE").unwrap()).unwrap();assert_eq!(u16::from_le_bytes(fixture[10..12].try_into().unwrap()),1);
 assert_eq!(i32::from_le_bytes(fixture[64..68].try_into().unwrap()),4021);
 let off=u64::from_le_bytes(fixture[72..80].try_into().unwrap())as usize;
 let original=fixture[off..off+0x60].to_vec();let mut magic=original.clone();assert_eq!(magic[0x26],0);
 let normal=i16::from_le_bytes(magic[0x12..0x14].try_into().unwrap())as i32;
 let charged=i16::from_le_bytes(magic[0x5e..0x60].try_into().unwrap())as i32;assert_eq!((normal,charged),(35,46));
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for s in &headers.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 jump(base,crate::HAS_EFFECT_RVA,has as*const()as usize);
 // Stop only after the original NEG and [context+C0] store have executed.
 jump(base,0x3849de,finish as*const()as usize);START.store((base+0x3849ad)as*mut u8,Ordering::Release);
 let p=crate::hook::prepare((base+RVA)as*mut u8,detour as*const u8,64).unwrap();assert_eq!(p.stolen,7);let saved_original=ORIGINAL.swap(p.trampoline,Ordering::AcqRel);crate::hook::write_code((base+RVA)as*mut u8,&p.patch).unwrap();
 let saved_base=BASE.swap(base,Ordering::AcqRel);let saved_active=ACTIVE.swap(true,Ordering::AcqRel);
 let mut world=vec![0usize;0x17500/8];let mut actor=[0usize;0x200/8];let mut vt=[0usize;0x350/8];let actor=actor.as_mut_ptr()as usize;put(actor,vt.as_mut_ptr()as usize);vt[0x348/8]=modifier as*const()as usize;
 world[0x174e8/8]=actor;put(base+crate::WORLD_RVA,world.as_mut_ptr()as usize);put(actor+0x190,actor);
 let mut context=[0usize;0x100/8];let ctx=context.as_mut_ptr()as usize;put(ctx,actor);
 let mut snapshot=[0u64;22];let snapshot_ptr=snapshot.as_mut_ptr()as usize;
 let run=|row:usize,mode:u8,expected:i32|{
  assert_eq!(calculate(ctx,row,mode,snapshot_ptr),-expected);assert_eq!(crate::read_i32(ctx+0xc0),-expected);
  assert_eq!(&snapshot[..4],&[0x88888888,0x99999999,0xaaaaaaaa,0xbbbbbbbb]);assert!(snapshot[4..16].iter().all(|&n|n==u64::MAX));
  assert_eq!(snapshot[16],snapshot[18]);assert_eq!(snapshot[17],u64::from(mode));assert_eq!(snapshot[19],u64::from(u32::MAX));assert_eq!(snapshot[20],u64::from(mode));assert_eq!(snapshot[21],ctx as u64);
 };
 let row=magic.as_mut_ptr()as usize;
 for(scale,flat)in [(100,0),(80,3),(150,0),(0,0)]{
  SCALE.store(scale,Ordering::Relaxed);FLAT.store(flat,Ordering::Relaxed);
  for(mode,native)in [(0,normal),(1,charged)]{run(row,mode,(native*scale/100+flat)*3/2);}
 }
 SCALE.store(100,Ordering::Relaxed);FLAT.store(0,Ordering::Relaxed);
 for kind in [0,1,2,3,255]{ptr::write((row+0x26)as*mut u8,kind);run(row,0,if kind<=1{52}else{35});}ptr::write((row+0x26)as*mut u8,0);
 OWNED.store(false,Ordering::Relaxed);run(row,0,35);OWNED.store(true,Ordering::Relaxed);
 world[0x174e8/8]=0;run(row,1,46);world[0x174e8/8]=actor;
 ACTIVE.store(false,Ordering::Release);run(row,0,35);ACTIVE.store(true,Ordering::Release);
 run(0,0,0);run(row,2,0);assert_eq!(magic,original);
 for cost in [0,-1,-30]{assert_eq!(stamina(ctx,row,cost),cost);}
 assert_eq!(stamina(0,row,35),35);assert_eq!(stamina(ctx,0,35),35);
 // Consumption uses the cached negative value. Reuse pays once each native
 // interval, never compounds the multiplier in the cached context.
 let mut stats=[0u32;0x200/4];let stats=stats.as_mut_ptr()as usize;int(stats+0x15c,200);int(stats+0x160,200);
 put(base+0x3c0f338,1);ptr::write((base+0x3c0f2f7)as*mut u8,0);
 let consume:unsafe extern "system" fn(usize,i32,u8)=std::mem::transmute(base+crate::STAMINA_ADD_RVA);
 run(row,0,52);let cost=crate::read_i32(ctx+0xc0);
 for remaining in [148,96,44,-8,-50]{consume(stats,cost,0);assert_eq!(crate::read_i32(stats+0x15c),remaining);assert_eq!(crate::read_i32(ctx+0xc0),-52);}
 for flag in [1u8,0x20]{int(stats+0x15c,200);ptr::write((stats+0x189)as*mut u8,flag);consume(stats,cost,0);assert_eq!(crate::read_i32(stats+0x15c),200);}
 // Ordinary behavior costs use a different constructor: its code is unpatched.
 let section=headers.sections.iter().find(|s|0x384340>=s.virtual_address&&0x384480<s.virtual_address+s.raw_size).unwrap();let start=(0x384340-section.virtual_address+section.raw_offset)as usize;
 assert_eq!(std::slice::from_raw_parts((base+0x384340)as*const u8,0x140),&raw[start..start+0x140]);
 BASE.store(saved_base,Ordering::Release);ACTIVE.store(saved_active,Ordering::Release);ORIGINAL.store(saved_original,Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
