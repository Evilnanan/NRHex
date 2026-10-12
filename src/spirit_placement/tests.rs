use super::*;
const ORIGIN: [f32; 4] = [1., 2., 3., 1.];
const ANCHOR: [f32; 4] = [1., 2., 6., 1.];
#[test]
fn large_boss_clearance_probe_starts_above_ground_and_uses_full_radius() {
    for radius in [0.4, 0.8, 1.5, 2., 3., 5., 8., 12.] {
        let p = candidate(ORIGIN, ANCHOR, radius, 0., 0.).unwrap();
        assert!(valid_position(ORIGIN, p, radius));
        let start = clearance_start(p, radius);
        assert!((start.0[1] - radius - CLEARANCE_SWEEP - (p[1] + 0.25)).abs() < 0.00001);
        assert_eq!(search(ORIGIN, ANCHOR, radius, Some, |_| true), Some(p));
    }
}
#[test]
fn wall_in_front_tries_sides_then_outer_ring_and_stops_at_first_fit() {
    let mut calls = 0;
    let p = search(
        ORIGIN,
        ANCHOR,
        3.,
        |p| {
            calls += 1;
            Some(p)
        },
        |p| p[0] < 0.,
    )
    .unwrap();
    assert_eq!(calls, 2);
    assert!(p[0] < 0.);
    let mut calls = 0;
    let p = search(
        ORIGIN,
        ANCHOR,
        3.,
        |p| {
            calls += 1;
            Some(p)
        },
        |p| (p[0] - 1.).hypot(p[2] - 3.) > 6.,
    )
    .unwrap();
    assert_eq!(calls, 9);
    assert_eq!(p, [1., 2., 10., 1.]);
}
#[test]
fn enclosed_and_missing_floor_remain_bounded_without_unsafe_fallback() {
    let mut calls = 0;
    assert_eq!(
        search(
            ORIGIN,
            ANCHOR,
            2.,
            |p| {
                calls += 1;
                Some(p)
            },
            |_| false
        ),
        None
    );
    assert_eq!(calls, 16);
    for bad in [
        ORIGIN,
        [f32::NAN; 4],
        [1., 20., 8., 1.],
        [100., 2., 100., 1.],
    ] {
        assert_eq!(
            search(
                ORIGIN,
                ANCHOR,
                2.,
                |_| Some(bad),
                |_| panic!("Rejected points must not reach clearance")
            ),
            None
        );
    }
    assert_eq!(search(ORIGIN, ANCHOR, 2., |_| None, |_| panic!()), None);
    for radius in [0., -1., f32::NAN, f32::INFINITY, 65.] {
        assert_eq!(
            search(ORIGIN, ANCHOR, radius, |_| panic!(), |_| panic!()),
            None
        );
    }
    assert_eq!(search(ORIGIN, ORIGIN, 1., |_| panic!(), |_| panic!()), None);
}
#[test]
fn destination_sweep_abi_passes_aligned_vectors_and_full_width_outputs() {
    unsafe {
        unsafe extern "system" fn worker(
            geometry: usize,
            hit: *mut u32,
            mask: u32,
            start: *const f32,
            motion: *const f32,
            radius: f32,
            ignore: usize,
            flags: *mut u32,
            contact: *mut f32,
            normal: *mut f32,
            fraction: *mut f32,
            extra: *mut f32,
        ) -> *mut u32 {
            assert_eq!(mask, 0x79);
            assert_eq!(ignore, 0);
            assert_eq!(radius, 3.);
            assert!(extra.is_null());
            assert_eq!(ptr::read(start as *const [f32; 4]), [1., 5.5, 8., 1.]);
            assert_eq!(ptr::read(motion as *const [f32; 4]), [0., -0.25, 0., 0.]);
            for p in [start, motion, contact, normal] {
                assert_eq!(p as usize % 16, 0);
            }
            ptr::write(flags, 0x12345678);
            ptr::write(fraction, 0.5);
            for p in [contact, normal] {
                ptr::write_unaligned(p as *mut [f32; 4], [1., 2., 3., 4.]);
            }
            ptr::write(hit, if geometry == 1 { 0x12 } else { 0xffffff });
            hit
        }
        assert!(!clear_destination(worker, 1, [1., 2., 8., 1.], 3.));
        assert!(clear_destination(worker, 2, [1., 2., 8., 1.], 3.));
    }
}
