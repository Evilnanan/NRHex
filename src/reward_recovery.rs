//! Move existing local rewards after revival; no lottery or receipt changes.
#[derive(Default)]
pub struct Recovery { armed: bool, candidate: Option<(usize, [f32; 3])> }
impl Recovery {
    pub fn observe(&mut self, player: usize, expedition: bool, hub: bool, incapacitated: bool, grounded: bool, position: [f32; 3]) -> bool {
        if hub { *self=Self::default(); return false; }
        if incapacitated {
            if expedition { self.armed=true; }
            self.candidate=None;
            return false;
        }
        if !self.armed { return false; }
        if !expedition || !grounded || !position.iter().all(|v|v.is_finite()) {
            self.candidate=None; return false;
        }
        let stable=self.candidate.is_some_and(|(p,old)|p==player && old.iter().zip(position).map(|(a,b)|(a-b)*(a-b)).sum::<f32>()<=0.25*0.25);
        self.candidate=Some((player,position));
        stable
    }
    pub fn complete(&mut self) { *self=Self::default(); }
}

pub fn eligible(kind:u32,owner:u16,lot:i32,good:u32,slot:u8)->bool {
    (1..=3).contains(&slot) && kind&31==2 && owner&0x40!=0 && owner&0x800!=0
        && ((owner>>7)&3)==slot as u16
        && ((crate::hex_pool::is_template_lot(lot)
        && crate::reward_choices::is_hex_good(good)) || crate::red_envelope::bonus_lot(lot))
}

/// The native node owns its links, identity, choices and VFX handles. Only
/// its map-relative destination/origin and completed-drop state are changed.
pub unsafe fn set_location(node:usize,map:u32,position:[f32;3]) {
    use std::ptr;
    ptr::write_unaligned((node+0x94) as *mut u32,map);
    ptr::write_unaligned((node+0x98) as *mut [f32;3],position);
    ptr::write_unaligned((node+0xa4) as *mut [f32;3],position);
    ptr::write((node+0xf9) as *mut u8,0);
    ptr::write_unaligned((node+0xfc) as *mut f32,0.);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observe(r:&mut Recovery,dead:bool,grounded:bool,x:f32)->bool {r.observe(1,true,false,dead,grounded,[x,0.,0.])}
    #[test] fn revival_waits_for_ground_and_settled_destination_and_runs_once() {
        let mut r=Recovery::default();assert!(!observe(&mut r,false,true,0.));
        assert!(!observe(&mut r,true,true,0.));
        assert!(!observe(&mut r,false,false,0.));
        assert!(!observe(&mut r,false,true,0.));
        assert!(!observe(&mut r,false,true,100.));
        assert!(observe(&mut r,false,true,100.));
        r.complete();assert!(!observe(&mut r,false,true,100.));
        assert!(!observe(&mut r,true,true,100.));
        assert!(!observe(&mut r,false,true,200.));assert!(observe(&mut r,false,true,200.));
    }
    #[test] fn survives_player_replacement_and_temporarily_missing_expedition_gate() {
        let mut r=Recovery::default();observe(&mut r,true,true,0.);
        assert!(!r.observe(2,false,false,false,true,[5.,0.,0.]));
        assert!(!r.observe(2,true,false,false,true,[5.,0.,0.]));
        assert!(r.observe(2,true,false,false,true,[5.,0.,0.]));
    }
    #[test] fn hub_resets_and_invalid_positions_do_not_consume_recovery() {
        let mut r=Recovery::default();observe(&mut r,true,true,0.);
        assert!(!observe(&mut r,false,true,f32::NAN));
        assert!(!observe(&mut r,false,true,5.));assert!(observe(&mut r,false,true,5.));
        assert!(!r.observe(1,false,true,false,true,[0.;3]));
        assert!(!observe(&mut r,false,true,5.));
    }
    #[test] fn only_local_private_enabled_hex_choices_are_eligible() {
        let good=0x40000000|crate::hex_pool::OWNER_IDS[0] as u32;
        for slot in 1..=3 {
            let owner=0x843|((slot as u16)<<7);
            assert!(eligible(0x2022,owner,95200000,good,slot));
            for lot in crate::hex_pool::TEMPLATE_LOT..crate::hex_pool::TEMPLATE_LOT+3{assert!(eligible(0x2022,owner,lot,good,slot));}
            assert!(!eligible(0x2022,owner,crate::hex_pool::TEMPLATE_LOT+3,good,slot));
            for other in 1..=3 {assert_eq!(eligible(0x2022,owner,95200000,good,other),other==slot);}
            assert!(!eligible(0x2032,owner,95200000,good,slot));
            assert!(!eligible(0x2022,owner&!0x40,95200000,good,slot));
            assert!(!eligible(0x2022,owner,6100002,good,slot));
            assert!(!eligible(0x2022,owner,95200000,0,slot));
        }
    }
    #[test] fn relocates_all_three_choices_without_changing_identity_links_or_contents() {unsafe{
        let mut nodes=[[0xabu8;0x100];3];
        for node in &mut nodes {
            let before=*node;set_location(node.as_mut_ptr() as usize,0x3c000000,[5.,10.,20.]);
            for i in 0..node.len() {if !(0x94..0xb0).contains(&i)&&i!=0xf9&&!(0xfc..0x100).contains(&i) {assert_eq!(node[i],before[i],"offset {i:x}");}}
            assert_eq!(&node[0x98..0xa4],&node[0xa4..0xb0]);
            assert_eq!(u32::from_le_bytes(node[0x94..0x98].try_into().unwrap()),0x3c000000);
            assert_eq!(node[0xf9],0);
        }
    }}
    #[test] fn red_envelope_weapon_and_power_choices_keep_local_ownership() {
        for slot in 1..=3 {
            let owner=0x843|((slot as u16)<<7);
            for base in crate::red_envelope::LOTS {for lot in base..base+3 {for good in [0x10000000,0x40000000|8000000] {
                assert!(eligible(2,owner,lot,good,slot));
                assert!(!eligible(0x12,owner,lot,good,slot));
                for other in 1..=3 {assert_eq!(eligible(2,owner,lot,good,other),slot==other);}
            }}}
        }
    }
}
