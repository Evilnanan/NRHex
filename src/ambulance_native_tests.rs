//! Opt-in integration: run actual native periodic summation, rescue calculator,
//! original packet decoder and stamina clamp with the compiled detours installed.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
use std::sync::atomic::{AtomicBool,AtomicUsize};
static HAS:AtomicBool=AtomicBool::new(true);
static SELF:AtomicUsize=AtomicUsize::new(0);
static ALLY:AtomicUsize=AtomicUsize::new(0);
static OTHER:AtomicUsize=AtomicUsize::new(0);
unsafe fn integer(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn pointer(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn float(p:usize,v:f32){ptr::write_unaligned(p as*mut f32,v);}
unsafe fn bytes(base:usize,rva:usize,v:&[u8]){ptr::copy_nonoverlapping(v.as_ptr(),(base+rva)as*mut u8,v.len());}
unsafe fn jump(base:usize,rva:usize,f:usize){let mut b=[0x48,0xb8,0,0,0,0,0,0,0,0,0xff,0xe0];b[2..10].copy_from_slice(&f.to_le_bytes());bytes(base,rva,&b);}
unsafe extern "system" fn has(_:usize,effect:i32)->u8{u8::from(effect==OWNER&&HAS.load(Ordering::Relaxed))}
unsafe extern "system" fn party(_:usize,slot:i32)->usize{match slot{1=>ALLY.load(Ordering::Relaxed),2=>OTHER.load(Ordering::Relaxed),_=>0}}
unsafe extern "system" fn resolve(_:usize,id:usize)->usize{match ptr::read_unaligned(id as*const u64){1=>SELF.load(Ordering::Relaxed),2=>ALLY.load(Ordering::Relaxed),3=>OTHER.load(Ordering::Relaxed),_=>0}}
unsafe extern "system" fn rotation(actor:usize,out:*mut f32)->*mut f32{ptr::copy_nonoverlapping((actor+0x120)as*const f32,out,4);out}
unsafe extern "system" fn number(vm:usize,index:i32)->f32{ptr::read_unaligned((vm+(index as usize-2)*4)as*const f32)}
unsafe extern "system" fn state(actor:usize,value:u8,_time:f32){ptr::write((actor+0x710)as*mut u8,value);}
unsafe extern "system" fn ceil(value:f32)->f32{value.ceil()}
unsafe extern "system" fn yes(_:usize)->u8{1}
unsafe extern "system" fn no(_:usize)->u8{0}
unsafe extern "system" fn empty(reference:usize)->usize{pointer(reference,0);reference}
unsafe fn actor(base:usize,handle:usize,position:[f32;4])->(Vec<u64>,Vec<usize>,Vec<u64>,Vec<u64>,Vec<usize>){
 let mut a=vec![0u64;0x780/8];let mut m=vec![0usize;0x200/8];let mut stats=vec![0u64;0x200/8];let mut physics=vec![0u64;0x100/8];let mut vt=vec![0usize;0x130/8];
 vt[0x118/8]=yes as*const()as usize;vt[0x120/8]=no as*const()as usize;
 let p=a.as_mut_ptr()as usize;pointer(p,vt.as_ptr()as usize);pointer(p+8,handle);pointer(p+0x190,p);pointer(p+0x60,p);pointer(p+0x1b8,m.as_ptr()as usize);
 m[0]=stats.as_mut_ptr()as usize;m[0x68/8]=physics.as_mut_ptr()as usize;integer(stats.as_ptr()as usize+0x140,100);integer(stats.as_ptr()as usize+0x160,100);
 ptr::write_unaligned((physics.as_ptr()as usize+0x70)as*mut[f32;4],position);float(p+0x124,0.);let _=base;
 (a,m,stats,physics,vt)
}
unsafe fn install(base:usize,rva:usize,f:*const u8,original:&AtomicPtr<u8>){
 let p=crate::hook::prepare((base+rva)as*mut u8,f,64).unwrap();original.store(p.trampoline,Ordering::Release);crate::hook::write_code((base+rva)as*mut u8,&p.patch).unwrap();
}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; run alone"]
fn ambulance_compiled_detours_execute_native_cost_rescue_and_packets(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let h=crate::pe::Headers::parse(&raw,0).unwrap();let allocation=VirtualAlloc(ptr::null_mut(),h.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!allocation.is_null());let base=allocation as usize;
 for s in &h.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 let mut world=vec![0u64;0x17500/8];let w=world.as_mut_ptr()as usize;pointer(base+crate::WORLD_RVA,w);
 let(mut a,_am,_as,_ap,_av)=actor(base,1,[0.;4]);let(mut b,_bm,_bs,mut bp,_bv)=actor(base,2,[0.,0.,-10.,1.]);let(mut c,_cm,_cs,_cp,_cv)=actor(base,3,[20.,0.,10.,1.]);
 let p=a.as_mut_ptr()as usize;let t=b.as_mut_ptr()as usize;let o=c.as_mut_ptr()as usize;SELF.store(p,Ordering::Relaxed);ALLY.store(t,Ordering::Relaxed);OTHER.store(o,Ordering::Relaxed);pointer(w+0x174e8,p);ptr::write((t+0x710)as*mut u8,1);
 let stats=native_actor::stats(p);integer(stats+0x15c,0);
 for(rva,f)in [(crate::HAS_EFFECT_RVA,has as*const()as usize),(crate::CHR_ROTATION_RVA,rotation as*const()as usize),(0x5197e0,party as*const()as usize),(0x519140,resolve as*const()as usize),(REMOTE_RVA,resolve as*const()as usize),(NUMBER_RVA,number as*const()as usize),(0x6aeb20,no as*const()as usize),(0xe76d50,empty as*const()as usize),(0xe765d0,empty as*const()as usize),(0x217e710,ceil as*const()as usize),(0x3d42a0,no as*const()as usize),(0x3d4180,no as*const()as usize),(0x66f3b0,state as*const()as usize)]{jump(base,rva,f);}
 for rva in [0x4b3510,0x4e78c0]{bytes(base,rva,&[0xc3]);}jump(base,0x4e78a0,empty as*const()as usize);
 install(base,ENV_RVA,env as*const u8,&ENV_ORIGINAL);install(base,COST_RVA,cost as*const u8,&COST_ORIGINAL);install(base,RESCUE_RVA,rescue as*const u8,&RESCUE_ORIGINAL);install(base,RECEIVE_RVA,receive as*const u8,&RECEIVE_ORIGINAL);install(base,CREDIT_RVA,credit as*const u8,&CREDIT_ORIGINAL);
 let old_base=BASE.swap(base,Ordering::AcqRel);let old_active=ACTIVE.swap(true,Ordering::AcqRel);*STATE.lock().unwrap()=State::default();
 let query:Env=std::mem::transmute(base+ENV_RVA);let context=[p];let mut vm=[0f32,2.];
 assert_eq!(query(context.as_ptr()as usize,QUERY,vm.as_ptr()as usize),1);
 // Sideways and behind fail; any one of multiple downed teammates can qualify.
 for angle in [31f32,90.,180.]{vm[0]=angle;assert_eq!(query(context.as_ptr()as usize,QUERY,vm.as_ptr()as usize),0);}
 vm[0]=0.;vm[1]=0.;assert_eq!(query(context.as_ptr()as usize,QUERY,vm.as_ptr()as usize),0);vm[1]=2.;
 ptr::write((t+0x710)as*mut u8,2);assert_eq!(query(context.as_ptr()as usize,QUERY,vm.as_ptr()as usize),0);ptr::write((t+0x710)as*mut u8,1);
 // Native summation versus the compiled filter: only these two locomotion
 // nodes disappear. Ordinary periodic costs, including an unused native row,
 // are preserved. Attack/roll/HKS1001 and magic consumers are not intercepted.
 let mut manager=[0usize;3];let mut nodes=[[0usize;0x70/8];4];let mut rows=[[0u32;0xb8/4];4];
 for i in 0..4{nodes[i][0]=rows[i].as_mut_ptr()as usize;nodes[i][1]=[100300,102022,100320,123][i];nodes[i][0x68/8]=4;rows[i][0xb4/4]=[1,1,1,7][i];if i<3{nodes[i][0x38/8]=nodes[i+1].as_ptr()as usize;}}
 manager[1]=nodes[0].as_ptr()as usize;manager[2]=p;pointer(p+0x190,manager.as_ptr()as usize);
 let sum:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(base+COST_RVA);let mut out=[0f32;2];let mp=manager.as_ptr()as usize;
 let call_sum=|out:&mut[f32;2]|sum(mp,out.as_mut_ptr()as usize);
 query(context.as_ptr()as usize,QUERY,vm.as_ptr()as usize);call_sum(&mut out);assert_eq!(out,[0.,8.]);
 HAS.store(false,Ordering::Relaxed);call_sum(&mut out);assert_eq!(out,[0.,10.]);HAS.store(true,Ordering::Relaxed);
 vm[0]=90.;query(context.as_ptr()as usize,QUERY,vm.as_ptr()as usize);call_sum(&mut out);assert_eq!(out,[0.,10.]);vm[0]=0.;
 bp[0x70/8]=100f32.to_bits()as u64;query(context.as_ptr()as usize,QUERY,vm.as_ptr()as usize);call_sum(&mut out);assert_eq!(out,[0.,10.]);bp[0x70/8]=0;
 query(context.as_ptr()as usize,QUERY,vm.as_ptr()as usize);update(p,base);update(p,base);call_sum(&mut out);assert_eq!(out,[0.,10.]);
 // Native actual rescue, including native per-hit correction and overkill cap.
 let apply:unsafe extern "system" fn(usize,usize,usize)=std::mem::transmute(base+RESCUE_RVA);let mut hit=vec![0u32;0x270/4];let hp=hit.as_mut_ptr()as usize;
 float(hp+0x150,40.);float(hp+0x258,0.5);float(t+0x714,100.);apply(t,p,hp);assert_eq!(ptr::read_unaligned((t+0x714)as*const f32),80.);assert_eq!(read_i32(stats+0x15c),10);
 float(hp+0x150,1000.);float(hp+0x258,1.);float(t+0x714,12.);apply(t,p,hp);assert_eq!(read_i32(stats+0x15c),16);
 apply(t,p,hp);assert_eq!(read_i32(stats+0x15c),16);
 float(t+0x714,20.);apply(t,o,hp);assert_eq!(read_i32(stats+0x15c),16);
 HAS.store(false,Ordering::Relaxed);float(t+0x714,20.);apply(t,p,hp);assert_eq!(read_i32(stats+0x15c),16);HAS.store(true,Ordering::Relaxed);
 // Execute native type-3 receive decoder. Both target ID formats are tested;
 // a duplicate snapshot has zero delta, and another rescuer receives nothing.
 pointer(base+0x3c0f0a8,w); // native WorldChrMan assertions at packet resolution
 let recv:unsafe extern "system" fn(usize,usize,usize,u8)->u8=std::mem::transmute(base+RECEIVE_RVA);
 let send=|remaining:i32,source:u64,kind:u8,local_format:u8|{
  let mut packet=Vec::new();if local_format==0{packet.extend_from_slice(&2u64.to_le_bytes());}
  packet.push(kind);packet.extend_from_slice(&1f32.to_le_bytes());packet.extend_from_slice(&remaining.to_le_bytes());packet.push(1);packet.extend_from_slice(&source.to_le_bytes());
  let mut stream=[0usize;2];stream[0]=packet.as_ptr()as usize;integer(stream.as_mut_ptr()as usize+8,packet.len()as i32);let id=2u64;
  let result=recv(0,stream.as_mut_ptr()as usize,&id as*const u64 as usize,local_format);assert_eq!(result,1);assert_eq!(read_i32(stream.as_ptr()as usize+8),0);
 };
 float(t+0x714,100.);send(80,1,3,0);assert_eq!(read_i32(stats+0x15c),26);send(80,1,3,0);assert_eq!(read_i32(stats+0x15c),26);
 send(60,3,3,0);assert_eq!(read_i32(stats+0x15c),26);send(40,1,3,1);assert_eq!(read_i32(stats+0x15c),36);
 send(20,1,1,0);assert_eq!(read_i32(stats+0x15c),36);send(0,1,3,0);assert_eq!(read_i32(stats+0x15c),46);
 // Odd rescues retain their half point; full stamina discards excess credit.
 integer(stats+0x15c,0);*STATE.lock().unwrap()=State::default();float(t+0x714,2.);float(hp+0x150,1.);apply(t,p,hp);assert_eq!(read_i32(stats+0x15c),0);apply(t,p,hp);assert_eq!(read_i32(stats+0x15c),1);
 integer(stats+0x15c,99);float(t+0x714,100.);float(hp+0x150,100.);apply(t,p,hp);assert_eq!(read_i32(stats+0x15c),100);assert_eq!(STATE.lock().unwrap().fraction,0.);
 BASE.store(old_base,Ordering::Release);ACTIVE.store(old_active,Ordering::Release);*STATE.lock().unwrap()=State::default();assert_ne!(VirtualFree(allocation,0,MEM_RELEASE),0);
}}
