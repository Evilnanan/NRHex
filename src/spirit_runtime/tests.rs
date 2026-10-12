use super::*;
use crate::win::*;
use crate::{
    read_i32, CHR_POSITION_RVA, SPIRIT_ANCHOR_RVA, SPIRIT_NPC_ROW_RVA, SPIRIT_POSITION_RVA,
    SPIRIT_SLOT_PATTERN, SPIRIT_SWEEP_RVA,
};
use std::cell::RefCell;
#[derive(Default)]
struct Calls {
    actor: usize,
    creator: usize,
    implementation: usize,
    npc: i32,
    npc_row: usize,
    placement: u8,
    events: Vec<&'static str>,
    packet: Option<spirit::Packet>,
}
thread_local! {static CALLS:RefCell<Calls>=RefCell::new(Calls::default());}
unsafe extern "system" fn original_model(out: *mut i32, id: i32) -> *mut i32 {
    ptr::write(out, id / 10000 % 10000);
    out
}
#[test]
fn model_override_preserves_output_pointer_and_leaves_other_ids_and_inactive_calls_native() {
    unsafe {
        let private = spirit::private_npc(46400010).unwrap();
        let mut result = -1;
        let out = &mut result as *mut i32;
        assert_eq!(model_call(original_model, out, private, true), out);
        assert_eq!(result, 4640);
        model_call(original_model, out, 46400010, true);
        assert_eq!(result, 4640);
        model_call(original_model, out, private, false);
        assert_eq!(result, 9540);
    }
}
unsafe extern "system" fn npc_think(actor: usize) -> i32 {
    read_i32(actor + 0x5c0)
}
#[test]
fn native_npc_capture_uses_think_getter_and_preserves_negative_pc_sentinel() {
    unsafe {
        let mut vtable = vec![0usize; 0x1b0 / 8];
        let mut actor = vec![0usize; 0x5c8 / 8];
        vtable[0x1a8 / 8] = npc_think as *const () as usize;
        vtable[0x190 / 8] = 1; // Not callable PlayerGameData.
        actor[0] = vtable.as_mut_ptr() as usize;
        let actor = actor.as_mut_ptr() as usize;
        ptr::write((actor + 0x68) as *mut [i32; 2], [32500090, 3250]);
        ptr::write((actor + 0x5c0) as *mut i32, 32500010);
        let soul = npc_soul(actor, 32500090).unwrap();
        assert_eq!(
            soul,
            spirit::Soul {
                npc: 32500090,
                think: 32500010,
                chara_init: -1
            }
        );
        let mut s = spirit::Spirits::default();
        s.synchronize(1, true);
        s.ultimate();
        assert_eq!(s.take(true), None);
        s.defeated(soul, 123, 100);
        s.ultimate();
        assert_eq!(s.take(true), Some(soul));
        let packet = spirit::Packet::new(soul, 123, 777, [1., 2., 3.], 0.);
        assert_eq!(
            ptr::read_unaligned((&packet as *const _ as usize + 12) as *const [i32; 3]),
            [32500090, 32500010, -1]
        );
    }
}
unsafe extern "system" fn position(_actor: usize, out: *mut f32) -> *const f32 {
    ptr::write(out as *mut [f32; 4], [1., 2., 3., 1.]);
    out
}
unsafe extern "system" fn anchor(_family: usize, out: *mut f32, yaw: *mut f32) -> u8 {
    assert_eq!(out as usize % 16, 0);
    ptr::write(out as *mut [f32; 4], [1., 2., 6., 1.]);
    ptr::write(yaw, std::f32::consts::PI);
    CALLS.with(|c| u8::from(c.borrow().placement != 1))
}
unsafe extern "system" fn npc_row(out: *mut usize, npc: i32) {
    CALLS.with(|c| {
        let c = c.borrow();
        assert_eq!(npc, c.npc);
        ptr::write(out, if c.placement == 4 { 0 } else { c.npc_row });
    });
}
unsafe extern "system" fn correct(
    _scratch: *mut f32,
    out: *mut f32,
    desired: *const f32,
    origin: *const f32,
    radius: f32,
) -> *mut f32 {
    for p in [out, desired as *mut f32, origin as *mut f32] {
        assert_eq!(p as usize % 16, 0);
    }
    assert!((radius - 0.4).abs() < 0.00001);
    assert_eq!(ptr::read(origin as *const [f32; 4]), [1., 2., 3., 1.]);
    let mode = CALLS.with(|c| c.borrow().placement);
    ptr::write(
        out as *mut [f32; 4],
        if mode == 2 {
            ptr::read(origin as *const [f32; 4])
        } else if mode == 3 {
            [f32::NAN; 4]
        } else {
            ptr::read(desired as *const [f32; 4])
        },
    );
    out
}

unsafe extern "system" fn sweep(
    geometry: usize,
    hit: *mut u32,
    mask: u32,
    start: *const f32,
    motion: *const f32,
    radius: f32,
    ignore: usize,
    flags: *mut u32,
    contact: *mut f32,
    normal: *mut f32,
    fraction: *mut f32,
    extra: *mut f32,
) -> *mut u32 {
    assert_eq!(geometry, 1);
    assert_eq!(mask, 0x79);
    assert_eq!(ignore, 0);
    assert!(extra.is_null());
    for p in [start, motion, contact, normal] {
        assert_eq!(p as usize % 16, 0);
    }
    assert_eq!(ptr::read(motion as *const [f32; 4]), [0., -0.25, 0., 0.]);
    assert!((radius - 0.8).abs() < 0.0001);
    assert!((ptr::read(start.add(1)) - 3.3).abs() < 0.0001);
    ptr::write(flags, 0);
    ptr::write(fraction, 1.);
    ptr::write(
        hit,
        if CALLS.with(|c| c.borrow().placement == 5) {
            7
        } else {
            0xffffff
        },
    );
    hit
}
unsafe extern "system" fn map(_actor: usize, out: *mut i32) -> *const i32 {
    ptr::write(out, 777);
    out
}
unsafe extern "system" fn local(out: *mut f32, input: *const f32, map: *mut i32) -> u8 {
    assert_eq!(out as usize % 16, 0);
    assert_eq!(ptr::read(map), 777);
    ptr::write(
        out as *mut [f32; 4],
        [
            ptr::read(input) + 100.,
            ptr::read(input.add(1)),
            ptr::read(input.add(2)) - 100.,
            1.,
        ],
    );
    1
}
unsafe extern "system" fn owner(actor: usize) -> i32 {
    CALLS.with(|c| {
        let mut c = c.borrow_mut();
        assert_eq!(actor, c.creator);
        c.events.push("index");
    });
    2
}
unsafe extern "system" fn create(
    implementation: usize,
    out: *mut u64,
    owner: u32,
    slot: i32,
    parameters: *const i32,
    flag: u8,
) -> *mut u64 {
    CALLS.with(|c| {
        let mut c = c.borrow_mut();
        assert_eq!(implementation, c.implementation);
        assert_eq!((owner, slot, flag), (2, 2, 1));
        assert_eq!(
            ptr::read(parameters as *const [i32; 3]),
            [c.npc, 32500010, -1]
        );
        c.events.push("create");
    });
    ptr::write(out, 123);
    out
}
unsafe extern "system" fn resolve(_world: usize, handle: *const u64) -> usize {
    assert_eq!(ptr::read(handle), 123);
    CALLS.with(|c| c.borrow().actor)
}
unsafe extern "system" fn activate(actor: usize) {
    CALLS.with(|c| {
        let mut c = c.borrow_mut();
        assert_eq!(actor, c.actor);
        c.events.push("activate");
    });
}
unsafe extern "system" fn mode(actor: usize, mode: i32) {
    assert_eq!(mode, 5);
    CALLS.with(|c| {
        let mut c = c.borrow_mut();
        assert_eq!(actor, c.actor);
        c.events.push("mode");
    });
}
unsafe extern "system" fn teleport(actor: usize, position: *const f32, rotation: *const f32) {
    for (actual, expected) in ptr::read(position as *const [f32; 4])
        .iter()
        .zip([1., 2., 8., 1.])
    {
        assert!((actual - expected).abs() < 0.00001);
    }
    assert_eq!(
        ptr::read(rotation as *const [f32; 4]),
        [0., std::f32::consts::PI, 0., 0.]
    );
    CALLS.with(|c| {
        let mut c = c.borrow_mut();
        assert_eq!(actor, c.actor);
        c.events.push("teleport");
    });
}
unsafe extern "system" fn collision(_manager: usize, handle: *const u64, flag: i32) {
    assert_eq!(ptr::read(handle), 123);
    assert_eq!(flag, 1);
    CALLS.with(|c| c.borrow_mut().events.push("collision"));
}
unsafe extern "system" fn mask(_manager: usize, handle: *const u64, flag: i32) {
    assert_eq!(ptr::read(handle), 123);
    assert_eq!(flag, 0xfff);
    CALLS.with(|c| c.borrow_mut().events.push("mask"));
}
unsafe extern "system" fn broadcast(
    _manager: usize,
    kind: u32,
    packet: *const spirit::Packet,
    length: u32,
) {
    assert_eq!((kind, length), (0x1b, 48));
    CALLS.with(|c| {
        let mut c = c.borrow_mut();
        c.events.push("broadcast");
        c.packet = Some(ptr::read(packet));
    });
}
unsafe extern "system" fn dead(actor: usize) -> u8 {
    ptr::read((actor + 0x1f4) as *const u8) & 1
}
#[test]
fn death_observer_never_kills_live_spirits() {
    unsafe {
        let memory = VirtualAlloc(
            ptr::null_mut(),
            0x3c22000,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );
        assert!(!memory.is_null());
        let base = memory as usize;
        for (rva, function) in [
            (0x519140, resolve as *const ()),
            (SPIRIT_DEAD_RVA, dead as *const ()),
        ] {
            let mut code = vec![0x48, 0xb8];
            code.extend_from_slice(&(function as u64).to_le_bytes());
            code.extend_from_slice(&[0xff, 0xe0]);
            ptr::copy_nonoverlapping(code.as_ptr(), (base + rva) as *mut u8, code.len());
        }
        ptr::write((base + WORLD_RVA) as *mut usize, 1);
        let mut actor = vec![0usize; 0x5c0 / 8];
        let mut modules = vec![0usize; 0x170 / 8];
        let mut vtable = vec![0usize; 0x1a8 / 8];
        let mut reference = [0i32; 3];
        let a = actor.as_mut_ptr() as usize;
        actor[0] = vtable.as_mut_ptr() as usize;
        actor[0x60 / 8] = 1;
        actor[0x1b8 / 8] = modules.as_mut_ptr() as usize;
        modules[0x68 / 8] = 1;
        vtable[0x1a0 / 8] = base + 0x4d1ea0;
        reference[2] = spirit::FIRST_NPC;
        actor[0x5b8 / 8] = reference.as_mut_ptr() as usize;
        CALLS.with(|c| {
            *c.borrow_mut() = Calls {
                actor: 0,
                ..Calls::default()
            }
        });
        assert!(!soul_finished(a, base, 123));
        CALLS.with(|c| c.borrow_mut().actor = a);
        assert!(!soul_finished(a, base, 123));
        assert!(!soul_finished(a, base, 123));
        ptr::write((a + 0x1f4) as *mut u8, 1);
        assert!(soul_finished(a, base, 123));
        ptr::write((a + 0x1f4) as *mut u8, 0);
        ptr::write(reference.as_mut_ptr().add(2), 32500010);
        assert!(soul_finished(a, base, 123));
        assert_eq!(dead(a), 0);
        // Removed replicas can no longer resolve. The owner's native slot is
        // still sufficient to observe retirement, without killing anything.
        let mut wrapper = [0usize; 3];
        let mut implementation = vec![0u8; 0x230];
        wrapper[2] = implementation.as_mut_ptr() as usize;
        ptr::write_unaligned((modules.as_mut_ptr() as usize + 0x168) as *mut usize, wrapper.as_mut_ptr() as usize);
        ptr::write_unaligned(implementation.as_mut_ptr().add(0x50) as *mut u64, 123);
        CALLS.with(|c| c.borrow_mut().actor = 0);
        for state in [0, 1, 2, 3, 0xff] {
            ptr::write(implementation.as_mut_ptr().add(0x58), state);
            assert_eq!(soul_finished(a, base, 123), !matches!(state, 0 | 1));
        }
        CALLS.with(|c| assert!(c.borrow().events.is_empty()));
        assert_ne!(VirtualFree(memory, 0, MEM_RELEASE), 0);
    }
}
#[test]
fn native_spawn_uses_a_free_slot_and_preserves_six_argument_creation_and_four_argument_packet_abi()
{
    unsafe {
        let memory = VirtualAlloc(
            ptr::null_mut(),
            0x3c22000,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );
        assert!(!memory.is_null());
        let base = memory as usize;
        // Execute the verified native selector bytes, not a permissive mock.
        let selector: Vec<u8> = SPIRIT_SLOT_PATTERN
            .split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).unwrap())
            .collect();
        ptr::copy_nonoverlapping(
            selector.as_ptr(),
            (base + SPIRIT_SLOT_RVA) as *mut u8,
            selector.len(),
        );
        for (rva, function) in [
            (CHR_POSITION_RVA, position as *const ()),
            (SPIRIT_ANCHOR_RVA, anchor as *const ()),
            (SPIRIT_NPC_ROW_RVA, npc_row as *const ()),
            (SPIRIT_POSITION_RVA, correct as *const ()),
            (SPIRIT_SWEEP_RVA, sweep as *const ()),
            (0x3d0040, map as *const ()),
            (MAP_POSITION_RVA, local as *const ()),
            (0x3918d0, create as *const ()),
            (0x66c7d0, owner as *const ()),
            (0x519140, resolve as *const ()),
            (0x3d78d0, activate as *const ()),
            (0x3d7490, mode as *const ()),
            (0x3d7130, teleport as *const ()),
            (0x4eafe0, collision as *const ()),
            (0x4ea470, mask as *const ()),
            (0xdf24d0, broadcast as *const ()),
        ] {
            let mut code = vec![0x48, 0xb8];
            code.extend_from_slice(&(function as u64).to_le_bytes());
            code.extend_from_slice(&[0xff, 0xe0]);
            ptr::copy_nonoverlapping(code.as_ptr(), (base + rva) as *mut u8, code.len());
        }
        let mut world = vec![0usize; 0x17538 / 8];
        let mut player = vec![0usize; 0x1c0 / 8];
        let mut modules = vec![0usize; 0x170 / 8];
        let mut wrapper = vec![0usize; 3];
        let mut implementation = vec![0usize; 0x240 / 8];
        let mut actor = vec![0usize; 0x1c0 / 8];
        let mut npc_row = vec![0f32; 8];
        npc_row[6] = 0.8;
        let imp = implementation.as_mut_ptr() as usize;
        let player_ptr = player.as_mut_ptr() as usize;
        let actor_ptr = actor.as_mut_ptr() as usize;
        ptr::write(
            (base + WORLD_RVA) as *mut usize,
            world.as_mut_ptr() as usize,
        );
        ptr::write((base + 0x3c21ec8) as *mut usize, 1);
        world[0x17530 / 8] = 1;
        player[0x1b8 / 8] = modules.as_mut_ptr() as usize;
        modules[0x168 / 8] = wrapper.as_mut_ptr() as usize;
        modules[0x160 / 8] = wrapper.as_mut_ptr() as usize;
        wrapper[2] = imp;
        ptr::write(
            (base + 0x3c1e028) as *mut usize,
            world.as_mut_ptr() as usize,
        );
        world[0x98 / 8] = 1;
        ptr::write((base + 0x3c1d9a8) as *mut usize, 1);
        for (i, state) in [2u8, 3, 255, 0, 1, 2].into_iter().enumerate() {
            ptr::write((imp + 0x58 + i * 0x50) as *mut u8, state);
        }
        let original = implementation.clone();
        let source = crate::spirit_bosses::BOSS_NPCS[0];
        let npc = spirit::private_npc(source).unwrap();
        CALLS.with(|c| {
            *c.borrow_mut() = Calls {
                actor: actor_ptr,
                creator: player_ptr,
                implementation: imp,
                npc,
                npc_row: npc_row.as_ptr() as usize,
                ..Calls::default()
            }
        });
        assert_eq!(
            summon(
                player_ptr,
                spirit::Soul {
                    npc: source,
                    think: 32500010,
                    chara_init: -1
                },
                base
            ),
            None
        );
        CALLS.with(|c| assert!(c.borrow().events.is_empty()));
        player[0x60 / 8] = 1;
        modules[0x68 / 8] = 1;
        for mode in 1..=5 {
            CALLS.with(|c| {
                let mut c = c.borrow_mut();
                c.placement = mode;
                c.events.clear();
            });
            assert_eq!(
                summon(
                    player_ptr,
                    spirit::Soul {
                        npc: source,
                        think: 32500010,
                        chara_init: -1
                    },
                    base
                ),
                None
            );
            assert_eq!(implementation, original);
            CALLS.with(|c| assert_eq!(c.borrow().events, ["index"]));
        }
        CALLS.with(|c| {
            let mut c = c.borrow_mut();
            c.placement = 0;
            c.events.clear();
        });
        assert_eq!(
            summon(
                player_ptr,
                spirit::Soul {
                    npc: source,
                    think: 32500010,
                    chara_init: -1
                },
                base
            ),
            Some(123)
        );
        CALLS.with(|c| {
            let c = c.borrow();
            assert_eq!(
                c.events,
                [
                    "index",
                    "create",
                    "activate",
                    "mode",
                    "teleport",
                    "collision",
                    "mask",
                    "broadcast"
                ]
            );
            let p = c.packet.unwrap();
            assert_eq!(
                (p.handle, p.kind, p.npc, p.think, p.chara_init, p.map),
                (123, 0, npc, 32500010, -1, 777)
            );
            assert_eq!(p.position, [101., 2., -92.]);
            assert_eq!(p.yaw, std::f32::consts::PI);
        });
        let entry = imp + 0xf0;
        assert_eq!(
            ptr::read((entry + 0x30) as *const f32),
            std::f32::consts::PI
        );
        assert_eq!(
            ptr::read((entry + 0x38) as *const [f32; 3]),
            [101., 2., -92.]
        );
        assert!((ptr::read((entry + 0x20) as *const f32) - 1.).abs() < 0.00001);
        assert_eq!(ptr::read((entry + 0x28) as *const f32), 8.);
        assert_eq!(ptr::read(entry as *const u64), 123);
        assert_eq!(ptr::read((entry + 8) as *const u8), 0);
        assert_eq!(ptr::read((entry + 0x44) as *const u8), 1);
        for i in 0..6 {
            if i != 2 {
                assert_eq!(
                    &implementation[(0x50 + i * 0x50) / 8..(0x50 + (i + 1) * 0x50) / 8],
                    &original[(0x50 + i * 0x50) / 8..(0x50 + (i + 1) * 0x50) / 8]
                );
            }
        }
        // Every non-free byte must refuse creation, including the old >1 bug.
        // No native create/teardown, packet, or slot write may happen on refusal.
        for state in 0u8..255 {
            ptr::write((entry + 8) as *mut u8, state);
            let before = implementation.clone();
            CALLS.with(|c| c.borrow_mut().events.clear());
            assert_eq!(
                summon(
                    player_ptr,
                    spirit::Soul {
                        npc: source,
                        think: 32500010,
                        chara_init: -1
                    },
                    base
                ),
                None
            );
            assert_eq!(implementation, before);
            CALLS.with(|c| assert!(c.borrow().events.is_empty()));
        }
        // Native retirement (391780) alone returns the slot to FF. Repeat with
        // different NPC descriptors, keeping dying passive spirits untouched.
        for source in crate::spirit_bosses::BOSS_NPCS.iter().copied().take(12) {
            ptr::write((entry + 8) as *mut u8, 255);
            CALLS.with(|c| {
                let mut c = c.borrow_mut();
                c.npc = spirit::private_npc(source).unwrap();
                c.events.clear();
            });
            assert_eq!(
                summon(
                    player_ptr,
                    spirit::Soul {
                        npc: source,
                        think: 32500010,
                        chara_init: -1
                    },
                    base
                ),
                Some(123)
            );
            for state in [1, 2, 3] {
                ptr::write((entry + 8) as *mut u8, state);
                CALLS.with(|c| c.borrow_mut().events.clear());
                assert_eq!(
                    summon(
                        player_ptr,
                        spirit::Soul {
                            npc: source,
                            think: 32500010,
                            chara_init: -1
                        },
                        base
                    ),
                    None
                );
                CALLS.with(|c| assert!(c.borrow().events.is_empty()));
            }
        }
        assert_ne!(VirtualFree(memory, 0, MEM_RELEASE), 0);
    }
}
