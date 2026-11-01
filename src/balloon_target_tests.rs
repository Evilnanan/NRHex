use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};

#[test]
fn balloon_target_bonus_is_per_recipient_and_only_changes_temporary_physical_input(){unsafe{
 let mut target=[0usize;0x200/8];let t=target.as_mut_ptr()as usize;
 let mut stats=[0i32;0x150/4];stats[0x140/4]=1;stats[0x144/4]=10000;
 let modules=[stats.as_mut_ptr()as usize];target[0x1b8/8]=modules.as_ptr()as usize;
 let mut module=[0usize,t];let m=module.as_mut_ptr()as usize;
 let mut hit=[0u8;0x2c8];let h=hit.as_mut_ptr()as usize;
 ptr::write_unaligned((h+0x40)as*mut i32,balloon::ATTACK);ptr::write_unaligned((h+0x44)as*mut i32,1);
 let saved=hit;
 for maximum in [1,1000,10000,70000,i32::MAX]{
  stats[0x144/4]=maximum;
  for source in [123usize,456]{
   let mut attack=[1500.,21.,32.,43.,54.];add(true,m,source,h,attack.as_mut_ptr());
   assert_eq!(attack[0],1500.+maximum as f32*0.07);assert_eq!(&attack[1..],&[21.,32.,43.,54.]);assert_eq!(hit,saved);
  }
 }
 for case in 0..12{
  hit=saved;module[1]=t;stats[0x140/4]=1;stats[0x144/4]=10000;target[0x1b8/8]=modules.as_ptr()as usize;
  let mut active=true;let mut source=123;let mut module_arg=m;let mut hit_arg=h;
  match case{
   0=>active=false,1=>module_arg=0,2=>source=0,3=>source=t,4=>hit_arg=0,
   5=>ptr::write_unaligned((h+0x40)as*mut i32,12064105),
   6=>ptr::write_unaligned((h+0x44)as*mut i32,0),7=>hit[0x2c1]=8,
   8=>module[1]=0,9=>target[0x1b8/8]=0,10=>stats[0x140/4]=0,11=>stats[0x144/4]=0,_=>unreachable!()
  }
  let mut attack=[1500.,21.,32.,43.,54.];let before=attack;add(active,module_arg,source,hit_arg,attack.as_mut_ptr());assert_eq!(attack,before,"case {case}");
 }
 ptr::copy_nonoverlapping(saved.as_ptr(),h as*mut u8,saved.len());module[1]=t;stats[0x140/4]=1;stats[0x144/4]=10000;target[0x1b8/8]=modules.as_ptr()as usize;
 for value in [-1.,f32::NAN,f32::INFINITY]{let mut a=[value;5];add(true,m,123,h,a.as_mut_ptr());assert_eq!(a[0].to_bits(),value.to_bits());}
}}

static ENTRY:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
// Recreate 439E80's register/frame contract, then execute its actual vector
// copy, installed Rust hook and relocated MOVZX/SHR/AND instructions.
#[unsafe(naked)]
unsafe extern "system" fn fixture(_:usize,_:usize,_:usize,_:*mut u8){
 core::arch::naked_asm!(
  "push rbp", "push rbx", "push r12", "push r14", "push r15", "sub rsp,0x100",
  "lea rbp,[rsp+0x80]", "mov r15,rcx", "mov r12,rdx", "mov r14,r8", "mov [rsp+0xc0],r9",
  "xor ebx,ebx", "mov eax,0x12345678", "mov r10d,0x41480000", "movd xmm1,r10d",
  "movaps xmm2,xmm1", "movaps xmm3,xmm1", "movaps xmm4,xmm1", "movaps xmm5,xmm1",
  "mov r10d,0x7788", "mov r11d,0x8899", "jmp qword ptr [rip+{entry}]",entry=sym ENTRY
 );
}
#[unsafe(naked)]
unsafe extern "system" fn resume(){
 core::arch::naked_asm!(
  "mov [rsp+0xd0],rax", "mov rax,[rsp+0xc0]",
  "mov [rax+0x28],rcx", "mov [rax+0x30],rdx", "mov [rax+0x38],r8", "mov [rax+0x40],r9",
  "mov [rax+0x48],r10", "mov [rax+0x50],r11", "mov [rax+0x58],rbx",
  "movdqu [rax+0x60],xmm0", "movdqu [rax+0x70],xmm1", "movdqu [rax+0x80],xmm2",
  "movdqu [rax+0x90],xmm3", "movdqu [rax+0xa0],xmm4", "movdqu [rax+0xb0],xmm5",
  "mov r10,[rsp+0xd0]", "mov [rax+0x20],r10",
  "movups xmm0,[rbp-0x78]", "movups [rax],xmm0", "movss xmm0,[rbp-0x68]", "movss [rax+0x10],xmm0",
  "add rsp,0x100", "pop r15", "pop r14", "pop r12", "pop rbx", "pop rbp", "ret"
 );
}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; run alone"]
fn balloon_target_native_input_hook_and_defense_pipeline(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for sec in &headers.sections{ptr::copy_nonoverlapping(raw[sec.raw_offset as usize..].as_ptr(),(base+sec.virtual_address as usize)as*mut u8,sec.raw_size as usize);}
 let signature=PATTERN.split_whitespace().map(|s|u8::from_str_radix(s,16).unwrap()).collect::<Vec<_>>();
 let hits=headers.sections.iter().filter(|s|s.is_executable()).flat_map(|s|raw[s.raw_offset as usize..s.raw_offset as usize+s.raw_size as usize].windows(signature.len()).enumerate().filter(|(_,b)|*b==signature).map(move |(n,_)|s.virtual_address as usize+n)).collect::<Vec<_>>();assert_eq!(hits,vec![RVA]);
 assert_eq!(std::slice::from_raw_parts((base+RVA)as*const u8,signature.len()),signature);
 let prepared=crate::hook::prepare((base+RVA)as*mut u8,detour as*const u8,64).unwrap();assert_eq!(prepared.stolen,8);
 let saved_original=ORIGINAL.swap(prepared.trampoline,Ordering::AcqRel);let saved_active=ACTIVE.swap(true,Ordering::AcqRel);
 ptr::copy_nonoverlapping(prepared.patch.as_ptr(),(base+RVA)as*mut u8,prepared.patch.len());
 let mut jump=vec![0xff,0x25,0,0,0,0];jump.extend_from_slice(&(resume as*const()as usize).to_le_bytes());ptr::copy_nonoverlapping(jump.as_ptr(),(base+0x43a3ac)as*mut u8,jump.len());
 ENTRY.store((base+0x43a38c)as*mut u8,Ordering::Release);
 let mut actor=[0usize;0x200/8];let target=actor.as_mut_ptr()as usize;let mut stats=[0i32;0x150/4];stats[0x140/4]=1;
 let modules=[stats.as_mut_ptr()as usize];actor[0x1b8/8]=modules.as_ptr()as usize;let module=[0usize,target];let m=module.as_ptr()as usize;
 let mut hit=[0u8;0x2c8];let h=hit.as_mut_ptr()as usize;ptr::write_unaligned(h as*mut[f32;5],[1500.,0.,0.,0.,0.]);ptr::write_unaligned((h+0x40)as*mut i32,balloon::ATTACK);ptr::write_unaligned((h+0x44)as*mut i32,1);hit[0x161]=2;
 type Combine=unsafe extern "system" fn(*mut f32,*const f32,*const f32,*const f32,*const f32,*const f32,*const f32,*const f32,f32,f32,f32,f32,f32,f32,f32)->f32;
 let combine:Combine=std::mem::transmute(base+0x6c5fc0);
 let one=[1.;5];
 for maximum in [1000,10000,50000]{stats[0x144/4]=maximum;for source in [123usize,456]{for cached in [false,true]{
  hit[0x2c1]=if cached{8}else{0};let before=hit;let mut out=[0u64;0xc0/8];let o=out.as_mut_ptr()as*mut u8;
  fixture(m,source,h,o);assert_eq!(hit,before,"Shared hit must remain unchanged");let attack=ptr::read_unaligned(o as*const[f32;5]);
  let expected=1500.+if cached{0.}else{maximum as f32*0.07};assert_eq!(attack,[expected,0.,0.,0.,0.]);
  assert_eq!(&out[0x20/8..0x60/8],&[0x12345678,m as u64,source as u64,h as u64,o as u64,0x7788,0x8899,1]);
  for offset in (0x70..0xc0).step_by(16){assert_eq!(ptr::read_unaligned(o.add(offset)as*const[f32;4]),[12.5,0.,0.,0.]);}
  for rate in [0.5f32,1.,2.]{for cut in [0.2f32,1.,1.5]{for guard in [0f32,0.25,1.]{
   let rates=[rate;5];let cuts=[cut;5];let guards=[guard;5];let mut result=[0.;5];
   let actual=combine(result.as_mut_ptr(),attack.as_ptr(),rates.as_ptr(),cuts.as_ptr(),one.as_ptr(),guards.as_ptr(),one.as_ptr(),one.as_ptr(),1.,1.,1.,1.,1.,1.,1.);
   let mut reference=[0.;5];let input=[expected,0.,0.,0.,0.];let wanted=combine(reference.as_mut_ptr(),input.as_ptr(),rates.as_ptr(),cuts.as_ptr(),one.as_ptr(),guards.as_ptr(),one.as_ptr(),one.as_ptr(),1.,1.,1.,1.,1.,1.,1.);
   assert!(actual.is_finite()&&actual>=0.);assert!((actual-expected*rate*cut*guard).abs()<=actual.abs().max(1.)*1e-5);assert_eq!(actual,wanted);assert_eq!(result,reference);
   if !cached&&rate==1.&&cut==0.2&&guard==1.{let original=[1500.,0.,0.,0.,0.];let old=combine(reference.as_mut_ptr(),original.as_ptr(),rates.as_ptr(),cuts.as_ptr(),one.as_ptr(),guards.as_ptr(),one.as_ptr(),one.as_ptr(),1.,1.,1.,1.,1.,1.,1.);assert!((actual-(old+maximum as f32*0.07)).abs()>1.,"Bonus must be reduced before final HP damage");}
  }}}
 }}}
 ACTIVE.store(saved_active,Ordering::Release);ORIGINAL.store(saved_original,Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
