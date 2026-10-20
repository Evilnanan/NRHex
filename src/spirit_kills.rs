use crate::spirit::Soul;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Key {
    pub map: u32,
    pub part: u32,
}

impl Key {
    pub fn from_packet(packet: &[u8; 20]) -> Option<Self> {
        Self::new(
            u32::from_le_bytes(packet[0..4].try_into().unwrap()),
            u32::from_le_bytes(packet[12..16].try_into().unwrap()),
        )
    }

    pub fn new(map: u32, part: u32) -> Option<Self> {
        (map != u32::MAX && part != u32::MAX && part >> 28 == 1)
            .then_some(Self { map, part })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Candidate {
    pub soul: Soul,
    pub corpse: u64,
    pub max_hp: i32,
}

struct Entry {
    candidate: Candidate,
    key: Option<Key>,
    settled: bool,
}

#[derive(Default)]
pub(crate) struct Kills {
    entries: Vec<Entry>,
    pending: std::collections::HashSet<Key>,
}

impl Kills {
    pub fn contains(&self, corpse: u64) -> bool {
        self.entries.iter().any(|entry| entry.candidate.corpse == corpse)
    }

    pub fn observe(&mut self, candidate: Candidate, key: Option<Key>) -> Option<Candidate> {
        if self.entries.iter().any(|entry| entry.candidate.corpse == candidate.corpse) {
            return None;
        }
        let settled = key.is_some_and(|key| self.pending.remove(&key));
        self.entries.push(Entry { candidate, key, settled });
        settled.then_some(candidate)
    }

    pub fn settle(&mut self, corpse: u64) -> Option<Candidate> {
        let entry = self.entries.iter_mut().find(|entry| entry.candidate.corpse == corpse)?;
        if entry.settled {
            return None;
        }
        entry.settled = true;
        Some(entry.candidate)
    }

    pub fn receive(&mut self, key: Key) -> Option<Candidate> {
        if let Some(entry) = self.entries.iter_mut().rev().find(|entry| entry.key == Some(key)) {
            if entry.settled {
                return None;
            }
            entry.settled = true;
            Some(entry.candidate)
        } else {
            self.pending.insert(key);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(corpse: u64) -> Candidate {
        Candidate { soul: Soul { npc: 48010010, think: 48010000, chara_init: -1 }, corpse, max_hp: 1000 }
    }

    #[test]
    fn death_alone_does_not_grant_and_native_settlement_is_once_per_corpse() {
        let mut kills = Kills::default();
        assert_eq!(kills.observe(candidate(1), None), None);
        assert_eq!(kills.settle(2), None);
        assert_eq!(kills.settle(1), Some(candidate(1)));
        assert_eq!(kills.observe(candidate(1), None), None);
        assert_eq!(kills.settle(1), None);
        assert_eq!(kills.observe(candidate(2), None), None);
        assert_eq!(kills.settle(2), Some(candidate(2)));
    }

    #[test]
    fn network_receipt_matches_exact_map_and_part_in_either_arrival_order() {
        let key = Key { map: 0x3c000000, part: 0x10000001 };
        for receipt_first in [false, true] {
            let mut kills = Kills::default();
            if receipt_first {
                assert_eq!(kills.receive(key), None);
                assert_eq!(kills.observe(candidate(1), Some(key)), Some(candidate(1)));
            } else {
                assert_eq!(kills.observe(candidate(1), Some(key)), None);
                assert_eq!(kills.receive(Key { map: key.map + 1, ..key }), None);
                assert_eq!(kills.receive(Key { part: key.part + 1, ..key }), None);
                assert_eq!(kills.receive(key), Some(candidate(1)));
            }
            assert_eq!(kills.receive(key), None);
            assert_eq!(kills.settle(1), None);
        }
    }

    #[test]
    fn packet_identity_ignores_rune_amount_and_special_kill_flag() {
        let mut packet = [0u8; 20];
        packet[..4].copy_from_slice(&0x3c000000u32.to_le_bytes());
        packet[12..16].copy_from_slice(&0x10000001u32.to_le_bytes());
        let expected = Key::from_packet(&packet);
        assert!(expected.is_some());
        packet[8..12].copy_from_slice(&10000u32.to_le_bytes());
        packet[17] = 1;
        assert_eq!(Key::from_packet(&packet), expected);
        packet[15] = 0xff;
        assert_eq!(Key::from_packet(&packet), None);
    }
}
