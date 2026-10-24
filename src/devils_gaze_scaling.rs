//! Raw private NPC attacks consume live Libra base/depth corrections once.
//! Preserve private attack edits and let native defenses/rounding run afterward.
use crate::{ptr, read_i32};

pub const CORRECTION_RVA: usize = 0xe71fc0;
pub const EFFECT_RVA: usize = 0xea17e0;
// Guard the native table queries in the function bodies, not the entry points:
// difficulty mods may legitimately detour either lookup (DeeperNight: EA17E0).
pub const CORRECTION_QUERY_RVA: usize = 0xe7201d;
pub const CORRECTION_QUERY_PATTERN: &str = "45 33 C0 BA F4 00 00 00 E8 56 C6 02 00 48 85 C0 0F 84 D7 00 00 00";
pub const EFFECT_QUERY_RVA: usize = 0xea1846;
pub const EFFECT_QUERY_PATTERN: &str = "45 33 C0 41 8D 50 0F E8 2E CE FF FF 48 85 C0 0F 84 D3 00 00 00";
const DEFAULT_BASE: f32 = 3.36;
const DEFAULT_DEPTH: [f32; 6] = [1., 1.3, 1.547, 1.841, 2.54, 2.947];
const ENEMY_FRACTION: f32 = 0.25;
const HOLY_RATE: usize = 0x1e0;

// Borrowed stack references, written through RCX; neither query returns a row
// in RAX. Correction lookup writes +0/+8, effect lookup also writes +C.
#[repr(C)]
pub(super) struct ParamRef { pub row: usize, pub id: i32, pub version: u8, pub pad: [u8; 3] }
impl ParamRef {
    fn empty() -> Self { Self { row: 0, id: -1, version: 0, pad: [0; 3] } }
}
pub(super) type Lookup = unsafe extern "system" fn(*mut ParamRef, i32);

fn fallback(depth: usize) -> f32 { DEFAULT_BASE * DEFAULT_DEPTH[depth] * ENEMY_FRACTION }

pub unsafe fn multiplier(depth: usize, base: usize) -> f32 {
    if depth > 5 { return 1.; }
    if base == 0 { return fallback(depth); }
    let correction: Lookup = std::mem::transmute(base + CORRECTION_RVA);
    let effect: Lookup = std::mem::transmute(base + EFFECT_RVA);
    resolve(depth, |out, id| correction(out, id), |out, id| effect(out, id))
}

unsafe fn effect_rate(effect: &mut impl FnMut(*mut ParamRef, i32), id: i32) -> Option<f32> {
    let mut reference = ParamRef::empty();
    effect(&mut reference, id);
    if reference.row == 0 || reference.id != id { return None; }
    let rate = ptr::read_unaligned((reference.row + HOLY_RATE) as *const f32);
    (rate.is_finite() && rate >= 0. && (rate * 4.).is_finite()).then_some(rate)
}
unsafe fn resolve(depth: usize, correction: impl FnOnce(*mut ParamRef, i32),
    mut effect: impl FnMut(*mut ParamRef, i32)) -> f32 {
    if depth > 5 { return 1.; }
    let base_rate = effect_rate(&mut effect, 7760).unwrap_or(DEFAULT_BASE);
    let mut depth_rate = DEFAULT_DEPTH[depth];
    if depth > 0 {
        let mut reference = ParamRef::empty();
        correction(&mut reference, 7760);
        if reference.row != 0 && reference.id == 7760 {
            let id = read_i32(reference.row + (depth - 1) * 4);
            if id > 0 { depth_rate = effect_rate(&mut effect, id).unwrap_or(depth_rate); }
        }
    }
    // Zero is intentional; invalid/missing rows use vanilla coefficients only
    // as a fallback, never as divisors for precomputed attack damage.
    let scale = base_rate * depth_rate * ENEMY_FRACTION;
    if scale.is_finite() && (scale * 4.).is_finite() { scale } else { fallback(depth) }
}

#[cfg(test)] mod tests {
    use super::*;
    unsafe fn fixture(depth: usize, base_rate: f32, rate: f32) -> f32 {
        let group = [901, 807, 713, 619, 525];
        let mut base_row = [0f32; 0x400 / 4]; base_row[HOLY_RATE / 4] = base_rate;
        let mut depth_row = [0f32; 0x400 / 4]; depth_row[HOLY_RATE / 4] = rate;
        let saved = (base_row, depth_row);
        let result = resolve(depth, |out, id| {
            assert_eq!(id, 7760); (*out).row = group.as_ptr() as usize; (*out).id = id;
        }, |out, id| {
            assert!(id == 7760 || depth > 0 && id == group[depth - 1]);
            (*out).row = if id == 7760 { base_row.as_ptr() } else { depth_row.as_ptr() } as usize; (*out).id = id;
        });
        assert_eq!(base_row.map(f32::to_bits), saved.0.map(f32::to_bits));
        assert_eq!(depth_row.map(f32::to_bits), saved.1.map(f32::to_bits)); result
    }
    #[test] fn raw_attacks_receive_live_base_and_depth_coefficients_once() { unsafe {
        for depth in 0..=5 { assert_eq!(fixture(depth, DEFAULT_BASE, DEFAULT_DEPTH[depth]), fallback(depth)); }
        for extra in DEFAULT_DEPTH.into_iter().skip(1) {
            let live = DEFAULT_DEPTH[5] + extra - 1.;
            assert_eq!(fixture(5, DEFAULT_BASE, live), DEFAULT_BASE * live * 0.25);
        }
        for rate in [4.894, 2.947, 0., 1.5, 2.947] {
            assert_eq!(fixture(5, DEFAULT_BASE, rate), DEFAULT_BASE * rate * 0.25);
        }
        for base in [1., 0., 5., DEFAULT_BASE] {
            assert_eq!(fixture(0, base, 99.), base * 0.25);
            assert_eq!(fixture(5, base, 2.), base * 2. * 0.25);
        }
    }}
    #[test] fn missing_or_invalid_parameters_use_vanilla_fallbacks_without_baked_attacks() { unsafe {
        for depth in [6, usize::MAX] {
            assert_eq!(resolve(depth, |_, _| panic!("invalid depth queried"), |_, _| panic!()), 1.);
        }
        for depth in 0..=5 {
            assert_eq!(multiplier(depth, 0), fallback(depth));
            assert_eq!(resolve(depth, |_, _| {}, |_, _| {}), fallback(depth));
            assert_eq!(resolve(depth, |out, _| { (*out).row = 1; (*out).id = 99; },
                |out, _| { (*out).row = 1; (*out).id = 456; }), fallback(depth));
        }
        assert_eq!(std::mem::size_of::<ParamRef>(), 16);
        for rate in [-1., f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX] {
            assert_eq!(fixture(5, DEFAULT_BASE, rate), fallback(5));
            assert_eq!(fixture(5, rate, 2.), DEFAULT_BASE * 2. * 0.25);
        }
        for id in [-1, 0, 123] {
            let group = [id; 5];
            assert_eq!(resolve(5, |out, id| { (*out).row = group.as_ptr() as usize; (*out).id = id; }, |_, _| {}), fallback(5));
        }
    }}
}
