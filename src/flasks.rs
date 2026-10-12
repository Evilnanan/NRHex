//! Acquisition is a handshake with the native event script. Saved PAID is
//! written only after the capacity grant and remaining-count readback succeed.
pub const PAID: i32 = 95000502;
pub const GRANTED: i32 = 95000504;
pub const COUNT_BASE: i32 = 95000520;
pub const PREPARE: i32 = 95000505;
pub const CAPTURE: i32 = 95000506;
pub const CHURCH: i32 = 95000507;
pub const CHURCH_REQUEST: i32 = 95000508;
#[derive(Default)]
pub struct Flasks { object: (usize, usize), pending: Option<i32>, granted: bool, probing: bool, ready: bool }
impl Flasks {
    pub fn synchronize(&mut self, object: (usize,usize), alive: bool, owned: bool, paid: bool) {
        if self.object != object || !alive || !owned || paid {
            *self = Self { object, ..Self::default() };
        }
    }
    pub fn request(&mut self, count: i32) -> Option<i32> {
        if self.pending.is_some() || !(0..=20).contains(&count) { return None; }
        self.pending=Some(count); Some(COUNT_BASE+count)
    }
    pub fn applied(&mut self, effect: i32) {
        if effect==GRANTED && self.pending.is_some() { self.granted=true; }
        if effect==CAPTURE && self.probing { self.ready=true; }
    }
    pub fn probe(&mut self) -> Option<i32> {
        if self.probing || self.pending.is_some() {return None;}
        self.probing=true; Some(PREPARE)
    }
    pub fn capacity_ready(&self)->bool { self.ready && self.pending.is_none() }
    pub fn probe_failed(&mut self) {self.probing=false; self.ready=false;}
    pub fn request_failed(&mut self) { if !self.granted { self.pending=None; } }
    pub fn target(&self) -> Option<i32> { self.pending.filter(|_|self.granted).map(|n|(n*2).min(20)) }
    pub fn complete(&mut self) -> Option<i32> {
        let n=self.pending.take()?; self.granted=false; Some(COUNT_BASE+n)
    }
}
#[cfg(test)] mod tests {
 use super::*;
 #[test] fn handshake_requires_grant_and_bounds_and_does_not_repeat() {
  let mut f=Flasks::default(); f.synchronize((1,2),true,true,false);
  assert_eq!(f.request(-1),None); assert_eq!(f.request(21),None);
  assert_eq!(f.request(4),Some(COUNT_BASE+4)); assert_eq!(f.request(4),None);
  assert_eq!(f.target(),None); f.applied(GRANTED); assert_eq!(f.target(),Some(8));
  assert_eq!(f.complete(),Some(COUNT_BASE+4)); assert_eq!(f.target(),None);
  f.synchronize((1,2),true,true,true); assert_eq!(f.target(),None);
 }
 #[test] fn transitions_and_death_discard_pending_inventory_writes() {
  let mut f=Flasks::default(); f.synchronize((1,2),true,true,false);
  f.request(14); f.applied(GRANTED); assert_eq!(f.target(),Some(20));
  f.synchronize((3,4),true,true,false); assert_eq!(f.target(),None);
  f.request(4); f.applied(GRANTED); f.synchronize((3,4),false,true,false);
  assert_eq!(f.target(),None);
 }
 #[test] fn capacity_probe_must_complete_before_count_request() {
  let mut f=Flasks::default(); f.synchronize((1,2),true,true,false);
  f.applied(CAPTURE); assert!(!f.capacity_ready());
  assert_eq!(f.probe(),Some(PREPARE)); assert_eq!(f.probe(),None);
  f.applied(CAPTURE); assert!(f.capacity_ready());
  // The event has refilled: remaining=capacity, independent of the old 1/5.
  assert_eq!(f.request(5),Some(COUNT_BASE+5)); assert!(!f.capacity_ready());
  f.applied(GRANTED); assert_eq!(f.target(),Some(10));
 }
}
