//! Retain one native Libra sound-bank reference for each audio-manager lifetime.
//! All peers preload it, including players who do not own Demon's Gaze.
use crate::read_ptr;
use std::sync::{Mutex, OnceLock};

pub const REQUEST_RVA: usize = 0xeedfb0;
pub const REQUEST_PATTERN: &str = "48 89 5C 24 08 57 48 83 EC 60 48 8B 05 ? ? ? ? 48 33 C4 48 89 44 24 50 4C 8B C2 8B 02 FF C8 83 F8 FD 0F 87";

#[derive(Default)]
struct Bank {
    manager: usize,
    requested: bool,
}
impl Bank {
    fn update(&mut self, manager: usize, scene: usize, mut request: impl FnMut(usize, i32)) {
        if self.manager != manager {
            *self = Self {
                manager,
                requested: false,
            };
        }
        if manager == 0 || scene == 0 || self.requested {
            return;
        }
        // EEDFB0 formats cs_c%04d and calls F14360 just as for native ChrIns.
        // A retained reference keeps the bank audible when Libra despawns or
        // a map unload releases its own reference. Do not add refs per cast.
        request(scene, 7560);
        self.requested = true;
    }
}
static BANK: OnceLock<Mutex<Bank>> = OnceLock::new();
pub unsafe fn update(base: usize) {
    let manager = read_ptr(base + 0x3c2a8a8);
    let scene = read_ptr(base + 0x3c2a3b0);
    if let Ok(mut bank) = BANK.get_or_init(Default::default).lock() {
        bank.update(manager, scene, |scene, model| {
            let request: unsafe extern "system" fn(usize, *const i32) =
                std::mem::transmute(base + REQUEST_RVA);
            request(scene, &model);
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_reference_survives_character_scene_and_ownership_changes() {
        let mut state = Bank::default();
        let mut calls = vec![];
        for (manager, scene) in [(0, 0), (1, 0), (1, 9), (1, 9), (1, 10), (1, 0), (1, 9)] {
            state.update(manager, scene, |s, m| calls.push((s, m)));
        }
        assert_eq!(calls, [(9, 7560)]);
        state.update(0, 0, |_, _| panic!("manager destroyed"));
        state.update(2, 11, |s, m| calls.push((s, m)));
        assert_eq!(calls, [(9, 7560), (11, 7560)]);
    }
}
