//! Execute the game's collector, six-output ABI, getters and periodic call
//! ordering. Actor/world lookup, notifications and the HP sink are fixtures.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
static CALLS:Mutex<Vec<i32>>=Mutex::new(Vec::new());
static RAIN:std::sync::atomic::AtomicI32=std::sync::atomic::AtomicI32::new(99810);
static OWNED:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(true);
unsafe fn put<T:Copy>(address:usize,value:T){ptr::write_unaligned(address as*mut T,value);}
unsafe fn bytes(base:usize,rva:usize,data:&[u8]){ptr::copy_nonoverlapping(data.as_ptr(),(base+rva)as*mut u8,data.len());}
unsafe fn jump(base:usize,rva:usize,target:usize){let mut b=vec![0x48,0xb8];b.extend_from_slice(&target.to_le_bytes());b.extend_from_slice(&[0xff,0xe0]);bytes(base,rva,&b);}
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from((id==OWNER&&OWNED.load(Ordering::Relaxed))||id==RAIN.load(Ordering::Relaxed))}
unsafe extern "system" fn debit(stats:usize,delta:i32,_mode:u8,_limit:u8,_rate:f32,_window:f32,_refresh:u8){CALLS.lock().unwrap().push(delta);put(stats+0x140,read_i32(stats+0x140).saturating_add(delta).clamp(0,read_i32(stats+0x144)));}
#[test]
#[ignore="requires supported NRHEX_NATIVE_EXE; run alone"]
fn original_periodic_collector_filters_only_local_rain_and_heals_before_damage(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();
 assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let headers=crate::pe::Headers::parse(&raw,0).unwrap();
 let memory=VirtualAlloc(ptr::null_mut(),headers.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for s in &headers.sections{bytes(base,s.virtual_address as usize,&raw[s.raw_offset as usize..(s.raw_offset+s.raw_size)as usize]);}
 let mut player=vec![0usize;0x720/8];let player=player.as_mut_ptr()as usize;
 let mut stats=vec![0usize;0x200/8];let stats=stats.as_mut_ptr()as usize;
 let mut modules=vec![0usize;0x200/8];let modules=modules.as_mut_ptr()as usize;
 let mut manager=vec![0usize;0x40/8];let manager=manager.as_mut_ptr()as usize;
 let mut damage=vec![0usize;0x90/8];let damage=damage.as_mut_ptr()as usize;
 let mut world=vec![0usize;0x17500/8];let world=world.as_mut_ptr()as usize;
 let mut vtable=vec![0usize;0x220/8];let vtable=vtable.as_mut_ptr()as usize;
 let mut rows=[vec![0u8;0x400],vec![0u8;0x400]];
 let mut nodes=[vec![0usize;0x80/8],vec![0usize;0x80/8]];
 put(player,vtable);put(player+0x1b8,modules);put(player+0x190,manager);put(modules,stats);put(modules+8,stats);
 put(damage,vtable);put(damage+8,player);put(manager+0x10,player);
 put(world+0x174e8,player);put(base+crate::WORLD_RVA,world);put(world+0x9910,stats);
 put(vtable+0x120,base+0x3d3e10);put(vtable+0x210,base+0x3d3e10);put(vtable+0x98,base+0x282970);
 for (rva,b) in [(0x3d3e10,&[0x31,0xc0,0xc3][..]),(0x519ef0,&[0xb0,1,0xc3]),(0x282970,&[0x48,0xc7,0x02,0xff,0xff,0xff,0xff,0x48,0x8b,0xc2,0xc3]),(0x63fe50,&[0x48,0xc7,0x01,0xff,0xff,0xff,0xff,0x48,0x8b,0xc1,0xc3]),(0x42c120,&[0x48,0x8b,0x41,8,0xc3]),(0x42c130,&[0x48,0x8b,0x41,8,0x48,0x8b,0x80,0x90,1,0,0,0xc3]),(0x43ced0,&[0xc3])]{bytes(base,rva,b);}
 jump(base,crate::HAS_EFFECT_RVA,has as*const()as usize);jump(base,crate::death_dance::DEBIT_RVA,debit as*const()as usize);
 for k in 0..2{let row=rows[k].as_mut_ptr()as usize;let node=nodes[k].as_mut_ptr()as usize;
  put(node,row);put(node+8,if k==0{99810i32}else{7000i32});put(node+0x68,4u32);put(node+0x70,usize::MAX);
  for off in [0x58,0x5c,0x60,0x64]{put(node+off,1f32);}
  put(row+0x9c,if k==0{2f32}else{0f32});put(row+0xa0,if k==0{15i32}else{10i32});put(row+0x120,1f32);put(row+0x304,1f32);
 }
 put(nodes[0].as_mut_ptr()as usize+0x38,nodes[1].as_ptr()as usize);put(manager+8,nodes[0].as_ptr()as usize);
 let mut prepared=Vec::new();
 for (rva,detour,original) in [(COLLECT_RVA,collect as*const u8,&COLLECT_ORIGINAL),(RATE_RVA,rate as*const u8,&RATE_ORIGINAL),(POINT_RVA,point as*const u8,&POINT_ORIGINAL)]{
  let p=crate::hook::prepare((base+rva)as*mut u8,detour,64).unwrap();original.store(p.trampoline,Ordering::Release);crate::hook::write_code((base+rva)as*mut u8,&p.patch).unwrap();prepared.push(p);
 }
 let old_base=BASE.swap(base,Ordering::AcqRel);let old_active=ACTIVE.swap(true,Ordering::AcqRel);
 let run:unsafe extern "system" fn(usize)=std::mem::transmute(base+0x431b30);
 let reset=|active:bool,due:bool,hp:i32|{*STATE.get_or_init(||Mutex::new(State::default())).lock().unwrap()=State{active,due,elapsed:30.,..State::default()};put(stats+0x140,hp);put(stats+0x144,1000i32);CALLS.lock().unwrap().clear();};
 reset(false,false,200);run(damage);assert_eq!(*CALLS.lock().unwrap(),vec![-45]);assert_eq!(read_i32(stats+0x140),155);
 reset(true,true,200);run(damage);assert_eq!(*CALLS.lock().unwrap(),vec![70,-80]);assert_eq!(read_i32(stats+0x140),190);
 // Pending tick consumed once, original non-rain damage remains every call.
 CALLS.lock().unwrap().clear();run(damage);assert_eq!(*CALLS.lock().unwrap(),vec![-10]);
 reset(true,true,800);run(damage);assert_eq!(*CALLS.lock().unwrap(),vec![70,-140]);assert_eq!(read_i32(stats+0x140),730);
 reset(true,true,990);run(damage);assert_eq!(*CALLS.lock().unwrap(),vec![70,-159]);assert_eq!(read_i32(stats+0x140),841);
 for kind in 0..3{reset(true,true,200);OWNED.store(kind!=0,Ordering::Relaxed);put(world+0x174e8,if kind==1{0usize}else{player});put(player+0x710,if kind==2{1u8}else{0u8});run(damage);assert_eq!(*CALLS.lock().unwrap(),vec![-45]);}
 OWNED.store(true,Ordering::Relaxed);put(world+0x174e8,player);put(player+0x710,0u8);
 for id in [99871,99872,99876,99877]{reset(true,true,200);RAIN.store(id,Ordering::Relaxed);put(nodes[0].as_mut_ptr()as usize+8,id);run(damage);assert_eq!(*CALLS.lock().unwrap(),vec![-45]);}
 RAIN.store(99810,Ordering::Relaxed);
 // The original PARAM rows and linked list are never changed.
 assert_eq!(read_i32(rows[0].as_ptr()as usize+0xa0),15);assert_eq!(read_ptr(nodes[0].as_ptr()as usize+0x38),nodes[1].as_ptr()as usize);
 OWNED.store(true,Ordering::Relaxed);BASE.store(old_base,Ordering::Release);ACTIVE.store(old_active,Ordering::Release);*STATE.get().unwrap().lock().unwrap()=State::default();
 for p in prepared{VirtualFree(p.trampoline as*mut _,0,MEM_RELEASE);if !p.relay.is_null(){VirtualFree(p.relay as*mut _,0,MEM_RELEASE);}}
 VirtualFree(memory,0,MEM_RELEASE);
}}
