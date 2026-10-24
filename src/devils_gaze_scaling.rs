//! Unbake only the depth coefficient, then consume the live parameter query.
//! Keep private attack IDs/rounding and other mods' attack-row edits intact.
use crate::{ptr, read_i32};

pub const CORRECTION_RVA: usize = 0xe71fc0;
pub const EFFECT_RVA: usize = 0xea17e0;
// Guard the native table queries in the function bodies, not the entry points:
// difficulty mods may legitimately detour either lookup (DeeperNight: EA17E0).
pub const CORRECTION_QUERY_RVA: usize = 0xe7201d;
pub const CORRECTION_QUERY_PATTERN: &str = "45 33 C0 BA F4 00 00 00 E8 56 C6 02 00 48 85 C0 0F 84 D7 00 00 00";
pub const EFFECT_QUERY_RVA: usize = 0xea1846;
pub const EFFECT_QUERY_PATTERN: &str = "45 33 C0 41 8D 50 0F E8 2E CE FF FF 48 85 C0 0F 84 D3 00 00 00";
const BAKED: [f32; 5] = [1.3, 1.547, 1.841, 2.54, 2.947];
const HOLY_RATE: usize = 0x1e0;

// Borrowed stack references, written through RCX; neither query returns a row
// in RAX. Correction lookup writes +0/+8, effect lookup also writes +C.
#[repr(C)]
pub(super) struct ParamRef { pub row: usize, pub id: i32, pub version: u8, pub pad: [u8; 3] }
impl ParamRef {
    fn empty() -> Self { Self { row: 0, id: -1, version: 0, pad: [0; 3] } }
}
pub(super) type Lookup = unsafe extern "system" fn(*mut ParamRef, i32);

pub unsafe fn multiplier(depth: usize, base: usize) -> f32 {
    if base == 0 || !(1..=5).contains(&depth) { return 1.; }
    let correction: Lookup = std::mem::transmute(base + CORRECTION_RVA);
    let effect: Lookup = std::mem::transmute(base + EFFECT_RVA);
    resolve(depth, |out, id| correction(out, id), |out, id| effect(out, id))
}

unsafe fn resolve(depth: usize, correction: impl FnOnce(*mut ParamRef, i32),
    effect: impl FnOnce(*mut ParamRef, i32)) -> f32 {
    if !(1..=5).contains(&depth) { return 1.; }
    let mut reference = ParamRef::empty();
    correction(&mut reference, 7760);
    if reference.row == 0 || reference.id != 7760 { return 1.; }
    let id = read_i32(reference.row + (depth - 1) * 4);
    if id <= 0 { return 1.; }
    reference = ParamRef::empty();
    effect(&mut reference, id);
    if reference.row == 0 || reference.id != id { return 1.; }
    let rate = ptr::read_unaligned((reference.row + HOLY_RATE) as *const f32);
    let ratio = rate / BAKED[depth - 1];
    // Zero is a valid live override. Negative/non-finite values are not.
    if ratio.is_finite() && ratio >= 0. && (ratio * 4.).is_finite() { ratio } else { 1. }
}

#[cfg(test)] mod tests {
    use super::*;
    unsafe fn fixture(depth: usize, group: &[i32; 5], rate: f32) -> f32 {
        let mut row = [0f32; 0x400 / 4]; row[HOLY_RATE / 4] = rate;
        let saved = row;
        let result = resolve(depth, |out, id| {
            assert_eq!(id, 7760); (*out).row = group.as_ptr() as usize; (*out).id = id;
        }, |out, id| {
            assert_eq!(id, group[depth - 1]); (*out).row = row.as_ptr() as usize; (*out).id = id;
        });
        assert_eq!(row.map(f32::to_bits), saved.map(f32::to_bits)); result
    }
    #[test] fn unchanged_depths_are_exactly_neutral_and_extended_rows_are_not_double_scaled() { unsafe {
        let group = [901, 807, 713, 619, 525]; // No arithmetic effect-ID assumption.
        for depth in 1..=5 { assert_eq!(fixture(depth, &group, BAKED[depth - 1]), 1.); }
        for extra in BAKED {
            let live = BAKED[4] + extra - 1.;
            assert_eq!(fixture(5, &group, live), live / BAKED[4]);
        }
        // Repeated calls see current rows, including rollback and zero damage.
        for rate in [4.894, 2.947, 0., 1.5, 2.947] {
            assert_eq!(fixture(5, &group, rate), rate / BAKED[4]);
        }
    }}
    #[test] fn missing_or_invalid_parameters_keep_baked_damage() { unsafe {
        for depth in [0, 6, usize::MAX] {
            assert_eq!(resolve(depth, |_, _| panic!("ordinary/invalid depth queried"), |_, _| panic!()), 1.);
        }
        assert_eq!(multiplier(5, 0), 1.);
        assert_eq!(std::mem::size_of::<ParamRef>(), 16);
        assert_eq!(resolve(5, |_, _| {}, |_, _| panic!()), 1.);
        assert_eq!(resolve(5, |out, _| { (*out).row = 1; (*out).id = 99; }, |_, _| panic!()), 1.);
        for id in [-1, 0, 123] {
            let group = [id; 5];
            assert_eq!(resolve(5, |out, id| { (*out).row = group.as_ptr() as usize; (*out).id = id; },
                |out, _| { assert_eq!(id, 123); (*out).row = 1; (*out).id = 456; }), 1.);
            assert_eq!(resolve(5, |out, id| { (*out).row = group.as_ptr() as usize; (*out).id = id; },
                |_, _| { assert_eq!(id, 123); }), 1.);
        }
        for rate in [-1., f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX] {
            assert_eq!(fixture(5, &[1; 5], rate), 1.);
        }
    }}
}
