//! Ground routing and destination clearance are separate queries. A boss-sized
//! sphere must not be swept from the player's feet at the family's fixed 2 m height.
use crate::{
    nlog_info, ptr, read_ptr, CHR_POSITION_RVA, SPIRIT_ANCHOR_RVA, SPIRIT_NPC_ROW_RVA,
    SPIRIT_POSITION_RVA, SPIRIT_SWEEP_RVA,
};

#[repr(align(16))]
pub(crate) struct Vector(pub [f32; 4]);
const ROUTE_RADIUS: f32 = 0.4; // Native 3886A0 family ground search probe.
const CLEARANCE_LIFT: f32 = 0.5;
const CLEARANCE_SWEEP: f32 = 0.25;
const DIRECTIONS: [f32; 8] = [0., 45., -45., 90., -90., 135., -135., 180.];

fn valid_radius(radius: f32) -> bool {
    radius.is_finite() && radius > 0. && radius <= 64.
}
fn candidate(
    origin: [f32; 4],
    anchor: [f32; 4],
    radius: f32,
    angle: f32,
    extra: f32,
) -> Option<[f32; 4]> {
    if !origin.iter().chain(anchor.iter()).all(|v| v.is_finite()) || !valid_radius(radius) {
        return None;
    }
    let dx = anchor[0] - origin[0];
    let dz = anchor[2] - origin[2];
    let length = dx.hypot(dz);
    if !length.is_finite() || length < 0.1 {
        return None;
    }
    let (sin, cos) = angle.to_radians().sin_cos();
    let reach = 5f32.max(radius + 1.) + extra;
    Some([
        origin[0] + (dx * cos - dz * sin) / length * reach,
        anchor[1],
        origin[2] + (dx * sin + dz * cos) / length * reach,
        1.,
    ])
}
fn valid_position(origin: [f32; 4], position: [f32; 4], radius: f32) -> bool {
    if !origin.iter().chain(position.iter()).all(|v| v.is_finite()) || !valid_radius(radius) {
        return false;
    }
    let distance = (position[0] - origin[0]).hypot(position[2] - origin[2]);
    // Reject native fallback-to-player and overlap. Allow the second search ring.
    distance >= (radius + 0.5).max(0.75)
        && distance <= (radius + 4.).max(9.)
        && (position[1] - origin[1]).abs() <= 4.
}
fn clearance_start(position: [f32; 4], radius: f32) -> Vector {
    let mut start = position;
    start[1] += radius + CLEARANCE_LIFT;
    Vector(start)
}
// DB0B80 verified at 385668/3858E9: arguments 5..12 are on the stack,
// including f32 radius. Returns the collision-ID output pointer.
pub(crate) type Sweep = unsafe extern "system" fn(
    usize,
    *mut u32,
    u32,
    *const f32,
    *const f32,
    f32,
    usize,
    *mut u32,
    *mut f32,
    *mut f32,
    *mut f32,
    *mut f32,
) -> *mut u32;
pub(crate) unsafe fn clear_destination(
    sweep: Sweep,
    geometry: usize,
    position: [f32; 4],
    radius: f32,
) -> bool {
    let start = clearance_start(position, radius);
    let motion = Vector([0., -CLEARANCE_SWEEP, 0., 0.]);
    let mut hit = u32::MAX;
    let mut flags = 0u32;
    let mut fraction = 0f32;
    let mut contact = Vector([0.; 4]);
    let mut normal = Vector([0.; 4]);
    let returned = sweep(
        geometry,
        &mut hit,
        0x79,
        start.0.as_ptr(),
        motion.0.as_ptr(),
        radius,
        0,
        &mut flags,
        contact.0.as_mut_ptr(),
        normal.0.as_mut_ptr(),
        &mut fraction,
        ptr::null_mut(),
    );
    returned == &mut hit as *mut u32 && hit & 0xffffff == 0xffffff
}
// Bounded and read-only: try nearby directions before the outer ring.
fn search(
    origin: [f32; 4],
    anchor: [f32; 4],
    radius: f32,
    mut ground: impl FnMut([f32; 4]) -> Option<[f32; 4]>,
    mut clear: impl FnMut([f32; 4]) -> bool,
) -> Option<[f32; 4]> {
    for extra in [0., 2.] {
        for angle in DIRECTIONS {
            let desired = candidate(origin, anchor, radius, angle, extra)?;
            if let Some(position) = ground(desired) {
                if valid_position(origin, position, radius) && clear(position) {
                    return Some(position);
                }
            }
        }
    }
    None
}
pub(crate) unsafe fn locate(player: usize, npc: i32, base: usize) -> Option<([f32; 4], f32)> {
    let modules = read_ptr(player + 0x1b8);
    if modules == 0 {
        return None;
    }
    let wrapper = read_ptr(modules + 0x160);
    if wrapper == 0 {
        return None;
    }
    let family = read_ptr(wrapper + 0x10);
    if family == 0 {
        return None;
    }
    let geometry = read_ptr(base + 0x3c1e028);
    if geometry == 0 || read_ptr(geometry + 0x98) == 0 || read_ptr(base + 0x3c1d9a8) == 0 {
        return None;
    }
    let mut origin = Vector([f32::NAN; 4]);
    let position: unsafe extern "system" fn(usize, *mut f32) -> *const f32 =
        std::mem::transmute(base + CHR_POSITION_RVA);
    position(player, origin.0.as_mut_ptr());
    let mut anchor = Vector([f32::NAN; 4]);
    let mut yaw = f32::NAN;
    let find: unsafe extern "system" fn(usize, *mut f32, *mut f32) -> u8 =
        std::mem::transmute(base + SPIRIT_ANCHOR_RVA);
    if find(family, anchor.0.as_mut_ptr(), &mut yaw) == 0 || !yaw.is_finite() {
        return None;
    }
    let mut row = [0usize; 2];
    let lookup: unsafe extern "system" fn(*mut usize, i32) =
        std::mem::transmute(base + SPIRIT_NPC_ROW_RVA);
    lookup(row.as_mut_ptr(), npc);
    if row[0] == 0 {
        return None;
    }
    let radius = ptr::read_unaligned((row[0] + 0x18) as *const f32);
    let correct: unsafe extern "system" fn(
        *mut f32,
        *mut f32,
        *const f32,
        *const f32,
        f32,
    ) -> *mut f32 = std::mem::transmute(base + SPIRIT_POSITION_RVA);
    let sweep: Sweep = std::mem::transmute(base + SPIRIT_SWEEP_RVA);
    let result = search(
        origin.0,
        anchor.0,
        radius,
        |desired| {
            let desired = Vector(desired);
            let mut output = Vector([f32::NAN; 4]);
            let mut scratch = Vector([0.; 4]);
            let returned = correct(
                scratch.0.as_mut_ptr(),
                output.0.as_mut_ptr(),
                desired.0.as_ptr(),
                origin.0.as_ptr(),
                ROUTE_RADIUS,
            );
            (returned == output.0.as_mut_ptr()).then_some(output.0)
        },
        |position| clear_destination(sweep, read_ptr(geometry + 0x98), position, radius),
    );
    if let Some(position) = result {
        nlog_info!("Stored soul placement: npc={npc} radius={radius} position={position:?}");
    }
    result.map(|position| (position, yaw))
}
#[cfg(test)]
mod tests;
