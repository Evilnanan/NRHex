//! One Wildcard Arts selection for the lifetime of a skill chain.
use crate::arts::{self, Weapon};

pub const QUERY: i32 = 990063;
pub const REQUEST: i32 = 990064;
pub const NAMES: [&[u8]; 4] = [b"MAArtsActionV1\0", b"MAArtsWeapon\0", b"MAArtsCast\0", b"MAArtsSample\0"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cast {
    pub hand: i32,
    pub weapon: i32,
    pub skill: i32,
}

impl Cast {
    pub fn matches(self, weapon: Weapon) -> bool {
        self.weapon == weapon.base && (self.skill == weapon.original
            || arts::original(weapon.base, -1) == Some(self.skill)
            || (arts::pool(weapon.base).contains(&self.skill) && arts::roll_allowed(weapon.base, self.skill)))
    }
}

#[derive(Default)]
pub struct Chain {
    pub serial: u32,
    pub cast: Option<Cast>,
    handle: u32,
    observed: bool,
    closing: bool,
    quiet: u8,
}

impl Chain {
    pub fn close(&mut self) { self.closing = self.cast.is_some(); }

    pub fn observe(&mut self, hand: i32, weapon: Weapon, skill: i32, request: bool) {
        if self.closing && self.cast.is_some() {
            if !request { return; }
            self.cast = None;
        }
        self.closing = false;
        self.observed = true;
        if self.cast.is_some_and(|c| c.hand == hand) && self.handle == weapon.handle { return; }
        self.serial = self.serial.wrapping_add(1);
        self.handle = weapon.handle;
        self.cast = Some(Cast { hand, weapon: weapon.base, skill });
    }

    pub fn tick(&mut self, busy: bool) {
        if self.observed || busy { self.quiet = 0; } else { self.quiet = self.quiet.saturating_add(1); }
        self.observed = false;
        if self.quiet >= 2 { self.cast = None; }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample { pub cast: Option<Cast>, pub serial: u32, pub sample: u32 }

impl Sample {
    pub fn supersedes(self, last: Self) -> bool {
        let forward = |a: u32, b: u32| (1..0x8000_0000).contains(&a.wrapping_sub(b));
        forward(self.sample, last.sample) && (forward(self.serial, last.serial) ||
            (self.serial == last.serial && (self.cast == last.cast || self.cast.is_none())))
    }

    pub fn words(self) -> [u32; 4] {
        let (action, weapon) = self.cast.map_or((0, 0), |c| (2 + (c.skill as u32)*2 + c.hand as u32, c.weapon as u32));
        [action, weapon, self.serial, self.sample]
    }

    pub fn decode([action, weapon, serial, sample]: [u32; 4]) -> Option<Self> {
        let cast = match action {
            0 if weapon == 0 => None,
            2..=0x7fffffff if weapon > 0 && weapon <= i32::MAX as u32 =>
                Some(Cast { hand: (action & 1) as i32, weapon: weapon as i32, skill: ((action - 2) / 2) as i32 }),
            _ => return None,
        };
        Some(Self { cast, serial, sample })
    }
}

pub unsafe fn begin(actor: usize, hand: i32, request: bool, base: usize) {
    if !matches!(hand, 0|1) || !crate::is_local(actor,base) || !crate::has_effect(actor,arts::OWNER,base) { return; }
    let Some(w) = crate::arts_hand_weapon(actor,hand,base) else { return; };
    if crate::lightning_ram::selected(actor,w,base) || crate::onyx_mastery::selected(actor,w,base) { return; }
    if let Some(mut state) = crate::ARTS.get().and_then(|s|s.lock().ok()) { state.observe_cast(hand,w,request); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn late_packets_cannot_reopen_an_ended_or_replaced_cast() {
        let a = Sample { cast: Some(Cast { hand: 1, weapon: 2000000, skill: 1173 }), serial: u32::MAX, sample: 31 };
        let end = Sample { cast: None, sample: 32, ..a };
        let b = Sample { cast: Some(Cast { hand: 0, weapon: 2000000, skill: 1047 }), serial: 0, sample: 33 };
        assert!(end.supersedes(a) && b.supersedes(end) && b.supersedes(a));
        assert!(!end.supersedes(b) && !a.supersedes(b) && !b.supersedes(b));
        assert!(!Sample { sample: 34, ..a }.supersedes(end));
        assert!(!Sample { serial: a.serial, ..b }.supersedes(a));
    }

    #[test]
    fn startup_and_followups_keep_the_first_selection() {
        let w = Weapon {handle:123,base:2000000,original:100};
        let mut chain = Chain::default();
        chain.observe(0,w,1173,true);
        let first = chain.cast;
        for _ in 0..3 {
            chain.observe(0,w,1047,false);
            chain.tick(true);
            assert_eq!(chain.cast,first);
            assert_eq!(chain.serial,1);
        }
        chain.close();
        chain.observe(0,w,1047,false);
        assert_eq!(chain.cast,first);
        chain.observe(0,w,1047,true);
        assert_eq!(chain.serial,2);
        chain.tick(false); chain.tick(false); chain.tick(false);
        assert!(chain.cast.is_none());
        chain.observe(1,w,1047,true);
        assert_eq!(chain.serial,3);
        for cast in [first,None,chain.cast] {
            let frame=Sample{cast,serial:chain.serial,sample:u32::MAX};
            assert_eq!(Sample::decode(frame.words()),Some(frame));
        }
        assert!(Sample::decode([1,0,0,0]).is_none());
    }
}
