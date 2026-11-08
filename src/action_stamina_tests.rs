use super::*;
use crate::win::*;
use std::sync::atomic::{AtomicBool,AtomicI32};
static ONYX_STARTED:AtomicBool=AtomicBool::new(false);
static ONYX_FINISHED:AtomicBool=AtomicBool::new(false);
static ONYX:AtomicBool=AtomicBool::new(false);
static WEAPON_ID:AtomicI32=AtomicI32::new(2000000);
static ECHO:AtomicBool=AtomicBool::new(true);
static ARTS:AtomicBool=AtomicBool::new(true);
static ROW:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static ENTRY:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static START:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static SOURCE:AtomicI32=AtomicI32::new(-1);
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn jump(base:usize,rva:usize,f:usize){let mut b=vec![0xff,0x25,0,0,0,0];b.extend_from_slice(&f.to_le_bytes());ptr::copy_nonoverlapping(b.as_ptr(),(base+rva)as*mut u8,b.len());}
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from(match id{crate::onyx_mastery::STARTED=>ONYX_STARTED.load(Ordering::Relaxed),crate::onyx_mastery::FINISH=>ONYX_FINISHED.load(Ordering::Relaxed),crate::onyx_mastery::OWNER=>ONYX.load(Ordering::Relaxed),crate::echo_cast::OWNER=>ECHO.load(Ordering::Relaxed),crate::arts::OWNER=>ARTS.load(Ordering::Relaxed),_=>false})}
unsafe extern "system" fn lookup(out:usize,id:i32)->usize{put(out+8,if id==4021{ROW.load(Ordering::Relaxed)as usize}else{0});out}
unsafe extern "system" fn source(_:usize,out:*mut u32,hand:i32)->*mut u32{SOURCE.store(hand,Ordering::Relaxed);*out=123;out}
unsafe extern "system" fn reference(out:usize,_:usize)->usize{put(out,123);put(out+8,ENTRY.load(Ordering::Relaxed)as usize);int(out+0x10,WEAPON_ID.load(Ordering::Relaxed));out}
unsafe extern "system" fn release(_:usize){}
#[unsafe(naked)]unsafe extern "system" fn run(_:usize){core::arch::naked_asm!(
 "push rbx", "sub rsp,0x20", "mov rbx,rcx", "jmp qword ptr [rip+{start}]",start=sym START
);}
#[unsafe(naked)]unsafe extern "system" fn finish(){core::arch::naked_asm!("add rsp,0x20", "pop rbx", "ret");}
#[test]fn storm_uses_actual_normal_variants_not_the_input_button(){
 for family in [400000,400100,400500,400900]{for suffix in [220,225,230,235,240,245]{assert!(storm_behavior(family*1000+suffix));}}
 for id in [-1,0,4200,4220,400000200,400000205,400000210,400000215,400000250,100000220,300000220]{assert!(!storm_behavior(id));}
}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; execute alone"]
fn native_action_stamina_payment_and_immunity(){unsafe{
 let path=std::path::PathBuf::from(std::env::var_os("NRHEX_NATIVE_EXE").unwrap());crate::verify_executable(&path).unwrap();
 let raw=std::fs::read(path).unwrap();let headers=crate::pe::Headers::parse(&raw,0).unwrap();
 let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for s in &headers.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 jump(base,crate::HAS_EFFECT_RVA,has as*const()as usize);jump(base,crate::echo_cost::RVA,lookup as*const()as usize);
 jump(base,crate::ARTS_REF_RVA,reference as*const()as usize);jump(base,0x6a1d90,release as*const()as usize);
 jump(base,RVA+5,finish as*const()as usize);START.store((base+0x384a8b)as*mut u8,Ordering::Release);
 let p=crate::hook::prepare((base+RVA)as*mut u8,detour as*const u8,64).unwrap();assert_eq!(p.stolen,5);crate::hook::write_code((base+RVA)as*mut u8,&p.patch).unwrap();
 let saved_base=BASE.swap(base,Ordering::AcqRel);let saved_active=ACTIVE.swap(true,Ordering::AcqRel);
 CONTINUE.store((base+RVA+5)as*mut u8,Ordering::Release);CLAMP.store((base+CLAMP_RVA)as*mut u8,Ordering::Release);
 let mut world=vec![0usize;0x17500/8];let mut actor=[0usize;0x800/8];let a=actor.as_mut_ptr()as usize;
 let mut stats=[0u32;0x200/4];let st=stats.as_mut_ptr()as usize;let modules=[st];put(a+0x1b8,modules.as_ptr()as usize);put(a+0x190,a);
 let mut vt=[0usize;0x260/8];vt[0x258/8]=source as*const()as usize;put(a,vt.as_ptr()as usize);
 let mut entry=[0usize;4];int(entry.as_mut_ptr()as usize+8,123);int(entry.as_mut_ptr()as usize+0x1c,100);ENTRY.store(entry.as_mut_ptr()as*mut u8,Ordering::Release);
 let state=crate::ARTS.get_or_init(||std::sync::Mutex::new(crate::arts::Arts::default()));let w=crate::arts::Weapon{handle:123,base:2000000,original:100};
 {let mut arts=state.lock().unwrap();arts.synchronize(42,true,true);arts.reforge(w,crate::arts::STORM);}
 world[0x174e8/8]=a;put(base+crate::WORLD_RVA,world.as_mut_ptr()as usize);
 let mut ctx=[0u8;0xd0];let context=ctx.as_mut_ptr()as usize;put(context,a);int(context+8,-1);int(context+0xc,4021);int(context+0x38,-1);
 let mut magic=[0u8;0x60];ROW.store(magic.as_mut_ptr(),Ordering::Relaxed);
 // Actual normal/charged parameter getter, not a handwritten fee selector.
 ptr::write_unaligned(magic.as_mut_ptr().add(0x12)as*mut i16,35);ptr::write_unaligned(magic.as_mut_ptr().add(0x5e)as*mut i16,46);
 let reference=[0usize,magic.as_ptr()as usize];let getter:unsafe extern "system" fn(usize,u8)->i32=std::mem::transmute(base+crate::echo_cost::GETTER_RVA);
 assert_eq!(getter(reference.as_ptr()as usize,0),35);assert_eq!(getter(reference.as_ptr()as usize,1),46);
 put(base+0x3c0f338,1);int(st+0x160,200);
 let mut cases=0;
 for flag in [0u8,1,0x20,0x21]{for global in [0u8,1]{
  ptr::write((st+0x189)as*mut u8,flag);ptr::write((base+0x3c0f2f7)as*mut u8,global);
  let immune=flag!=0||global!=0;
  for cost in [0,1,2,3,5,35,46,1000]{
   int(context+0xc,4021);int(context+8,-1);int(context+0xc0,-cost);let before=ctx;
   for owned in [false,true]{ECHO.store(owned,Ordering::Relaxed);
    int(st+0x15c,200);run(context);
    let extra=if owned{((i64::from(cost)*7+5)/10)as i32}else{0};
    assert_eq!(read_i32(st+0x15c),(200-if immune{0}else{cost}-extra).max(-50),"{flag} {global} {cost} {owned}");
    assert_eq!(ctx,before);assert_eq!(ptr::read((st+0x189)as*const u8),flag);assert_eq!(ptr::read((base+0x3c0f2f7)as*const u8),global);cases+=1;
   }
  }
  ECHO.store(true,Ordering::Relaxed);int(context+0xc0,-35);
  for kind in [0u8,1,2,3,255]{magic[0x26]=kind;int(st+0x15c,200);run(context);let extra=if kind<=1{25}else{0};assert_eq!(read_i32(st+0x15c),200-if immune{0}else{35}-extra);cases+=1;}magic[0x26]=0;
  int(context+0xc,-1);
  for behavior in [400000,400100,400500,400900].into_iter().flat_map(|family|[220,225,230,235,240,245,200,205,210,215,250].map(|suffix|family*1000+suffix)).chain([100000220]){
   int(context+8,behavior);int(context+0xc0,-30);
   for hand in [-1,-2]{int(context+0x38,hand);
    for owned in [false,true]{ARTS.store(owned,Ordering::Relaxed);int(st+0x15c,200);run(context);
     assert_eq!(read_i32(st+0x15c),if !immune||(owned&&storm_behavior(behavior)){170}else{200});if owned&&storm_behavior(behavior){assert_eq!(SOURCE.load(Ordering::Relaxed),hand);}cases+=1;
    }
   }
  }
 }}
 // Cached fee reuse never compounds, and failed/skipped native payments do not charge.
 ECHO.store(true,Ordering::Relaxed);ARTS.store(true,Ordering::Relaxed);int(context+8,-1);int(context+0xc,4021);int(context+0xc0,-35);ptr::write((st+0x189)as*mut u8,1);ptr::write((base+0x3c0f2f7)as*mut u8,0);int(st+0x15c,200);
 for expected in [175,150,125,100]{run(context);assert_eq!(read_i32(st+0x15c),expected);assert_eq!(read_i32(context+0xc0),-35);}
 for mode in 0..4{int(st+0x15c,200);match mode{0=>ECHO.store(false,Ordering::Relaxed),1=>ACTIVE.store(false,Ordering::Release),2=>world[0x174e8/8]=0,3=>int(context+0xc,999999),_=>unreachable!()};run(context);assert_eq!(read_i32(st+0x15c),200);ECHO.store(true,Ordering::Relaxed);ACTIVE.store(true,Ordering::Release);world[0x174e8/8]=a;int(context+0xc,4021);}
 // A native Storm weapon and a different current overlay both keep immunity.
 int(context+0xc,-1);int(context+8,400000220);int(context+0xc0,-30);
 int(entry.as_mut_ptr()as usize+0x1c,crate::arts::STORM);int(st+0x15c,200);run(context);assert_eq!(read_i32(st+0x15c),200);int(entry.as_mut_ptr()as usize+0x1c,100);
 state.lock().unwrap().reforge(w,100);int(st+0x15c,200);run(context);assert_eq!(read_i32(st+0x15c),200);
 // Mastery discounts the actual source weapon, including paired attacks, and
 // retains native immunity. A spell or the other hand's weapon is unaffected.
 ECHO.store(false,Ordering::Relaxed);ARTS.store(false,Ordering::Relaxed);
 // Use the real PlayerIns ID getter and source/slot decoder. Deliberately
 // remove the instance record; Onyx must not depend on randomized-art data.
 let mut equipment=[0u32;0x200/4];let eq=equipment.as_mut_ptr()as usize;
 put(a+0x670,eq);vt[0x250/8]=base+0x66cde0;ENTRY.store(ptr::null_mut(),Ordering::Relaxed);
 for owned in [false,true]{ONYX.store(owned,Ordering::Relaxed);
  for weapon in [8010000,8010025,2000000]{for slot in 0..6{int(eq+0x9c+slot*4,weapon);}
   for immune in [false,true]{ptr::write((st+0x189)as*mut u8,u8::from(immune));
    for (behavior,cost,dash)in [(100400000,35,false),(100400120,24,true),(100400125,30,true),(100400320,32,true),(300805901,30,false),(crate::onyx_mastery::BEHAVIOR,20,false)]{
     for hand in [-1,-2]{int(context+8,behavior);int(context+0xc,-1);int(context+0x38,hand);int(context+0xc0,-cost);int(st+0x15c,200);run(context);
      let wanted=if immune{0}else if owned&&crate::onyx_mastery::weapon(weapon){if dash{0}else{cost/2+cost%2}}else{cost};
      assert_eq!(read_i32(st+0x15c),200-wanted,"weapon={weapon} behavior={behavior} owned={owned} immune={immune}");
      assert_eq!(read_i32(context+0xc0),-cost);cases+=1;
     }
    }
    int(context+8,-1);int(context+0xc,4021);int(context+0xc0,-35);int(st+0x15c,200);run(context);assert_eq!(read_i32(st+0x15c),if immune{200}else{165});
   }
  }
 }
 // An empty-FP animation never produces the startup marker. The same motion
 // category on another weapon must not acquire the channel either.
 int(st+0x140,100);
 for owner in [false,true]{ONYX.store(owner,Ordering::Relaxed);for weapon in [8010000,2000000]{for slot in 0..6{int(eq+0x9c+slot*4,weapon);}
  for started in [false,true]{ONYX_STARTED.store(started,Ordering::Relaxed);for finish in [false,true]{ONYX_FINISHED.store(finish,Ordering::Relaxed);
   for hand in [0,1]{
    assert_eq!(crate::onyx_mastery::query(a,0,241,hand,base),0);
    assert_eq!(crate::onyx_mastery::may_loop(a,base),owner&&weapon==8010000&&started);
    assert_eq!(crate::onyx_mastery::query(a,1,241,hand,base),i32::from(!owner||weapon!=8010000||finish));
    assert_eq!(crate::onyx_mastery::free_channel(a,base),owner&&weapon==8010000);
    assert_eq!(crate::onyx_mastery::query(a,1,13,hand,base),0);assert!(!crate::onyx_mastery::free_channel(a,base));assert!(!crate::onyx_mastery::may_loop(a,base));
   }
  }}
 }}
 // Only the actual source hand qualifies, across all nine selected-slot pairs.
 // Empty hands, other weapons and invalid source selectors never borrow Onyx.
 ONYX.store(true,Ordering::Relaxed);ONYX_STARTED.store(true,Ordering::Relaxed);ONYX_FINISHED.store(false,Ordering::Relaxed);ptr::write((st+0x189)as*mut u8,0);
 for left in 0..3{for right in 0..3{int(eq+4,left);int(eq+8,right);
  for onyx_hand in [0,1]{
   for slot in 0..6{int(eq+0x9c+slot*4,110000);}
   int(eq+0x9c+if onyx_hand==0{left as usize*8}else{right as usize*8+4},8010002);
   for (hand,source)in [(0,-2),(1,-1)]{
    let selected=hand==onyx_hand;
    crate::onyx_mastery::query(a,0,241,hand,base);
    assert_eq!(crate::onyx_mastery::may_loop(a,base),selected);
    assert_eq!(crate::onyx_mastery::free_channel(a,base),selected);
    assert_eq!(crate::onyx_mastery::behavior(a,crate::onyx_mastery::JUDGE,source,base),selected.then_some(crate::onyx_mastery::BEHAVIOR));
    int(context+8,100400000);int(context+0xc,-1);int(context+0x38,source);int(context+0xc0,-35);int(st+0x15c,200);run(context);
    assert_eq!(read_i32(st+0x15c),if selected{182}else{165});cases+=1;
   }
  }
 }}
 for source in [-7,16,i32::MIN,i32::MAX]{assert_eq!(crate::onyx_mastery::behavior(a,crate::onyx_mastery::JUDGE,source,base),None);}
 int(eq+4,0);int(eq+8,0);for slot in 0..6{int(eq+0x9c+slot*4,8010000);}
 ONYX_FINISHED.store(true,Ordering::Relaxed);
 for (hp,down)in [(0,0),(100,1),(100,2)]{int(st+0x140,hp);ptr::write((a+0x710)as*mut u8,down);assert_eq!(crate::onyx_mastery::query(a,1,241,1,base),0);assert!(!crate::onyx_mastery::may_loop(a,base));}
 int(st+0x140,100);ptr::write((a+0x710)as*mut u8,0);
 ONYX.store(false,Ordering::Relaxed);WEAPON_ID.store(2000000,Ordering::Relaxed);
 // Recovery, zero fees and signed edge values are never forced debits.
 int(context+0xc,4021);
 for delta in [0,10]{int(context+0xc0,delta);int(st+0x15c,100);run(context);assert_eq!(read_i32(st+0x15c),100+delta);}
 // The old constructor hook is absent; native fee computation is intact.
 for (rva,size) in [(0x384340,0x175),(0x384800,0x224)]{let s=headers.sections.iter().find(|s|rva>=s.virtual_address&&rva+size<=s.virtual_address+s.raw_size).unwrap();let off=(rva-s.virtual_address+s.raw_offset)as usize;assert_eq!(std::slice::from_raw_parts((base+rva as usize)as*const u8,size as usize),&raw[off..off+size as usize]);}
 println!("PASS: {cases} native payment cases; Echo split, both immunity flags/global switch, Storm normal/charged/left/right/source isolation, unchanged caches and constructors");
 BASE.store(saved_base,Ordering::Release);ACTIVE.store(saved_active,Ordering::Release);CONTINUE.store(ptr::null_mut(),Ordering::Release);CLAMP.store(ptr::null_mut(),Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
