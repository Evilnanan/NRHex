//! Native saved SpEffects are the authoritative per-expedition receipts.
pub const LEVELS: [i32; 4] = [2, 7, 10, 13];
pub const RECEIPTS: [i32; 4] = [95001000, 95001001, 95001002, 95001003];
pub const REACHED: [i32; 4] = [95001010, 95001011, 95001012, 95001013];
pub const EXPEDITION: i32 = 95001004;

#[derive(Default)]
pub struct Delivery { player: usize, level: i32, pending_ball: bool }
impl Delivery {
    /// Give native acquisition one update to install the chosen saved effect
    /// before rolling the next ball, even when the old reward disables first.
    pub fn just_collected(&mut self, pending_ball: bool) -> bool {
        let collected=self.pending_ball && !pending_ball;
        self.pending_ball=pending_ball;
        collected
    }
    /// Level changes bypass the background polling interval immediately.
    pub fn changed(&mut self, player: usize, level: i32) -> bool {
        let changed=self.player!=player || self.level!=level;
        self.player=player; self.level=level;
        changed
    }
}

pub fn reached(level: i32) -> u8 {
    if !(1..=15).contains(&level) {return 0;}
    LEVELS.iter().enumerate().fold(0,|mask,(i,&threshold)|mask|if level>=threshold {1<<i} else {0})
}
pub fn queued(reached: u8, receipts: u8, alive: bool, expedition: bool, pending_ball: bool) -> Option<usize> {
    if !alive || !expedition || pending_ball { return None; }
    (0..LEVELS.len()).find(|&i|reached & (1<<i)!=0 && receipts & (1<<i)==0)
}
#[cfg(test)]
pub fn next(level: i32, receipts: u8, alive: bool, expedition: bool) -> Option<usize> {
    queued(reached(level),receipts,alive,expedition,false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn collection_has_one_settling_check_before_the_next_roll() {
        let mut d=Delivery::default();
        assert!(!d.just_collected(false));
        assert!(!d.just_collected(true));assert!(!d.just_collected(true));
        assert!(d.just_collected(false));assert!(!d.just_collected(false));
    }
    #[test] fn unclaimed_ball_queues_all_reached_levels_even_after_level_loss() {
        let reached=reached(13);
        assert_eq!(queued(reached,1,true,true,true),None);
        let mut receipts=1;
        for expected in [1,2,3] {
            assert_eq!(queued(reached,receipts,true,true,false),Some(expected));
            receipts |= 1<<expected;
        }
        assert_eq!(queued(reached,receipts,true,true,false),None);
        assert_eq!(queued(0,0,true,true,false),None);
    }
    #[test] fn only_four_milestones() {
        let mut mask = 0;
        let mut awards = Vec::new();
        for level in 1..=15 {
            if let Some(i) = next(level, mask, true, true) { mask |= 1 << i; awards.push(level); }
        }
        assert_eq!(awards, [2, 7, 10, 13]);
    }
    #[test] fn first_character_and_level_changes_trigger_immediate_checks() {
        let mut d=Delivery::default();
        assert!(d.changed(42,1));
        assert!(!d.changed(42,1));
        assert!(d.changed(42,2));
        assert!(!d.changed(42,2));
        assert!(d.changed(42,7));
        assert!(d.changed(43,7));
    }
    #[test] fn immediate_checks_after_level_loss_still_respect_saved_receipts() {
        let mut d=Delivery::default();d.changed(42,7);
        for level in [6,5,4,3,2,1,2,3,4,5,6,7,8,9] {
            assert!(d.changed(42,level));
            assert_eq!(next(level,3,true,true),None);
        }
        assert!(d.changed(42,10));
        assert_eq!(next(10,3,true,true),Some(2));
    }
    #[test] fn death_and_relevelling_do_not_duplicate() {
        assert_eq!(next(7, 3, false, true), None);
        for level in [6, 5, 4, 3, 2, 1, 2, 3, 4, 5, 6, 7, 8, 9] { assert_eq!(next(level, 3, true, true), None); }
        assert_eq!(next(10, 3, true, true), Some(2));
    }
    #[test] fn saved_receipts_survive_a_runtime_restart_and_catch_up_in_order() {
        assert_eq!(next(13, 7, true, true), Some(3));
        let mut receipts = 0;
        let mut awards = Vec::new();
        while let Some(i) = next(10, receipts, true, true) { receipts |= 1 << i; awards.push(i); }
        assert_eq!(awards, [0, 1, 2]);
    }
    #[test] fn hub_never_issues_and_cleared_native_receipts_allow_a_new_run() {
        assert_eq!(next(13, 0, true, false), None);
        assert_eq!(next(2, 15, true, true), None);
        assert_eq!(next(2, 0, true, true), Some(0));
        for level in [0, -1, 16, i32::MAX] { assert_eq!(next(level, 0, true, true), None); }
    }
}
