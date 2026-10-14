//! Native branch shooter notifications are persistent until the local event
//! consumes them, so a slow event update cannot lose a completed item use.
pub const OWNER: i32 = 95002700;
pub const USED: i32 = 95002702;

pub unsafe fn suppress_notification(player: usize, effect: i32, base: usize) -> bool {
    effect == USED && crate::is_local(player, base) && !crate::has_effect(player, OWNER, base)
}
