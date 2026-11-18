//! Exercise the production receive and animation detours together in the mapped
//! test EXE. Equipment, parameter rows and the final native apply are fixtures.
use super::*;
use std::sync::atomic::Ordering::SeqCst;

thread_local! {
    static ITEMS: Cell<(usize,usize)> = const { Cell::new((0,0)) };
    static RESULT: Cell<i32> = const { Cell::new(-1) };
}
unsafe extern "system" fn hand(_:usize, out:*mut u32, source:i32)->*mut u32 {
    *out = if source == -2 { 5 } else { 6 };
    out
}
unsafe extern "system" fn reference(out:*mut usize, handle:*const u32)->usize {
    let (left,right) = ITEMS.with(Cell::get);
    let entry = if *handle == 5 { left } else { right };
    *out = *handle as usize;
    *out.add(1) = entry;
    *out.add(2) = read_i32(entry+0x10) as usize;
    out as usize
}
unsafe extern "system" fn release(_:usize) {}
unsafe extern "system" fn lookup(out:*mut usize, skill:i32)->usize {
    static A: [u16;5] = [0,0,0,0,173];
    static B: [u16;5] = [0,0,0,0,247];
    *out.add(1) = match skill {1173=>A.as_ptr() as usize,1047=>B.as_ptr() as usize,_=>0};
    out as usize
}
unsafe extern "system" fn native_animation(_:usize,out:usize)->usize {
    *(out as *mut i32)=99;
    out
}
unsafe extern "system" fn native_apply(module:usize,_:usize) {
    let mut out=-1i32;
    art_animation(module, &mut out as *mut i32 as usize);
    RESULT.with(|r|r.set(out));
}

pub(super) unsafe fn verify(base:usize) {
    for (rva, target) in [(crate::ARTS_REF_RVA,reference as *const () as usize),
        (0x6a1d90,release as *const () as usize),(0xea2340,lookup as *const () as usize),
        (0xea2370,release as *const () as usize)] {
        let mut stub=vec![0xff,0x25,0,0,0,0];
        stub.extend_from_slice(&target.to_le_bytes());
        ptr::copy_nonoverlapping(stub.as_ptr(), (base+rva) as *mut u8, stub.len());
    }
    let mut actor=[0usize;64]; let a=actor.as_mut_ptr() as usize;
    let mut vtable=[0usize;80]; vtable[0x258/8]=hand as *const () as usize;
    let mut modules=[0usize;8]; let mut module=[0usize;4];
    let mut instance=[0usize;8]; let mut character=[0usize;24];
    let mut graph=[base+0x2df1098]; character[0]=base+0x2e1a2b8;
    character[0x98/8]=graph.as_mut_ptr() as usize; instance[0x30/8]=character.as_mut_ptr() as usize;
    module[1]=a; module[2]=instance.as_mut_ptr() as usize;
    let m=module.as_mut_ptr() as usize;
    modules[0x28/8]=m; actor[0]=vtable.as_mut_ptr() as usize;
    actor[1]=42; actor[0x1b8/8]=modules.as_mut_ptr() as usize;
    let id=identity(a,base).unwrap();
    let mut left=[0u32;9]; let mut right=[0u32;9];
    // Both are the same weapon model, and neither action instance knows A.
    for (item,h) in [(&mut left,5),(&mut right,6)] {
        item[2]=h; item[4]=2000000; item[7]=1047;
    }
    ITEMS.with(|v|v.set((left.as_mut_ptr() as usize,right.as_mut_ptr() as usize)));
    let ids=[711,712,713,714];
    let old_apply=APPLY.swap(native_apply as *mut u8,SeqCst);
    let old_get=ART_ANIMATION.swap(native_animation as *mut u8,SeqCst);
    STATE.lock().unwrap().bindings.push(Binding{id,ids:None,art_ids:Some(ids),local:false});
    let a_cast=Cast{hand:1,weapon:2000000,skill:1173};
    let b_cast=Cast{hand:0,skill:1047,..a_cast};
    let mut bytes=[0u8;SIZE];bytes[0x14c]=1;put(&mut bytes,0,123|0x10000);
    for (number,serial,cast,expected,newest) in [(1,1,Some(a_cast),173,1),
        (3,2,Some(b_cast),247,3),(2,1,Some(a_cast),173,3),(4,2,None,99,4),
        (5,3,Some(Cast{weapon:1,..b_cast}),99,5)] {
        assert!(write_words(&mut bytes,ids,Sample{sample:number,serial,cast}.words()));
        let original=bytes;
        STATE.lock().unwrap().seen=Some(Instant::now());
        apply(m,bytes.as_ptr() as usize);
        assert_eq!(RESULT.with(Cell::get),expected);
        assert_eq!(STATE.lock().unwrap().casts[0].1.sample,newest);
        assert_eq!(bytes,original);
    }
    STATE.lock().unwrap().bindings.clear();
    STATE.lock().unwrap().casts.clear();
    APPLY.store(old_apply,SeqCst);ART_ANIMATION.store(old_get,SeqCst);
}
