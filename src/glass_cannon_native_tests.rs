//! Compiled calculator/object hooks over the real native code. No game process
//! or engine dependency stubs; input vectors and scope identity are fixtures.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
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
 for damage in [252f32,328.,390.,464.,640.,743.]{for scale in [1f32,4.]{for guard in [1f32,0.25]{
  let attack=[0f32,0.,0.,0.,damage];let defense=[0.5f32;5];let guards=[guard;5];let mut output=[0.;5];
  SCOPES.with(|s|s.borrow_mut().push((false,0,scale)));
  let total=call(output.as_mut_ptr(),attack.as_ptr(),one.as_ptr(),one.as_ptr(),defense.as_ptr(),guards.as_ptr(),one.as_ptr(),one.as_ptr(),1.,1.,1.,1.,1.,1.,1.);leave();
  close(total,damage*scale*0.5*guard);assert_eq!(&output[..4],&[0f32;4]);close(output[4],total);assert_eq!(attack[4],damage);
 }}}
 ORIGINAL.store(old,Ordering::Release);OBJECT_ORIGINAL.store(old_object,Ordering::Release);assert!(!enabled());assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
