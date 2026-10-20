//! Retain value descriptors, never a pointer to a defeated/streamed-out actor.
pub const OWNER: i32 = 95001800;
pub const PASSIVE: i32 = 95001801;
pub const CAPTURE_CUE: i32 = 95001802;
pub const NATIVE_DRAIN: i32 = 705200;
pub const FIRST_NPC: i32 = 95400000;
// Native 391DC0/3918D0 descriptor: NPC row, AI row, optional PC initialization.
// A non-player NPC uses -1 in the third slot; it is not an invalid soul.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Soul {
    pub npc: i32,
    pub think: i32,
    pub chara_init: i32,
}
#[derive(Default)]
pub struct Spirits {
    pub(crate) kills: crate::spirit_kills::Kills,
    player: usize,
    owned: bool,
    soul: Option<Soul>,
    stored_max_hp: i32,
    pending: bool,
    live: u64,
    capture_cue: Option<u64>,
    retry_in: f32,
}
impl Spirits {
    pub fn synchronize(&mut self, player: usize, owned: bool) {
        if self.player != player {
            *self = Self {
                player,
                owned,
                ..Self::default()
            };
        }
        if !owned {
            self.kills = Default::default();
            self.soul = None;
            self.stored_max_hp = 0;
            self.pending = false;
            self.capture_cue = None;
            self.retry_in = 0.;
        }
        self.owned = owned;
    }
    pub fn defeated(&mut self, soul: Soul, corpse: u64, max_hp: i32) -> bool {
        if self.owned && soul.npc >= 0 && soul.think >= 0 && soul.chara_init >= -1
            && max_hp > 0 && (self.soul.is_none() || max_hp > self.stored_max_hp) {
            self.soul = Some(soul);
            self.stored_max_hp = max_hp;
            self.capture_cue = (corpse != 0 && corpse != u64::MAX).then_some(corpse);
            true
        } else {
            false
        }
    }
    pub fn ultimate(&mut self) {
        if self.owned && self.live == 0 && self.soul.is_some() && !self.pending {
            self.pending = true;
            self.retry_in = 0.;
        }
    }
    // A successful Ultimate remains paid for while placement is unavailable.
    pub fn cancel_request(&mut self) {
        self.pending = false;
        self.retry_in = 0.;
    }
    pub fn advance(&mut self, dt: f32) {
        if dt.is_finite() && dt > 0. {
            self.retry_in = (self.retry_in - dt).max(0.);
        }
    }
    pub fn take(&mut self, alive: bool) -> Option<Soul> {
        if !alive || !self.owned || self.live != 0 {
            self.pending = false;
            return None;
        }
        if self.pending && self.retry_in <= 0. {
            self.retry_in = 0.5;
            self.soul
        } else {
            None
        }
    }
    pub fn created(&mut self, handle: u64) {
        if handle != 0 && handle != u64::MAX {
            self.live = handle;
            self.soul = None;
            self.stored_max_hp = 0;
            self.pending = false;
        }
    }
    // The private native passive now owns HP drain; no independent expiry.
    pub fn take_capture_cue(&mut self) -> Option<u64> {
        self.capture_cue.take().filter(|_| self.owned)
    }
    pub fn live(&self) -> u64 {
        self.live
    }
    pub fn request_status(&self) -> (Option<i32>, u64, bool) {
        (self.soul.map(|s| s.npc), self.live, self.pending)
    }
    pub fn owned_by(&self, player: usize) -> bool {
        self.owned && self.player == player
    }
    pub fn synchronize_life(&mut self, player: usize, owned: bool, incapacitated: bool) {
        let retained = incapacitated && self.owned_by(player);
        self.synchronize(player, owned || retained);
    }
    pub fn finished(&mut self) {
        self.live = 0;
    }
}
pub fn private_npc(source: i32) -> Option<i32> {
    crate::spirit_npcs::NPCS
        .binary_search_by_key(&source, |&(id, _)| id)
        .ok()
        .map(|i| crate::spirit_npcs::NPCS[i].1)
}
pub fn is_private_npc(id: i32) -> bool {
    (FIRST_NPC..FIRST_NPC + crate::spirit_npcs::NPCS.len() as i32).contains(&id)
}
// Native NPC model decoding uses the numeric NPC prefix. Private rows need
// their source model restored; map-placed cage rows have an explicit model.
pub fn private_model(id: i32) -> Option<i32> {
    let index = id.checked_sub(FIRST_NPC)?;
    if index < 0 {
        return None;
    }
    let &(source, private) = crate::spirit_npcs::NPCS.get(index as usize)?;
    if private != id {
        return None;
    }
    let model = crate::spirit_cage_models::MODELS
        .binary_search_by_key(&source, |&(npc, _)| npc)
        .ok()
        .map(|i| crate::spirit_cage_models::MODELS[i].1)
        .unwrap_or(source / 10000 % 10000);
    Some(model)
}
pub fn boss_npc(id: i32) -> bool {
    crate::spirit_bosses::BOSS_NPCS.binary_search(&id).is_ok()
        || crate::spirit_bosses::THREAT_NPCS.binary_search(&id).is_ok()
        || crate::spirit_bosses::CAGE_BOSS_NPCS
            .binary_search(&id)
            .is_ok()
}
// Native 3922F0 moves a dead summon to 2, then 3; 391780 retires it to FF.
// An active/loading slot (0/1) wins even if a retired slot retains the same handle.
pub fn slot_finished(handle: u64, slots: &[(u64, u8); 6]) -> bool {
    !slots.iter().any(|&(h, state)| h == handle && matches!(state, 0 | 1))
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Packet {
    pub handle: u64,
    pub kind: u8,
    pub pad: [u8; 3],
    pub npc: i32,
    pub think: i32,
    pub chara_init: i32,
    pub yaw: f32,
    pub map: i32,
    pub position: [f32; 3],
    pub tail: i32,
}
impl Packet {
    pub fn new(soul: Soul, handle: u64, map: i32, position: [f32; 3], yaw: f32) -> Self {
        Self {
            handle,
            kind: 0,
            pad: [0; 3],
            npc: soul.npc,
            think: soul.think,
            chara_init: soul.chara_init,
            yaw,
            map,
            position,
            tail: 0,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_prefers_max_hp_and_consumption_resets_priority() {
        let mut s = Spirits::default();
        s.synchronize(1, true);
        assert!(s.defeated(soul(1), 11, 10000));
        assert_eq!(s.take_capture_cue(), Some(11));
        for hp in [0, -1, 9999, 10000] {
            assert!(!s.defeated(soul(2), 22, hp));
            assert_eq!(s.take_capture_cue(), None);
        }
        s.ultimate();
        assert_eq!(s.take(true), Some(soul(1)));
        assert!(s.defeated(soul(3), 33, 20000));
        assert_eq!(s.take_capture_cue(), Some(33));
        s.advance(0.5);
        assert_eq!(s.take(true), Some(soul(3)));
        s.created(0); // Failed native creation preserves the higher priority.
        assert!(!s.defeated(soul(2), 22, 10000));
        s.created(99);
        assert!(s.defeated(soul(2), 22, 100)); // Successful creation consumed it.
        s.finished();
        s.ultimate();
        assert_eq!(s.take(true), Some(soul(2)));
        s.synchronize(1, false);
        s.synchronize(1, true);
        assert!(s.defeated(soul(1), 11, 1));
    }
    #[test]
    fn native_slot_retirement_unlocks_summons_without_resolving_a_corpse() {
        let mut slots = [(u64::MAX, 0xff); 6];
        for state in [0, 1] {
            slots[3] = (77, state);
            assert!(!slot_finished(77, &slots));
        }
        for state in [2, 3, 0xff] {
            slots[3] = (77, state);
            assert!(slot_finished(77, &slots));
        }
        slots[4] = (77, 0);
        assert!(!slot_finished(77, &slots));
        slots[4] = (88, 0);
        assert!(slot_finished(77, &slots));
        let mut s = Spirits::default(); s.synchronize(1, true); s.created(77);
        s.defeated(soul(42700030), 88, 10000);
        if slot_finished(77, &slots) { s.finished(); }
        s.ultimate(); assert_eq!(s.take(true), Some(soul(42700030)));
    }
    #[test]
    fn native_threat_classification_survives_effect_cleanup_at_death() {
        for id in [42700020,42700030,42700035,42700040] { assert!(boss_npc(id)); }
        assert!(!boss_npc(FIRST_NPC));
        assert!(!boss_npc(-1));
        assert!(crate::spirit_bosses::THREAT_NPCS.windows(2).all(|w| w[0] < w[1]));
        assert!(crate::spirit_bosses::THREAT_NPCS.iter().all(|&id| private_npc(id).is_some()));
    }
    #[test]
    fn private_models_restore_native_and_explicit_cage_models_only() {
        assert_eq!(private_model(private_npc(46400010).unwrap()), Some(4640));
        assert_eq!(private_model(private_npc(32500090).unwrap()), Some(3250));
        for &(source, model) in crate::spirit_cage_models::MODELS {
            assert_eq!(private_model(private_npc(source).unwrap()), Some(model));
        }
        for id in [
            -1,
            0,
            46400010,
            FIRST_NPC - 1,
            FIRST_NPC + crate::spirit_npcs::NPCS.len() as i32,
        ] {
            assert_eq!(private_model(id), None);
        }
    }
    fn soul(n: i32) -> Soul {
        Soul {
            npc: n,
            think: 32500000,
            chara_init: -1,
        }
    }
    #[test]
    fn capture_cue_requires_real_storage_and_coalesces_team_death_callbacks() {
        let mut s = Spirits::default();
        s.defeated(soul(1), 123, 100);
        assert_eq!(s.take_capture_cue(), None);
        s.synchronize(1, true);
        s.defeated(soul(1), 123, 100);
        s.defeated(soul(2), 123, 200);
        assert_eq!(s.take_capture_cue(), Some(123));
        assert_eq!(s.take_capture_cue(), None);
        s.defeated(soul(3), 123, 300);
        s.synchronize(2, true);
        assert_eq!(s.take_capture_cue(), None);
    }
    #[test]
    fn native_drain_has_no_runtime_deadline_and_owner_loss_does_not_force_death() {
        let mut s = Spirits::default();
        s.synchronize(1, true);
        s.created(123);
        for _ in 0..10000 {
            s.synchronize_life(1, true, false);
            s.ultimate();
            assert_eq!(s.live(), 123);
            assert_eq!(s.take(true), None);
        }
        s.synchronize(1, false);
        assert_eq!(s.live(), 123);
        s.finished();
        assert_eq!(s.live(), 0);
    }
    #[test]
    fn no_soul_ultimate_does_not_queue_a_future_kill_and_repeated_signals_coalesce() {
        let mut s = Spirits::default();
        s.synchronize(1, true);
        s.ultimate();
        s.defeated(soul(1), 123, 100);
        assert_eq!(s.take(true), None);
        s.ultimate();
        s.ultimate();
        assert_eq!(s.take(true), Some(soul(1)));
        assert_eq!(s.take(true), None);
    }
    #[test]
    fn successful_summon_consumes_soul_even_after_its_death() {
        let mut s = Spirits::default();
        s.synchronize(1, true);
        s.defeated(soul(1), 123, 100);
        s.ultimate();
        assert_eq!(s.take(true), Some(soul(1)));
        s.created(123);
        s.finished();
        s.ultimate();
        assert_eq!(s.take(true), None);
        s.defeated(soul(2), 123, 200);
        s.ultimate();
        assert_eq!(s.take(true), Some(soul(2)));
    }
    #[test]
    fn failed_creation_keeps_soul_for_retry() {
        let mut s = Spirits::default();
        s.synchronize(1, true);
        s.defeated(soul(1), 123, 100);
        for handle in [0, u64::MAX] {
            s.ultimate();
            assert_eq!(s.take(true), Some(soul(1)));
            s.created(handle);
            assert_eq!(s.live(), 0);
            s.advance(0.5);
        }
        s.ultimate();
        assert_eq!(s.take(true), Some(soul(1)));
    }
    #[test]
    fn higher_hp_soul_replaces_stored_and_active_summon_cannot_be_duplicated() {
        let mut s = Spirits::default();
        s.synchronize(1, true);
        s.defeated(soul(1), 123, 100);
        s.defeated(soul(2), 123, 200);
        s.ultimate();
        s.ultimate();
        assert_eq!(s.take(true), Some(soul(2)));
        s.created(123);
        s.ultimate();
        assert_eq!(s.take(true), None);
        s.defeated(soul(3), 123, 300);
        s.finished();
        s.ultimate();
        assert_eq!(s.take(true), Some(soul(3)));
    }
    #[test]
    fn dead_owner_loss_and_map_object_replacement_clear_queued_requests() {
        let mut s = Spirits::default();
        s.synchronize(1, true);
        s.defeated(soul(1), 123, 100);
        s.ultimate();
        assert_eq!(s.take(false), None);
        s.ultimate();
        s.synchronize(1, false);
        assert_eq!(s.take(true), None);
        s.synchronize(2, true);
        s.ultimate();
        assert_eq!(s.take(true), None);
    }
    #[test]
    fn private_source_map_roundtrips_and_has_no_foreign_ids() {
        for &(source, id) in crate::spirit_npcs::NPCS {
            assert_eq!(private_npc(source), Some(id));
            assert!(is_private_npc(id));
        }
        assert!(!is_private_npc(FIRST_NPC - 1));
        assert_eq!(private_npc(-1), None);
        assert_eq!(std::mem::size_of::<Packet>(), 48);
        assert_eq!(std::mem::offset_of!(Packet, npc), 12);
        assert_eq!(std::mem::offset_of!(Packet, map), 28);
        assert_eq!(std::mem::offset_of!(Packet, position), 32);
    }
    #[test]
    fn stored_soul_survives_native_down_cleanup_but_not_a_real_owner_or_actor_loss() {
        let mut s = Spirits::default();
        s.synchronize_life(1, true, false);
        s.defeated(soul(1), 123, 100);
        s.synchronize_life(1, false, true);
        s.ultimate();
        assert_eq!(s.take(false), None);
        s.defeated(soul(2), 123, 200);
        s.synchronize_life(1, true, false);
        s.ultimate();
        assert_eq!(s.take(true), Some(soul(2)));
        s.synchronize_life(1, false, false);
        s.ultimate();
        assert_eq!(s.take(true), None);
        s.synchronize_life(2, false, true);
        assert!(!s.owned_by(2));
    }
    #[test]
    fn native_npc_zero_and_negative_chara_init_sentinel_are_valid() {
        let mut s = Spirits::default();
        s.synchronize(1, true);
        let zero = Soul {
            npc: 0,
            think: 32500000,
            chara_init: -1,
        };
        s.defeated(zero, 123, 100);
        s.defeated(
            Soul {
                npc: -1,
                think: 1,
                chara_init: -1,
            },
            123,
            100,
        );
        s.ultimate();
        assert_eq!(s.take(true), Some(zero));
        assert!(private_npc(0).is_some());
    }

    #[test]
    fn one_ultimate_retries_on_game_time_until_created_without_repaying() {
        let mut s = Spirits::default();
        s.synchronize(1, true);
        s.defeated(soul(1), 77, 100);
        s.ultimate();
        for _ in 0..20 {
            assert_eq!(s.take(true), Some(soul(1)));
            for dt in [0., -1., f32::NAN, f32::INFINITY] {
                s.advance(dt);
                s.ultimate();
                assert_eq!(s.take(true), None);
            }
            s.advance(0.25);
            assert_eq!(s.take(true), None);
            s.advance(0.25);
        }
        s.created(123);
        s.advance(10.);
        s.finished();
        assert_eq!(s.take(true), None);
    }
    #[test]
    fn cancelled_requests_do_not_resume_after_revive_or_actor_replacement() {
        let mut s = Spirits::default();
        s.synchronize(1, true);
        s.defeated(soul(1), 77, 100);
        s.ultimate();
        s.cancel_request();
        s.advance(1.);
        assert_eq!(s.take(true), None);
        s.ultimate();
        assert_eq!(s.take(true), Some(soul(1)));
        assert_eq!(s.take(false), None);
        s.advance(1.);
        assert_eq!(s.take(true), None);
        s.ultimate();
        s.synchronize(2, true);
        s.advance(1.);
        assert_eq!(s.take(true), None);
    }
    #[test]
    fn collection_cue_follows_latest_corpse_and_never_falls_back_to_owner() {
        let mut s = Spirits::default();
        s.synchronize(1, true);
        s.defeated(soul(1), 77, 100);
        s.defeated(soul(2), 88, 200);
        assert_eq!(s.take_capture_cue(), Some(88));
        assert_eq!(s.take_capture_cue(), None);
        for handle in [0, u64::MAX] {
            s.defeated(soul(1), handle, 100);
            assert_eq!(s.take_capture_cue(), None);
        }
    }
}
