//! Timed Libra ground hazards. No actor pointer survives a game update.
pub const OWNER: i32 = 95002200;
pub const FIRST_BULLET: i32 = 95002250;
pub const RADIUS: f32 = 16.;

#[derive(Default)]
pub struct Gaze {
    context: (usize, usize),
    armed: bool,
    remaining: f32,
}
impl Gaze {
    pub fn tick(&mut self, context: (usize, usize), enabled: bool, dt: f32, random: u32) -> bool {
        if self.context != context || !enabled {
            *self = Self {
                context,
                ..Self::default()
            };
        }
        if !enabled || !dt.is_finite() || !(0. ..=1.).contains(&dt) {
            return false;
        }
        let interval = 6. + (random as f64 / u32::MAX as f64 * 10.) as f32;
        if !self.armed {
            self.armed = true;
            self.remaining = interval;
            return false;
        }
        self.remaining -= dt;
        if self.remaining > 0. {
            return false;
        }
        self.remaining = interval;
        true
    }
}
pub fn points(origin: [f32; 4]) -> Option<[[f32; 4]; 4]> {
    if !origin.iter().all(|v| v.is_finite()) {
        return None;
    }
    let mut points = [origin; 4];
    for i in 0..3 {
        let (s, c) = (i as f32 * std::f32::consts::TAU / 3.).sin_cos();
        points[i + 1] = [
            origin[0] + RADIUS * s,
            origin[1],
            origin[2] + RADIUS * c,
            1.,
        ];
    }
    Some(points)
}
pub fn bullet(depth: i32) -> Option<i32> {
    (0..=5)
        .contains(&depth)
        .then_some(FIRST_BULLET + depth * 10)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn triangle_has_three_equal_edges_and_one_fixed_center() {
        let o = [100., 10., -50., 1.];
        let p = points(o).unwrap();
        assert_eq!(p[0], o);
        for i in 1..4 {
            assert!(((p[i][0] - o[0]).hypot(p[i][2] - o[2]) - 16.).abs() < 0.0001);
            let q = p[1 + i % 3];
            assert!(((p[i][0] - q[0]).hypot(p[i][2] - q[2]) - 16. * 3f32.sqrt()).abs() < 0.0001);
        }
        assert!(points([0., f32::NAN, 0., 1.]).is_none());
    }
    #[test]
    fn cadence_resets_on_death_loss_and_new_actor_without_catchup_bursts() {
        let mut s = Gaze::default();
        assert!(!s.tick((1, 2), true, 0.1, 0));
        for _ in 0..5 {
            assert!(!s.tick((1, 2), true, 1., 0));
        }
        assert!(s.tick((1, 2), true, 1., u32::MAX));
        for _ in 0..15 {
            assert!(!s.tick((1, 2), true, 1., 0));
        }
        assert!(s.tick((1, 2), true, 1., 0));
        assert!(!s.tick((1, 2), false, 1., 0));
        assert!(!s.tick((1, 2), true, 1., 0));
        assert!(!s.tick((3, 4), true, 1., 0));
        assert!(!s.tick((3, 4), true, 99., 0));
        assert!(!s.tick((3, 4), true, f32::NAN, 0));
    }
    #[test]
    fn depth_selects_private_chain_and_rejects_uninitialized_depth() {
        for d in 0..=5 {
            assert_eq!(bullet(d), Some(95002250 + d * 10));
        }
        assert_eq!(bullet(-1), None);
        assert_eq!(bullet(6), None);
    }
}
