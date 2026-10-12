//! Native public equipment update, queued only when an overlay changes.
//! 669D70 -> 20CA00 -> 4F2710 uses packet 29, subtype 4, actor network ID,
//! followed by the native PlayerGameData snapshot. Do not deserialize it locally:
//! that would replace local instance handles and invalidate the overlay cache.
use crate::{read_ptr, ARTS};
#[cfg(test)] use crate::arts;
pub const SNAPSHOT_RVA: usize = 0x20ca00;
pub const SNAPSHOT_PATTERN: &str = "48 89 5C 24 10 48 89 6C 24 18 48 89 74 24 20 57 41 54 41 55 41 56 41 57 48 81 EC B0 00 00 00 48 8B 05 42 95 8D 03 48 33";
type Serialize = unsafe extern "system" fn(usize, *mut usize) -> usize;
type Broadcast = unsafe extern "system" fn(usize, u32, *const u8, u32);
#[repr(C, align(16))]
struct Payload([u8; 0x400]);

unsafe fn send(serialize: Serialize, broadcast: Broadcast, data: usize, network: usize, identity: u64) -> bool {
    // Native 669D70 allocates 400 bytes for this same serializer. Leave the
    // descriptor's second word zero, matching its native caller.
    let mut payload = Payload([0u8; 0x400]);
    let mut descriptor = [payload.0.as_mut_ptr() as usize, 0];
    let length = serialize(data, descriptor.as_mut_ptr());
    if length == 0 || length > payload.0.len() { return false; }
    let mut packet = [0u8; 0x409];
    packet[0] = 4;
    packet[1..9].copy_from_slice(&identity.to_le_bytes());
    packet[9..9 + length].copy_from_slice(&payload.0[..length]);
    broadcast(network, 0x29, packet.as_ptr(), (length + 9) as u32);
    true
}

pub(crate) unsafe fn update(player: usize, base: usize) {
    let Some(state) = ARTS.get() else { return; };
    if !state.lock().is_ok_and(|s| s.network_pending()) { return; }
    let data = read_ptr(player + 0x5a0);
    let network = read_ptr(base + 0x3c21ec8);
    // Exact native 669D70 send prerequisites; defer across join/loading.
    if data == 0 || network == 0 || *(data.wrapping_add(0x86e) as *const u8) == 0
        || *(data.wrapping_add(0x86f) as *const u8) == 0 { return; }
    let identity = std::ptr::read_unaligned((player + 0x78) as *const u64);
    if identity == u64::MAX { return; }
    if send(std::mem::transmute(base + SNAPSHOT_RVA), std::mem::transmute(base + 0xdf24d0), data, network, identity) {
        if let Ok(mut s) = state.lock() { s.network_sent(); }
        crate::nlog_info!("Arts equipment snapshot sent: actor={identity}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    unsafe extern "system" fn serialize(data: usize, out: *mut usize) -> usize {
        assert_eq!(data, 123);
        assert_eq!(*out.add(1), 0);
        assert_eq!(*out % 16, 0);
        let bytes = std::slice::from_raw_parts_mut(*out as *mut u8, 0x400);
        bytes.fill(0x67); 0x210
    }
    unsafe extern "system" fn broadcast(network: usize, kind: u32, p: *const u8, length: u32) {
        assert_eq!((network, kind, length), (456, 0x29, 0x219));
        let p = std::slice::from_raw_parts(p, length as usize);
        assert_eq!(p[0], 4);
        assert_eq!(&p[1..9], &0x123456789abcdef0u64.to_le_bytes());
        assert!(p[9..].iter().all(|&b| b == 0x67));
    }
    #[test]
    fn snapshot_preserves_native_body_and_network_identity() {
        assert!(unsafe { send(serialize, broadcast, 123, 456, 0x123456789abcdef0) });
    }
    #[test]
    fn changes_coalesce_and_ownership_loss_sends_restoration() {
        let mut s = arts::Arts::default();
        s.synchronize(1, true, true);
        let w = arts::Weapon { handle: 123, base: 2000000, original: 100 };
        s.reforge(w, 1173); s.reforge(w, 1047);
        assert!(s.network_pending()); s.network_sent();
        assert!(!s.network_pending());
        s.synchronize(1, false, true);
        assert!(s.network_pending()); s.network_sent();
        s.synchronize(1, false, true);
        assert!(!s.network_pending());
    }
}
