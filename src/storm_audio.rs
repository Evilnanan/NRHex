use crate::read_ptr;
use std::sync::{Mutex, OnceLock};

pub const REQUEST_RVA: usize = 0xf14360;
pub const REQUEST_PATTERN: &str = "E9 A0 53 A2 FF";
const NAME: [u16; 7] = [99, 115, 95, 109, 49, 57, 0];

#[derive(Default)]
struct Bank {
    manager: usize,
    requested: bool,
}
impl Bank {
    fn update(&mut self, manager: usize, scene: usize, mut request: impl FnMut(usize, *const u16, *const u16, u8) -> usize) {
        if self.manager != manager {
            *self = Self { manager, requested: false };
        }
        if manager == 0 || scene == 0 || self.requested { return; }
        self.requested = request(manager, NAME.as_ptr(), NAME.as_ptr(), 0) != 0;
    }
}
static BANK: OnceLock<Mutex<Bank>> = OnceLock::new();

pub unsafe fn update(base: usize) {
    let manager = read_ptr(base + 0x3c2a8a8);
    let scene = read_ptr(base + 0x3c2a3b0);
    if let Ok(mut bank) = BANK.get_or_init(Default::default).lock() {
        bank.update(manager, scene, |manager, key, path, flag| {
            let request: unsafe extern "system" fn(usize, *const u16, *const u16, u8) -> usize =
                std::mem::transmute(base + REQUEST_RVA);
            request(manager, key, path, flag)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_one_native_reference_per_manager_across_scene_and_owner_changes() {
        let mut bank = Bank::default();
        let mut calls = vec![];
        for (manager, scene) in [(0, 0), (1, 0), (1, 9), (1, 9), (1, 10), (1, 0), (1, 9), (0, 0), (2, 11)] {
            bank.update(manager, scene, |manager, key, path, flag| {
                assert_eq!(unsafe { std::slice::from_raw_parts(key, NAME.len()) }, NAME);
                assert_eq!(key, path);
                assert_eq!(flag, 0);
                calls.push(manager);
                123
            });
        }
        assert_eq!(calls, [1, 2]);
    }

    #[test]
    fn failed_request_retries_until_a_native_reference_is_acquired() {
        let mut bank = Bank::default();
        bank.update(1, 9, |_, _, _, _| 0);
        assert!(!bank.requested);
        bank.update(1, 9, |_, _, _, _| 123);
        assert!(bank.requested);
        bank.update(1, 9, |_, _, _, _| panic!("duplicate bank reference"));
    }
}
