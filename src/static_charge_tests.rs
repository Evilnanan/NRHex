use super::*;
use crate::win::*;
use sha2::{Digest,Sha256};
unsafe fn int(p:usize,v:i32){ptr::write_unaligned(p as*mut i32,v);}
unsafe fn float(p:usize,v:f32){ptr::write_unaligned(p as*mut f32,v);}
unsafe extern "system" fn accepted(_:usize,_:usize,_:usize)->u8{assert!(active_contact());1}
unsafe extern "system" fn rejected(_:usize,_:usize,_:usize)->u8{0}
thread_local!{static ROW:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};}
unsafe extern "system" fn lookup(out:usize,kind:i32,id:i32)->usize{assert_eq!(kind,1);assert!(private_attack(id,kind));int(out,id);int(out+8,kind);ptr::write_unaligned((out+0x10)as*mut usize,ROW.with(std::cell::Cell::get));out}
thread_local!{static TEAM:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};}
unsafe extern "system" fn has(_:usize,id:i32)->u8{u8::from(id==OWNER||id==crate::soul_critical::OWNER)}
unsafe extern "system" fn team(_:usize,_:i32)->usize{TEAM.with(std::cell::Cell::get)}
unsafe extern "system" fn common(out:usize)->usize{ptr::write_unaligned(out as*mut usize,0);out}
unsafe extern "system" fn mode()->u8{0}
unsafe extern "system" fn apply(_:usize,id:i32,_:u8)->u8{assert_eq!(id,MARK);1}
unsafe extern "system" fn effective_team(actor:usize,out:usize)->usize{ptr::write(out as*mut u8,ptr::read((actor+0x74)as*const u8));out}
unsafe extern "system" fn immune(actor:usize,_:usize)->u8{ptr::read((actor+0x80)as*const u8)}
unsafe extern "system" fn not_dead(_:usize)->u8{0}
unsafe extern "system" fn module_actor(module:usize)->usize{read_ptr(module+8)}
unsafe fn patch(base:usize,rva:usize,target:usize){let mut code=vec![0x48,0xb8];code.extend_from_slice(&target.to_le_bytes());code.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(code.as_ptr(),(base+rva)as*mut u8,code.len());}

// Replay the observed player entity handle through the real game lookup and
// deadline update. Only the final emission and registry contents are fixtures.
unsafe fn check_timed_release(base: usize, world: &mut [usize], player: usize, target: usize) {
    const PLAYER: u64 = 0xffff_ffff_10e0_0000;
    const TARGET: u64 = PLAYER + 1;
    thread_local! { static EMISSIONS: RefCell<Vec<(u64,i32,[f32;4])>> = const { RefCell::new(Vec::new()) }; }
    unsafe extern "system" fn capture(_:usize,out:usize,descriptor:usize,_:usize)->usize {
        EMISSIONS.with(|e| e.borrow_mut().push((read_ptr(descriptor) as u64,
            read_i32(descriptor+0x14),ptr::read_unaligned((descriptor+0x80) as *const [f32;4]))));
        int(out,1);out
    }
    let slots = [player,0,target,0];
    let mut vt = [0usize;6];
    vt[4] = base + 0x4ac680;
    vt[5] = base + 0x4ac6e0;
    let registry = [vt.as_ptr() as usize,14,2,slots.as_ptr() as usize];
    world[0x170b8/8+14] = registry.as_ptr() as usize;
    // Native kind-1 decoding tables normally initialized during game startup.
    let layout = base + 0x39a8870 + 24;
    int(layout,0xff);int(layout+4,20);int(layout+8,0xfffff);
    ptr::write_unaligned((player+8) as *mut u64,PLAYER);
    ptr::write_unaligned((target+8) as *mut u64,TARGET);
    assert_eq!(crate::native_actor::resolve(base,PLAYER),0);
    assert_eq!(resolve_actor(base,PLAYER),player);
    assert_eq!(resolve_actor(base,TARGET),target);
    let original = crate::echo_cast::EMIT_ORIGINAL.swap(capture as *const () as *mut u8,Ordering::AcqRel);
    ptr::write_unaligned((base+0x3c0ae30) as *mut usize,1);
    let point = [2.,3.,4.,1.];
    for (handle,leak,bullet) in [(TARGET,false,BULLET),(PLAYER,true,LEAK_BULLET)] {
        reset();
        {let mut s=STATE.lock().unwrap();s.meter.distance=DISTANCE;assert!(s.mark(PLAYER,handle,100.,1,leak,point));}
        update(player,base,1.);
        assert!(EMISSIONS.with(|e| e.borrow().is_empty()));
        update(player,base,1.);
        assert_eq!(EMISSIONS.with(|e| std::mem::take(&mut *e.borrow_mut())),[(PLAYER,bullet,point)]);
    }
    reset();
    crate::echo_cast::EMIT_ORIGINAL.store(original,Ordering::Release);
    world[0x170b8/8+14]=0;
}
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
 for id in ATTACK..=ATTACK+3{int(cp+8,id);context[0xf8]=0;crate::ability_damage::before_construct(cp);assert_eq!(context[0xf8],1);}int(cp+8,ATTACK);
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
 // Soul Critical uses the same native transport, with a fixed 5:3 split and
 // new-target defenses. Source offense must not be applied a second time.
 CALC.with(|s|s.borrow_mut().push(Calculation{lightning:Some(800.),soul:true,object:0.5,..Default::default()}));
 let result=crate::glass_cannon::detour(out.as_mut_ptr(),inflated.as_ptr(),twice.as_ptr(),cuts.as_ptr(),one.as_ptr(),cuts.as_ptr(),one.as_ptr(),one.as_ptr(),1.,2.,1.,2.,2.,1.,2.);
 assert_eq!(result,100.);assert_eq!(out,[62.5,37.5,0.,0.,0.]);leave();
 // Root factory roundtrip retains a separate scalar for each proc. Poise and
 // rescue are independent of HP and do not inherit the critical's scaling.
 int(pp+4,crate::soul_critical::BULLET);float(pp+0x124,900.);
 int(bp+0xa4,crate::soul_critical::BULLET);
 {let _f=factory(pp);factory_done(gp);let _i=initialize(gp);bound(bp);}
 int(hp+0x40,crate::soul_critical::ATTACK);int(cp+8,crate::soul_critical::ATTACK);
 context[0xf8]=0;crate::ability_damage::before_construct(cp);assert_eq!(context[0xf8],1);
 constructed(hp,cp);assert_eq!(ptr::read_unaligned(hp as*const[f32;5]),[562.5,337.5,0.,0.,0.]);
 float(rp+0x84,10.);assert_eq!(poise(0,0,0,999.,cp,0),10.);assert_eq!(rescue(cp),30.);
 // Entry visuals carry no damage snapshot and cannot reserve a hit target.
 let leadin_target=[0u64,200];let lp=leadin_target.as_ptr()as usize;
 int(pp+4,crate::soul_critical::PRELUDE);float(pp+0x124,0.);int(bp+0xa4,crate::soul_critical::PRELUDE);
 {let _f=factory(pp);factory_done(gp);let _i=initialize(gp);bound(bp);}
 assert!(seed(bp).is_none());assert_eq!(contact(accepted,hp,lp,bp),0);
 int(pp+4,BULLET);float(pp+0x124,777.25);int(bp+0xa4,BULLET);int(hp+0x40,ATTACK);int(cp+8,ATTACK);float(rp+0x84,30.);
 {let _f=factory(pp);factory_done(gp);let _i=initialize(gp);bound(bp);}
 {let _child=child(bp,BULLET+1);int(pp+4,BULLET+1);let _f=factory(pp);factory_done(gp);let _i=initialize(gp);bound(cb);}

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
 int(pp+4,LEAK_BULLET);float(pp+0x124,187.5);int(bp+0xa4,LEAK_BULLET);
 {let _f=factory(pp);factory_done(gp);let _i=initialize(gp);bound(bp);}
 int(cb+0xa4,LEAK_BULLET+1);
 {let _c=child(bp,LEAK_BULLET+1);int(pp+4,LEAK_BULLET+1);let _f=factory(pp);factory_done(gp);let _i=initialize(gp);bound(cb);}
 assert!(Arc::ptr_eq(&seed(bp).unwrap(),&seed(cb).unwrap()));assert_eq!(seed(cb).unwrap().damage,187.5);
 assert_eq!(contact(rejected,hp,tp,bp),0);assert_eq!(contact(accepted,hp,tp,cb),1);assert_eq!(contact(accepted,hp,tp,bp),0);
 // Check every root/wave stage at contact time. Rejecting leakage must not
 // call native damage/rescue, nor spend dedup eligibility before revival.
 let mut downed=[0u8;0x740];let dp=downed.as_mut_ptr()as usize;
 ptr::write_unaligned((dp+8)as*mut u64,0xffff_ffff_10e0_0000);
 unsafe extern "system" fn must_not_contact(_:usize,_:usize,_:usize)->u8{panic!("Downed owner reached native leakage contact")}
 for id in BULLET..=BULLET+7{int(bp+0xa4,id);
  for own in [false,true]{ptr::write_unaligned((bp+0xb0)as*mut u64,read_ptr(dp+8)as u64+u64::from(!own));
   for state in [0u8,1,2]{downed[0x710]=state;seed(bp).unwrap().targets.lock().unwrap().clear();
    let blocked=id>=LEAK_BULLET&&own&&state!=0;
    let before=downed;
    assert_eq!(contact(if blocked{must_not_contact}else{accepted},hp,dp,bp),u8::from(!blocked));
    assert_eq!(downed,before);assert_eq!(seed(bp).unwrap().targets.lock().unwrap().len(),usize::from(!blocked));
    if blocked{downed[0x710]=0;assert_eq!(contact(accepted,hp,dp,bp),1);}
   }
  }
 }
 retired(bp);assert!(seed(bp).is_none());retired(cb);finished(gp);
 // Exercise the actual outgoing callback with a downed registered teammate
 // and zero HP damage. Native rescue coefficients, not HP loss, decide it.
 for(rva,target)in [(crate::APPLY_SELF_RVA,apply as*const()as usize),(crate::HAS_EFFECT_RVA,has as*const()as usize),(0x5197e0,team as*const()as usize),(crate::ambulance::COMMON_RVA,common as*const()as usize),(crate::ambulance::MODE_RVA,mode as*const()as usize)]{patch(base,rva,target);}
 let old_base=BASE.swap(base,Ordering::AcqRel);
 let mut source=[0usize;0x740/8];let sp=source.as_mut_ptr()as usize;source[1]=123;source[0x190/8]=1;
 let mut ally=[0usize;0x740/8];let ap=ally.as_mut_ptr()as usize;ally[1]=456;ptr::write((ap+0x710)as*mut u8,1);
 let mut world=vec![0usize;0x17500/8];world[0x174e8/8]=sp;ptr::write_unaligned((base+crate::WORLD_RVA)as*mut usize,world.as_ptr()as usize);TEAM.with(|s|s.set(ap));
 crate::soul_critical::native_fixture(base,sp,ap,hp);
 check_timed_release(base,&mut world,sp,ap);source[1]=123;ally[1]=456;
 let module=[0usize,ap];let mp=module.as_ptr()as usize;
 int(hp+0x40,100);int(hp+0x44,1);int(hp+0x280,0);float(hp+0x150,30.);float(hp+0x258,1.);
 SAMPLES.lock().unwrap().insert(hp,Sample{key:key(sp,ap,hp),value:500.});STATE.lock().unwrap().meter.distance=DISTANCE;
 // Drive the real outgoing callback at an exact hit distance. Failed contacts keep the charge.
 let mut physics=[0u8;0x90];let mut parts=[0usize;0x70/8];parts[0x68/8]=physics.as_mut_ptr() as usize;
 source[0x60/8]=sp;source[0x1b8/8]=parts.as_ptr() as usize;
 float(hp+0x1f4,crate::sword::CASTER_HEIGHT);
 for (melee,ranged,distance) in [(false,false,1.),(true,true,1.),(true,false,6.01)] {
  int(hp+0x124+4*4,if melee{4}else{0});int(hp+0x124+3*4,if ranged{1<<9}else{0});
  float(hp+0x1f0,distance);
  SAMPLES.lock().unwrap().insert(hp,Sample{key:key(sp,ap,hp),value:500.});
  let _dispatch=dispatch_scope(mp,hp);outgoing(mp,sp,hp);
  assert_eq!(STATE.lock().unwrap().meter.distance,DISTANCE);
 }
 float(hp+0x1f0,6.);SAMPLES.lock().unwrap().insert(hp,Sample{key:key(sp,ap,hp),value:500.});
 {let _dispatch=dispatch_scope(mp,hp);outgoing(mp,sp,hp);outgoing(mp,sp,hp);}assert_eq!(STATE.lock().unwrap().marks.len(),1);assert_eq!(STATE.lock().unwrap().marks[0].sum,500.);
 STATE.lock().unwrap().marks.clear();STATE.lock().unwrap().meter.distance=DISTANCE;
 int(hp+0x40,ATTACK);{let _dispatch=dispatch_scope(mp,hp);outgoing(mp,sp,hp);}assert_eq!(STATE.lock().unwrap().meter.distance,DISTANCE);
 int(hp+0x40,100);ptr::write((ap+0x710)as*mut u8,0);{let _dispatch=dispatch_scope(mp,hp);outgoing(mp,sp,hp);}assert_eq!(STATE.lock().unwrap().meter.distance,DISTANCE);
 // Hostile incoming damage creates a self mark, first hit included; later
 // unrelated sources add once per dispatch, even with zero final HP damage.
 int(hp+0x124+4*4,0);int(hp+0x124+3*4,1<<9);float(hp+0x1f0,30.);
 TEAM.with(|s|s.set(0));let incoming_module=[0usize,sp];let im=incoming_module.as_ptr()as usize;
 let mut game=[0usize;2];let mut data=[0i32;0x70/4];data[0x68/4]=15;game[1]=data.as_ptr()as usize;ptr::write_unaligned((base+crate::GAME_DATA_RVA)as*mut usize,game.as_ptr()as usize);
 SAMPLES.lock().unwrap().insert(hp,Sample{key:key(ap,sp,hp),value:400.});
 {let _dispatch=dispatch_scope(im,hp);outgoing(im,ap,hp);outgoing(im,ap,hp);}
 {let s=STATE.lock().unwrap();assert_eq!(s.marks.len(),1);let m=s.marks[0];assert!(m.leak);assert_eq!(m.target,123);assert_eq!(m.damage(),550.);}
 enter(im,0,hp);let environmental=[200.,0.,0.,0.,0.];
 crate::glass_cannon::detour(out.as_mut_ptr(),environmental.as_ptr(),one.as_ptr(),cuts.as_ptr(),one.as_ptr(),one.as_ptr(),one.as_ptr(),one.as_ptr(),1.,1.,1.,1.,1.,1.,1.);leave();
 assert_eq!(SAMPLES.lock().unwrap().get(&hp).unwrap().value,200.);
 {let _dispatch=dispatch_scope(im,hp);outgoing(im,0,hp);}
 assert_eq!(STATE.lock().unwrap().marks[0].damage(),750.);
 // Native target masks: selfTarget controls the owner; opposeTarget controls
 // enemies. Use the original relationship methods, not a reimplementation.
 let friendly_method=base+0x52c910;let hostile_method=base+0x52c920;
 let friendly=&friendly_method as*const usize as usize;let hostile=&hostile_method as*const usize as usize;
 ptr::write_unaligned((base+0x398d410+(2*81+2)*8)as*mut usize,&friendly as*const usize as usize);
 ptr::write_unaligned((base+0x398d410+(2*81+6)*8)as*mut usize,&hostile as*const usize as usize);
 ptr::write((sp+0x74)as*mut u8,2);ptr::write((ap+0x74)as*mut u8,6);
 patch(base,0x3d1b30,effective_team as*const()as usize);
 let relationship:unsafe extern "system" fn(usize,usize,usize)->i32=std::mem::transmute(base+0x52cb90);
 let reference=[0usize,0,rp];
 for(mask,self_hit,enemy_hit)in [(1,0,1),(4,1,0),(5,1,1)]{
  ptr::write((rp+0x81)as*mut u8,mask);assert_eq!(relationship(sp,sp,reference.as_ptr()as usize),self_hit);assert_eq!(relationship(sp,ap,reference.as_ptr()as usize),enemy_hit);
 }
 // Native Chr-owned contact gate rejects TAE invulnerability before the
 // relationship query. Leakage uses exactly this same gate as normal bolts.
 patch(base,0x3d31b0,not_dead as*const()as usize);patch(base,0x42c120,module_actor as*const()as usize);
 let mut vt=[0usize;0x208/8];vt[0x200/8]=immune as*const()as usize;source[0]=vt.as_ptr()as usize;
 let mut stats=[0u8;0x260];let modules=[0usize,stats.as_ptr()as usize];source[0x1b8/8]=modules.as_ptr()as usize;
 int(hp+0x40,ATTACK+2);ptr::write((rp+0x81)as*mut u8,4);
 let gate:unsafe extern "system" fn(usize,usize,usize,u8)->u8=std::mem::transmute(base+0x436370);
 ptr::write((sp+0x80)as*mut u8,0);assert_eq!(gate(im,sp,hp,0),1);
 ptr::write((sp+0x80)as*mut u8,1);assert_eq!(gate(im,sp,hp,0),0);
 ptr::write((sp+0x80)as*mut u8,0);assert_eq!(gate(im,sp,hp,0),1);
 // Execute the real common immunity predicate through the installed detour,
 // then continue the real contact gate's later rejection/relationship checks.
 let prepared=crate::hook::prepare((base+crate::soul_immunity::RVA)as*mut u8,crate::soul_immunity::immunity as*const u8,64).unwrap();
 let previous=crate::soul_immunity::ORIGINAL.swap(prepared.trampoline,Ordering::AcqRel);
 crate::hook::write_code((base+crate::soul_immunity::RVA)as*mut u8,&prepared.patch).unwrap();
 unsafe extern "system" fn native_immunity(actor:usize,reference:usize)->u8{
  let f:unsafe extern "system" fn(usize,usize,u8)->u8=std::mem::transmute(BASE.load(Ordering::Acquire)+crate::soul_immunity::RVA);f(actor,reference,0)
 }
 thread_local!{static GATE_MODULE:Cell<usize>=const{Cell::new(0)};}
 unsafe extern "system" fn native_contact(hit:usize,target:usize,_:usize)->u8{
  let f:unsafe extern "system" fn(usize,usize,usize,u8)->u8=std::mem::transmute(BASE.load(Ordering::Acquire)+0x436370);f(GATE_MODULE.with(Cell::get),target,hit,0)
 }
 GATE_MODULE.with(|s|s.set(im));vt[0x200/8]=native_immunity as*const()as usize;
 stats[0x58]=1; // Native complete TAE invulnerability, not a synthetic bool getter.
 let victim=read_ptr(sp+8)as u64;let limbs=crate::soul_immunity::encode(victim);
 ptr::write_unaligned((dp+0x30)as*mut[f32;4],[900.,limbs[0],limbs[1],limbs[2]]);
 copy(pp,dp);int(pp+4,crate::soul_critical::BULLET);int(bp+0xa4,crate::soul_critical::BULLET);int(hp+0x40,crate::soul_critical::ATTACK);
 {let _f=factory(pp);assert_eq!(ptr::read_unaligned((pp+0x124)as*const[f32;4]),[1.;4]);factory_done(gp);let _i=initialize(gp);bound(bp);}
 assert_eq!(seed(bp).unwrap().victim,Some(victim));
 assert_eq!(gate(im,sp,hp,0),0); // Same attack without its matching contact scope.
 // Rejected unrelated gates do not spend this wave's target allowance.
 stats[0x25c]=1;assert_eq!(contact(native_contact,hp,sp,bp),0);stats[0x25c]=0;
 ptr::write((rp+0x81)as*mut u8,0);assert_eq!(contact(native_contact,hp,sp,bp),0);ptr::write((rp+0x81)as*mut u8,4);
 source[1]=victim as usize+1;assert_eq!(contact(native_contact,hp,sp,bp),0);source[1]=victim as usize;
 int(hp+0x40,ATTACK);assert_eq!(contact(native_contact,hp,sp,bp),0);int(hp+0x40,crate::soul_critical::ATTACK);
 // Nested ordinary collisions must mask, then restore, the outer exception.
 unsafe extern "system" fn nested_contact(hit:usize,target:usize,_:usize)->u8{
  assert_eq!(contact(native_contact,hit,target,0),0);native_contact(hit,target,0)
 }
 assert_eq!(contact(nested_contact,hp,sp,bp),1);assert_eq!(contact(native_contact,hp,sp,bp),0);
 assert_eq!(gate(im,sp,hp,0),0);assert_eq!(stats[0x58],1); // No actor flags were changed.
 // Metadata survives independent root reconstruction; it is not last-target state.
 {let _f=factory(pp);factory_done(gp);let _i=initialize(gp);bound(bp);}
 assert_eq!(contact(native_contact,hp,sp,bp),1);
 // Old/non-tagged packets and lightning never gain an exception.
 ptr::write_unaligned((pp+0x128)as*mut[f32;3],[1.;3]);
 {let _f=factory(pp);factory_done(gp);let _i=initialize(gp);bound(bp);}
 assert_eq!(seed(bp).unwrap().victim,None);assert_eq!(contact(native_contact,hp,sp,bp),0);
 int(pp+4,BULLET);int(bp+0xa4,BULLET);ptr::write_unaligned((pp+0x128)as*mut[f32;3],limbs);int(hp+0x40,ATTACK);
 {let _f=factory(pp);factory_done(gp);let _i=initialize(gp);bound(bp);}
 assert_eq!(seed(bp).unwrap().victim,None);assert_eq!(contact(native_contact,hp,sp,bp),0);
 crate::soul_immunity::ORIGINAL.store(previous,Ordering::Release);
 crate::soul_target::native_fixture(base);
 reset();BASE.store(old_base,Ordering::Release);
 SAMPLES.lock().unwrap().clear();ACTIVE.store(old_active,Ordering::Release);crate::glass_cannon::ORIGINAL.store(old_calc,Ordering::Release);VirtualFree(memory,0,MEM_RELEASE);
}}
