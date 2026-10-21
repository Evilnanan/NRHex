use crate::{read_ptr, ARTS, MAGE};
#[cfg(test)] use crate::arts;
pub const SNAPSHOT_RVA: usize = 0x20bde0;
pub const SNAPSHOT_PATTERN: &str = "48 81 C1 D8 01 00 00 E9 14 66 FE FF";
type Serialize = unsafe extern "system" fn(usize, *mut u8) -> u8;
type Broadcast = unsafe extern "system" fn(usize, u32, *const u8, u32);
#[repr(C, align(16))]
struct Payload([u8; 0x1f8]);

unsafe fn send(serialize: Serialize, broadcast: Broadcast, data: usize, network: usize) -> bool {
    let mut payload = Payload([0u8; 0x1f8]);
    serialize(data, payload.0.as_mut_ptr());
    broadcast(network, 0x0c, payload.0.as_ptr(), payload.0.len() as u32);
    true
}

fn has_peers(begin: usize, end: usize) -> bool {
    begin != 0 && end.checked_sub(begin).is_some_and(|length| length > 0 && length % 0xf0 == 0)
}

unsafe fn publish(serialize: Serialize, broadcast: Broadcast, data: usize, network: usize) -> bool {
    if data == 0 || network == 0
        || !has_peers(read_ptr(network + 0x78), read_ptr(network + 0x80)) { return false; }
    send(serialize, broadcast, data, network)
}

pub(crate) unsafe fn update(player: usize, base: usize) {
    let Some(state) = ARTS.get() else { return; };
    if !state.lock().is_ok_and(|s| s.network_pending()) && !MAGE.get().is_some_and(|state|state.lock().is_ok_and(|s|s.network_pending())) { return; }
    let data = read_ptr(player + 0x5a0);
    let network = read_ptr(base + 0x3c21ec8);
    if publish(std::mem::transmute(base + SNAPSHOT_RVA), std::mem::transmute(base + 0xdf24d0), data, network) {
        if let Ok(mut s) = state.lock() { s.network_sent(); }
        if let Some(state)=MAGE.get(){if let Ok(mut s)=state.lock(){s.network_sent();}}
        crate::nlog_info!("Equipment overlay player snapshot submitted: packet=0x0c");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    unsafe extern "system" fn serialize(data: usize, out: *mut u8) -> u8 {
        assert_eq!(data, 123);
        assert_eq!(out as usize % 16, 0);
        let bytes = std::slice::from_raw_parts_mut(out, 0x1f8);
        assert!(bytes.iter().all(|byte|*byte==0));
        bytes.fill(0x67); 0
    }
    unsafe extern "system" fn broadcast(network: usize, kind: u32, p: *const u8, length: u32) {
        assert_eq!((network, kind, length), (456, 0x0c, 0x1f8));
        let p = std::slice::from_raw_parts(p, length as usize);
        assert!(p.iter().all(|&byte| byte == 0x67));
    }
    #[test]
    fn player_snapshot_uses_native_body_without_npc_subtype_or_identity_prefix() {
        assert!(unsafe { send(serialize, broadcast, 123, 456) });
    }
    #[test]
    fn peer_vector_requires_complete_nonempty_native_entries() {
        assert!(!has_peers(0, 0));
        assert!(!has_peers(0, 0xf0));
        assert!(!has_peers(0x1000, 0x1000));
        assert!(!has_peers(0x1000, 0xff0));
        assert!(!has_peers(0x1000, 0x1001));
        assert!(has_peers(0x1000, 0x10f0));
        assert!(has_peers(0x1000, 0x11e0));
    }
    #[test]
    fn local_equipment_publishes_without_received_snapshot_flags() { unsafe {
        unsafe extern "system" fn serialize_local(data: usize, output: *mut u8) -> u8 {
            assert_eq!(std::ptr::read((data + 0x86e) as *const u8), 0);
            assert_eq!(std::ptr::read((data + 0x86f) as *const u8), 0);
            std::ptr::write(output, 0x67);
            0
        }
        unsafe extern "system" fn broadcast_local(network: usize, kind: u32, packet: *const u8, length: u32) {
            assert_ne!(network, 0);
            assert_eq!((kind, length), (0x0c, 0x1f8));
            let body=std::slice::from_raw_parts(packet,length as usize);
            assert_eq!(body[0],0x67);assert!(body[1..].iter().all(|byte|*byte==0));
        }
        let data = [0u8; 0x870];
        let mut network = [0usize; 17];
        let address = network.as_mut_ptr() as usize;
        let mut state = arts::Arts::default();
        state.synchronize(1, true, true);
        state.reforge(arts::Weapon { handle: 123, base: 2000000, original: 100 }, 1173);
        assert!(!publish(serialize_local, broadcast_local, data.as_ptr() as usize, address));
        assert!(state.network_pending());
        network[0x78 / 8] = 0x1000;
        network[0x80 / 8] = 0x10f0;
        assert!(!publish(serialize_local, broadcast_local, 0, address));
        assert!(!publish(serialize_local, broadcast_local, data.as_ptr() as usize, 0));
        assert!(publish(serialize_local, broadcast_local, data.as_ptr() as usize, address));
        state.network_sent();
        assert!(!state.network_pending());
    } }
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
