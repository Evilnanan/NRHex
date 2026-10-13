//! Native environment-owned NPC bullets: native attack calculation, no PC buff source.
use crate::spirit_placement::{Sweep, Vector};
use crate::{devils_gaze, has_effect, native_actor, position, ptr, read_ptr, spatial_ready};
use std::sync::{Mutex, OnceLock};
static STATE: OnceLock<Mutex<devils_gaze::Gaze>> = OnceLock::new();
pub const EMIT_RVA: usize = 0x35a460;
pub const EMIT_PATTERN:&str="40 53 55 56 57 48 81 EC 68 03 00 00 48 8B 05 ? ? ? ? 48 33 C4 48 89 84 24 50 03 00 00 49 8B D9 49 8B F0 48 8B FA 48 8B E9";
pub const DEPTH_RVA: usize = 0xb66eb0;
pub const DEPTH_PATTERN: &str = "48 8B 09 48 85 C9 0F 85 F4 7D FE FF 33 C0 C3 CC";
type Emit = unsafe extern "system" fn(usize, *mut i32, *const u8, *mut i32) -> *mut i32;

// Native 6D4CA0/6F9D20 descriptor layout, with a map asset owner and a direct Bullet ID.
// 33D010 resolves a non-ChrIns source; 340530 skips PC buff snapshots; attack kind is NPC.
#[repr(C, align(16))]
struct Descriptor([u8; 0x110]);
fn descriptor(owner: u64, id: i32, p: [f32; 4]) -> Descriptor {
    let mut d = Descriptor([0; 0x110]);
    for off in [8, 0xc, 0x14, 0x18, 0x1c, 0x24] {
        d.0[off..off + 4].copy_from_slice(&(-1i32).to_le_bytes());
    }
    d.0[..8].copy_from_slice(&owner.to_le_bytes()); // Native network kind 6.
    d.0[0x14..0x18].copy_from_slice(&id.to_le_bytes());
    for off in [0x28, 0x2c] {
        d.0[off..off + 4].copy_from_slice(&16i32.to_le_bytes());
    }
    for off in [0x30, 0x34, 0x38, 0x3c] {
        d.0[off..off + 4].copy_from_slice(&1f32.to_le_bytes());
    }
    // independent creation + force direct reference + native network queue.
    d.0[0x44..0x48].copy_from_slice(&13u32.to_le_bytes());
    let matrix = [
        1f32, 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., p[0], p[1], p[2], 1.,
    ];
    for (i, v) in matrix.iter().enumerate() {
        d.0[0x50 + i * 4..0x54 + i * 4].copy_from_slice(&v.to_le_bytes());
    }
    d.0[0xa8] = 0xff;
    d.0[0x105] = 0xff;
    // Empty native target descriptor (348260): no refcounted target object.
    d.0[0xb8..0xc0].fill(0xff);
    d.0[0xe0..0xe4].fill(0xff);
    d.0[0xf0..0xf4].fill(0xff);
    d
}
fn valid_ground(desired: [f32; 4], p: [f32; 4], normal: [f32; 4]) -> bool {
    p[..3]
        .iter()
        .chain(normal[..3].iter())
        .all(|v| v.is_finite())
        && (desired[0] - p[0]).hypot(desired[2] - p[2]) <= 0.15
        && (desired[1] - p[1]).abs() <= 4.5
        && normal[1] >= 0.35
}
// 70CA60: MapItemMan's map-key tree -> loaded map -> static asset array.
// Read only during the synchronous update. Never retain an asset pointer or
// invent a handle: kind 6 is accepted by native bullet network validation.
unsafe fn environment_owner(base: usize, origin: [f32; 4]) -> Option<u64> {
    let manager = read_ptr(base + 0x3c134f8);
    if manager == 0 {
        return None;
    }
    let head = read_ptr(manager + 0x20);
    if head == 0 {
        return None;
    }
    let mut pending = vec![read_ptr(head + 8)];
    let mut visited = 0;
    let mut best = None;
    let mut distance = 128f32 * 128.;
    while let Some(node) = pending.pop() {
        if node == 0 || node == head {
            continue;
        }
        visited += 1;
        if visited > 256 {
            return None;
        }
        if ptr::read_unaligned((node + 0x19) as *const u8) != 0 {
            continue;
        }
        pending.push(read_ptr(node));
        pending.push(read_ptr(node + 0x10));
        let map = read_ptr(node + 0x28);
        if map == 0 || ptr::read_unaligned((map + 0x185) as *const u8) == 0 {
            continue;
        }
        let start = read_ptr(map + 0x278);
        let end = read_ptr(map + 0x280);
        if start == 0 || end < start || (end - start) % 8 != 0 || (end - start) / 8 > 20000 {
            continue;
        }
        for slot in (start..end).step_by(8) {
            let asset = read_ptr(slot);
            if asset == 0 {
                continue;
            }
            let handle = ptr::read_unaligned((asset + 8) as *const u64);
            if (handle >> 28) & 15 != 6 {
                continue;
            }
            // Asset world matrix translation, same field used by 6F9D20.
            let p = ptr::read_unaligned((asset + 0x260) as *const [f32; 3]);
            let d = (0..3).map(|i| (p[i] - origin[i]).powi(2)).sum::<f32>();
            if !d.is_finite() || d >= distance {
                continue;
            }
            let vt = read_ptr(asset);
            if vt == 0 {
                continue;
            }
            let kind: unsafe extern "system" fn(usize) -> i32 =
                std::mem::transmute(read_ptr(vt + 0x10));
            let table: unsafe extern "system" fn(usize) -> u8 =
                std::mem::transmute(read_ptr(vt + 0x28));
            if kind(asset) == 1 || table(asset) != 0 {
                continue;
            }
            distance = d;
            best = Some(handle);
        }
    }
    best
}
unsafe fn ground(base: usize, desired: [f32; 4], origin: [f32; 4]) -> Option<[f32; 4]> {
    let manager = read_ptr(base + 0x3c1e028);
    if manager == 0 {
        return None;
    }
    let geometry = read_ptr(manager + 0x98);
    if geometry == 0 || read_ptr(base + 0x3c1d9a8) == 0 {
        return None;
    }
    let sweep: Sweep = std::mem::transmute(base + crate::SPIRIT_SWEEP_RVA);
    // The center uses the player's current floor, not a roof above them.
    let lift = if desired == origin { 0.3 } else { 4.5 };
    let start = Vector([desired[0], desired[1] + lift, desired[2], 1.]);
    let motion = Vector([0., -(lift + 4.5), 0., 0.]);
    let mut hit = u32::MAX;
    let mut flags = 0;
    let mut fraction = 0.;
    let mut p = Vector([f32::NAN; 4]);
    let mut n = Vector([f32::NAN; 4]);
    let returned = sweep(
        geometry,
        &mut hit,
        0x79,
        start.0.as_ptr(),
        motion.0.as_ptr(),
        0.05,
        0,
        &mut flags,
        p.0.as_mut_ptr(),
        n.0.as_mut_ptr(),
        &mut fraction,
        ptr::null_mut(),
    );
    if returned != &mut hit as *mut u32
        || hit & 0xffffff == 0xffffff
        || !valid_ground(desired, p.0, n.0)
    {
        return None;
    }
    p.0[3] = 1.;
    // A separate small clearance probe rejects centers inside walls. It does
    // not reject a whole 4.4 m disk or rotate the original horizontal visual.
    if !crate::spirit_placement::clear_destination(sweep, geometry, p.0, 0.15) {
        return None;
    }
    if desired != origin {
        let from = Vector([origin[0], origin[1] + 0.65, origin[2], 1.]);
        let motion = Vector([
            p.0[0] - origin[0],
            p.0[1] - origin[1],
            p.0[2] - origin[2],
            0.,
        ]);
        let mut contact = Vector([0.; 4]);
        let mut normal = Vector([0.; 4]);
        hit = u32::MAX;
        let returned = sweep(
            geometry,
            &mut hit,
            0x79,
            from.0.as_ptr(),
            motion.0.as_ptr(),
            0.1,
            0,
            &mut flags,
            contact.0.as_mut_ptr(),
            normal.0.as_mut_ptr(),
            &mut fraction,
            ptr::null_mut(),
        );
        if returned != &mut hit as *mut u32 || hit & 0xffffff != 0xffffff {
            return None;
        }
    }
    Some(p.0)
}
unsafe fn depth(base: usize) -> Option<i32> {
    let data = read_ptr(base + 0x3c078d0);
    if data == 0 {
        return None;
    }
    if ptr::read_unaligned((data + 0xda) as *const u8) != 2 {
        return Some(0);
    }
    let matching = read_ptr(base + 0x3c046a8);
    if matching == 0 || read_ptr(matching + 0x68) == 0 {
        return None;
    }
    let get: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + DEPTH_RVA);
    let value = get(read_ptr(matching + 0x68));
    (1..=5).contains(&value).then_some(value)
}
pub unsafe fn update(player: usize, base: usize, dt: f32) {
    let stats = native_actor::stats(player);
    if stats == 0 {
        return;
    }
    let enabled = has_effect(player, devils_gaze::OWNER, base)
        && has_effect(player, crate::level_rewards::EXPEDITION, base)
        && !native_actor::incapacitated(player, stats);
    let mut random = 0u32;
    if enabled
        && crate::BCryptGenRandom(ptr::null_mut(), &mut random as *mut u32 as *mut u8, 4, 2) < 0
    {
        return;
    }
    let fire = STATE
        .get_or_init(Default::default)
        .lock()
        .ok()
        .is_some_and(|mut s| s.tick((player, stats), enabled, dt, random));
    if !fire || !spatial_ready(player) {
        return;
    }
    let Some(id) = depth(base).and_then(devils_gaze::bullet) else {
        return;
    };
    let manager = read_ptr(base + 0x3c0ae30);
    if manager == 0 {
        return;
    }
    let Some(points) = devils_gaze::points(position(player, base)) else {
        return;
    };
    let Some(owner) = environment_owner(base, points[0]) else {
        return;
    };
    let emit: Emit = std::mem::transmute(base + EMIT_RVA);
    for desired in points {
        if let Some(p) = ground(base, desired, points[0]) {
            let d = descriptor(owner, id, p);
            let mut handle = -1;
            let mut error = -1;
            emit(manager, &mut handle, d.0.as_ptr(), &mut error);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn environment_descriptor_has_neutral_rates_and_fixed_horizontal_world_pose() {
        let d = descriptor(0x3c00000060000001, 95002270, [2., 3., 4., 1.]);
        assert_eq!(
            u64::from_le_bytes(d.0[..8].try_into().unwrap()),
            0x3c00000060000001
        );
        for off in [0x30, 0x34, 0x38, 0x3c] {
            assert_eq!(
                f32::from_le_bytes(d.0[off..off + 4].try_into().unwrap()),
                1.
            );
        }
        assert_eq!(
            &d.0[0x80..0x90],
            &[2f32, 3., 4., 1.].map(f32::to_le_bytes).concat()
        );
        for off in [0x90, 0x98, 0xa0, 0xc0] {
            assert_eq!(&d.0[off..off + 8], &[0; 8]);
        }
    }
    #[test]
    fn slope_height_is_supported_without_moving_horizontally_or_accepting_wall() {
        let p = [2., 3., 4., 1.];
        assert!(valid_ground(p, [2., 1., 4., 1.], [0., 0.7, 0.7, 0.]));
        assert!(!valid_ground(p, [2., -3., 4., 1.], [0., 1., 0., 0.]));
        assert!(!valid_ground(p, [2., 3., 4., 1.], [1., 0., 0., 0.]));
        assert!(!valid_ground(p, [0., 3., 4., 1.], [0., 1., 0., 0.]));
    }
}
