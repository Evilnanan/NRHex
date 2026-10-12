//! Bounded transition-only observations for the next cooperative acceptance run.
use std::{collections::BTreeMap, sync::{Mutex, OnceLock}};
use crate::{native_actor, read_i32, has_effect};
static LAST: OnceLock<Mutex<BTreeMap<u64, (i32, i32, bool)>>> = OnceLock::new();
pub(crate) unsafe fn observe(actor: usize, npc: i32, base: usize) {
    let stats = native_actor::stats(actor);
    if stats == 0 { return; }
    let handle = std::ptr::read_unaligned((actor + 8) as *const u64);
    let hp = read_i32(stats + 0x140);
    let maximum = read_i32(stats + 0x144);
    let current = (npc, maximum, hp > 0);
    let changed = if let Ok(mut s) = LAST.get_or_init(||Mutex::new(BTreeMap::new())).lock() {
        if s.len() >= 512 && !s.contains_key(&handle) { s.clear(); }
        s.insert(handle, current) != Some(current)
    } else { false };
    if changed {
        let network = std::ptr::read_unaligned((actor + 0x78) as *const u64);
        let flags = std::ptr::read_unaligned((actor + 0x1f8) as *const u16);
        crate::nlog_info!("Cage actor: npc={npc} handle={handle} network={network} hp={hp}/{maximum} scaled={} networkFlags={flags:#x}", has_effect(actor,7999,base));
    }
}
