use super::*;
use crate::win::*;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
static OWNED:AtomicBool=AtomicBool::new(false);
static RESET:AtomicBool=AtomicBool::new(false);
static START:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static LABELS:Mutex<Vec<(usize,Vec<u8>)>>=Mutex::new(Vec::new());
static SNAPSHOTS:Mutex<Vec<(usize,u32,[u8;0x60])>>=Mutex::new(Vec::new());
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from(match id{crate::basics::OWNER=>OWNED.load(Ordering::Relaxed),crate::reward_cycle_runtime::RESET=>RESET.load(Ordering::Relaxed),_=>false})}
unsafe extern "system" fn visibility(_:usize,_:u8,_:usize){}
unsafe extern "system" fn label(object:usize,name:*const usize)->u8{
 let p=*name as*const std::ffi::c_char;LABELS.lock().unwrap().push((object,std::ffi::CStr::from_ptr(p).to_bytes().to_vec()));1
}
unsafe extern "system" fn capture(object:usize,dt:f32,state:usize){SNAPSHOTS.lock().unwrap().push((object,dt.to_bits(),ptr::read_unaligned(state as*const [u8;0x60])));}
unsafe fn jump(base:usize,rva:usize,target:usize){let mut bytes=vec![0xff,0x25,0,0,0,0];bytes.extend_from_slice(&target.to_le_bytes());ptr::copy_nonoverlapping(bytes.as_ptr(),(base+rva)as*mut u8,bytes.len());}
#[unsafe(naked)]unsafe extern "system" fn run(_:usize,_:usize,_:f32){core::arch::naked_asm!(
 "push rsi", "push rdi", "sub rsp,0x38", "movups [rsp+0x20],xmm6",
 "mov rdi,rcx", "mov rsi,rdx", "movaps xmm6,xmm2", "jmp qword ptr [rip+{start}]",start=sym START
);}
#[unsafe(naked)]unsafe extern "system" fn finish(){core::arch::naked_asm!("movups xmm6,[rsp+0x20]", "add rsp,0x38", "pop rdi", "pop rsi", "ret");}
#[test]#[ignore="requires NRHEX_NATIVE_EXE; execute alone"]
fn native_basics_hud_only_dims_the_ultimate_display(){unsafe{
 let path=std::path::PathBuf::from(std::env::var_os("NRHEX_NATIVE_EXE").unwrap());crate::verify_executable(&path).unwrap();
 let raw=std::fs::read(path).unwrap();let headers=crate::pe::Headers::parse(&raw,0).unwrap();
 let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for s in &headers.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 jump(base,crate::HAS_EFFECT_RVA,has as*const()as usize);jump(base,0x7bb2f0,visibility as*const()as usize);jump(base,0x77a7f0,label as*const()as usize);
 // Run the exact native selector, then its epilogue. Subsequent rendering is
 // not executed: GFx timelines are separately verified from the original GFX.
 jump(base,0x99a0d1,base+0x99a3bf);jump(base,RVA+5,finish as*const()as usize);
 let p=crate::hook::prepare((base+RVA)as*mut u8,detour as*const u8,64).unwrap();assert_eq!(p.stolen,5);crate::hook::write_code((base+RVA)as*mut u8,&p.patch).unwrap();
 let saved_base=BASE.swap(base,Ordering::AcqRel);let saved_active=ACTIVE.load(Ordering::Acquire);
 CONTINUE.store((base+RVA+5)as*mut u8,Ordering::Release);START.store((base+SITE_RVA)as*mut u8,Ordering::Release);
 let mut world=vec![0usize;0x17500/8];let mut actor=[0usize;0x200/8];let a=actor.as_mut_ptr()as usize;actor[0x190/8]=a;world[0x174e8/8]=a;
 ptr::write_unaligned((base+crate::WORLD_RVA)as*mut usize,world.as_mut_ptr()as usize);
 let mut hud=vec![0u64;0x8000/8];let h=hud.as_mut_ptr()as usize;
 let mut packet=vec![0xabu8;0x1138];packet[0x1078]=1;packet[0x10d8]=1;let mut cases=0;
 for active in [false,true]{for owned in [false,true]{for reset in [false,true]{for missing in [false,true]{
  ACTIVE.store(active,Ordering::Release);OWNED.store(owned,Ordering::Relaxed);RESET.store(reset,Ordering::Relaxed);world[0x174e8/8]=if missing{0}else{a};
  for dialogue in [0u8,1]{for gray in [0u8,1]{
   for off in [0x1078,0x10d8]{packet[off+2]=dialogue;packet[off+3]=gray;}
   let before=packet.clone();LABELS.lock().unwrap().clear();run(h,packet.as_ptr()as usize,0.016);
   let forced=active&&owned&&!reset&&!missing;
   let name=|gray:bool|if gray{b"Grayout".to_vec()}else{b"Normal".to_vec()};
   assert_eq!(*LABELS.lock().unwrap(),vec![(h+0x72c8+0x70,name(dialogue!=0&&gray==0)),(h+0x72c8+0xe8,name(gray!=0)),(h+0x78f8+0x70,name(dialogue!=0&&gray==0&&!forced)),(h+0x78f8+0xe8,name(gray!=0||forced))]);
   assert_eq!(packet,before,"shared HUD snapshot must stay byte-identical");cases+=1;
  }}
 }}}}
 // Repeated acquire/loss/reset calls must recover native state on the next
 // display update; zero delta still updates the visual selector.
 ACTIVE.store(true,Ordering::Release);world[0x174e8/8]=a;packet[0x10db]=0;
 for (owned,reset)in [(true,false),(false,false),(true,false),(true,true),(true,false),(false,false)]{
  OWNED.store(owned,Ordering::Relaxed);RESET.store(reset,Ordering::Relaxed);LABELS.lock().unwrap().clear();run(h,packet.as_ptr()as usize,0.);
  assert_eq!(LABELS.lock().unwrap()[3].1,if owned&&!reset{b"Grayout".as_slice()}else{b"Normal".as_slice()});
 }
 // Verify the complete renderer ABI and payload, including native charge,
 // stack count and any embedded references; only ultimate byte3 may differ.
 jump(base,UPDATE_RVA,capture as*const()as usize);OWNED.store(true,Ordering::Relaxed);RESET.store(false,Ordering::Relaxed);
 for dt in [0.,1./60.,1.]{SNAPSHOTS.lock().unwrap().clear();run(h,packet.as_ptr()as usize,dt);let data=SNAPSHOTS.lock().unwrap();assert_eq!(data.len(),2);
  for (index,offset)in [0x1078,0x10d8].into_iter().enumerate(){let mut expected:[u8;0x60]=packet[offset..offset+0x60].try_into().unwrap();if index==1{expected[3]=1;}
   assert_eq!(data[index],(h+if index==0{0x72c8}else{0x78f8},dt.to_bits(),expected));}
 }
 println!("PASS: {cases} native selector cases; separate skill/ultimate calls, ownership/reset/absence, next-frame restoration, source snapshot and complete renderer ABI");
 BASE.store(saved_base,Ordering::Release);ACTIVE.store(saved_active,Ordering::Release);CONTINUE.store(ptr::null_mut(),Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
