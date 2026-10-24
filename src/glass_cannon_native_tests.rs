//! Compiled calculator/object hooks over the real native code. No game process
//! access. Gaze uses real parameter lookups with a fixture resource manager;
//! input vectors, loaded tables and scope/party identity are fixtures.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
use crate::devils_gaze_scaling::{self as gaze, ParamRef, Lookup};
use std::cell::Cell;
thread_local! {
 static TABLES:Cell<[usize;2]>=const{Cell::new([0;2])};
 static SHADOW:Cell<usize>=const{Cell::new(0)};
}
static EFFECT_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
unsafe extern "system" fn resource(manager:usize,kind:i32,zero:i32)->usize{
 assert_eq!((manager,zero),(1,0));
 TABLES.with(|t|t.get()[match kind{0xf4=>0,0xf=>1,_=>panic!("unexpected param type {kind}")}])
}
unsafe extern "system" fn overlay(out:*mut ParamRef,id:i32){
 let original:Lookup=std::mem::transmute(EFFECT_ORIGINAL.load(Ordering::Acquire));original(out,id);
 // A difficulty-mod-style immutable replacement, with native backing row intact.
 let shadow=SHADOW.with(Cell::get);if id==123456&&(*out).row!=0&&shadow!=0{(*out).row=shadow;}
}
struct Table{data:Vec<u64>,_inner:Box<[usize;17]>,outer:Box<[usize;17]>}
impl Table{
 unsafe fn new(id:i32,bytes:&[u8])->Self{Self::many(&[(id,bytes)])}
 unsafe fn many(rows:&[(i32,&[u8])])->Self{
  assert!(!rows.is_empty()&&rows.len()<=2&&rows.windows(2).all(|v|v[0].0<v[1].0));
  let mut data=vec![0u64;(0x120+rows.len()*0x400)/8];let p=data.as_mut_ptr()as usize+0x10;
  ptr::write_unaligned((p-0x10)as*mut i32,0x80);ptr::write_unaligned((p-0xc)as*mut i32,rows.len()as i32);
  ptr::write((p+0x2d)as*mut u8,3);
  for(i,(id,bytes))in rows.iter().enumerate(){
   assert!(bytes.len()<=0x400);
   ptr::write_unaligned((p+0x80+i*8)as*mut i32,*id);
   ptr::write_unaligned((p+0x84+i*8)as*mut i32,i as i32);
   ptr::write_unaligned((p+0x44+i*12)as*mut i32,(0x100+i*0x400)as i32);
   ptr::copy_nonoverlapping(bytes.as_ptr(),(p+0x100+i*0x400)as*mut u8,bytes.len());
  }
  let mut inner=Box::new([0usize;17]);inner[16]=p;
  let mut outer=Box::new([0usize;17]);outer[16]=inner.as_ptr()as usize;
  Self{data,_inner:inner,outer}
 }
 fn row(&mut self)->usize{self.row_at(0)}
 fn row_at(&mut self,index:usize)->usize{self.data.as_mut_ptr()as usize+0x110+index*0x400}
}
fn close(a:f32,b:f32){assert!((a-b).abs()<0.005,"{a} != {b}");}
unsafe fn install(base:usize,rva:usize,f:*const u8,original:&AtomicPtr<u8>){
 let p=crate::hook::prepare((base+rva)as*mut u8,f,64).unwrap();original.store(p.trampoline,Ordering::Release);crate::hook::write_code((base+rva)as*mut u8,&p.patch).unwrap();
}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; run alone"]
fn native_glass_cannon_retains_net_negative_resistance(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let h=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),h.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for s in &h.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 let old=ORIGINAL.load(Ordering::Acquire);let old_object=OBJECT_ORIGINAL.load(Ordering::Acquire);
 install(base,RVA,detour as*const u8,&ORIGINAL);install(base,OBJECT_RVA,object_detour as*const u8,&OBJECT_ORIGINAL);
 let call:Calculate=std::mem::transmute(base+RVA);
 let attack=[100f32,200.,300.,400.,500.];let rates=[1.2f32,1.5,2.,0.5,1.];let one=[1f32;5];
 let scalar=[0.25f32,0.5,0.25,2.,1.5,0.5,0.75];
 for bypass in [false,true]{for layer in 0..5{for cut in [0.2f32,0.5,2.]{
  let mut coefficients=[[1f32;5];5];coefficients[layer]=[cut;5];let saved=coefficients;let mut output=[0.;5];
  push(bypass);
  let total=call(output.as_mut_ptr(),attack.as_ptr(),rates.as_ptr(),coefficients[0].as_ptr(),coefficients[1].as_ptr(),coefficients[2].as_ptr(),coefficients[3].as_ptr(),coefficients[4].as_ptr(),scalar[0],scalar[1],scalar[2],scalar[3],scalar[4],scalar[5],scalar[6]);leave();
  // Layers: effects, model/equipment resistance, guard, two effect groups.
  let applied=if bypass{if layer==2{1.}else{cut.max(1.)}}else{cut};
  let factor=if bypass{0.5*2.*1.5*0.75}else{0.25*0.5*0.25*2.*1.5*0.5*0.75};
  let mut sum=0.;for i in 0..5{let expected=attack[i]*rates[i]*applied*factor;close(output[i],expected);sum+=expected;}close(total,sum);assert_eq!(coefficients,saved);
 }}}
 // Native output, not a duplicate of the hook's formula: mixed attributes
 // cancel model resistance against effect reductions/vulnerabilities first.
 let effects=[0.5f32,0.75,1.5,0.,1.];let armor=[0.5f32,1.2,2.,1.,2.];let guard=[3f32;5];
 let conditional=[1.5f32,2.,1.,4.,1.];let additional=[1f32,1.,0.5,2.,2.];let saved=(effects,armor,guard,conditional,additional);
 for _ in 0..2{
  let mut output=[0.;5];push(true);
  let total=call(output.as_mut_ptr(),one.map(|_|100.).as_ptr(),one.as_ptr(),effects.as_ptr(),armor.as_ptr(),guard.as_ptr(),conditional.as_ptr(),additional.as_ptr(),2.,1.,2.,1.,1.,2.,1.);leave();
  for(a,b)in output.into_iter().zip([100.,180.,150.,100.,400.]){close(a,b);}close(total,930.);assert_eq!((effects,armor,guard,conditional,additional),saved);
 }
 // The user's core examples: +40%, 0%, -20% intrinsic resistance. Also
 // preserve the old bypass of numerical immunity and shield coefficients.
 for(armor,expected)in [([0.6f32;5],100.),([1.;5],100.),([1.2;5],120.),([0.;5],100.)]{
  let mut output=[0.;5];let a=[100f32;5];push(true);
  let total=call(output.as_mut_ptr(),a.as_ptr(),one.as_ptr(),one.as_ptr(),armor.as_ptr(),[0f32;5].as_ptr(),one.as_ptr(),one.as_ptr(),1.,1.,1.,1.,1.,1.,1.);leave();
  for got in output{close(got,expected);}close(total,expected*5.);
 }
 // Actual native object-defense query: this remains defense bypass, even
 // when its own coefficient exceeds one; nested/other targets are unaffected.
 let mut manager=[0usize;2];let mut node=[0usize;0x70/8];let mut effect=[0u32;0x320/4];manager[1]=node.as_mut_ptr()as usize;node[0]=effect.as_mut_ptr()as usize;
 let query:unsafe extern "system" fn(usize)->f32=std::mem::transmute(base+OBJECT_RVA);let mp=manager.as_mut_ptr()as usize;
 for flags in [0usize,1,0x40000,0x80000000]{for value in [0f32,0.2,0.5,2.]{
  node[0x68/8]=flags;effect[0x318/4]=value.to_bits();close(query(mp),if flags==0{value}else{1.});
  SCOPES.with(|s|s.borrow_mut().push((true,mp,1.)));close(query(mp),1.);push(false);close(query(mp),if flags==0{value}else{1.});leave();close(query(mp),1.);leave();
 }}
 // Shared Devil's Gaze recipient scaling still retains all native defenses.
 for damage in [300f32,150.,0.,600.,420.,301.]{for scale in [1f32,4.]{for guard in [1f32,0.25]{
  let attack=[0f32,0.,0.,0.,damage];let defense=[0.5f32;5];let guards=[guard;5];let mut output=[0.;5];
  SCOPES.with(|s|s.borrow_mut().push((false,0,scale)));
  let total=call(output.as_mut_ptr(),attack.as_ptr(),one.as_ptr(),one.as_ptr(),defense.as_ptr(),guards.as_ptr(),one.as_ptr(),one.as_ptr(),1.,1.,1.,1.,1.,1.,1.);leave();
  close(total,damage*scale*0.5*guard);assert_eq!(&output[..4],&[0f32;4]);close(output[4],total);assert_eq!(attack[4],damage);
 }}}
 live_gaze_cases(base,call);
 ORIGINAL.store(old,Ordering::Release);OBJECT_ORIGINAL.store(old_object,Ordering::Release);assert!(!enabled());assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}

unsafe fn live_gaze_cases(base:usize,call:Calculate){
 // Execute both original lookup bodies; stub only the game's resource manager.
 let group=[123456i32;5];let bytes=std::slice::from_raw_parts(group.as_ptr()as*const u8,20);
 let correction=Table::new(7760,bytes);let mut effect=Table::many(&[(7760,&[0u8;0x400]),(123456,&[0u8;0x400])]);
 ptr::write_unaligned((effect.row()+0x1e0)as*mut f32,3.36);
 TABLES.with(|t|t.set([correction.outer.as_ptr()as usize,effect.outer.as_ptr()as usize]));
 ptr::write_unaligned((base+0x3c28880)as*mut usize,1);
 let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&(resource as*const ()as usize).to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);
 crate::hook::write_code((base+0xe9e680)as*mut u8,&jump).unwrap();
 let mut world=vec![0usize;0x17500/8];world[0x174e8/8]=11;
 let mut shadow=[0f32;0x400/4];
 let one=[1f32;5];let defense=[0.5f32;5];
 let vanilla=[1f32,1.3,1.547,1.841,2.54,2.947];
 // First exercise an unhooked EA17E0, then a chained hook which replaces its
 // output pointer. No assumptions about DLL load order or original row address.
 for hooked in [false,true]{
  if hooked{install(base,gaze::EFFECT_RVA,overlay as*const u8,&EFFECT_ORIGINAL);}
  for depth in 0usize..=12{
   // 11: restore vanilla; 12: live zero override. 6..10: DeeperNight values.
   if !hooked&&depth>5{continue;}
   let encoded=depth.min(5);let live=match depth{0..=5=>vanilla[depth],6..=10=>vanilla[5]+vanilla[depth-5]-1.,11=>vanilla[5],_=>0.};
   ptr::write_unaligned((effect.row_at(1)+0x1e0)as*mut f32,vanilla[encoded]);shadow[0x1e0/4]=live;
   SHADOW.with(|s|s.set(if hooked{shadow.as_ptr()as usize}else{0}));
   let original_table=effect.data.clone();let original_shadow=shadow;
   let mut hit=[0u8;0x48];hit[0x40..0x44].copy_from_slice(&(95002200+encoded as i32).to_le_bytes());
   for player in [false,true]{for guard in [1f32,0.25]{
    ptr::write_unaligned((base+crate::WORLD_RVA)as*mut usize,if player{world.as_ptr()as usize}else{0});
    let scale=crate::devils_gaze_hit::damage_multiplier(11,hit.as_ptr()as usize,base);
    let expected_scale=if player{1.}else{0.25}*3.36*if encoded==0{1.}else{live};close(scale,expected_scale);
    let attack=[0f32,0.,0.,0.,300.];let guards=[guard;5];let mut output=[0.;5];
    SCOPES.with(|s|s.borrow_mut().push((false,0,scale)));
    for _ in 0..2{
     let total=call(output.as_mut_ptr(),attack.as_ptr(),one.as_ptr(),one.as_ptr(),defense.as_ptr(),guards.as_ptr(),one.as_ptr(),one.as_ptr(),1.,1.,1.,1.,1.,1.,1.);
     close(total,300.*expected_scale*0.5*guard);assert_eq!(&output[..4],&[0f32;4]);close(output[4],total);
     push(false);assert_eq!(SCOPES.with(|s|s.borrow().last().unwrap().2),1.);leave();
    }
    leave();assert_eq!(attack[4],300.);
   }}
   assert_eq!(effect.data,original_table);assert_eq!(shadow,original_shadow);
  }
 }
 // Base correction also remains live in ordinary mode; zero and invalid
 // overrides follow the same rules as depth corrections.
 SHADOW.with(|s|s.set(0));
 for rate in [5f32,0.,f32::NAN]{
  ptr::write_unaligned((effect.row()+0x1e0)as*mut f32,rate);
  close(gaze::multiplier(0,base),if rate.is_nan(){3.36}else{rate}*0.25);
  close(gaze::multiplier(5,base),if rate.is_nan(){3.36}else{rate}*2.947*0.25);
 }
 // Missing current parameters fall back to vanilla coefficients, even with
 // the replacement hook installed. Unrelated attacks never query parameters.
 TABLES.with(|t|t.set([0;2]));close(gaze::multiplier(5,base),3.36*2.947*0.25);
 SHADOW.with(|s|s.set(0));EFFECT_ORIGINAL.store(ptr::null_mut(),Ordering::Release);
}
