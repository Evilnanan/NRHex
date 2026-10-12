//! Schedule native, same-ID refreshes on the owning character only.
//! These requests do not remember ownership across a loss or an actor change.
pub const LEASE_SECONDS: f32 = 2.0;
pub const READY_INTERVAL_MS: u64 = (LEASE_SECONDS * 1000.0 * 0.375) as u64;
pub const OWNER_INTERVAL_MS: u64 = 5000;

/// Native UI entries are five 32-bit fields. Only the comparator receives a
/// permanent-state sort key; live leases, their text and their actual timers stay intact.
#[repr(C)]
#[derive(Clone,Copy,Debug,PartialEq)]
pub struct DisplayEntry { pub effect:i32,pub icon:i32,pub index:i32,pub remaining:f32,pub count:i32 }
pub fn display_key(mut entry:DisplayEntry)->DisplayEntry{
 // These states remain available until consumed, rather than lasting two
 // seconds. Their short native lifetime is a replication lease, not a UI timer.
 // Both native comparators sort negative lifetimes by icon instead of time.
 if matches!(entry.effect,crate::runtime::ULTIMATE_READY|crate::crystal::READY|crate::escape::READY){entry.remaining=-1.0;}
 entry
}
pub type Compare=unsafe extern "system" fn(*const DisplayEntry,*const DisplayEntry)->u8;
pub unsafe fn compare(original:Compare,left:*const DisplayEntry,right:*const DisplayEntry)->u8{
 let a=core::ptr::read_unaligned(left);let b=core::ptr::read_unaligned(right);
 let x=display_key(a);let y=display_key(b);
 if x.remaining.to_bits()==a.remaining.to_bits()&&y.remaining.to_bits()==b.remaining.to_bits(){return original(left,right);}
 original(&x,&y)
}

#[derive(Default)]
pub struct Refresh {
    actor: (usize, usize, usize),
    slots: std::collections::HashMap<i32, u64>,
}
impl Refresh {
    pub fn due(&mut self, actor: (usize, usize, usize), effect: i32, wanted: bool,
               now: u64, interval: u64) -> bool {
        if self.actor != actor { self.actor = actor; self.slots.clear(); }
        if !wanted { self.slots.remove(&effect); return false; }
        if self.slots.get(&effect).is_some_and(|&next| now < next) { return false; }
        // Reserve before calling native code, which can reenter our hooks.
        self.slots.insert(effect, now.saturating_add(interval));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn actual_native_sort_reproduces_swap_and_shadow_keys_stabilize_it(){unsafe{
        use crate::win::*;
        // Unmodified 1.3.3 native comparator, RVA 82BA40, 555 bytes. These
        // Index-zero entries avoid icon lookup. Permanent entries additionally
        // consult the game's effect-kind registry; replace only those two calls
        // with an explicit absent-entry stub in this private test fixture.
        let code=include_bytes!("../data/reference/ui/status-sort.bin");
        let memory=VirtualAlloc(core::ptr::null_mut(),1024,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);
        assert!(!memory.is_null());core::ptr::copy_nonoverlapping(code.as_ptr(),memory as *mut u8,code.len());
        let bytes=memory as *mut u8;
        core::ptr::copy_nonoverlapping([0xb8,0xff,0xff,0xff,0xff,0xc3].as_ptr(),bytes.add(768),6);
        for call in [0x49usize,0x52]{
            assert_eq!(*bytes.add(call),0xe8);
            core::ptr::write_unaligned(bytes.add(call+1) as *mut i32,(768-call-5) as i32);
        }
        let prepared=crate::hook::prepare(memory as *mut u8,crate::status_sort_detour as *const u8,64).unwrap();
        assert_eq!(prepared.stolen,5);
        let original:Compare=core::mem::transmute(prepared.trampoline);
        let entries=[DisplayEntry{effect:95000403,icon:20300,index:0,remaining:0.1,count:1},
                     DisplayEntry{effect:95000701,icon:20297,index:0,remaining:1.9,count:2},
                     DisplayEntry{effect:95001203,icon:20222,index:0,remaining:0.5,count:3}];
        let mut a=entries[0];let mut b=entries[1];
        assert_eq!(original(&a,&b),0);a.remaining=1.9;b.remaining=0.1;assert_eq!(original(&a,&b),1);
        for x in [0.01,0.2,0.75,1.3,1.99]{for y in [0.01,0.2,0.75,1.3,1.99]{
            a.remaining=x;b.remaining=y;let saved=(a,b);
            assert_eq!(compare(original,&a,&b),0);assert_eq!(compare(original,&b,&a),1);
            assert_eq!((a,b),saved);
        }}
        // Same-ID layer ordering remains native, and mixed native timed
        // effects retain a consistent total key for comparisons in both ways.
        let mut native=DisplayEntry{effect:700600,icon:123,index:0,remaining:1.5,count:7};
        for e in entries{assert_eq!(compare(original,&e,&native),1);assert_eq!(compare(original,&native,&e),0);}
        native.remaining=2.5;
        for e in entries{assert_eq!(compare(original,&e,&native),1);assert_eq!(compare(original,&native,&e),0);}
        native.remaining=-1.;native.icon=20298;
        assert_eq!(compare(original,&entries[1],&native),1);assert_eq!(compare(original,&native,&entries[0]),1);
        let mut layer=a;layer.index=1;assert_eq!(compare(original,&a,&layer),1);assert_eq!(compare(original,&layer,&a),0);
        // Permanent native effect kinds 100..200 retain their menu priority.
        // The private registry stub now reports kind 100 for ID 9700, -1 for
        // all others; all native comparator branches remain original bytes.
        let registry=[0x81,0xf9,0xe4,0x25,0,0,0xb8,0xff,0xff,0xff,0xff,0xba,0x64,0,0,0,0x0f,0x44,0xc2,0xc3];
        core::ptr::copy_nonoverlapping(registry.as_ptr(),bytes.add(768),registry.len());
        native.effect=9700;native.icon=20800;
        for e in entries{assert_eq!(compare(original,&native,&e),1);assert_eq!(compare(original,&e,&native),0);}
        assert_eq!(VirtualFree(memory,0,MEM_RELEASE),1);
    }}
    #[test]fn readiness_sort_keys_stay_fixed_across_independent_renewals(){
        assert_eq!(std::mem::size_of::<DisplayEntry>(),20);
        for effect in [95000403,95000701,95001203,700600,95000501]{
            for remaining in [0.01,0.7,1.99,2.0,-1.]{
                let entry=DisplayEntry{effect,icon:123,index:0,remaining,count:7};let key=display_key(entry);
                assert_eq!((key.effect,key.icon,key.index,key.count),(effect,123,0,7));
                assert_eq!(key.remaining,if [95000403,95000701,95001203].contains(&effect){-1.}else{remaining});
            }
        }
    }
    #[test]fn timed_icons_and_foreign_effects_keep_their_exact_native_entries_and_pointers(){unsafe{
        thread_local!{static EXPECTED:std::cell::Cell<(usize,usize)>=const{std::cell::Cell::new((0,0))};}
        unsafe extern "system" fn native(left:*const DisplayEntry,right:*const DisplayEntry)->u8{
            EXPECTED.with(|expected|(expected.get()==(left as usize,right as usize)) as u8)
        }
        for effect in [95000501,95005001,95005079,95000801,95000100,9700,700600]{
            for remaining in [-1.0,0.01,0.99,15.0]{
                let a=DisplayEntry{effect,icon:20006,index:0,remaining,count:7};
                let b=DisplayEntry{effect:1681,icon:20006,index:0,remaining:40.,count:9};
                EXPECTED.with(|expected|expected.set((&a as *const _ as usize,&b as *const _ as usize)));
                assert_eq!(display_key(a),a);assert_eq!(compare(native,&a,&b),1);
                assert_eq!((a.count,b.count),(7,9));
            }
        }
    }}
    #[test]
    fn pending_native_requests_are_throttled_and_death_stops_renewal() {
        let mut r = Refresh::default();
        assert!(r.due((1, 2, 3), 7, true, 0, READY_INTERVAL_MS));
        for now in 1..READY_INTERVAL_MS { assert!(!r.due((1, 2, 3), 7, true, now, READY_INTERVAL_MS)); }
        assert!(!r.due((1, 2, 3), 7, false, 750, READY_INTERVAL_MS));
        assert!(r.due((1, 2, 3), 7, true, 751, READY_INTERVAL_MS));
    }
    #[test]
    fn actor_rebuild_ownership_loss_and_peer_viewer_revival() {
        let mut r = Refresh::default();
        assert!(r.due((1, 2, 3), 7, true, 0, OWNER_INTERVAL_MS));
        // A viewer can lose a replica while the owner keeps the same actor.
        assert!(!r.due((1, 2, 3), 7, true, 4999, OWNER_INTERVAL_MS));
        assert!(r.due((1, 2, 3), 7, true, 5000, OWNER_INTERVAL_MS));
        assert!(r.due((1, 2, 4), 7, true, 5001, OWNER_INTERVAL_MS));
        assert!(!r.due((1, 2, 4), 7, false, 5002, OWNER_INTERVAL_MS));
        assert!(r.due((1, 2, 4), 7, true, 5003, OWNER_INTERVAL_MS));
        assert!(r.due((1, 2, 4), 8, true, 5003, OWNER_INTERVAL_MS));
    }
}
