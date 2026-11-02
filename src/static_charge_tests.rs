use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn float(p:usize,v:f32){ptr::write_unaligned(p as*mut f32,v);}
unsafe extern "system" fn accepted(_:usize,_:usize,_:usize)->u8{assert!(active_contact());1}
unsafe extern "system" fn rejected(_:usize,_:usize,_:usize)->u8{0}
thread_local!{static ROW:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};}
unsafe extern "system" fn lookup(out:usize,kind:i32,id:i32)->usize{assert_eq!((kind,id),(1,ATTACK));ptr::write_unaligned((out+0x10)as*mut usize,ROW.with(std::cell::Cell::get));out}
thread_local!{static TEAM:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};}
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from(id==OWNER)}
unsafe extern "system" fn team(_:usize,_:i32)->usize{TEAM.with(std::cell::Cell::get)}
unsafe extern "system" fn common(out:usize)->usize{ptr::write_unaligned(out as*mut usize,0);out}
unsafe extern "system" fn mode()->u8{0}
unsafe fn patch(base:usize,rva:usize,target:usize){let mut code=vec![0x48,0xb8];code.extend_from_slice(&target.to_le_bytes());code.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(code.as_ptr(),(base+rva)as*mut u8,code.len());}
#[test]#[ignore="requires NRHEX_NATIVE_EXE; run alone"]
fn native_static_snapshot_binding_damage_and_hit_dedup(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let h=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),h.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for s in &h.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 let old_active=ACTIVE.swap(true,Ordering::AcqRel);
 let old_calc=crate::glass_cannon::ORIGINAL.swap((base+crate::glass_cannon::RVA)as*mut u8,Ordering::AcqRel);
 // Run the real descriptor -> payload coefficient copy; other parser work is
 // not needed here. The native packet builder copies this same four-float
 // vector verbatim at 33E1A3, before the networking subsystem serializes it.
 ptr::write((base+0x340599)as*mut u8,0xc3);
 let copy:unsafe extern "system" fn(usize,usize)=std::mem::transmute(base+0x34058e);
 let mut descriptor=[0u8;0x110];let dp=descriptor.as_mut_ptr()as usize;
 let mut payload=[0u8;0x220];let pp=payload.as_mut_ptr()as usize;
 int(pp+4,BULLET);ptr::write_unaligned((dp+0x30)as*mut[f32;4],[777.25,1.,1.,1.]);copy(pp,dp);
 assert_eq!(ptr::read_unaligned((pp+0x124)as*const[f32;4]),[777.25,1.,1.,1.]);
 let mut group=[99i32;2];let gp=group.as_mut_ptr()as usize;
 let mut bullet=[0usize;0x300/8];let bp=bullet.as_mut_ptr()as usize;int(bp+0xa4,BULLET);ptr::write_unaligned((bp+8)as*mut u64,55);
 {
  let _guard=factory(pp);assert_eq!(ptr::read_unaligned((pp+0x124)as*const[f32;4]),[1.;4]);
  factory_done(gp);let _init=initialize(gp);bound(bp);
 }
 assert_eq!(ptr::read_unaligned((pp+0x124)as*const[f32;4]),[777.25,1.,1.,1.]);
 let mut child_bullet=[0usize;0x300/8];let cb=child_bullet.as_mut_ptr()as usize;int(cb+0xa4,BULLET+1);ptr::write_unaligned((cb+8)as*mut u64,56);
 {let _child=child(bp,BULLET+1);int(pp+4,BULLET+1);let _f=factory(pp);factory_done(gp);let _i=initialize(gp);bound(cb);}
 assert!(Arc::ptr_eq(&seed(bp).unwrap(),&seed(cb).unwrap()));
 let mut hit=[0u8;0x300];let hp=hit.as_mut_ptr()as usize;int(hp+0x40,ATTACK);int(hp+0x44,1);let mut context=[0u8;0x120];let cp=context.as_mut_ptr()as usize;ptr::write_unaligned(cp as*mut usize,bp);int(cp+8,ATTACK);int(cp+12,1);
 crate::ability_damage::before_construct(cp);assert_eq!(context[0xf8],1);
 let mut row=[0u8;0x200];let rp=row.as_mut_ptr()as usize;float(rp+0x84,30.);float(rp+0x1c0,30.);ROW.with(|s|s.set(rp));patch(base,0xe6d630,lookup as*const()as usize);
 let poise:unsafe extern "system" fn(usize,i32,usize,f32,usize,usize)->f32=std::mem::transmute(base+0x6ce2f0);
 let rescue:unsafe extern "system" fn(usize)->f32=std::mem::transmute(base+0x6cd640);
 assert_eq!(poise(0,0,0,999.,cp,0),30.);assert_eq!(rescue(cp),30.);
 // Native source/recipient modifiers to poise/rescue are left intact.
 float(hp+0x148,45.);float(hp+0x150,30.);constructed(hp,cp);
 assert_eq!(ptr::read_unaligned(hp as*const[f32;5]),[0.,0.,0.,777.25,0.]);
 assert_eq!(ptr::read_unaligned((hp+0x148)as*const f32),45.);assert_eq!(ptr::read_unaligned((hp+0x150)as*const f32),30.);
 // Deliberately supply a second attack bonus and unrelated components. Only
 // the immutable lightning snapshot and NEW recipient defenses survive.
 let one=[1.;5];let inflated=[9999.;5];let twice=[2.;5];let cuts=[0.5;5];let mut out=[0.;5];
 CALC.with(|s|s.borrow_mut().push(Calculation{lightning:Some(800.),object:0.5,..Default::default()}));
 let result=crate::glass_cannon::detour(out.as_mut_ptr(),inflated.as_ptr(),twice.as_ptr(),cuts.as_ptr(),one.as_ptr(),cuts.as_ptr(),one.as_ptr(),one.as_ptr(),1.,2.,1.,2.,2.,1.,2.);
 assert_eq!(result,100.);assert_eq!(out,[0.,0.,0.,100.,0.]);leave();
 // Real neutral native combiner keeps the captured offense and removes cuts.
 let k=Key{source:1,target:2,attack:3,table:1,components:[0;5],point:[0;3]};
 CALC.with(|s|s.borrow_mut().push(Calculation{key:Some(k),hit:hp,delta:[50.,0.,0.,0.,0.],..Default::default()}));
 let a=[50.,20.,30.,40.,10.];
 crate::glass_cannon::detour(out.as_mut_ptr(),a.as_ptr(),twice.as_ptr(),cuts.as_ptr(),cuts.as_ptr(),cuts.as_ptr(),cuts.as_ptr(),cuts.as_ptr(),0.5,1.,0.5,1.,1.,0.5,1.);
 assert_eq!(SAMPLES.lock().unwrap().get(&hp).unwrap().value,400.);leave();
 // Rejected collision can be retried; every accepted target is shared across
 // all stages. A fresh release can hit the same generation-bearing handle.
 let target=[0u64,100];let tp=target.as_ptr()as usize;
 assert_eq!(contact(rejected,hp,tp,bp),0);assert_eq!(contact(accepted,hp,tp,cb),1);assert_eq!(contact(accepted,hp,tp,bp),0);assert_eq!(contact(accepted,hp,tp,cb),0);
 int(pp+4,BULLET);float(pp+0x124,123.);
 {let _f=factory(pp);factory_done(gp);let _i=initialize(gp);bound(bp);}
 assert!(!Arc::ptr_eq(&seed(bp).unwrap(),&seed(cb).unwrap()));assert_eq!(contact(accepted,hp,tp,bp),1);
 retired(bp);assert!(seed(bp).is_none());retired(cb);finished(gp);
 // Exercise the actual outgoing callback with a downed registered teammate
 // and zero HP damage. Native rescue coefficients, not HP loss, decide it.
 for(rva,target)in [(crate::HAS_EFFECT_RVA,has as*const()as usize),(0x5197e0,team as*const()as usize),(crate::ambulance::COMMON_RVA,common as*const()as usize),(crate::ambulance::MODE_RVA,mode as*const()as usize)]{patch(base,rva,target);}
 let old_base=BASE.swap(base,Ordering::AcqRel);
 let mut source=[0usize;0x740/8];let sp=source.as_mut_ptr()as usize;source[1]=123;source[0x190/8]=1;
 let mut ally=[0usize;0x740/8];let ap=ally.as_mut_ptr()as usize;ally[1]=456;ptr::write((ap+0x710)as*mut u8,1);
 let mut world=vec![0usize;0x17500/8];world[0x174e8/8]=sp;ptr::write_unaligned((base+crate::WORLD_RVA)as*mut usize,world.as_ptr()as usize);TEAM.with(|s|s.set(ap));
 let module=[0usize,ap];let mp=module.as_ptr()as usize;
 int(hp+0x40,100);int(hp+0x44,1);int(hp+0x280,0);float(hp+0x150,30.);float(hp+0x258,1.);
 SAMPLES.lock().unwrap().insert(hp,Sample{key:key(sp,ap,hp),value:500.});STATE.lock().unwrap().meter.distance=30.;
 outgoing(mp,sp,hp);outgoing(mp,sp,hp);assert_eq!(STATE.lock().unwrap().pending.len(),1);assert_eq!(STATE.lock().unwrap().pending[0].damage,500.);
 STATE.lock().unwrap().pending.clear();STATE.lock().unwrap().meter.distance=30.;
 int(hp+0x40,ATTACK);outgoing(mp,sp,hp);assert_eq!(STATE.lock().unwrap().meter.distance,30.);
 int(hp+0x40,100);ptr::write((ap+0x710)as*mut u8,0);outgoing(mp,sp,hp);assert_eq!(STATE.lock().unwrap().meter.distance,30.);
 reset();BASE.store(old_base,Ordering::Release);
 SAMPLES.lock().unwrap().clear();ACTIVE.store(old_active,Ordering::Release);crate::glass_cannon::ORIGINAL.store(old_calc,Ordering::Release);VirtualFree(memory,0,MEM_RELEASE);
}}
