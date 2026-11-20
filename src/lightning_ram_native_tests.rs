//! Execute the shipping mid-hook, relocated instructions and actual capsule
//! velocity setter. Scene contacts/ownership are controlled fixtures.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
unsafe extern "system" fn has(actor:usize,id:i32)->u8{u8::from(id==OWNER&&read_i32(actor+0x78)==1||id==5750&&read_i32(actor+0x7c)==1)}
unsafe extern "system" fn stamina_rate(_:usize,fee:i32,_:i32,_:u8,_:i32)->i32{fee}
unsafe extern "system" fn rotation(_:usize,out:*mut f32)->*const f32{ptr::copy_nonoverlapping([0.;4].as_ptr(),out,4);out}
unsafe extern "system" fn set_rotation(_:usize,p:*const f32){assert!(ptr::read(p.add(1)).is_finite());}
unsafe extern "system" fn weapon(actor:usize,source:i32)->i32{assert!(matches!(source,-1|-2));read_i32(actor+if source==-2{0x80}else{0x84})}
static ENTRIES:std::sync::atomic::AtomicUsize=std::sync::atomic::AtomicUsize::new(0);
static OBJECT_ROWS:std::sync::atomic::AtomicUsize=std::sync::atomic::AtomicUsize::new(0);
unsafe extern "system" fn object_lookup(out:*mut usize,kind:i32,id:i32){
 assert_eq!(kind,1);assert!(matches!(id,ATTACK)|matches!(id,x if x==OWNER+69));
 let rows=OBJECT_ROWS.load(Ordering::Relaxed);
 ptr::write(out.add(2),if rows==0{0}else{rows+if id==ATTACK{0}else{0x80}});
}
static WALL:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
static STEP:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
static FLOOR:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(true);
static FLOOR_TILT:std::sync::atomic::AtomicI32=std::sync::atomic::AtomicI32::new(100);
static FLAT_WALL:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
static SLANT:std::sync::atomic::AtomicI32=std::sync::atomic::AtomicI32::new(0);
static LEDGE:std::sync::atomic::AtomicI32=std::sync::atomic::AtomicI32::new(0);
unsafe extern "system" fn handle(actor:usize,out:*mut u32,source:i32)->*const u32{
 let index=if source==-2{0}else{1};ptr::write(out,read_i32(read_ptr(actor+0x670)+0x1c+index*4)as u32);out
}
unsafe extern "system" fn reference(out:*mut u64,handle:*const u32)->usize{
 let id=ptr::read(handle);let entry=ENTRIES.load(Ordering::Acquire)+((id-0x80800010)as usize)*0x20;
 ptr::write(out,id as u64);ptr::write(out.add(1),entry as u64);ptr::write(out.add(2),read_i32(entry+0x10)as u64);out as usize
}
unsafe extern "system" fn release(_:usize){}
unsafe extern "system" fn no_allocator()->usize{0}
unsafe extern "system" fn contact_plane_fixture(_:usize,contact:usize,step:usize,delta:f32,plane:usize){
 assert_eq!(step,0x1234);assert_eq!(delta,1./60.);
 ptr::write_bytes(plane as*mut u8,0,0x50);
 ptr::copy_nonoverlapping((contact+0x10)as*const u8,plane as*mut u8,16);
 ptr::write_unaligned((plane+0x44)as*mut u32,ptr::read_unaligned((contact+0x28)as*const u32));
}
unsafe extern "system" fn collected_query_fixture(collector:usize,hit:usize){
 ptr::write_unaligned((collector+0xc)as*mut u32,ptr::read_unaligned((hit+0x48)as*const u32)+1);
 ptr::write_unaligned((collector+0x10)as*mut[f32;4],[0.25;4]);
 ptr::write((collector+0x20)as*mut u8,1);
}
unsafe extern "system" fn entity_slot(pool:usize,index:u32)->usize{if index<2{pool+(index as usize+1)*8}else{0}}
thread_local!{static PARTY:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};}
unsafe extern "system" fn no_party(_:usize,_:i32)->usize{PARTY.with(std::cell::Cell::get)}
unsafe extern "system" fn module_actor(module:usize)->usize{read_ptr(module+8)}
unsafe extern "system" fn zero_float(_:usize)->f32{0.}
unsafe extern "system" fn no_flag(_:usize)->u8{0}
unsafe extern "system" fn ultimate_lookup(out:*mut usize,kind:i32,id:i32){
 assert_eq!(kind,1);assert!(attack(id));
 ptr::write(out.add(2),OBJECT_ROWS.load(Ordering::Relaxed)+if id==EXTREME_ATTACK{0x200}else{0});
}
unsafe extern "system" fn raw_art(reference:usize)->i32{read_i32(read_ptr(reference+8)+0x1c)}
unsafe extern "system" fn id_art(reference:usize)->i32{match raw_art(reference){504=>113,503=>112,_=>93}}
unsafe extern "system" fn param_art(reference:usize,out:usize)->usize{ptr::write_unaligned(out as*mut i32,raw_art(reference));out}
unsafe extern "system" fn default_rate(_:usize)->f32{1.}
unsafe fn quote(module:usize,button:i32)->i32{
 let mut row=[0u8;0x18];for offset in [0x10,0x12,0x14,0x16]{row[offset..offset+2].copy_from_slice(&5i16.to_le_bytes());}
 let reference=[504usize,row.as_ptr()as usize];let actor=read_ptr(module+8);
 let fee:unsafe extern "system" fn(usize,i32,usize)->i32=std::mem::transmute(BASE.load(Ordering::Acquire)+crate::onyx_mastery::ARTS_FEE_RVA);
 fee(reference.as_ptr()as usize,button,read_ptr(actor+0x190))
}
unsafe extern "system" fn check_fp(module:usize,button:i32,_:i32)->u8{let fp=read_i32(native_actor::stats(read_ptr(module+8))+0x150);u8::from(fp>0&&fp>=quote(module,button)/2)}
unsafe extern "system" fn reserve_fp(module:usize,button:i32,_:i32){ptr::write_unaligned((module+0x10)as*mut i32,quote(module,button));}
unsafe extern "system" fn pay(stats:usize,amount:i32){let p=(stats+0x150)as*mut i32;ptr::write(p,(ptr::read(p)+amount).max(0));}
unsafe extern "system" fn debit_hp(stats:usize,amount:i32,mode:u8,limit:u8,rate:f32,window:f32,refresh:u8){
 assert_eq!((mode,limit,rate,window,refresh),(1,0,0.,0.,0));let p=(stats+0x140)as*mut i32;ptr::write(p,(ptr::read(p)+amount).max(0));
}
unsafe extern "system" fn finish_hp(module:usize,lost:i32,source:usize,hit:usize,before:i32,flag:u8){
 assert_eq!((source,hit,flag),(0,0,0));assert_ne!(ptr::read((module+0x78)as*const u8)&8,0);
 let stats=native_actor::stats(read_ptr(module+8));assert_eq!(lost,before-read_i32(stats+0x140));
 ptr::write_unaligned((module+0x80)as*mut i32,read_i32(module+0x80)+1);
}
unsafe extern "system" fn collision(_:usize,hit:*mut u32,_:u32,start:*const f32,motion:*const f32,radius:f32,_:usize,_:*mut u32,_:*mut f32,normal:*mut f32,fraction:*mut f32,_:*mut f32)->*mut u32{
 let ledge=LEDGE.load(Ordering::Relaxed);
 if ledge!=0{
  ptr::write(hit,u32::MAX);
  let sy=ptr::read(start.add(1));let dy=ptr::read(motion.add(1));let dz=ptr::read(motion.add(2));
  let contact=if (radius-0.3).abs()<0.001&&dz>0.{
   Some((0.74423593,V(0.12084629,-0.1304672,-0.9840602)))
  }else if (radius-0.05).abs()<0.001&&dy<0.&&ledge!=2{
   let top=if ledge==4{0.9}else{0.6};Some(((top+radius-sy)/dy,V(0.,1.,0.)))
  }else if ledge==3&&radius>0.35&&dz>0.&&dy>0.2{
   Some((0.5,V(0.,-1.,0.)))
  }else{None};
  if let Some((t,n))=contact.filter(|(t,_)|(0. ..=1.).contains(t)){
   ptr::write(hit,0x0500002a);ptr::write(fraction,t);ptr::copy_nonoverlapping(n.array().as_ptr(),normal,4);
  }
  return hit;
 }
 if FLAT_WALL.load(Ordering::Relaxed)&&ptr::read(motion.add(2))< -0.01{
  let t=((radius-ptr::read(start.add(2)))/ptr::read(motion.add(2))).max(0.);
  if t<=1.{ptr::write(hit,0x0500002a);ptr::write(fraction,t);ptr::copy_nonoverlapping([0.,0.,1.,0.].as_ptr(),normal,4);}else{ptr::write(hit,u32::MAX);}return hit;
 }
 if SLANT.load(Ordering::Relaxed)!=0&&ptr::read(motion.add(2))< -0.01{
  let y=SLANT.load(Ordering::Relaxed)as f32/100.;let n=V(0.,y,(1.-y*y).sqrt());
  let offset=if y>0.{0.2}else{0.6};let from=V(ptr::read(start),ptr::read(start.add(1)),ptr::read(start.add(2)));
  let delta=V(ptr::read(motion),ptr::read(motion.add(1)),ptr::read(motion.add(2)));
  let t=((radius-n.dot(from)-offset)/n.dot(delta)).max(0.);
  if t<=1.{ptr::write(hit,0x0500002a);ptr::write(fraction,t);ptr::copy_nonoverlapping(n.array().as_ptr(),normal,4);}else{ptr::write(hit,u32::MAX);}
  return hit;
 }
 if (WALL.load(Ordering::Relaxed)||(STEP.load(Ordering::Relaxed)&&ptr::read(start.add(1))<0.7))&&ptr::read(motion.add(2))< -0.01{ptr::write(hit,0x0500002a);ptr::write(fraction,0.5);ptr::copy_nonoverlapping([0.,0.,1.,0.].as_ptr(),normal,4);}
 else if FLOOR.load(Ordering::Relaxed)&&ptr::read(motion.add(1))< -0.01{
  let y=FLOOR_TILT.load(Ordering::Relaxed)as f32/100.;let n=V(0.,y,(1.-y*y).sqrt());
  let from=V(ptr::read(start),ptr::read(start.add(1)),ptr::read(start.add(2)));let delta=V(ptr::read(motion),ptr::read(motion.add(1)),ptr::read(motion.add(2)));
  let t=(radius-from.dot(n))/delta.dot(n);
  if (0. ..=1.).contains(&t){ptr::write(hit,42);ptr::write(fraction,t);ptr::copy_nonoverlapping(n.array().as_ptr(),normal,4);}else{ptr::write(hit,u32::MAX);}
 }else{ptr::write(hit,u32::MAX);}hit
}
#[unsafe(naked)]unsafe extern "system" fn fixture(_:usize,_:*mut f32,_:*const f32,_:usize){
 core::arch::naked_asm!("push rdi","push rbp","push r14","sub rsp,0x40","mov rdi,rcx","lea rbp,[rdx+0x80]","mov r14,r8","jmp r9");
}
#[unsafe(naked)]unsafe extern "system" fn snap_fixture(_:usize,_:*const f32,_:usize){
 core::arch::naked_asm!("push rdi","sub rsp,0x70",
  "movdqu [rsp+0x20],xmm6","movdqu [rsp+0x30],xmm7","movdqu [rsp+0x40],xmm8","movdqu [rsp+0x50],xmm9","movdqu [rsp+0x60],xmm13",
  "mov rdi,rcx","movss xmm6,[rdx]","movss xmm7,[rdx+4]","movss xmm8,[rdx+8]","xorps xmm9,xmm9",
  "pcmpeqd xmm13,xmm13","pslld xmm13,31","xor eax,eax","jmp r8");
}
#[unsafe(naked)]unsafe extern "system" fn snap_exit(){
 core::arch::naked_asm!("movdqu xmm6,[rsp+0x20]","movdqu xmm7,[rsp+0x30]","movdqu xmm8,[rsp+0x40]","movdqu xmm9,[rsp+0x50]","movdqu xmm13,[rsp+0x60]",
  "add rsp,0x70","pop rdi","ret");
}
#[test]#[ignore="requires NRHEX_NATIVE_EXE; run alone"]
fn native_ram_velocity_submission_and_local_owner_boundary(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for section in &headers.sections{ptr::copy_nonoverlapping(raw[section.raw_offset as usize..].as_ptr(),(base+section.virtual_address as usize)as*mut u8,section.raw_size as usize);}
 for(rva,f)in[(0x47cc80,release as*const()as usize),(0x47cf00,release as*const()as usize),(crate::death_dance::DEBIT_RVA,debit_hp as*const()as usize),(crate::death_dance::FINISH_RVA,finish_hp as*const()as usize),(crate::HAS_EFFECT_RVA,has as*const()as usize),(crate::CHR_ROTATION_RVA,rotation as*const()as usize),(crate::parry_turn::SET_ROTATION_RVA,set_rotation as*const()as usize),(0xdb04a0,collision as*const()as usize),(crate::onyx_mastery::FP_ADD_RVA,pay as*const()as usize),(crate::ARTS_REF_RVA,reference as*const()as usize),(0x6a1d90,release as*const()as usize)]{
  let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&f.to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(jump.as_ptr(),(base+rva)as*mut u8,jump.len());
 }
 let mut lookup_patch=vec![0x48,0xb8];lookup_patch.extend_from_slice(&(object_lookup as*const()as usize).to_le_bytes());lookup_patch.extend_from_slice(&[0xff,0xe0]);
 ptr::copy_nonoverlapping(lookup_patch.as_ptr(),(base+crate::devils_gaze_hit::LOOKUP_RVA)as*mut u8,lookup_patch.len());
 let mut attack_rows=[0u8;0x100];attack_rows[0x62]=10;attack_rows[0xe2]=20;OBJECT_ROWS.store(attack_rows.as_ptr()as usize,Ordering::Relaxed);
 let mut world=vec![0usize;0x17500/8];let mut actor=vec![0usize;0x780/8];let mut modules=vec![0usize;0x120/8];let mut stats=vec![0i32;0x200/4];let mut physics=vec![0usize;0x600/8];let mut controller=vec![0usize;0xc8/8];let mut geometry=vec![0usize;0xa0/8];
 #[repr(align(16))]struct Vec4([f32;4]);let mut desired=Vec4([0.,0.,-4.,0.]);let mut submitted=Vec4([f32::NAN;4]);
 let mut vt=vec![0usize;0x350/8];vt[0x348/8]=stamina_rate as*const()as usize;vt[0x250/8]=weapon as*const()as usize;vt[0x258/8]=handle as*const()as usize;actor[0]=vt.as_mut_ptr()as usize;
 let mut equipment=vec![0u32;0x100/4];let mut entries=vec![0u32;6*0x20/4];
 ENTRIES.store(entries.as_mut_ptr()as usize,Ordering::Release);
 for i in 0..6{let id=0x80800010+i as u32;equipment[0x1c/4+i]=id;entries[i*8+2]=id;entries[i*8+4]=110000;entries[i*8+7]=503;}
 actor[0x670/8]=equipment.as_mut_ptr()as usize;
 let a=actor.as_mut_ptr()as usize;actor[1]=123;actor[0x190/8]=a;actor[0x1b8/8]=modules.as_mut_ptr()as usize;modules[0]=stats.as_mut_ptr()as usize;modules[0x68/8]=physics.as_mut_ptr()as usize;stats[0x140/4]=100;stats[0x15c/4]=100000;stats[0x160/4]=100000;
 ptr::write((stats.as_mut_ptr()as usize+0x189)as*mut u8,0x21);physics[1]=a;
 ptr::write_unaligned((a+0x78)as*mut i32,1);ptr::write_unaligned((a+0x80)as*mut i32,110000);ptr::write_unaligned((a+0x84)as*mut i32,110000);
 physics[0x98/8]=controller.as_mut_ptr()as usize;controller[0xa0/8]=submitted.0.as_mut_ptr()as usize;world[0x174e8/8]=a;geometry[0x98/8]=123;
 ptr::write_unaligned((base+crate::WORLD_RVA)as*mut usize,world.as_ptr()as usize);ptr::write_unaligned((base+0x3c1e028)as*mut usize,geometry.as_ptr()as usize);ptr::write_unaligned((base+0x3c1d9a8)as*mut usize,1);
 let prepared=crate::hook::prepare((base+MOTION_RVA)as*mut u8,motion_detour as*const u8,64).unwrap();assert_eq!(prepared.stolen,11);
 let old_orig=MOTION_ORIGINAL.swap(prepared.trampoline,Ordering::AcqRel);
 crate::hook::write_code((base+MOTION_RVA)as*mut u8,&prepared.patch).unwrap();
 // End after native CALL D977F0. The setter itself is unmodified game code.
 let epilogue=[0x48,0x83,0xc4,0x40,0x41,0x5e,0x5d,0x5f,0xc3];ptr::copy_nonoverlapping(epilogue.as_ptr(),(base+MOTION_RVA+16)as*mut u8,epilogue.len());
 let old_base=BASE.swap(base,Ordering::AcqRel);let old_active=ACTIVE.swap(true,Ordering::AcqRel);let time=[0.,0.,1./60.,0.];
 for enabled in [false,true]{
  *STATE.lock().unwrap()=State{actor:a,handle:123,active:enabled,started:true,rolling:true,drive:true,wish:V(0.,0.,-1.),motion:Motion{velocity:V(0.,0.,-12.),..Default::default()},..Default::default()};
  desired.0=[0.,0.,-4.,0.];fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);
  assert_eq!(submitted.0,desired.0);assert_eq!(desired.0[0],0.);assert_eq!(desired.0[1],0.);
  if enabled{assert!(desired.0[2]< -12.&&desired.0[2]> -13.);}else{assert_eq!(desired.0[2],-4.);}
 }
 // Actual DA26C0 -> DAF8C0 lookup: fresh sphere ID -> native body/userdata.
 // Only the underlying scene query is mocked, not the identity resolver.
 {
  let mut manager=[0usize;8];let mut body_world=[0usize;8];let mut bodies=vec![0usize;43*0xb0/8];
  let mut entry=[0usize;8];let mut asset=[0usize;0x1a0/8];let mut row=[0u8;0x50];let user=[entry.as_mut_ptr()as usize];
  manager[1]=body_world.as_mut_ptr()as usize;body_world[0x28/8]=bodies.as_mut_ptr()as usize;bodies[(42*0xb0+0xa0)/8]=user.as_ptr()as usize;
  entry[1]=5;entry[2]=asset.as_mut_ptr()as usize;asset[4]=row.as_ptr()as usize;row[0x10]=1;geometry[0x98/8]=manager.as_mut_ptr()as usize;
  assert!(breakable(base,0x0500002a));row[0x10..0x12].copy_from_slice(&(-1i16).to_le_bytes());assert!(!breakable(base,0x0500002a));
  // The live building/tree rows have HP=1, defense=90. HP alone does not
  // make a body destructible by this player's rolling contact attacks.
  row[0x10..0x12].copy_from_slice(&1i16.to_le_bytes());row[0x12..0x14].copy_from_slice(&90u16.to_le_bytes());
  assert!(!breakable(base,0x0500002a),"live wall/tree defense must not receive breakable compliance");
  for defense in [0u16,9,10,19,20,21,90]{row[0x12..0x14].copy_from_slice(&defense.to_le_bytes());assert_eq!(breakable(base,0x0500002a),defense<20);}
  OBJECT_ROWS.store(0,Ordering::Relaxed);row[0x12..0x14].copy_from_slice(&0u16.to_le_bytes());assert!(!breakable(base,0x0500002a));OBJECT_ROWS.store(attack_rows.as_ptr()as usize,Ordering::Relaxed);
  attack_rows[0xe2]=30;row[0x12..0x14].copy_from_slice(&25u16.to_le_bytes());assert!(breakable(base,0x0500002a));attack_rows[0xe2]=20;
  row[0x12..0x14].copy_from_slice(&0u16.to_le_bytes());
  let mut proxy=vec![0usize;0x140/8];proxy[0]=base+0x2c9e9b8;controller[0]=base+0x2c9e6a0;controller[0x88/8]=proxy.as_mut_ptr()as usize;
  ptr::write_unaligned((controller.as_mut_ptr()as usize+0xc0)as*mut f32,0.4);
  ptr::write_unaligned((proxy.as_mut_ptr()as usize+0x130)as*mut f32,0.05);
  WALL.store(true,Ordering::Relaxed);
  for soft in [true,false]{
   row[0x10..0x12].copy_from_slice(&(if soft{1i16}else{-1i16}).to_le_bytes());
   let v=if soft{V(0.,0.,-16.)}else{V(10.,0.,-12.)};
   let mut s=State{actor:a,handle:123,active:true,started:true,rolling:true,drive:true,wish:v.unit(),motion:Motion{velocity:v,..Default::default()},..Default::default()};
   s.travel.submit(V::default(),v,1./60.);*STATE.lock().unwrap()=s;
   fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);
   let s=STATE.lock().unwrap();
   if soft{assert!(s.active&&!s.crash&&!s.push);assert!(V::from(submitted.0).len()>13.&&V::from(submitted.0).len()<16.);assert_eq!(s.effective_speed,0.);}
   // 12 m/s toward the wall, but 15.62 m/s total: an accepted frontal
   // collision now takes the 7-impact tier and discards the whole velocity.
   else{assert!(!s.active&&!s.push&&s.crash&&s.crash_pending);assert_eq!(submitted.0,[0.;4]);}
  }
  // Native capsule has rejected the normal component but accepted 30m/s
  // along the wall. Even a 16m/s normal impact must not stop this grazing roll.
  row[0x10..0x12].copy_from_slice(&1i16.to_le_bytes());row[0x12..0x14].copy_from_slice(&90u16.to_le_bytes());
  for hz in [30.,60.,144.]{
   let dt=1./hz;let v=V(30.,0.,-16.);
   let mut s=State{actor:a,handle:123,active:true,started:true,rolling:true,drive:true,wish:v.unit(),motion:Motion{velocity:v,spin:v,initialized:true,support:Some(V(0.,1.,0.)),..Default::default()},..Default::default()};
   s.travel.submit(V::default(),v,dt);*STATE.lock().unwrap()=s;
   ptr::write_unaligned((physics.as_mut_ptr()as usize+0x70)as*mut[f32;4],[30.*dt,0.,0.,0.]);
   fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),[0.,0.,dt,0.].as_ptr(),base+MOTION_RVA);
   let s=STATE.lock().unwrap();assert!(s.active&&!s.push&&!s.crash&&!s.ending,"grazing recovery hz={hz}");
   // Existing rolling drag at 30m/s removes about 1.1m/s over a 30Hz step;
   // verify tangent survives, allowing that physical drag rather than reset.
   assert!(submitted.0[0]>28.&&submitted.0[0]<=30.,"native wall tangent lost hz={hz}: {:?}",submitted.0);
  }
  ptr::write_unaligned((physics.as_mut_ptr()as usize+0x70)as*mut[f32;4],[0.;4]);
  println!("PASS: high-speed grazing wall contact preserves native tangent at 30/60/144Hz without recovery/push/crash");
  WALL.store(false,Ordering::Relaxed);STEP.store(true,Ordering::Relaxed);
  row[0x10..0x12].copy_from_slice(&(-1i16).to_le_bytes());
  let v=V(0.,0.,-20.);let mut s=State{actor:a,handle:123,active:true,started:true,rolling:true,drive:true,motion:Motion{velocity:v,..Default::default()},..Default::default()};
  s.travel.submit(V::default(),v,1./60.);*STATE.lock().unwrap()=s;
  fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);
  {let s=STATE.lock().unwrap();assert!(s.active&&!s.push&&!s.crash);assert_eq!(submitted.0,[0.;4]);}
  STEP.store(false,Ordering::Relaxed);
  // Captured lip normal / incoming / accepted velocity. The native capsule
  // already lifted the feet, but the old underside fallback forced a push.
  for mode in [1,2,3,4]{
   LEDGE.store(mode,Ordering::Relaxed);let dt=0.018845731;
   let v=V(-2.6931846,1.8786988,13.187162);let accepted=V(-1.4149963,3.9738512,1.8345562);
   let mut s=State{actor:a,handle:123,active:true,started:true,rolling:true,drive:true,motion:Motion{velocity:v,spin:v,initialized:true,support:Some(V(-0.021931034,0.9891307,-0.14539467)),..Default::default()},..Default::default()};
   s.travel.submit(V::default(),v,dt);*STATE.lock().unwrap()=s;
   ptr::write_unaligned((physics.as_mut_ptr()as usize+0x70)as*mut[f32;4],accepted.scale(dt).array());
   fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),[0.,0.,dt,0.].as_ptr(),base+MOTION_RVA);
   let s=STATE.lock().unwrap();
   if mode==1{
    assert!(s.active&&s.motion.airborne&&!s.push&&!s.crash&&!s.crash_pending);
    assert!(submitted.0[1]>3.&&submitted.0[2]>10.);assert!(V::from(submitted.0).len()<v.len());
    assert_eq!(s.step_body,Some(0x0500002a));assert!(s.step_grace>0.);
   }else{assert!(!s.active&&s.push&&!s.crash,"underside/roof/tall obstacle mode={mode}");}
  }
  LEDGE.store(0,Ordering::Relaxed);
  ptr::write_unaligned((physics.as_mut_ptr()as usize+0x70)as*mut[f32;4],[0.;4]);
  println!("PASS: captured downward lip launches over verified low top without adding kinetic energy; missing top, overhead obstruction and tall top retain wall feedback");
  for tilt in [-50,50,53,65]{
   SLANT.store(tilt,Ordering::Relaxed);
   let v=V(0.,0.,-20.);let mut s=State{actor:a,handle:123,active:true,started:true,rolling:true,drive:true,motion:Motion{velocity:v,..Default::default()},..Default::default()};
   s.travel.submit(V::default(),v,1./60.);*STATE.lock().unwrap()=s;
   fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);
   let s=STATE.lock().unwrap();assert!(s.crash,"slanted wall normal Y={tilt}%");
   assert!(!s.active&&s.crash_pending);
  }
  SLANT.store(0,Ordering::Relaxed);
  // Native capsule stops at radius .4 + keep-distance .05. Our upper probe
  // must see this same wall in the FIRST rejected frame, before the accepted
  // displacement reduces 15.8m/s to 10.2m/s. Geometry is an exact plane.
  FLAT_WALL.store(true,Ordering::Relaxed);
  row[0x10..0x12].copy_from_slice(&1i16.to_le_bytes());row[0x12..0x14].copy_from_slice(&90u16.to_le_bytes());
  let v=V(0.,0.,-15.8);let mut s=State{actor:a,handle:123,active:true,started:true,rolling:true,drive:true,motion:Motion{velocity:v,spin:v,initialized:true,..Default::default()},..Default::default()};
  s.travel.submit(V(0.,0.,0.62),v,1./60.);*STATE.lock().unwrap()=s;
  ptr::write_unaligned((physics.as_mut_ptr()as usize+0x70)as*mut[f32;4],[0.,0.,0.45,0.]);
  fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);
  {let s=STATE.lock().unwrap();assert!(s.crash&&s.crash_pending,"first native wall contact missed at 15.8m/s");}
  for hz in [30.,60.,144.]{for phase in [0.1,0.4,0.7,0.95]{
   let dt=1./hz;let v=V(0.,0.,-15.8);let mut s=State{actor:a,handle:123,active:true,started:true,rolling:true,drive:true,motion:Motion{velocity:v,spin:v,initialized:true,..Default::default()},..Default::default()};
   s.travel.submit(V(0.,0.,0.45+15.8*dt*phase),v,dt);*STATE.lock().unwrap()=s;
   fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),[0.,0.,dt,0.].as_ptr(),base+MOTION_RVA);
   assert!(STATE.lock().unwrap().crash,"wall contact hz={hz} phase={phase}");
  }}
  println!("PASS: live HP=1/defense=90 wall catches first rejection at 30/60/144Hz; actual rolling object power, equality/missing row and weak breakables verified; low steps and slanted walls retained");
  FLAT_WALL.store(false,Ordering::Relaxed);
  for tilt in [100,86,73,69,65]{for gap in [-0.09,-0.07,-0.028,-0.0215,0.05,0.08,0.2]{
   FLOOR_TILT.store(tilt,Ordering::Relaxed);let y=tilt as f32/100.;let foot=V(0.,(0.4+gap)/y-0.4,0.);
   ptr::write_unaligned((physics.as_mut_ptr()as usize+0x70)as*mut[f32;4],foot.array());ptr::write((physics.as_mut_ptr()as usize+0x92)as*mut u8,1);
   let n=V(0.,y,(1.-y*y).sqrt());
   assert_eq!(curved_support(base,physics.as_ptr()as usize,foot,Some(n),V(0.,0.,-3.).plane(n),None).is_some(),(-0.08..=0.065).contains(&gap),"continuous slope tilt={tilt} gap={gap}");
   let v=V(0.,-0.5,-3.);*STATE.lock().unwrap()=State{actor:a,handle:123,active:true,started:true,rolling:true,drive:true,motion:Motion{velocity:v,initialized:true,airborne:true,..Default::default()},..Default::default()};
   fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);
   let s=STATE.lock().unwrap();assert_eq!(!s.motion.airborne,(-0.08..=0.065).contains(&gap),"floor tilt={tilt} capsule gap={gap}");assert!(s.active&&!s.crash&&!s.push);
  }}
  FLOOR_TILT.store(100,Ordering::Relaxed);
  println!("PASS: capsule bottom support accepts flat/stone slopes at native skin distance, rejects genuine separation despite native grounded flag");
  controller[0]=0;controller[0x88/8]=0;
  ptr::write_unaligned((physics.as_mut_ptr()as usize+0x70)as*mut[f32;4],[0.;4]);
  geometry[0x98/8]=123;reset();
 }
 // The native ground transition may snap down / erase horizontal travel.
 // A clear ledge gets only a bounded inertial bridge, not a position teleport.
 FLOOR.store(false,Ordering::Relaxed);
 let v=V(0.,0.,-15.);let mut s=State{actor:a,handle:123,active:true,started:true,rolling:true,drive:true,motion:Motion{velocity:v,spin:v,initialized:true,support:Some(V(0.,1.,0.)),..Default::default()},..Default::default()};
 s.travel.submit(V::default(),v,1./60.);*STATE.lock().unwrap()=s;
 ptr::write_unaligned((physics.as_mut_ptr()as usize+0x70)as*mut[f32;4],[0.,-0.15,-0.02,0.]);
 fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);
 assert!(submitted.0[2]< -14.9&&submitted.0[1]<0.);
 assert!(STATE.lock().unwrap().motion.airborne);
 FLOOR.store(true,Ordering::Relaxed);reset();
 // A high landing consumes pre-contact normal speed once and converts only
 // bounded stored spin through friction. Geometry remains a controlled plane.
 let v=V(10.,-20.,0.);let mut s=State{actor:a,handle:123,active:true,started:true,rolling:true,motion:Motion{velocity:v,spin:V(30.,0.,0.),initialized:true,airborne:true,..Default::default()},..Default::default()};
 s.travel.submit(V(0.,20./60.,0.),v,1./60.);*STATE.lock().unwrap()=s;
 ptr::write_unaligned((physics.as_mut_ptr()as usize+0x70)as*mut[f32;4],[10./60.,0.,0.,0.]);
 fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);
 assert!(submitted.0[0]>10.&&submitted.0[1]>0.&&submitted.0[1]<=2.2);
 reset();ptr::write_unaligned((physics.as_mut_ptr()as usize+0x70)as*mut[f32;4],[0.;4]);
 println!("PASS: low step exclusion, clear ledge inertia and high-fall spin transfer through the real velocity hook");
 // Execute the actual coordinate rebase routine and the compiled wrapper.
 let rebase_hook=crate::hook::prepare((base+REBASE_RVA)as*mut u8,rebase as*const u8,64).unwrap();
 let old_rebase=REBASE_ORIGINAL.swap(rebase_hook.trampoline,Ordering::AcqRel);
 crate::hook::write_code((base+REBASE_RVA)as*mut u8,&rebase_hook.patch).unwrap();
 let shift_native:unsafe extern "system" fn(usize,*const f32)=std::mem::transmute(base+REBASE_RVA);
 physics[0x98/8]=0; // Native capsule handles omitted; coordinate/cache body is real.
 let phys=physics.as_mut_ptr()as usize;
 for shift in [V(0.,0.,-32.),V(-24.,0.,-32.)]{
  let from=V(31.2,59.7,32.3);let v=V(8.,-1.,14.);let dt=1./60.;
  let mut state=State{actor:a,handle:123,active:true,started:true,rolling:true,drive:true,trail:Some(from),motion:Motion{velocity:v,locked:true,..Default::default()},..Default::default()};
  state.travel.submit(from,v,dt);*STATE.lock().unwrap()=state;
  ptr::write_unaligned((phys+0x70)as*mut[f32;4],from.add(v.scale(dt)).array());
  CONTACTS.lock().unwrap().insert((123,1,456),from);CONTACTS.lock().unwrap().insert((999,1,456),from);
  shift_native(phys,shift.array().as_ptr());
  let actual=V::from(ptr::read_unaligned((phys+0x70)as*const[f32;4]));assert_eq!(actual,from.add(v.scale(dt)).add(shift));
  let mut state=STATE.lock().unwrap();let resolved=state.travel.resolve(actual).unwrap();assert!((resolved.velocity.len()-v.len()).abs()<0.001);assert!(!resolved.stopped);assert_eq!(state.motion.velocity,v);drop(state);
  assert_eq!(CONTACTS.lock().unwrap()[&(123,1,456)],from.add(shift));assert_eq!(CONTACTS.lock().unwrap()[&(999,1,456)],from);
 }
 CONTACTS.lock().unwrap().clear();REBASE_ORIGINAL.store(old_rebase,Ordering::Release);
 physics[0x98/8]=controller.as_mut_ptr()as usize;ptr::write_unaligned((phys+0x70)as*mut[f32;4],[0.;4]);reset();
 // A held left/right input must change the actual velocity submitted to
 // the original capsule; native facing is separately aligned with that velocity.
 let paid_start=||record_startup(a,1,true);
 stats[0x150/4]=100;stats[0x15c/4]=100;
 for angle in [-90.,90.]{
  reset();paid_start();assert_eq!(query(a,113,1,true,false,angle,base),0);
  {let mut s=STATE.lock().unwrap();s.started=true;s.rolling=true;s.motion.velocity=V(0.,0.,-12.);}
  desired.0=[0.,0.,-4.,0.];fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);
  assert!(desired.0[0]*angle<0.);assert!(desired.0[2]< -11.);
 }
 // A native blocked position must erase the previous high-speed submission.
 {let mut s=STATE.lock().unwrap();s.travel=Travel::default();s.motion.velocity=V(0.,0.,-20.);}
 fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);
 // No position change: the controlled native capsule rejected this step.
 {STATE.lock().unwrap().wish=V(1.,0.,0.);}
 fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);
 // Stronger grip may transfer a small amount of stored spin each step, but
 // cannot restore the rejected 20 m/s or exceed this step's maximum static
 // friction impulse (4.5*g*dt); slipping friction is lower at 3.375*g*dt.
 assert!(V::from(submitted.0).len()<=4.5*crate::ram_motion::GRAVITY/60.+0.001);assert_eq!(STATE.lock().unwrap().effective_speed,0.);
 assert!(!STATE.lock().unwrap().motion.locked);
 for _ in 0..20{
  fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);
  assert!(V::from(submitted.0).len()<=4.5*crate::ram_motion::GRAVITY/60.+0.001);
 }
 {let s=STATE.lock().unwrap();assert!(!s.active&&s.ending);assert_eq!(s.motion.spin,V::default());}
 // A crash settles max-HP cost exactly once, through the native cost ABI.
 let mut damage=vec![0usize;0xa0/8];damage[0]=base+crate::ominous_pact::PLAYER_DAMAGE_VTABLE_RVA;damage[1]=a;
 modules[crate::DAMAGE_MODULE_SLOT/8]=damage.as_mut_ptr()as usize;
 for speed in [6.,14.999]{
  *STATE.lock().unwrap()=State{actor:a,handle:123,stats:stats.as_mut_ptr()as usize,..Default::default()};
  stats[0x140/4]=100;damage[0x80/8]=0;STATE.lock().unwrap().wall_stop(speed);
  assert_eq!(query(a,113,1,true,false,0.,base),6);crash_damage(a,base);
  assert_eq!(stats[0x140/4],100);assert_eq!(damage[0x80/8],0);
 }
 for hp in [100,20]{
  stats[0x140/4]=hp;stats[0x144/4]=300;damage[0x80/8]=0;
  *STATE.lock().unwrap()=State{actor:a,handle:123,stats:stats.as_mut_ptr()as usize,..Default::default()};
  STATE.lock().unwrap().wall_stop(15.);
  crash_damage(a,base);assert_eq!(stats[0x140/4],(hp-75).max(0));assert_eq!(damage[0x80/8],1);
  crash_damage(a,base);assert_eq!(damage[0x80/8],1);
 }
 stats[0x140/4]=100;reset();
 // Identical action state must never override another player's capsule.
 world[0x174e8/8]=a+8;desired.0=[1.,2.,3.,0.];fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);assert_eq!(submitted.0,[1.,2.,3.,0.]);
 assert!(art(a,1,base),"remote owner must retain Ram presentation");assert_eq!(query(a,113,1,true,false,0.,base),0,"remote input must not drive local simulation");
 world[0x174e8/8]=a;reset();paid_start();stats[0x150/4]=100;stats[0x15c/4]=100;
 assert_eq!(query(a,113,1,true,false,0.,base),0);assert_eq!(stats[0x150/4],100);
 for _ in 0..5{assert_eq!(query(a,113,1,true,false,0.,base),0);}assert_eq!(stats[0x150/4],100,"native startup is not charged twice");
 assert_eq!(query(a,113,1,true,true,0.,base),2);assert_eq!(stats[0x150/4],100);
 assert_eq!(query(a,113,1,true,true,0.,base),0);assert_eq!(stats[0x150/4],100,"one continuation per window");
 {let mut s=STATE.lock().unwrap();s.started=true;s.motion.locked=true;}
 assert_eq!(query(a,113,1,false,false,0.,base),1);assert_eq!(stats[0x150/4],100,"release preserves inertia without paying");
 assert_eq!(query(a,113,1,false,true,0.,base),2);assert_eq!(stats[0x150/4],100);
 {let mut s=STATE.lock().unwrap();s.motion.locked=false;}
 assert_eq!(query(a,113,1,false,false,0.,base),3);assert!(!STATE.lock().unwrap().active);
 reset();stats[0x150/4]=0;assert_eq!(query(a,113,1,true,false,0.,base),0);assert!(!STATE.lock().unwrap().active);
 stats[0x150/4]=100;
 // FP can stay zero through repeated rolls; stamina gates every update.
 reset();paid_start();stats[0x150/4]=0;stats[0x15c/4]=40;
 assert_eq!(query(a,113,1,true,false,0.,base),0);
 {let mut s=STATE.lock().unwrap();s.started=true;s.rolling=true;s.motion.velocity=V(0.,0.,-12.);s.motion.locked=true;}
 assert_eq!(query(a,113,1,true,true,0.,base),2);assert_eq!(stats[0x150/4],0);
 assert_eq!(query(a,113,1,true,false,0.,base),1);
 assert_eq!(query(a,113,1,true,true,0.,base),2);assert_eq!(stats[0x150/4],0);
 stats[0x15c/4]=1;
 for _ in 0..4{assert_eq!(query(a,113,1,true,false,0.,base),1);assert!(STATE.lock().unwrap().drive);}
 let before=STATE.lock().unwrap().motion.velocity;
 assert_eq!(query(a,113,1,true,true,0.,base),2);
 assert!(STATE.lock().unwrap().drive);assert_eq!(stats[0x15c/4],1,"eligibility does not debit stamina");
 stats[0x15c/4]=-17; // Startup/native external costs can spend past zero.
 for _ in 0..4{assert_eq!(query(a,113,1,true,false,0.,base),1);assert!(!STATE.lock().unwrap().drive);}
 assert_eq!(query(a,113,1,true,true,0.,base),2);
 assert!(!STATE.lock().unwrap().drive);assert_eq!(STATE.lock().unwrap().motion.velocity,before);
 for stamina in [-50,-1,0,1,17,18]{
  reset();paid_start();stats[0x15c/4]=stamina;
  assert_eq!(query(a,113,1,true,false,0.,base),0);
  assert_eq!(STATE.lock().unwrap().active,stamina>0,"startup eligibility at {stamina}");
 }
 reset();paid_start();stats[0x15c/4]=1;query(a,113,1,true,false,0.,base);
 {let mut s=STATE.lock().unwrap();s.started=true;s.rolling=true;}
 assert_eq!(query(a,113,1,true,true,0.,base),2);
 stats[0x15c/4]=-17;
 assert_eq!(query(a,113,1,true,false,0.,base),3);assert!(!STATE.lock().unwrap().drive);
 assert_eq!(query(a,113,1,true,true,0.,base),3,"recover immediately when unpowered and slow");
 reset();stats[0x150/4]=100;stats[0x15c/4]=100;
 // Accepted Ram payments retain only the first native fee of this roll,
 // across low/high-speed behavior replacements and free-cost effects.
 let add_sp:unsafe extern "system" fn(usize,i32,u8)=std::mem::transmute(base+crate::STAMINA_ADD_RVA);
 ptr::write_unaligned((base+0x3c0f338)as*mut usize,1);
 for immunity in [0u8,1,0x20,0x21]{for debug_free in [0u8,1]{for startup in [0,9,18]{
  reset();paid_start();stats[0x15c/4]=90;ptr::write((stats.as_mut_ptr()as usize+0x189)as*mut u8,immunity);ptr::write((base+0x3c0f2f7)as*mut u8,debug_free);
  query(a,113,1,true,false,0.,base);
  assert_eq!(action_cost(a,300000710,110000,startup,base),Some(startup));
  add_sp(stats.as_mut_ptr()as usize,-startup,0);
  let expected=90-if immunity==0&&debug_free==0{startup}else{0};assert_eq!(stats[0x15c/4],expected);
  for behavior in [300000710,300000711,300000715,300000716,OWNER+90]{
   let cost=action_cost(a,behavior,110000,18,base).unwrap();assert_eq!(cost,0);add_sp(stats.as_mut_ptr()as usize,-cost,0);
  }
  assert_eq!(stats[0x15c/4],expected);assert_eq!(action_cost(a,300000720,110000,18,base),None);
  actor[0x78/8]=0;assert_eq!(action_cost(a,300000710,110000,18,base),None);ptr::write_unaligned((a+0x78)as*mut i32,1);
  world[0x174e8/8]=0;assert_eq!(action_cost(a,300000710,110000,18,base),None);world[0x174e8/8]=a;
 }}}
 // Same per-update flag as native HKS act(110), preserving unrelated flags.
 let mut coord=[0usize;0xd0/8];let mut action_flags=[0u32;0x30/4];coord[0xc8/8]=action_flags.as_mut_ptr()as usize;
 actor[0x60/8]=coord.as_mut_ptr()as usize;action_flags[0x18/4]=0x1234;disable_recovery(a);assert_eq!(action_flags[0x18/4],0x1274);
 actor[0x60/8]=0;ptr::write((stats.as_mut_ptr()as usize+0x189)as*mut u8,0x21);ptr::write((base+0x3c0f2f7)as*mut u8,0);reset();
 // Every empty slot uses the same three real art getter detours as Wildcard Arts.
 let old_id=crate::ARTS_ID_ORIG.swap(id_art as*const()as*mut u8,Ordering::AcqRel);
 let old_raw=crate::ARTS_RAW_ORIG.swap(raw_art as*const()as*mut u8,Ordering::AcqRel);
 let old_param=crate::ARTS_PARAM_ORIG.swap(param_art as*const()as*mut u8,Ordering::AcqRel);
 for owned in [0,1]{ptr::write_unaligned((a+0x78)as*mut i32,owned);
  for i in 0..6{let mut r=[0u64;3];reference(r.as_mut_ptr(),&equipment[0x1c/4+i]);let address=r.as_ptr()as usize;
   assert_eq!(crate::arts_raw_detour(address),if owned==1{504}else{503});
   assert_eq!(crate::arts_id_detour(address),if owned==1{113}else{112});
   let mut result=-1;assert_eq!(crate::arts_param_detour(address,&mut result as*mut i32 as usize),&mut result as*mut i32 as usize);assert_eq!(result,if owned==1{504}else{503});
   assert_eq!(entries[i*8+7],503,"inventory stays original");
  }
 }
 ptr::write_unaligned((a+0x78)as*mut i32,1);
 // An observer need not own Ram. Its teammate's empty slots can still have
 // the native Kick override even when the public equipment preview says Ram.
 let mut remote=vec![0usize;0x780/8];let peer=remote.as_mut_ptr()as usize;
 let mut remote_equipment=equipment.clone();remote[0x190/8]=peer;remote[0x670/8]=remote_equipment.as_mut_ptr()as usize;
 let mut party_jump=vec![0x48,0xb8];party_jump.extend_from_slice(&(no_party as*const()as usize).to_le_bytes());party_jump.extend_from_slice(&[0xff,0xe0]);
 ptr::copy_nonoverlapping(party_jump.as_ptr(),(base+0x5197e0)as*mut u8,party_jump.len());
 // Keep the local slots disjoint; merely sharing the same weapon type must
 // never grant the observer or an unequipped/NPC instance the replacement.
 for i in 0..6{equipment[0x1c/4+i]=0x80800100+i as u32;}
 ptr::write_unaligned((a+0x78)as*mut i32,0);PARTY.with(|p|p.set(peer));
 for owned in [0,1]{ptr::write_unaligned((peer+0x78)as*mut i32,owned);
  for i in 0..6{let mut r=[0u64;3];reference(r.as_mut_ptr(),&remote_equipment[0x1c/4+i]);let address=r.as_ptr()as usize;
   assert_eq!(crate::arts_raw_detour(address),if owned==1{504}else{503},"peer empty-slot raw skill");
   assert_eq!(crate::arts_id_detour(address),if owned==1{113}else{112},"peer animation selector");
   let mut result=-1;crate::arts_param_detour(address,&mut result as*mut i32 as usize);assert_eq!(result,if owned==1{504}else{503});
   assert_eq!(entries[i*8+7],503,"peer inventory must remain untouched");
  }
 }
 let mut r=[0u64;3];reference(r.as_mut_ptr(),&remote_equipment[0x1c/4]);let address=r.as_ptr()as usize;
 remote_equipment[0x1c/4]=0;assert_eq!(crate::arts_raw_detour(address),503,"unequipped peer instance");remote_equipment[0x1c/4]=0x80800010;
 entries[4]=2000000;r[2]=2000000;assert_eq!(crate::arts_raw_detour(address),503,"real peer weapon stays native");entries[4]=110000;r[2]=110000;
 PARTY.with(|p|p.set(0));assert_eq!(crate::arts_raw_detour(address),503,"departed peer must not retain replacement");
 for i in 0..6{equipment[0x1c/4+i]=0x80800010+i as u32;}ptr::write_unaligned((a+0x78)as*mut i32,1);
 // A real weapon already carrying Ram also qualifies; other skills do not.
 entries[4]=2000000;entries[7]=504;assert!(art(a,0,base));entries[7]=100;assert!(!art(a,0,base));entries[4]=110000;entries[7]=503;
 // Quote/reserve through the real fee calculator and exercise the startup decision.
 let old_check=crate::onyx_fp::CHECK_ORIGINAL.swap(check_fp as*const()as*mut u8,Ordering::AcqRel);
 let old_reserve=crate::onyx_fp::RESERVE_ORIGINAL.swap(reserve_fp as*const()as*mut u8,Ordering::AcqRel);
 let old_rate=crate::onyx_fp::RATE_ORIGINAL.swap(default_rate as*const()as*mut u8,Ordering::AcqRel);
 let rate_hook=crate::hook::prepare((base+crate::onyx_fp::RATE_RVA)as*mut u8,crate::onyx_fp::rate as*const u8,64).unwrap();crate::hook::write_code((base+crate::onyx_fp::RATE_RVA)as*mut u8,&rate_hook.patch).unwrap();
 let mut art_module=[0usize;4];art_module[1]=a;let am=art_module.as_mut_ptr()as usize;stats[0x150/4]=1;
 for owned in [0,1]{ptr::write_unaligned((a+0x78)as*mut i32,owned);
  for hand in [0,1]{for button in 0..4{
   assert_eq!(crate::onyx_fp::check(am,button,hand),u8::from(owned==1&&button!=3));
   crate::onyx_fp::reserve(am,button,hand);assert_eq!(read_i32(am+0x10),if owned==0{5}else if button==3{10}else{0});
   assert_eq!(quote(am,button),5,"scoped cost escaped the native reservation");
  }}
 }
 ptr::write_unaligned((a+0x78)as*mut i32,1);
 entries[4]=2000000;entries[7]=504;assert_eq!(crate::onyx_fp::check(am,0,0),1);
 entries[7]=100;assert_eq!(crate::onyx_fp::check(am,0,0),0);assert_eq!(crate::onyx_fp::check(am,0,1),1);
 entries[4]=110000;entries[7]=503;
 for fp in [0,4,5,9,10]{
  reset();stats[0x150/4]=fp;stats[0x15c/4]=100;
  let accepted=crate::onyx_fp::check(am,3,1)!=0;
  assert_eq!(accepted,fp>=5);
  query(a,113,1,true,false,0.,base);assert!(!STATE.lock().unwrap().active,"a quote alone cannot start a roll");
  crate::onyx_fp::reserve(am,3,1);assert_eq!(read_i32(am+0x10),10);
  stats[0x150/4]=0;query(a,113,0,true,false,0.,base);assert!(!STATE.lock().unwrap().active);
  query(a,113,1,true,false,0.,base);assert_eq!(STATE.lock().unwrap().active,accepted);
  if accepted{
   assert_eq!(query(a,113,1,true,true,0.,base),2);assert_eq!(stats[0x150/4],0);
   crate::onyx_fp::reserve(am,3,1);query(a,113,1,true,true,0.,base);
   assert!(!STATE.lock().unwrap().active,"a failed restart cannot reuse the prior startup");
  }
 }
 stats[0x150/4]=100;
 crate::onyx_fp::CHECK_ORIGINAL.store(old_check,Ordering::Release);crate::onyx_fp::RESERVE_ORIGINAL.store(old_reserve,Ordering::Release);crate::onyx_fp::RATE_ORIGINAL.store(old_rate,Ordering::Release);
 // Windup has real native translation (up to 1.16m/s) but must not start power.
 reset();paid_start();assert_eq!(query(a,113,1,true,false,0.,base),0);
 for v in [0.3,0.9,1.2,3.]{desired.0=[0.,0.,-v,0.];fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);assert_eq!(desired.0[2],-v);assert!(!STATE.lock().unwrap().started);}
 assert_eq!(behavior(a,3710,-1,base),Some(300000710));
 desired.0=[0.,0.,-4.,0.];fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);assert!(STATE.lock().unwrap().started);assert!(desired.0[2]< -4.);
 // Relocated original speed getter preserves native multipliers; no persistent write.
 let mut animation=vec![0usize;0x1900/8];modules[0x28/8]=animation.as_mut_ptr()as usize;
 let context=animation.as_mut_ptr()as usize+0x1800;ptr::write_unaligned((context+0x18)as*mut f32,0.8);
 let hook=crate::hook::prepare((base+crate::SPEED_RVA)as*mut u8,crate::speed_detour as*const u8,64).unwrap();assert_eq!(hook.stolen,5);
 let old_speed=crate::SPEED_ORIG.swap(hook.trampoline,Ordering::AcqRel);crate::hook::write_code((base+crate::SPEED_RVA)as*mut u8,&hook.patch).unwrap();
 let get:unsafe extern "system" fn(usize)->f32=std::mem::transmute(base+crate::SPEED_RVA);
 for v in [ROLL_SPEED,ROLL_SPEED*2.,ROLL_SPEED*3.]{let mut state=STATE.lock().unwrap();state.motion.velocity=V(0.,0.,-v);state.visual_speed=v;drop(state);assert!((get(context)-0.8*v/ROLL_SPEED).abs()<0.0001);}
 {STATE.lock().unwrap().active=false;}assert_eq!(get(context),0.8);
 // Escape's pre-existing 2x speed shares this exact installed entry.
 ptr::write_unaligned((context+8)as*mut usize,a);
 let old_escape=crate::ESCAPE_PLAYER.swap(a,Ordering::AcqRel);assert_eq!(get(context),2.);
 {let mut state=STATE.lock().unwrap();state.active=true;state.visual_speed=ROLL_SPEED*2.;}assert_eq!(get(context),4.);
 ACTIVE.store(false,Ordering::Release);assert_eq!(get(context),0.8);ACTIVE.store(true,Ordering::Release);
 crate::ESCAPE_PLAYER.store(old_escape,Ordering::Release);assert_eq!(ptr::read_unaligned((context+0x18)as*const f32),0.8);
 crate::SPEED_ORIG.store(old_speed,Ordering::Release);crate::ARTS_ID_ORIG.store(old_id,Ordering::Release);crate::ARTS_RAW_ORIG.store(old_raw,Ordering::Release);crate::ARTS_PARAM_ORIG.store(old_param,Ordering::Release);
 println!("PASS: six-slot native skill overlays, real-weapon Ram, windup gate and native animation-rate getter");
 // Execute the game's explicit impact override lookup, including player targets.
 let override_level:unsafe extern "system" fn(u8)->i32=std::mem::transmute(base+0xe6d510);
 for (speed,expected) in [(6.,3),(10.,7),(15.,10)]{
  let (_,level,id)=contact_tier(speed*speed*1.5).unwrap();
  assert_eq!(override_level(id),expected);assert_eq!(i32::from(level),expected);
 }
 println!("PASS: native impact override lookup for medium, fast and extreme enemy contacts");
 // Capture the real native resolver return through the shipping hook. The
 // native explicit-result path (+121=3) supplies controlled values; this is
 // not an assertion about an NPC animation. Deliberately stale action+1C
 // must not change the collision response, especially for kind 99999.
 let mut victim=vec![0usize;0x200/8];let mut victim_modules=[0usize;0x80/8];
 let mut action=[0u8;0x40];let mut module=[0usize;4];let mut hit=[0u8;0x300];
 let target=victim.as_mut_ptr()as usize;victim[0x1b8/8]=victim_modules.as_ptr()as usize;
 victim_modules[1]=action.as_mut_ptr()as usize;module[1]=target;
 let hitptr=hit.as_mut_ptr()as usize;ptr::write_unaligned((hitptr+0x40)as*mut i32,ATTACK);ptr::write_unaligned((hitptr+0x44)as*mut i32,1);hit[0x121]=3;hit[0x122]=3;
 let prepared=crate::hook::prepare((base+IMPACT_RVA)as*mut u8,resolved_impact as*const u8,64).unwrap();
 let old_impact=IMPACT_ORIGINAL.swap(prepared.trampoline,Ordering::AcqRel);crate::hook::write_code((base+IMPACT_RVA)as*mut u8,&prepared.patch).unwrap();
 let resolve_impact:unsafe extern "system" fn(usize,u8,usize)->u8=std::mem::transmute(base+IMPACT_RVA);
 // Real native preflight: 48D920 -> 507C40 -> EA0CF0 -> E6D510, with
 // controlled effect rows. Only the poise virtual getter is a read-only stub.
 // Verify no HP, poise, effect-list, hit or action bytes are modified.
 {
  let mut effects=[0usize;3];let mut node=[0usize;8];let mut row=[0u8;0x160];
  let vt=[0usize,0,zero_float as*const()as usize];let poise=[vt.as_ptr()as usize,0,0,0,0];
  victim[0x190/8]=effects.as_ptr()as usize;victim_modules[0x40/8]=poise.as_ptr()as usize;
  effects[1]=node.as_ptr()as usize;node[0]=row.as_ptr()as usize;
  row[0x13e..0x140].copy_from_slice(&1001u16.to_le_bytes());row[0x140]=1;
  for (mapping,expected)in[(0u8,2u8),(1,0),(10,5)]{
   row[0x14a]=mapping;
   let before=(victim.clone(),victim_modules,action,hit,effects,node,row,poise);
   let _scope=ContactScope::enter(true);
   for speed in [6.,10.,15.]{
    let _hit=contact_impact(hitptr,target,speed*speed*1.5,base);
    assert_eq!(ENEMY_CONTACT.with(|c|c.get().unwrap().medium),Some(expected));
    assert!(RESOLUTION.with(|r|r.get().unwrap().value.is_none()));
   }
   assert_eq!(before,(victim.clone(),victim_modules,action,hit,effects,node,row,poise));
   // Reuse the captured Golden Hippo contact: normal closing speed was only
   // 11.998669 although this was a valid tier-10 hit at 15.826884 m/s.
   let mut s=State{active:true,drive:true,motion:Motion{velocity:V(-14.900168,0.0004190615,5.336222),..Default::default()},..Default::default()};
   s.enemy_collision(EnemyContact{speed:15.826884,normal:V(0.7386203,-0.6479381,-0.18605481),medium:Some(expected)},EnemyResponse::Yield,true);
   assert_eq!(s.crash,expected==0);assert_eq!(s.active,expected!=0);assert!(!s.crash_pending);
  }
  victim[0x190/8]=0;victim_modules[0x40/8]=0;assert_eq!(medium_impact(target),None);
 }
 println!("PASS: native level-2 preflight distinguishes default/remapped reactions from immunity without modifying target, hit, poise, action, effects or current impact observation");
 for (speed,impact,kind,local) in [(15.,10,0,true),(15.,3,0,true),(15.,0,0,true),(8.,0,0,true),(15.,10,3,true),(15.,0,0,false),(15.,10,99999,true),(15.,0,99999,true),(15.,3,99999,true)]{
  *STATE.lock().unwrap()=State{actor:a,active:true,drive:true,motion:Motion{velocity:V(speed,0.,0.),..Default::default()},..Default::default()};
  action[0x1c]=if impact==0{10}else{0};
  let _contact=ContactScope::enter(local);let _override=impact_scope(hitptr,target,speed*speed*1.5,V(-1.,0.,0.),Some(0));
  reaction(module.as_ptr()as usize,hitptr,kind);assert_eq!(STATE.lock().unwrap().motion.velocity,V(speed,0.,0.));assert!(STATE.lock().unwrap().active);
  assert_eq!(resolve_impact(target+8,impact,hitptr),impact);assert!(RESOLUTION.with(|r|r.get().unwrap().value.is_none()));
  assert_eq!(resolve_impact(target,impact,hitptr+1),impact);assert!(RESOLUTION.with(|r|r.get().unwrap().value.is_none()));
  assert_eq!(resolve_impact(target,impact,hitptr),impact);
  reaction(module.as_ptr()as usize,hitptr,kind);
  let s=STATE.lock().unwrap();let once=s.motion.velocity;
  if !local{assert_eq!(once,V(speed,0.,0.));assert!(s.active);}
  else{
   assert!(s.active&&!s.crash&&!s.push);
   let pass=impact==0||(kind!=3&&impact==10);
   assert_eq!(s.yielding.knocked,if pass{0.12}else{0.});
   assert_eq!(once,V(if pass{speed-speed*0.05}else{0.},0.,0.));
  }
  drop(s);reaction(module.as_ptr()as usize,hitptr,kind);assert_eq!(STATE.lock().unwrap().motion.velocity,once);
 }
 // Execute the real reaction writer: native suppressed impacts enter the
 // resolver as zero; the private extreme row supplies ultimate behavior 12.
 // Controlled lookups replace only external object/parameter dependencies.
 {
  let saved_rows=OBJECT_ROWS.load(Ordering::Relaxed);let mut rows=[0u8;0x400];rows[0x282]=12;
  OBJECT_ROWS.store(rows.as_ptr()as usize,Ordering::Relaxed);
  for(rva,f)in[(0x42c120,module_actor as*const()as usize),(0xe6d630,ultimate_lookup as*const()as usize),(0x6cc190,zero_float as*const()as usize),(0x3e6780,release as*const()as usize),(0x52f780,no_flag as*const()as usize)]{
   let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&f.to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(jump.as_ptr(),(base+rva)as*mut u8,jump.len());
  }
  let writer:unsafe extern "system" fn(usize,usize,i32,usize)=std::mem::transmute(base+0x4381f0);
  let mut damage_module=[0usize;0x80/8];damage_module[1]=target;
  for (id,speed)in[(ATTACK,10.),(EXTREME_ATTACK,20.)]{for suppress in [0,0x2b9,0x2ba]{
   *STATE.lock().unwrap()=State{actor:a,active:true,drive:true,motion:Motion{velocity:V(speed,0.,0.),..Default::default()},..Default::default()};
   hit.fill(0);hit[0x121]=3;hit[0x122]=3;if suppress!=0{hit[suppress]=1;}
   ptr::write_unaligned((hitptr+0x40)as*mut i32,id);ptr::write_unaligned((hitptr+0x44)as*mut i32,1);
   let _contact=ContactScope::enter(true);let _override=impact_scope(hitptr,target,speed*speed*1.5,V(-1.,0.,0.),Some(0));
   writer(damage_module.as_mut_ptr()as usize,hitptr,99999,0);
   assert_eq!(action[0x24],if id==EXTREME_ATTACK{12}else{0});
   assert_eq!(RESOLUTION.with(|r|r.get().unwrap().suppressed),suppress!=0);
   reaction(damage_module.as_ptr()as usize,hitptr,99999);
   let s=STATE.lock().unwrap();
   // The native writer only queues behavior 12; that is not proof that HKS
   // can play it. With no animation graph, even a 10-impact stays deflection.
   assert!(s.active&&!s.crash&&!s.push&&!s.crash_pending&&s.pending.is_empty());
  }}
  OBJECT_ROWS.store(saved_rows,Ordering::Relaxed);
  let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&(object_lookup as*const()as usize).to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(jump.as_ptr(),(base+0xe6d630)as*mut u8,jump.len());
 }
 IMPACT_ORIGINAL.store(old_impact,Ordering::Release);
 println!("PASS: native writer queues behavior 12 but cannot alone trigger recoil; missing graph, suppressed hits, stale action and wrong hit/target excluded; response consumed once");
 // Actual Havok slope-plane append and native support snapping branches.
 // Allocator lookup is a controlled stub; sufficient plane capacity means no
 // allocation. The original real contact is byte-for-byte preserved.
 {
  use crate::ram_native_contact as nc;
  let image=crate::pe::FileImage::parse(&raw).unwrap();let regions=image.exec_regions();
  for (sig,rva) in [(nc::QUERY_PATTERN,nc::QUERY_RVA),(nc::CONTACTS_PATTERN,nc::CONTACTS_RVA),(nc::SLOPE_PATTERN,nc::SLOPE_RVA),(nc::SNAP_PATTERN,nc::SNAP_RVA),(IMPACT_PATTERN,IMPACT_RVA)]{
   let hits=crate::pattern::scan_regions(&regions,&crate::pattern::Pattern::parse(sig).unwrap());
   assert_eq!(hits.len(),1);assert_eq!(hits[0].1.rva,rva as u32);
  }
  let patch_jump=|rva:usize,target:usize|{
   let mut bytes=vec![0x48,0xb8];bytes.extend_from_slice(&target.to_le_bytes());bytes.extend_from_slice(&[0xff,0xe0]);
   ptr::copy_nonoverlapping(bytes.as_ptr(),(base+rva)as*mut u8,bytes.len());
  };
  patch_jump(0x181b5c0,no_allocator as*const()as usize);
  let mut proxy=vec![0usize;0x160/8];let p=proxy.as_mut_ptr()as usize;
  controller[0]=base+0x2c9e6a0;controller[0x88/8]=p;stats[0x140/4]=100;
  *STATE.lock().unwrap()=State{actor:a,handle:123,active:true,started:true,rolling:true,..Default::default()};
  let prepared=crate::hook::prepare((base+nc::SLOPE_RVA)as*mut u8,nc::slope as*const u8,64).unwrap();
  let old_slope=nc::SLOPE_ORIGINAL.swap(prepared.trampoline,Ordering::AcqRel);crate::hook::write_code((base+nc::SLOPE_RVA)as*mut u8,&prepared.patch).unwrap();
  let slope:unsafe extern "system" fn(usize,*mut u8,f32,*const f32,i32,usize,f32)->*mut u8=std::mem::transmute(base+nc::SLOPE_RVA);
  #[repr(align(16))]struct Planes([u8;0x140]);
  let up=Vec4([0.,1.,0.,0.]);let mut storage=Planes([0;0x140]);let mut list=[0usize;2];list[0]=storage.0.as_mut_ptr()as usize;
  for (managed,normal_y,expected) in [(false,0.4,2),(true,0.4,1),(true,0.,1),(false,0.,1),(false,0.8,1)]{
   STATE.lock().unwrap().rolling=managed;storage.0.fill(0);
   ptr::write_unaligned(storage.0.as_mut_ptr()as*mut[f32;4],[(1f32-normal_y*normal_y).sqrt(),normal_y,0.,0.]);
   let contact=storage.0[..0x50].to_vec();list[1]=(4usize<<32)|1;let mut flag=99u8;
   assert_eq!(slope(p,&mut flag,0.5,up.0.as_ptr(),0,list.as_mut_ptr()as usize,0.),&mut flag as*mut u8);
   assert_eq!(list[1]as u32,expected);assert_eq!(flag,u8::from(expected==2));assert_eq!(&storage.0[..0x50],contact.as_slice());
  }
  STATE.lock().unwrap().rolling=true;
  assert!(managed_proxy(p,base));assert!(!managed_proxy(p+8,base));
  actor[1]=124;assert!(!managed_proxy(p,base));actor[1]=123;
  stats[0x140/4]=0;assert!(!managed_proxy(p,base));stats[0x140/4]=100;
  // Execute the real manifold -> plane builder and real body-ID resolver.
  // Plane material/body velocity assembly is a fixture; the native builder,
  // slope append, penetration correction, hook and identity gates are real.
  {
   let mut manager=[0usize;8];let mut body_world=[0usize;8];let mut bodies=vec![0usize;6*0xb0/8];
   let mut entries=[[0usize;4];6];let mut userdata=[0usize;6];
   let mut npc=vec![0usize;0x600/8];let mut npc_vt=[0usize;0x1b0/8];let npc_ref=[0i32,0,48110020];
   npc_vt[0x1a0/8]=base+0x4d1ea0;npc[0]=npc_vt.as_ptr()as usize;npc[0x5b8/8]=npc_ref.as_ptr()as usize;
   for(i,kind)in [0usize,2,5,2,7,2].into_iter().enumerate(){
    entries[i][1]=kind;entries[i][2]=if i==1||i==5{npc.as_ptr()as usize}else if i==3{a}else{0};
    userdata[i]=entries[i].as_ptr()as usize;bodies[(i*0xb0+0xa0)/8]=userdata.as_ptr().add(i)as usize;
   }
   manager[1]=body_world.as_ptr()as usize;body_world[0x28/8]=bodies.as_ptr()as usize;
   let old_geometry=geometry[0x98/8];geometry[0x98/8]=manager.as_ptr()as usize;
   // Real proxy query collector: an NPC sweep must not reduce its fraction.
   // Exact player collectors are matched; other queries still report NPCs.
   #[repr(align(16))]struct Query([u8;0x160]);
   let mut query_storage=Query([0;0x160]);let q=query_storage.0.as_mut_ptr()as usize;
   let mut delegate=Query([0;0x160]);let d=delegate.0.as_mut_ptr()as usize;
   let delegate_vt=[0usize,0,0,0,collected_query_fixture as*const()as usize];
   let mut material_world=[0usize;8];let material=[0u8;0x70];material_world[0x30/8]=material.as_ptr()as usize;
   ptr::write_unaligned((q+0x148)as*mut usize,material_world.as_ptr()as usize);
   ptr::write_unaligned(d as*mut usize,delegate_vt.as_ptr()as usize);
   let prepared=crate::hook::prepare((base+nc::QUERY_RVA)as*mut u8,nc::query as*const u8,64).unwrap();
   let old_query=nc::QUERY_ORIGINAL.swap(prepared.trampoline,Ordering::AcqRel);
   crate::hook::write_code((base+nc::QUERY_RVA)as*mut u8,&prepared.patch).unwrap();
   let add_hit:unsafe extern "system" fn(usize,usize)=std::mem::transmute(base+nc::QUERY_RVA);
   for mode in 0..5{for id in 0..6u32{
    STATE.lock().unwrap().rolling=mode!=2;controller[0xa8/8]=if mode==3{0}else{d};controller[0xb0/8]=if mode==3{d}else{0};
    if mode==1{controller[0xa8/8]=d+8;}if mode==4{actor[1]=124;}
    ptr::write_unaligned((q+0x140)as*mut usize,d);
    ptr::write_unaligned((q+0xc)as*mut u32,0);ptr::write((q+0x20)as*mut u8,0);
    ptr::write_unaligned((q+0x10)as*mut[f32;4],[1.;4]);
    ptr::write_unaligned((d+0xc)as*mut u32,0);ptr::write((d+0x20)as*mut u8,0);
    ptr::write_unaligned((d+0x10)as*mut[f32;4],[1.;4]);
    let mut hit=[0u8;0x70];ptr::write_unaligned(hit.as_mut_ptr().add(0x48)as*mut u32,id);
    add_hit(q,hit.as_ptr()as usize);
    let bypass=matches!(mode,0|3)&&matches!(id,1|5);
    assert_eq!(ptr::read_unaligned((q+0xc)as*const u32),if bypass{0}else{id+1},"mode={mode} id={id}");
    assert_eq!(ptr::read_unaligned((q+0x10)as*const f32),if bypass{1.}else{0.25});
    assert_eq!(ptr::read_unaligned((d+0xc)as*const u32),if bypass{0}else{id+1});actor[1]=123;
   }}
   controller[0xa8/8]=0;controller[0xb0/8]=0;STATE.lock().unwrap().rolling=true;
   nc::QUERY_ORIGINAL.store(old_query,Ordering::Release);
   println!("PASS: real native proxy query preserves full sweep fraction for NPC bodies only; map/assets/player/unknown queries and nonrolling/unrelated/stale owner forward unchanged");
   #[repr(align(16))]struct Contacts([u8;0x180]);
   #[repr(align(16))]struct Output([u8;0x280]);
   let mut contacts=Contacts([0;0x180]);let mut output=Output([0;0x280]);let mut output_list=[output.0.as_mut_ptr()as usize,8usize<<32];
   proxy[0x18/8]=contacts.0.as_mut_ptr()as usize;
   ptr::write_unaligned((p+0x120)as*mut[f32;4],[0.,1.,0.,0.]);
   let prepared=crate::hook::prepare((base+nc::CONTACTS_RVA)as*mut u8,nc::contacts as*const u8,64).unwrap();
   let old_contacts=nc::CONTACTS_ORIGINAL.swap(prepared.trampoline,Ordering::AcqRel);
   crate::hook::write_code((base+nc::CONTACTS_RVA)as*mut u8,&prepared.patch).unwrap();
   patch_jump(0x1a6cae0,contact_plane_fixture as*const()as usize);
   let build:unsafe extern "system" fn(usize,usize,f32,usize)=std::mem::transmute(base+nc::CONTACTS_RVA);
   for mode in 0..8{
    *STATE.lock().unwrap()=State{actor:a,handle:123,active:true,started:true,rolling:true,..Default::default()};
    match mode{1=>STATE.lock().unwrap().rolling=false,2=>STATE.lock().unwrap().interrupted=true,3=>stats[0x140/4]=0,
     4=>actor[1]=124,5=>world[0x174e8/8]=0,6=>STATE.lock().unwrap().active=false,7=>controller[0x88/8]=p+8,_=>{}}
    for i in 0..6{
     contacts.0[i*0x40..(i+1)*0x40].fill(i as u8);
     ptr::write_unaligned(contacts.0.as_mut_ptr().add(i*0x40+0x10)as*mut[f32;4],if i==0{[0.,1.,0.,0.]}else{[1.,0.,0.,0.]});
     ptr::write_unaligned(contacts.0.as_mut_ptr().add(i*0x40+0x28)as*mut u32,i as u32);
    }
    let before=contacts.0;proxy[0x20/8]=(6usize<<32)|6;output_list[1]=8usize<<32;
    build(p,0x1234,1./60.,output_list.as_mut_ptr()as usize);
    let expected:Vec<usize>=if mode==0{vec![0,2,3,4]}else{(0..6).collect()};
    assert_eq!(proxy[0x20/8]as u32,expected.len()as u32,"mode {mode}");
    assert_eq!(output_list[1]as u32,expected.len()as u32);
    assert_eq!(proxy[0x20/8]>>32,6,"manifold capacity unchanged");
    for(j,i)in expected.into_iter().enumerate(){
     assert_eq!(&contacts.0[j*0x40..(j+1)*0x40],&before[i*0x40..(i+1)*0x40]);
     assert_eq!(ptr::read_unaligned(output.0.as_ptr().add(j*0x50+0x44)as*const u32),i as u32);
    }
    stats[0x140/4]=100;actor[1]=123;world[0x174e8/8]=a;controller[0x88/8]=p;
   }
   // Next native manifold rebuild restores NPC contacts immediately on exit.
   assert_eq!(proxy[0x20/8]as u32,6);
   nc::CONTACTS_ORIGINAL.store(old_contacts,Ordering::Release);geometry[0x98/8]=old_geometry;
   println!("PASS: real native solver builder excludes NPC body contacts only for live rolling owner; floor, walls, assets, player bodies and unknown kinds preserved; indices aligned and next nonrolling rebuild restored");
  }
  let prepared=crate::hook::prepare((base+nc::SNAP_RVA)as*mut u8,nc::snap_detour as*const u8,64).unwrap();
  let old_snap=nc::SNAP_ORIGINAL.swap(prepared.trampoline,Ordering::AcqRel);crate::hook::write_code((base+nc::SNAP_RVA)as*mut u8,&prepared.patch).unwrap();
  patch_jump(0x471e40,snap_exit as*const()as usize);patch_jump(0x471e50,snap_exit as*const()as usize);
  let ph=physics.as_mut_ptr()as usize;
  for (managed,airborne,distance,grounded,correction) in [(false,false,0.6,1,-0.6),(true,false,0.6,0,-0.3),(true,false,0.31,1,-0.31),(true,true,0.31,0,-0.3),(true,true,0.29,1,-0.29)]{
   {let mut s=STATE.lock().unwrap();s.rolling=managed;s.motion.airborne=airborne;}
   ptr::write_unaligned((ph+0x1e0)as*mut f32,1.);ptr::write((ph+0x92)as*mut u8,0);
   snap_fixture(ph,[0.7,0.3,distance,0.].as_ptr(),base+nc::SNAP_RVA);
   assert_eq!(ptr::read((ph+0x92)as*const u8),grounded,"managed={managed} air={airborne} distance={distance}");
   assert!((ptr::read_unaligned((ph+0x11c)as*const f32)-correction).abs()<0.00001);
  }
  nc::SLOPE_ORIGINAL.store(old_slope,Ordering::Release);nc::SNAP_ORIGINAL.store(old_snap,Ordering::Release);
  controller[0]=0;controller[0x88/8]=0;
  println!("PASS: new signatures unique; real native extra-slope plane removed only for rolling owner; real contact retained; native snap capped without disabling upward correction");
 }
 // Real entity identity lookup versus pool-handle lookup. This is the exact
 // local-player encoding captured in game, not a generic integer fixture.
 {
  let entity=0xffff_ffff_10e0_0000u64;
  let pool_index:unsafe extern "system" fn(*const u64)->u32=std::mem::transmute(base+0x63ff80);
  let slot_index:unsafe extern "system" fn(*const u64)->u32=std::mem::transmute(base+0x63ff10);
  let pool_id=pool_index(&entity)as usize;assert!(pool_id<=0x84);assert_eq!(slot_index(&entity),0);
  let mut pool_vt=[0usize;5];pool_vt[4]=entity_slot as*const()as usize;
  let mut remote=vec![0usize;0x200/8];let r=remote.as_mut_ptr()as usize;
  let mut pool=[pool_vt.as_ptr()as usize,a,r];world[(0x170b8/8)+pool_id]=pool.as_mut_ptr()as usize;
  actor[1]=entity as usize;remote[1]=(entity+1)as usize;actor[0x60/8]=a;remote[0x60/8]=r;remote[0x1b8/8]=modules.as_ptr()as usize;
  assert_eq!(native_actor::resolve(base,entity),0);
  assert_eq!(crate::static_charge::resolve_actor(base,entity),a);
  assert_eq!(crate::static_charge::resolve_actor(base,entity+1),r);
  // Real hkbIsNodeActive over controlled graph state. Confirm the full
  // deferred path only after a new native node and the behavior-12 query.
  {
   let mut rm=[0usize;0x80/8];rm[0]=stats.as_ptr()as usize;
   let mut anim=[0usize;4];let mut instance=[0usize;8];let mut character=[0usize;0xa0/8];let mut graph=[0usize;0x1c0/8];
   let mut node=[0usize;0x50/8];let active_node=[0usize,node.as_mut_ptr()as usize];let entries=[active_node.as_ptr()as usize];
   let mut array=[entries.as_ptr()as usize,0usize];graph[0]=base+0x2df1098;graph[0xe0/8]=array.as_mut_ptr()as usize;
   ptr::write((graph.as_mut_ptr()as usize+0x1b2)as*mut u8,1);
   character[0]=base+0x2e1a2b8;character[0x98/8]=graph.as_mut_ptr()as usize;instance[0x30/8]=character.as_mut_ptr()as usize;
   anim[1]=r;anim[2]=instance.as_mut_ptr()as usize;rm[0x28/8]=anim.as_mut_ptr()as usize;remote[0x1b8/8]=rm.as_mut_ptr()as usize;remote[0x190/8]=r;
   let expected=crate::ram_stagger::prepare(r,base).unwrap();
   assert!(!crate::ram_stagger::confirmed(r,expected,base));
   ptr::write_unaligned((r+0x7c)as*mut i32,1);assert!(crate::ram_stagger::prepare(r,base).is_none());ptr::write_unaligned((r+0x7c)as*mut i32,0);
   // Tagged hkStringPtr, as used by the game's actual active-node lookup.
   let name=std::ffi::CString::new("DamageExtraLarge_CMSG").unwrap();node[0x48/8]=name.as_ptr()as usize;
   array[1]=1;assert!(crate::ram_stagger::confirmed(r,expected,base));assert!(crate::ram_stagger::prepare(r,base).is_none());
   let pending=PendingStagger{target:r,handle:entity+1,contact:EnemyContact{speed:20.,normal:V(-1.,0.,0.),medium:Some(0)},incoming:V(20.,0.,0.),expected,age:0.,consumed:false};
   *STATE.lock().unwrap()=State{actor:a,handle:entity,active:true,generation:77,pending:vec![pending],..Default::default()};
   confirm_staggers(base,1./60.);assert!(STATE.lock().unwrap().active,"node alone is not attributable to our hit");
   stagger_consumed(r,entity+1);array[1]=0;
   confirm_staggers(base,1./60.);assert!(STATE.lock().unwrap().active,"queued behavior alone is insufficient");
   array[1]=1;confirm_staggers(base,1./60.);
   {let s=STATE.lock().unwrap();assert!(s.crash&&!s.active&&s.rebound>0.&&s.motion.velocity.0< -6.9&&!s.crash_pending);}
   {let mut s=STATE.lock().unwrap();s.started=true;s.interrupted=true;s.drive=true;s.wish=V(1.,0.,0.);}
   desired.0=[0.,0.,0.,0.];fixture(physics.as_mut_ptr()as usize,desired.0.as_mut_ptr(),time.as_ptr(),base+MOTION_RVA);
   assert!(submitted.0[0]< -6.9&&(submitted.0[1]-(2.5-crate::ram_motion::GRAVITY/60.)).abs()<0.001,"recoil must survive recovery state and forward input, with only gravity changing vertical speed");
   for expired in [false,true]{
    *STATE.lock().unwrap()=State{actor:a,handle:entity,active:true,generation:77,pending:vec![PendingStagger{consumed:true,age:if expired{0.26}else{0.},..pending}],..Default::default()};
    if !expired{remote[1]=123;}
    confirm_staggers(base,1./60.);{let s=STATE.lock().unwrap();assert!(s.active&&!s.crash);}
    remote[1]=(entity+1)as usize;
   }
   remote[0x1b8/8]=modules.as_ptr()as usize;
   println!("PASS: native active-node query rejects missing/already active graph, unconsumed behavior, absent transition, expired contact and recycled identity; confirmed new stagger launches outward without HP cost");
  }
  remote[1]=123;assert_eq!(crate::static_charge::resolve_actor(base,entity+1),0);remote[1]=(entity+1)as usize;
  // Only party enumeration is a fixture; both identity lookup functions run
  // unmodified EXE code through the real entity-pool virtual interface.
  let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&(no_party as*const()as usize).to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(jump.as_ptr(),(base+0x5197e0)as*mut u8,jump.len());
  let mut victim=vec![0usize;0x740/8];let target=victim.as_mut_ptr()as usize;victim[1]=0x2b01173311200006;victim[0x60/8]=target;victim[0x1b8/8]=modules.as_ptr()as usize;
  let mut bullet=[0usize;0xc0/8];let b=bullet.as_ptr()as usize;bullet[0xb0/8]=entity as usize;
  *STATE.lock().unwrap()=State{actor:a,handle:entity,active:true,generation:77,..Default::default()};CONTACTS.lock().unwrap().clear();
  ptr::write_unaligned((physics.as_mut_ptr()as usize+0x70)as*mut[f32;4],[0.,0.,0.,1.]);
  assert!(allow_contact(target,b,Some(77),base));assert!(!allow_contact(target,b,None,base));assert!(!allow_contact(a,b,Some(77),base));assert!(!allow_contact(target,b,Some(76),base));
  {
   let _contact=ContactScope::enter(true);
   let _hit=impact_scope(hitptr,target,150.,V(-1.,0.,0.),Some(0));
   STATE.lock().unwrap().motion.velocity=V(10.,0.,0.);
   accepted_contact(target,b,Some(77),0,base);assert!(allow_contact(target,b,Some(77),base));
   assert_eq!(STATE.lock().unwrap().motion.velocity,V(10.,0.,0.));
   accepted_contact(target,b,Some(77),1,base);assert!(!allow_contact(target,b,Some(77),base));
   assert_eq!(STATE.lock().unwrap().motion.velocity,V(9.5,0.,0.));
   assert!(ENEMY_CONTACT.with(|c|c.get().is_none()));
  }
  ptr::write_unaligned((physics.as_mut_ptr()as usize+0x70)as*mut[f32;4],[3.01,0.,0.,1.]);assert!(allow_contact(target,b,Some(77),base));
  STATE.lock().unwrap().active=false;assert!(!allow_contact(target,b,Some(77),base));
  bullet[0xb0/8]=(entity+1)as usize;assert!(allow_contact(target,b,Some(19),base));accepted_contact(target,b,Some(19),1,base);assert!(!allow_contact(target,b,Some(19),base));
  CONTACTS.lock().unwrap().clear();
  PARTY.with(|p|p.set(target));
  for down in [0u8,1,2,3]{ptr::write((target+0x710)as*mut u8,down);assert_eq!(allow_contact(target,b,Some(19),base),down==1);}
  ptr::write((target+0x710)as*mut u8,1);accepted_contact(target,b,Some(19),1,base);assert!(!allow_contact(target,b,Some(19),base));
  ptr::write_unaligned((physics.as_mut_ptr()as usize+0x70)as*mut[f32;4],[7.,0.,0.,1.]);assert!(allow_contact(target,b,Some(19),base));
  PARTY.with(|p|p.set(0));CONTACTS.lock().unwrap().clear();
  println!("PASS: old pool lookup rejects live entity handle; real entity lookup resolves local/remote owner; stale identity, roll generation, self and duplicate-contact guards preserved");
 }
 reset();MOTION_ORIGINAL.store(old_orig,Ordering::Release);BASE.store(old_base,Ordering::Release);ACTIVE.store(old_active,Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
