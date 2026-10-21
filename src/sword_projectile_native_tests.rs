//! Execute the native instance update, movement, state transition, group unlink
//! and reset with the compiled hooks. Engine allocation/rendering are fixtures.
use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
use std::sync::atomic::{AtomicUsize,AtomicBool};
static SOURCE:AtomicUsize=AtomicUsize::new(0);
static ROW:AtomicUsize=AtomicUsize::new(0);
static OWNED:AtomicBool=AtomicBool::new(true);
static EXITS:AtomicUsize=AtomicUsize::new(0);
static CHILDREN:AtomicUsize=AtomicUsize::new(0);
static COLLISIONS:Mutex<Vec<i32>>=Mutex::new(Vec::new());
static INHERITED:Mutex<Option<Flight>>=Mutex::new(None);
unsafe fn put(p:usize,v:usize){ptr::write_unaligned(p as*mut usize,v);}
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn float(p:usize,v:f32){ptr::write_unaligned(p as*mut f32,v);}
unsafe fn jump(base:usize,rva:usize,f:usize){let mut code=vec![0x48,0xb8];code.extend_from_slice(&f.to_le_bytes());code.extend_from_slice(&[0xff,0xe0]);crate::hook::write_code((base+rva)as*mut u8,&code).unwrap();}
unsafe fn install(base:usize,rva:usize,f:*const u8,original:&AtomicPtr<u8>){let p=crate::hook::prepare((base+rva)as*mut u8,f,64).unwrap();original.store(p.trampoline,Ordering::Release);crate::hook::write_code((base+rva)as*mut u8,&p.patch).unwrap();}
unsafe extern "system" fn resolve(handle:u64)->usize{if handle==11{SOURCE.load(Ordering::Relaxed)}else{0}}
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from(id==sword::OWNER&&OWNED.load(Ordering::Relaxed))}
unsafe extern "system" fn lookup(out:usize,_id:i32)->usize{put(out,ROW.load(Ordering::Relaxed));out}
unsafe extern "system" fn initialize(ins:usize,_payload:usize,_target:usize,transform:usize,mode:i32,a:i32,b:u32){
 assert_eq!((a,b),(7,9));assert!(matches!(mode,1|2));ptr::write_unaligned((ins+0x10)as*mut Point,launch(transform));
}
unsafe extern "system" fn yes(_:usize,_dt:f32)->u8{1}
unsafe extern "system" fn running(_:usize,_ins:usize,_dt:f32)->u8{0}
unsafe extern "system" fn exit(_:usize,_ins:usize){EXITS.fetch_add(1,Ordering::Relaxed);}
unsafe extern "system" fn descendant(ins:usize,_p:usize,_q:usize,_id:i32,_mode:u8)->u8{
 let _factory=factory_scope(ins+0xa0);*INHERITED.lock().unwrap()=FACTORY.with(Cell::get);CHILDREN.fetch_add(1,Ordering::Relaxed);1
}
unsafe extern "system" fn remove_collision(_:usize,id:i32){COLLISIONS.lock().unwrap().push(id);}
#[repr(C,align(16))]struct Ins([u8;0x630]);
#[repr(C,align(16))]struct Matrix([f32;12]);
fn matrix(p:Point)->Matrix{Matrix([1.,0.,0.,p[0],0.,1.,0.,p[1],0.,0.,1.,p[2]])}
#[test]fn exact_spell_root_catalog_includes_native_sentinels_and_rejects_children(){
 assert!(root(4021,10402100));assert!(root(-1,10402150));assert!(!root(4021,10402151));assert!(!root(4000,10402100));assert!(!root(-2,10402100));
 assert!(crate::sword_map::ROOTS.len()>crate::echo_cast_data::ROOTS.len());
 for &(bullet,magics)in crate::sword_map::ROOTS{assert!(root(-1,bullet));for &magic in magics{assert!(root(magic,bullet));}}
}
#[test]fn radial_distance_does_not_count_travelled_path_or_moving_source(){
 let flight=Flight{origin:Some([100.,200.,300.,1.])};
 for p in [[103.,204.,300.,1.],[100.,200.,295.,1.],[97.,196.,300.,1.]]{assert!(!outside(flight,p));}
 assert!(outside(flight,[103.001,204.,300.,1.]));assert!(outside(flight,[100.,205.001,300.,1.]));
 assert!(!outside(Flight::default(),[1000.;4]));
}
#[test]
#[ignore="requires NRHEX_NATIVE_EXE; run alone"]
fn native_sword_projectile_fixed_launch_retirement_and_inheritance(){unsafe{
 let raw=std::fs::read(std::env::var_os("NRHEX_NATIVE_EXE").unwrap()).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&raw)),crate::EXE_HASH);
 let h=crate::pe::Headers::parse(&raw,0).unwrap();let memory=VirtualAlloc(ptr::null_mut(),h.image_size as usize,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);assert!(!memory.is_null());let base=memory as usize;
 for s in &h.sections{ptr::copy_nonoverlapping(raw[s.raw_offset as usize..].as_ptr(),(base+s.virtual_address as usize)as*mut u8,s.raw_size as usize);}
 let old_base=BASE.swap(base,Ordering::AcqRel);let old_active=ACTIVE.swap(true,Ordering::AcqRel);
 let mut source=vec![0usize;0x200/8];source[1]=11;source[0x190/8]=1;SOURCE.store(source.as_mut_ptr()as usize,Ordering::Relaxed);
 let mut source_modules=[0usize;0x80/8];let mut source_physics=[0f32;0x80/4];
 source[0x60/8]=1;source[0x1b8/8]=source_modules.as_mut_ptr()as usize;source_modules[0x68/8]=source_physics.as_mut_ptr()as usize;
 let mut row=[0u8;0x110];row[0xf8]=2;let rowp=row.as_mut_ptr()as usize;ROW.store(rowp,Ordering::Relaxed);
 for(rva,f)in [(crate::CHR_HANDLE_RVA,resolve as*const()as usize),(crate::HAS_EFFECT_RVA,has as*const()as usize),(0xe87800,lookup as*const()as usize),(0x354af0,initialize as*const()as usize),(CHILD_RVA,descendant as*const()as usize),(0x539ac0,remove_collision as*const()as usize)]{jump(base,rva,f);}
 // Engine-only services. Native free-flight math, state 4, group removal,
 // handle generation/active flags and both collision-unregister calls run.
 for rva in [0x26909a0,0x355d20,0x355fe0,0x350410,0x34b750,0x33e770,0x33e570,0x353cb0,0x353bb0,0x362c00,0x3482e0,0x3482b0]{crate::hook::write_code((base+rva)as*mut u8,&[0xc3]).unwrap();}
 for rva in [0x341250,0x349c80]{crate::hook::write_code((base+rva)as*mut u8,&[0x48,0x8b,0xc1,0xc3]).unwrap();}
 crate::hook::write_code((base+0x34da00)as*mut u8,&[0x31,0xc0,0xc3]).unwrap();
 put(base+0x3c0f4c0,1); // collision manager only passed to fixture unregister
 install(base,BIND_RVA,bind as*const u8,&BIND_ORIGINAL);install(base,RESET_RVA,reset as*const u8,&RESET_ORIGINAL);
 install(base,CHILD_RVA,child as*const u8,&CHILD_ORIGINAL);install(base,STEP_RVA,step as*const u8,&STEP_ORIGINAL);STOP.store((base+STOP_RVA)as*mut u8,Ordering::Release);
 let mut vt=[0usize;6];vt[2]=running as*const()as usize;vt[3]=yes as*const()as usize;vt[4]=exit as*const()as usize;
 let stages=[1i32,2,3,4,4];
 let mut ins=Ins([0;0x630]);let ip=ins.0.as_mut_ptr()as usize;
 let mut group=vec![0u64;0x5a0/8];let gp=group.as_mut_ptr()as usize;int(gp,0);
 let mut payload=[0u64;0x230/8];let pp=payload.as_mut_ptr()as usize;int(pp,-1);int(pp+4,10402100);put(pp+0x10,11);int(pp+0x24,4021);
 let origin=[10.,20.,30.,1.];let pose=matrix(origin);let posep=pose.0.as_ptr()as usize;
 let mut reset_fixture=|speed:Point|{
  ptr::write_bytes(ip as*mut u8,0,0x630);put(ip+8,0x12340000);put(ip+0x88,stages.as_ptr()as usize);int(ip+0x90,1);int(ip+0x94,2);int(ip+0x54,0);
  put(ip+0x5c0,vt.as_ptr()as usize);put(ip+0x5c8,rowp);put(ip+0x598,rowp);
  ptr::write_unaligned((ip+0x30)as*mut Point,speed);float(ip+0x2d4,100.);ptr::write((ip+0x580)as*mut u8,0x21);int(ip+0x60,77);int(ip+0x64,88);
  group[1]=0x12340000;int(gp+0x320,1);
 };
 let initialize_native:unsafe extern "system" fn(usize,usize,usize,usize,i32,u32)=std::mem::transmute(base+BIND_RVA);
 let update:unsafe extern "system" fn(usize,f32)->u8=std::mem::transmute(base+0x34f130);
 let cleanup:unsafe extern "system" fn(usize,usize)=std::mem::transmute(base+0x35f5c0);
 let mut manager=[0usize;0x80/8];manager[0x20/8]=gp;
 let bind_root=||{
  let _factory=factory_scope(pp);factory_done(gp);let _init=initialize_scope(gp,posep).unwrap();initialize_native(ip,pp,0,posep,7,9);
 };
 // Exact 5m boundary, height and true native per-frame movement. Moving the
 // source to any location cannot modify an existing binding's launch point.
 for direction in [[1f32,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,-1.,0.]]{
  reset_fixture(direction);bind_root();let initial=instance(ip).unwrap().flight;assert_eq!(initial.origin,Some(origin));
  source_physics[0x70/4..].copy_from_slice(&[999.,888.,777.,1.]);
  assert_eq!(crate::position(SOURCE.load(Ordering::Relaxed),base),[999.,888.,777.,1.]);
  assert_eq!(update(ip,5.),1);assert_eq!(instance(ip).unwrap().flight,initial);assert_eq!(read_i32(ip+0x90),1);
  assert_eq!(update(ip,0.01),0);assert_eq!(read_i32(ip+0x90),3);assert_eq!(ptr::read((ip+0x583)as*const u8),2);
  assert_eq!(CHILDREN.load(Ordering::Relaxed),0);let old_handle=read_ptr(ip+8);
  cleanup(manager.as_mut_ptr()as usize,ip);assert!(instance(ip).is_none());assert_ne!(read_ptr(ip+8),old_handle);assert_eq!(ptr::read((ip+0x580)as*const u8)&1,0);assert_eq!(read_i32(gp+0x320),0);
 }
 assert_eq!(EXITS.load(Ordering::Relaxed),3);assert_eq!(*COLLISIONS.lock().unwrap(),[77,88,77,88,77,88]);
 // A looping trajectory may travel arbitrarily far inside the launch sphere.
 reset_fixture([1.,0.,0.,0.]);bind_root();
 for p in [[14.,20.,30.,1.],[10.,24.,30.,1.],[6.,20.,30.,1.],[10.,16.,30.,1.],origin]{ptr::write_unaligned((ip+0x10)as*mut Point,p);assert_eq!(after_move(ip),0);}
 assert_eq!(instance(ip).unwrap().flight.origin,Some(origin));
 // Descendants inherit the parent's fixed origin, and blocked emissions do
 // not enter native creation. An untracked parent cannot infer a fresh root.
 let p=[14.,20.,30.,1.];let q=[0f32;4];
 assert_eq!(child(ip,p.as_ptr()as usize,q.as_ptr()as usize,10402151,0),1);
 let out=[15.001,20.,30.,1.];assert_eq!(child(ip,out.as_ptr()as usize,q.as_ptr()as usize,10402151,0),0);assert_eq!(CHILDREN.load(Ordering::Relaxed),1);
 let flight=instance(ip).unwrap().flight;assert_eq!(*INHERITED.lock().unwrap(),Some(flight));
 {
  let _child=ChildGuard(CHILD.with(|s|s.replace(Some(Some(flight)))));let _factory=factory_scope(pp);factory_done(gp);
 }
 let bad=matrix(out);assert!(initialize_scope(gp,bad.0.as_ptr()as usize).is_none());
 let good=matrix(p);{let _init=initialize_scope(gp,good.0.as_ptr()as usize).unwrap();initialize_native(ip,pp,0,good.0.as_ptr()as usize,7,9);}
 assert_eq!(instance(ip).unwrap().flight.origin,Some(origin));ptr::write_unaligned((ip+0x10)as*mut Point,[15.01,20.,30.,1.]);assert_eq!(after_move(ip),1);
 {
  let _child=ChildGuard(CHILD.with(|s|s.replace(Some(None))));let _factory=factory_scope(pp);factory_done(gp);
 }
 assert!(!STATE.lock().unwrap().groups.contains_key(&gp));
 // Non-flight/attached effects preserve native range. Release from a moved
 // attachment anchors the newly independent projectile at that release point.
 for flags in [(1u8,0u8),(0,0x20),(0,0)]{
  row[0x9a]=flags.0;row[0x9c]=flags.1;reset_fixture([0.;4]);bind_root();assert_eq!(instance(ip).unwrap().flight.origin,None);
  ptr::write_unaligned((ip+0x10)as*mut Point,[110.,20.,30.,1.]);assert_eq!(after_move(ip),0);assert_eq!(instance(ip).unwrap().flight.origin,None);
  row[0x9a]=0;row[0x9c]=0;
  ptr::write_unaligned((ip+0x30)as*mut Point,[1.,0.,0.,0.]);
  let before=point(ip);let after=[before[0]+0.1,before[1],before[2],1.];ptr::write_unaligned((ip+0x10)as*mut Point,after);assert_eq!(after_move(ip),0);assert_eq!(instance(ip).unwrap().flight.origin,Some(before));
 }
 // Ownership is sampled from source, including a remote source; generation
 // mismatch, group cleanup and non-owner creation cannot reuse stale state.
 finish_group(gp);assert!(!STATE.lock().unwrap().groups.contains_key(&gp));
 reset_fixture([1.,0.,0.,0.]);bind_root();put(ip+8,0x22340000);assert!(instance(ip).is_none());
 OWNED.store(false,Ordering::Relaxed);{let _factory=factory_scope(pp);factory_done(gp);}assert!(!STATE.lock().unwrap().groups.contains_key(&gp));
 OWNED.store(true,Ordering::Relaxed);int(pp+0x24,-1);{let _factory=factory_scope(pp);factory_done(gp);}assert!(STATE.lock().unwrap().groups.contains_key(&gp));
 int(pp+0x24,4000);{let _factory=factory_scope(pp);factory_done(gp);}assert!(!STATE.lock().unwrap().groups.contains_key(&gp));
 STATE.lock().unwrap().instances.clear();STATE.lock().unwrap().groups.clear();BASE.store(old_base,Ordering::Release);ACTIVE.store(old_active,Ordering::Release);assert_ne!(VirtualFree(memory,0,MEM_RELEASE),0);
}}
