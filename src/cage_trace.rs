//! Bounded transition-only observations for the next cooperative acceptance run.
use std::{collections::BTreeMap, sync::{Mutex, OnceLock}};
use crate::{native_actor, read_i32, read_ptr, has_effect};
pub(crate) const PARTY_RVA:usize=0x519660;
pub(crate) const PARTY_PATTERN:&str="48 83 EC 28 48 83 3D CC 5C 6F 03 00 75 27";
static LAST: OnceLock<Mutex<BTreeMap<u64, (i32, i32, bool, i32, i32, bool, bool)>>> = OnceLock::new();
pub(crate) unsafe fn observe(actor: usize, npc: i32, base: usize) {
    let stats = native_actor::stats(actor);
    if stats == 0 { return; }
    let handle = std::ptr::read_unaligned((actor + 8) as *const u64);
    let hp = read_i32(stats + 0x140);
    let maximum = read_i32(stats + 0x144);
    let flags = std::ptr::read_unaligned((actor + 0x1f8) as *const u16);
    // Hidden candidate actors need no trace. Observe selected enabled actors,
    // including death and any HP packet exceeding the local maximum.
    if flags&0x10==0{return;}
    let count:unsafe extern "system" fn()->i32=std::mem::transmute(base+PARTY_RVA);
    let party=count();let ready=has_effect(actor,95001603,base);
    let mut coop=-1;let manager=read_ptr(actor+0x190);
    if manager!=0 {
        let mut node=read_ptr(manager+8);
        for _ in 0..256 {
            if node==0 {break;}
            let row=read_ptr(node);
            if row!=0&&std::ptr::read_unaligned((row+0x156) as *const u16)==282 {
                let id=read_i32(node+8);if has_effect(actor,id,base){coop=id;break;}
            }
            node=read_ptr(node+0x38);
        }
    }
    let current = (npc, maximum, hp > 0, coop, party, ready, hp>maximum);
    let changed = if let Ok(mut s) = LAST.get_or_init(||Mutex::new(BTreeMap::new())).lock() {
        if s.len() >= 512 && !s.contains_key(&handle) { s.clear(); }
        s.insert(handle, current) != Some(current)
    } else { false };
    if changed {
        let network = std::ptr::read_unaligned((actor + 0x78) as *const u64);
        crate::nlog_info!("Cage actor: npc={npc} handle={handle} network={network} hp={hp}/{maximum} coopEffect={coop} party={party} correctionReady={ready} hpExceedsMaximum={} networkFlags={flags:#x}",hp>maximum);
    }
}
