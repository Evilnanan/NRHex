//! Native grouped boss bars omit status icons. Restore only cage escalation.
use crate::{ptr, read_i32, read_ptr, ACTIVE, BASE, WORLD_RVA};
use std::sync::atomic::{AtomicPtr, Ordering};

pub const RVA: usize = 0x7d8020;
pub const PATTERN: &str = "48 8B C4 48 83 EC 58 48 8B 0D 42 CA 43 03 0F 29 70 D8 0F 28 F1 48 85 C9";
pub static ORIGINAL: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
const EFFECT: i32 = 95001604;
type Update = unsafe extern "system" fn(usize, f32);
type Resolve = unsafe extern "system" fn(usize, *const u64) -> usize;
type Collect = unsafe extern "system" fn(usize, usize);

pub unsafe extern "system" fn detour(context: usize, dt: f32) {
    let original: Update = std::mem::transmute(ORIGINAL.load(Ordering::Acquire));
    original(context, dt);
    if !ACTIVE.load(Ordering::Acquire) { return; }
    let base = BASE.load(Ordering::Acquire);
    let root = read_ptr(base + 0x3c14a70);
    let world = read_ptr(base + WORLD_RVA);
    if root == 0 || world == 0 { return; }
    let resolve: Resolve = std::mem::transmute(base + 0x518f30);
    let collect: Collect = std::mem::transmute(base + 0x511a00);
    supplement(root, world, resolve, collect, |actor| {
        crate::native_actor::npc_id(actor, base)
            .is_some_and(crate::cage_catalog::npc)
    });
}

unsafe fn supplement(root: usize, world: usize, resolve: Resolve, collect: Collect,
                     is_cage: impl Fn(usize) -> bool) {
    for slot in 0..3 {
        let bar = root + 0xcde0 + slot * 0x68;
        let entity = ptr::read_unaligned((bar + 4) as *const u32);
        // Match native 5DF490, without changing group HP/name/entity binding.
        if read_i32(bar) == -1 || !(5..=7).contains(&(entity % 10000 / 1000)) { continue; }
        let vector = bar + 0x38;
        if read_ptr(vector + 0x10) != read_ptr(vector + 0x18) { continue; }
        let first = read_ptr(bar + 0x10);
        let end = read_ptr(bar + 0x18);
        if first == 0 || end < first || (end - first) % 8 != 0 || end - first > 256 * 8 { continue; }
        for p in (first..end).step_by(8) {
            let handle = ptr::read_unaligned(p as *const u64);
            let actor = resolve(world, &handle);
            if actor == 0 || !is_cage(actor) { continue; }
            let stats = crate::native_actor::stats(actor);
            if stats == 0 || read_i32(stats + 0x140) <= 0 { continue; }
            let manager = read_ptr(actor + 0x190);
            if manager == 0 { continue; }
            let mut node = read_ptr(manager + 8);
            let mut found = false;
            for _ in 0..4096 {
                if node == 0 { break; }
                if read_i32(node + 8) == EFFECT && read_ptr(node) != 0
                    && ptr::read_unaligned((node + 0x68) as *const u32) & 0x800c0003 == 0 {
                    collect(node, vector);
                    found = true;
                }
                node = read_ptr(node + 0x38);
            }
            // All encounter members share a level: do not add their counts.
            if found { break; }
        }
    }
}

#[cfg(test)] mod tests {
    use super::*;
    unsafe extern "system" fn resolve(_: usize, handle: *const u64) -> usize { *handle as usize }
    unsafe extern "system" fn collect(_: usize, vector: usize) {
        let count = (vector + 8) as *mut usize;
        *count += 1;
    }
    unsafe fn put(p: usize, off: usize, value: usize) { ptr::write_unaligned((p + off) as *mut usize, value); }
    #[test] fn group_only_one_live_cage_member_and_only_active_escalation() { unsafe {
        let mut hud = vec![0usize; 0xd000 / 8]; let root = hud.as_mut_ptr() as usize;
        let bar = root + 0xcde0; let vector = bar + 0x38;
        let mut actor = vec![0usize; 0x600 / 8]; let a = actor.as_mut_ptr() as usize;
        let mut stats = vec![0usize; 0x180 / 8]; let s = stats.as_mut_ptr() as usize; put(s,0x140,100);
        let modules = [s]; put(a,0x1b8,modules.as_ptr() as usize);
        let mut manager = [0usize;4]; let m = manager.as_mut_ptr() as usize; put(a,0x190,m);
        let mut nodes = vec![[0usize;17];4]; let n = nodes.as_mut_ptr() as usize; put(m,8,n);
        for i in 0..4 { let p=n+i*136; put(p,0,s); put(p,8,EFFECT as usize); if i<3 {put(p,0x38,p+136);} }
        put(n+136,8,95001603); put(n+272,0x68,1); // unrelated / inactive
        let handles=[a as u64,a as u64]; put(bar,0x10,handles.as_ptr() as usize); put(bar,0x18,handles.as_ptr() as usize+16);
        for entity in [146606043u32,146602043] {
            ptr::write_unaligned((bar+4) as *mut u32,entity); put(vector,8,0);
            supplement(root,1,resolve,collect,|_|true);
            assert_eq!(read_ptr(vector+8),if entity==146606043 {2}else{0});
        }
        ptr::write_unaligned((bar+4) as *mut u32,146606043);
        for (cage,hp,occupied) in [(false,100,false),(true,0,false),(true,100,true)] {
            put(vector,8,0); put(s,0x140,hp); put(vector,0x18,usize::from(occupied));
            supplement(root,1,resolve,collect,|_|cage); assert_eq!(read_ptr(vector+8),0);
        }
    }}
}
