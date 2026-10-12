//! Scope is restricted to one synchronous custom native spawn on this thread.
#[derive(Clone, Copy, Default)]
pub struct Scope { pub manager: usize, pub lot: i32, pub slot: u8, pub created: u32 }

#[derive(Debug, PartialEq)]
pub enum Route { Original, Skip, Local(u32) }

pub fn route(scope: Scope, manager: usize, lot: i32, kind: u32, owner: u16) -> Route {
    if scope.manager == 0 || manager != scope.manager || lot != scope.lot || kind & 15 != 2 {
        return Route::Original;
    }
    if ((owner >> 7) & 3) as u8 != scope.slot { return Route::Skip; }
    Route::Local(kind & !0x10)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn filters_all_party_slots_and_only_clears_network_bit() {
        for slot in 1..=3 {
            let scope = Scope { manager: 42, lot: 95200000, slot, created: 0 };
            for owner in 1..=3 {
                let result = route(scope, 42, scope.lot, 0xfedcba12, 0x4043 | (owner << 7));
                assert_eq!(result, if owner == slot as u16 { Route::Local(0xfedcba02) } else { Route::Skip });
            }
        }
    }
    #[test] fn native_rewards_other_lots_other_managers_and_other_threads_are_untouched() {
        let scope = Scope { manager: 42, lot: 95200000, slot: 1, created: 0 };
        assert_eq!(route(Scope::default(), 42, scope.lot, 0x12, 0xc0), Route::Original);
        assert_eq!(route(scope, 43, scope.lot, 0x12, 0xc0), Route::Original);
        assert_eq!(route(scope, 42, 6100002, 0x12, 0xc0), Route::Original);
        assert_eq!(route(scope, 42, scope.lot, 0x11, 0xc0), Route::Original);
    }
}
