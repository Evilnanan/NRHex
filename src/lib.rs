#![allow(non_snake_case, clippy::missing_safety_doc)]
pub mod hook;
pub mod log;
pub mod pattern;
pub mod pe;
pub mod win;
mod hex_pool;
mod reward_choices;
mod level_rewards;
mod personal_reward;
mod reward_recovery;
mod strength;
mod escape;
mod jiahao;
mod arts;
mod arts_network;
mod noisy;
mod balloon;
mod ability_damage;
mod glass_cannon;
mod hero_audio;
mod spirit;
mod spirit_placement;
mod spirit_npcs;
mod spirit_bosses;
mod spirit_cage_models;
mod native_actor;
mod actor_lifecycle;
mod balloon_runtime;
mod spirit_runtime;
use native_actor::npc_id;
use actor_lifecycle::{on_effect,death_detour};
use balloon_runtime::flush_balloon;
mod arts_data;
mod runtime;
mod crystal;
mod chant;
static CHANT:OnceLock<Mutex<chant::Chant>>=OnceLock::new();
const STAMINA_ADD_RVA:usize=0x4261d0;
const STAMINA_ADD_PATTERN:&str="03 91 5C 01 00 00 E9 25 1D 00 00";
const STAMINA_SET_PATTERN:&str="48 89 5C 24 08 89 54 24 10 57 48 83 EC 20 8B FA 48 8B D9 3B 91 5C 01 00 00 7D 4E";
mod status_sync;
mod status_display;
mod cat;
mod cat_jump;
mod wall_input;
mod flasks;
mod sword;
mod sword_map;
mod cage_mutation;
mod cage_trace;
use hex_pool::ORDERS;

use std::ffi::c_void;
use std::cell::Cell;
use std::path::PathBuf;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use sha2::{Digest, Sha256};

const MAGIC_ANIM_PATTERN:&str="48 83 EC 38 48 8B 01 48 8D 54 24 40 FF 50 18 48 8D 4C 24 20 8B 10 33 C0 89 44 24 24 48 89 44 24 28 C7 44 24 20 FF FF FF FF E8 ? ? ? ? 48 8B 44 24 28 48 85 C0 74 09 0F B6 40 29";
const SPEED_RVA:usize=0x3ff060;
const SPEED_PATTERN:&str="F3 0F 10 41 18 C3 45 0F B7 04 0B E9 21 58 DD FF 0F B6 41 79 C3";
const EXE_HASH: &str = "9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a";
const UPDATE_RVA: usize = 0x3e0ab0;
const SPAWN_RVA: usize = 0x5769d0;
const WORLD_RVA: usize = 0x3c0f0a8;
const ITEMS_RVA: usize = 0x3c10b68;
const GAME_DATA_RVA: usize = 0x3c078d0;
const MAP_CONTEXT_RVA:usize=0x3c123f8;
const GAME_DATA_PATTERN: &str = "48 8B 0D ? ? ? ? F3 48 0F 2C C0";
// Native hero selection reads GameDataMan -> current PlayerGameData -> hero ID.
const PLAYER_HERO_PATTERN:&str="48 8B 05 ? ? ? ? 48 8B 48 08 48 85 C9 74 20 0F B6 81 C7 00 00 00";
const REWARD_CREATE_RVA: usize = 0x5759e0;
const CHR_POSITION_RVA:usize=0x3d0e60;
const CHR_ROTATION_RVA:usize=0x3ceb20;
const CHR_ROTATION_PATTERN:&str="40 53 48 83 EC 20 48 8B 49 60 48 8B DA E8 8E D0 FC FF 48 8B C3 48 83 C4 20 5B C3";
const CHR_POSITION_PATTERN:&str="40 53 48 83 EC 20 48 8B 49 60 48 8B DA E8 DE AE FC FF 48 8B C3 48 83 C4 20 5B C3";
const MAP_POSITION_RVA:usize=0x631740;
const MAP_POSITION_PATTERN:&str="48 89 5C 24 10 48 89 6C 24 20 56 57 41 56 48 81 EC D0 00 00 00 48 8B 05 ? ? ? ? 48 33 C4 48 89 44 24 70 48 8B 2D ? ? ? ? 45 32 F6 49 8B D8 48 8B FA 48 8B F1";
const REWARD_HIDE_RVA:usize=0x5749e0;
const REWARD_HIDE_PATTERN:&str="40 53 48 83 EC 30 B8 00 08 00 00 C6 81 F8 00 00 00 00 48 8B D9 66 85 81 CA 00 00 00 74 0A 48 83 C4 30 5B E9 ? ? ? ?";
const REWARD_SHOW_RVA:usize=0x572220;
const REWARD_SHOW_PATTERN:&str="40 57 48 83 EC 50 B8 00 08 00 00 C6 81 F8 00 00 00 00 48 8B F9 66 85 81 CA 00 00 00 74 1A E8 ? ? ? ? 84 C0 0F 85 ? ? ? ?";
const REWARD_CREATE_PATTERN: &str = "48 89 5C 24 08 57 48 83 EC 20 48 8B FA 48 8B D9 48 8D 4A 58 E8 ? ? ? ? 84 C0 0F 84 2D 01 00 00";
const APPLY_RVA: usize = 0x3db410;
const NPC_REF_PATTERN:&str="48 8B 81 B8 05 00 00 C3";
const MUTATION_SOURCE_PATTERN:&str="40 53 48 83 EC 30 4C 8B 41 28 8B DA 4D 85 C0 74 56 48 8B 41 08 48 85 C0 74 4D F6 80 FB 01 00 00 01";
const MUTATION_LOOT_PATTERN:&str="48 8B 01 48 85 C0 74 07 8B 80 38 03 00 00 C3 B8 FF FF FF FF C3";
const MUTATION_REFRESH_PATTERN:&str="B2 01 E9 49 FF FF FF";
const FLASK_COUNT_RVA: usize = 0x66bd60;
const FLASK_COUNT_PATTERN: &str = "40 53 48 83 EC 20 48 8B 01 48 8B DA FF 90 90 01 00 00 48 85 C0 74 16 48 8D 88 D8 01 00 00 4C 8B C3 B2 01 48 83 C4 20 5B E9 93 68 B8 FF";
const ACTION_REQUEST_RVA: usize = 0x3e91a0;
const ACTION_REQUEST_PATTERN: &str = "48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 30 41 8B F8 8B F2 48 8B D9 E8 ? ? ? ? 4C 8B 88 B8 01 00 00";
const APPLY_SELF_RVA: usize = 0x3c8b10;
const HAS_EFFECT_RVA: usize = 0x508df0;
const GAUGE_RVA: usize = 0x48b870;
const THROW_ANIM_RVA:usize=0x490890;
const THROW_ANIM_PATTERN:&str="48 8B 49 10 E9 27 2A 00 00 6A 18 E9 E4 EB F4 00";
const GAUGE_ADD_PERCENT_RVA: usize = 0x48bc60;
const NULLIFY_SITE_RVA: usize = 0x43d5ee;
const REBOUND_RVA: usize = 0x436e50;
// Current EXE RTTI: CSChrDamageModule, not CSChrHitStopModule at slot 0x90.
const DAMAGE_MODULE_SLOT: usize = 0x98;
const DAMAGE_MODULE_VTABLE_RVA: usize = 0x2c11710;
const REBOUND_PATTERN: &str = "48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 20 57 48 83 EC 20 49 8B D8 48 8B EA 48 8B F1 49 8B C8 E8 ? ? ? ? 84 C0";
const NULLIFY_SITE_PATTERN: &str = "F6 86 C3 02 00 00 02 74 53 48 8D 4C 24 48 E8 ? ? ? ? 48 8B 08 48 85 C9 74 08";
const REMOVE_EFFECT_RVA: usize = 0x504e80;
const HIT_COMPLETE_RVA: usize = 0x43e27e;
const DAMAGE_RVA: usize = 0x439e80;
const HIT_BUILD_RVA: usize = 0xe70350;
const ATTACK_RATES_RVA: usize = 0x502830;
const EFFECT_FILTER_RVA:usize=0x511010;
const EFFECT_FILTER_PATTERN:&str="48 89 5C 24 18 48 89 74 24 20 57 41 56 41 57 48 83 EC 50 0F B6 02 32 DB 40 32 F6 45 8B F9 FF C8 4C 8B F2 48 8B F9 83 F8 0B";
const ATTACK_RATES_PATTERN:&str="48 8B C4 55 53 56 57 41 54 41 55 41 56 41 57 48 8D A8 A8 FD FF FF 48 81 EC 18 03 00 00 0F 29 70 A8 0F 29 78 98 44 0F 29 40 88";
const HIT_BUILD_PATTERN:&str="48 8B C4 48 89 58 10 55 56 57 41 54 41 55 41 56 41 57 48 8D A8 88 FE FF FF 48 81 EC 40 02 00 00 0F 29 70 B8 0F 29 78 A8 44 0F 29 40 98 48 8B 05 ? ? ? ? 48 33 C4 48 89 85 00 01 00 00 41 0F B6 F9 4D 8B E0 4C 8B FA 48 8B F1 49 8B 00 48 89 45 E0";
const DAMAGE_PATTERN: &str = "48 8B C4 48 89 58 20 55 56 57 41 54 41 55 41 56 41 57 48 8D A8 58 FB FF FF 48 81 EC 70 05 00 00 0F 29 70 B8 0F 29 78 A8";
const FORWARD_RVA: usize = 0x3cfc20;
const FORWARD_PATTERN: &str = "40 53 48 83 EC 20 48 8B 49 60 48 8B DA E8 2E BE FC FF 48 8B C3 48 83 C4 20 5B C3";
const POSITION_RVA: usize = 0x466620;
const POSITION_PATTERN: &str = "48 83 EC 68 0F 10 59 70 0F 10 61 60 F3 0F 10 15 ? ? ? ? 0F 28 C3 0F 28 2D ? ? ? ? 0F 28 CB";
const FALL_HEIGHT_RVA:usize=0x442070;
const FALL_HEIGHT_PATTERN:&str="48 83 EC 58 0F 29 74 24 40 48 8B 05 ? ? ? ? 48 33 C4 48 89 44 24 30 80 79 1D 00 48 89 5C 24 50 48 8B D9 74 0A";
const SPRING_HEIGHT_RVA:usize=0xba7000;
const SPRING_HEIGHT_PATTERN:&str="48 8B 41 18 48 85 C0 74 08 48 8B C8 E9 3F F4 FF FF 48 8B 89 60 02 00 00 48 85 C9 0F 85 FF C9 FE FF 0F 57 C0 C3";
const SPRING_CONTEXT_RVA:usize=0x3c17d48;
const SPRING_CONTEXT_PATTERN:&str="48 8B 0D 81 D8 81 03 48 85 C9 75 2E 48 8D 0D";
const HIT_COMPLETE_PATTERN: &str = "8B 45 A7 89 44 24 20 44 0F B6 CB 44 0F B6 C7 8B 96 80 02 00 00 49 8B CE E8 ? ? ? ? 48 8B CE";
const HAS_STATE_RVA: usize = 0x508e20;
const HAS_STATE_PATTERN: &str = "44 0F B7 D2 4C 8B D9 66 45 85 D2 48 B8 89 88 88 88 88 88 88 88 45 8B C2 0F 95 C1 49 F7 E2 48 C1 EA 03 48 6B C2 0F BA 0F 00 00 00 4C 2B C0 41 02 C8 C0 E1 02 48 D3 E2 49 85 53 28 74 34 49 8B 43 08 48 85 C0 74 2B F7 40 68 03 00 0C 80 75 19 48 8B 08 48 85 C9 74 09 0F B7 91 56 01 00 00 EB 02 33 D2 66 41 3B D2 74 06 48 8B 40 38 EB D3 B0 01 C3 32 C0 C3";
const REMOVE_EFFECT_PATTERN: &str = "48 83 EC 28 8B C2 48 8B 51 08 48 85 D2 74 0F 90 39 42 08 74 10 48 8B 52 38 48 85 D2 75 F2 32 C0 48 83 C4 28 C3 41 B0 01 E8 ? ? ? ? B0 01 48 83 C4 28 C3";
const GAUGE_ADD_PERCENT_PATTERN: &str = "40 53 48 83 EC 40 F6 41 31 04 48 8B D9 0F 29 74 24 30 0F 28 F1 0F 85 AC 00 00 00 F6 41 30 12 0F 29 7C 24 20 0F 57 FF 74 09 0F 2F FE 0F 87 90 00 00 00";
const APPLY_PATTERN: &str = "48 89 5C 24 20 55 56 57 48 83 EC 60 49 8B E9 49 8B F8 8B F2 48 8B D9 4D 85 C0 74 0E";
const APPLY_SELF_PATTERN: &str = "48 8B C4 48 89 58 08 48 89 70 10 57 48 81 EC E0 00 00 00 0F 28 05 ? ? ? ? 48 8B F1 0F 28 0D ? ? ? ? 48 8D 48 88 0F 29 40 A8 41 0F B6 D8 0F 29 48 B8 0F 57 C0 0F 28 0D ? ? ? ? 8B FA 0F 29 40 C8 0F 29 48 D8 66 C7 40 E8 00 00 E8 ? ? ? ? 0F 10 44 24 70 C7 44 24 60 10 00 00 00 48 8D 84 24 90 00 00 00 C6 44 24 58 01 44 0F B6 CB C6 44 24 50 00 4C 8B C6 C6 44 24 48 00 8B D7 C6 44 24 40 00 48 8B CE C6 44 24 38 00 C6 44 24 30 00 48 89 44 24 28 48 8D 84 24";
const HAS_EFFECT_PATTERN: &str = "48 8B 41 08 48 85 C0 74 17 F7 40 68 03 00 0C 80 75 05 39 50 08 74 06 48 8B 40 38 EB E7 B0 01 C3 C3";
const GAUGE_PATTERN: &str = "40 53 48 83 EC 40 0F 29 74 24 30 48 8B D9 F3 0F 10 35 ? ? ? ? E8 ? ? ? ? 48 8B C8";
const WORLD_PATTERN: &str = "48 8B 05 ? ? ? ? 0F 28 F1 48 85 C0";
const SPAWN_PATTERN: &str = "40 55 53 56 57 41 54 41 55 41 56 41 57 48 8D AC 24 48 FF FF FF 48 81 EC B8 01 00 00 48 8B 05 ? ? ? ? 48 33 C4 48 89 85 A0 00 00 00 45 0F B6 E9 44 88 4C 24 30 44 89 44 24 48 48 8B DA 48 89 54 24 68 4C 8B F9";
static MODULE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIG: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
static APPLY_ORIG: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
static NULLIFY_ORIG: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
static HIT_COMPLETE_ORIG: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
static DAMAGE_ORIG: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
static HIT_BUILD_ORIG: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
static ATTACK_RATES_ORIG: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
static EFFECT_FILTER_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static STATUS_SORT_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
const STATUS_SORT_RVA:usize=0x82ba40;
const STATUS_SORT_PATTERN:&str="48 89 5C 24 10 48 89 74 24 18 48 89 7C 24 20 55 41 56 41 57 48 8B EC 48 83 EC 60 0F 29 74 24 50";
static HUD_SORT_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
const HUD_SORT_RVA:usize=0x7df110;
const HUD_SORT_PATTERN:&str="E9 91 76 F5 FF 48 89 5C 24 08 57 48 83 EC 20 50 48 8D 05 31 B7 21 00 53 48 8B 5C 24 08 48 89";
static ACTION_REQUEST_ORIG: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
static REWARD_CREATE_ORIG: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
static NEXT_LEVEL_CHECK: AtomicU64 = AtomicU64::new(0);
static LEVEL_DELIVERY: OnceLock<Mutex<level_rewards::Delivery>> = OnceLock::new();
static REWARD_RECOVERY:OnceLock<Mutex<reward_recovery::Recovery>>=OnceLock::new();
thread_local! { static PERSONAL_REWARD: Cell<personal_reward::Scope> = const { Cell::new(personal_reward::Scope { manager:0, lot:0, slot:0, created:0 }) }; }
static CAT: OnceLock<Mutex<cat::Cat>> = OnceLock::new();
static CAT_LANDING:OnceLock<Mutex<cat_jump::Landing>>=OnceLock::new();
static SWORD: OnceLock<Mutex<sword::Healing>> = OnceLock::new();
static SWORD_BONUSES: OnceLock<sword::Bonuses> = OnceLock::new();
static SWORD_CAST_PLAYER: AtomicUsize = AtomicUsize::new(0);
static SWORD_LAST_CAST: AtomicU64 = AtomicU64::new(0);
static SWORD_CAST_MAGIC: AtomicI32 = AtomicI32::new(-1);
static SWORD_CAST_HAND: AtomicI32 = AtomicI32::new(-1);
static SHELL: OnceLock<Mutex<crystal::Shell>> = OnceLock::new();
static STATUS_REFRESH: OnceLock<Mutex<status_sync::Refresh>> = OnceLock::new();
static ESCAPE: OnceLock<Mutex<escape::Escape>> = OnceLock::new();
static ESCAPE_PLAYER:AtomicUsize=AtomicUsize::new(0);
static SPEED_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static JIAHAO:OnceLock<Mutex<jiahao::Stance>>=OnceLock::new();
static ARTS:OnceLock<Mutex<arts::Arts>>=OnceLock::new();
static NOISY:OnceLock<Mutex<noisy::Lava>>=OnceLock::new();
static BALLOON:OnceLock<Mutex<balloon::Balloon>>=OnceLock::new();
static SPIRITS:OnceLock<Mutex<spirit::Spirits>>=OnceLock::new();
static SPIRIT_MODEL_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
const SPIRIT_MODEL_RVA:usize=0xe94990;
const SPIRIT_MODEL_PATTERN:&str="85 D2 78 30 B8 59 17 B7 D1 F7 E2 B8 AD 8B DB 68 44 8B C2 41 C1 E8 0D 41 F7 E8";
const SPIRIT_DEAD_RVA:usize=0x3d45a0;
const SPIRIT_DEAD_PATTERN:&str="48 8B 41 60 48 8B 88 C8 00 00 00 0F B6 41 24 24 01 C3";
const SPIRIT_ANCHOR_RVA:usize=0x3886a0;
const SPIRIT_ANCHOR_PATTERN:&str="48 8B C4 48 89 58 08 48 89 70 20 55 57 41 54 41 56 41 57 48 8D A8 78 FE FF FF 48 81 EC 60 02 00";
const SPIRIT_SWEEP_RVA:usize=0xdb0b80;
const SPIRIT_SWEEP_PATTERN:&str="40 53 55 56 57 48 81 EC B8 00 00 00 41 0F 10 09 48 8B DA 48 8B 84 24 00 01 00 00 4C 8D 4C 24 70";
const SPIRIT_POSITION_RVA:usize=0x385410;
const SPIRIT_POSITION_PATTERN:&str="48 8B C4 55 53 56 57 41 56 48 8D A8 C8 FE FF FF 48 81 EC 10 02 00 00 0F 29 70 C8 0F 29 78 B8 44";
const SPIRIT_NPC_ROW_RVA:usize=0xe954f0;
const SPIRIT_NPC_ROW_PATTERN:&str="48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 48 89 7C 24 20 41 56 48 83 EC 30 8B F2 4C 8B F1 33 FF 0F B7 DF 85 D2 0F 88 3F 01 00 00 48 8B 0D 5D 33 D9 02 48 85 C9 75 2E 48 8D 0D 75 F1 D6 02 E8";
const SPIRIT_SLOT_RVA:usize=0x3922c0;
const SPIRIT_SLOT_PATTERN:&str="33 C0 8B D0 48 83 C1 58 80 39 FF 74 14 FF C0 48 FF C2 48 83 C1 50 48 83 FA 06 7C EC B8 FF FF FF FF C3";
static DEATH_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
const DEATH_RVA:usize=0x3dd3d0;
static SELF_MARK_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static LAVA_SFX_CREATE_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static LAVA_SFX_UPDATE_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static LAVA_CLEANUP_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static SFX_MATRIX_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
const SELF_MARK_RVA:usize=0xb9ea10;
const SELF_MARK_PATTERN:&str="48 8B C4 48 89 58 20 55 56 57 41 56 41 57 48 8D 68 A1 48 81 EC 00 01 00 00 0F 29 70 C8 0F 29 78 B8 44 0F 29 40 A8";
const LAVA_SFX_CREATE_RVA:usize=0x353590;
const LAVA_SFX_CREATE_PATTERN:&str="48 8B C4 48 89 58 18 48 89 70 20 55 57 41 54 41 56 41 57 48 8D 68 A9 48 81 EC 00 01 00 00 0F 29 70 C8 0F 29 78 B8";
const LAVA_SFX_UPDATE_RVA:usize=0x355d20;
const LAVA_SFX_UPDATE_PATTERN:&str="48 8B C4 48 89 58 10 57 48 81 EC 10 01 00 00 0F 29 70 E8 0F 29 78 D8 44 0F 29 40 C8 44 0F 29 48 B8 44 0F 29 50 A8 44 0F 29 58 98 48 8B F9 0F 28 05 ? ? ? ? 0F 29 44 24 30 0F 28 0D ? ? ? ? 0F 29 4C 24 40 0F 28 05 ? ? ? ? 0F 29 44 24 50 0F 28 0D ? ? ? ? 0F 29 4C 24 60 83 B9 88 05 00 00 00";
const LAVA_CLEANUP_RVA:usize=0x353cb0;
const LAVA_CLEANUP_PATTERN:&str="48 89 5C 24 10 57 48 83 EC 20 48 8B D9 48 83 C1 68 E8 ? ? ? ? 85 C0 74 5A 0F B6 83 83 05 00 00 3C FF";
const SFX_MATRIX_RVA:usize=0xed60c0;
const SFX_MATRIX_PATTERN:&str="48 89 5C 24 10 57 48 83 EC 20 48 8B FA 48 8B D9 48 83 39 00 74 43";
const CHR_HANDLE_RVA:usize=0x63f840;
const CHR_HANDLE_PATTERN:&str="48 89 4C 24 08 48 83 EC 28 48 8B D1 48 C1 EA 1C 83 E2 0F 0F 84 4D 01 00 00 83 EA 01 0F 84 FB 00 00 00";
thread_local! {static LAVA_VISUAL_SCALE:Cell<f32>=const{Cell::new(1.)};}
struct LavaVisualGuard(f32);
impl LavaVisualGuard{fn enter(scale:f32)->Self{Self(LAVA_VISUAL_SCALE.with(|c|c.replace(scale)))}}
impl Drop for LavaVisualGuard{fn drop(&mut self){LAVA_VISUAL_SCALE.with(|c|c.set(self.0));}}
static ARTS_ID_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static ARTS_PARAM_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static ARTS_RAW_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static ARTS_BEHAVIOR_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static ARTS_DROP_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static ARTS_SERIALIZE_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static ARTS_REWARD_COPY_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static ARTS_PICKUP_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static ARTS_ITEM_CREATE_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
static ARTS_BULLET_ORIG:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
const ARTS_BULLET_RVA:usize=0x3975d0;
const ARTS_BULLET_PATTERN:&str="48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 48 89 7C 24 20 41 54 41 56 41 57 48 81 EC 10 01 00 00 49 8B F9 41 8B D8 44 8B E2 48 8B F1";
type ArtsBulletFn=unsafe extern "system" fn(usize,i32,i32,usize,i32,i32,u8,u8,u8,usize)->u8;
const ARTS_DROP_RVA:usize=0x7e91a0;
const ARTS_SERIALIZE_RVA:usize=0x6c16f0;
const ARTS_REWARD_COPY_RVA:usize=0x570410;
const ARTS_PICKUP_RVA:usize=0x57ca60;
const ARTS_ITEM_CREATE_RVA:usize=0x57b490;
const ARTS_DROP_PATTERN:&str="40 53 56 57 48 81 EC 90 00 00 00 48 8B 05 ? ? ? ? 48 33 C4 48 89 84 24 80 00 00 00 41 0F B6 F0 8B FA 48 8B D9";
const ARTS_SERIALIZE_PATTERN:&str="48 89 5C 24 10 48 89 74 24 18 55 57 41 56 48 8D 6C 24 B9 48 81 EC 90 00 00 00 48 8B FA 4C 8B F1";
// Two descriptor-copy entries have identical bodies. Guard the researched
// direct call to 570410 in native drop creation instead of guessing by prefix.
const ARTS_REWARD_COPY_PATTERN:&str="48 8D 55 80 48 8D 4D 40 E8 5B B4 FF FF";
const ARTS_PICKUP_PATTERN:&str="40 55 53 56 57 41 54 41 55 41 56 41 57 48 8D 6C 24 F9 48 81 EC E8 00 00 00 48 8B 05 E8 94 56 03";
const ARTS_ITEM_CREATE_PATTERN:&str="48 89 5C 24 08 48 89 54 24 10 57 48 83 EC 40 49 8B F8 48 8B DA C7 44 24 20 00 00 00 00 48 8B 0D 14 7D 69 03";
#[derive(Clone,Copy)]
struct ArtsDrop {weapon:arts::Weapon,payload:[u8;0x48]}
#[derive(Clone,Copy,Default)]
struct ArtsTransfer {active:bool,drop:Option<ArtsDrop>,pickup_uid:u32,pickup_payload:usize}
thread_local! {static ARTS_TRANSFER:Cell<ArtsTransfer>=Cell::new(ArtsTransfer::default());}
// Nested native calls must not replace the enclosing pickup's incoming UID.
struct ArtsTransferGuard(ArtsTransfer);
impl ArtsTransferGuard {fn enter(scope:ArtsTransfer)->Self{Self(ARTS_TRANSFER.with(|c|c.replace(scope)))}}
impl Drop for ArtsTransferGuard {fn drop(&mut self){ARTS_TRANSFER.with(|c|c.set(self.0));}}
type ArtsPickupFn=unsafe extern "system" fn(usize,usize,usize,usize,usize,u8,i32)->usize;
const ARTS_BEHAVIOR_RVA:usize=0x668b30;
const ARTS_BEHAVIOR_PATTERN:&str="89 54 24 10 55 56 57 41 55 41 56 41 57 48 8D 6C 24 D9 48 81 EC 88 00 00 00";
const PC_BEHAVIOR_LOOKUP_PATTERN:&str="48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 48 89 7C 24 20 41 56 48 83 EC 30 41 0F B6 E8 8B FA 48 8B F1 33 DB 85 D2 0F 88 2D 01 00 00 40 80 FD 02";
type ArtsBehaviorFn=unsafe extern "system" fn(usize,i32,i32,u32,u8)->i32;
const ARTS_ID_RVA:usize=0x6a20b0;
const ARTS_PARAM_RVA:usize=0x6a2180;
const ARTS_ID_PATTERN:&str="48 8B C4 48 89 58 08 48 89 70 10 57 48 83 EC 70 48 8B F9 33 DB 89 58 A8 8D 53 FF 48 8B 71 08";
const ARTS_PARAM_PATTERN:&str="E9 22 AF F3 06 41 52 4C 8D 15 8C A8";
const ARTS_REF_RVA:usize=0x6a1d70;
const ARTS_RAW_RVA:usize=0x6a2240;
const ARTS_RAW_PATTERN:&str="48 8B C4 48 89 70 10 57 48 83 EC 50 48 8B 79 08 48 8B F1 48 85 FF";
const ARTS_REF_PATTERN:&str="48 89 4C 24 08 53 48 83 EC 20 48 8B D9 45 33 C0 E8 FB EF FF FF 90 48 8B";
static RUNTIME: OnceLock<Mutex<runtime::Runtime>> = OnceLock::new();
static FLASKS: OnceLock<Mutex<flasks::Flasks>> = OnceLock::new();
static BASE: AtomicUsize = AtomicUsize::new(0);
static ACTIVE: AtomicBool = AtomicBool::new(false);
#[cfg(feature="test-hotkeys")]
static KEY_WAS_DOWN: AtomicBool = AtomicBool::new(false);
#[cfg(feature="test-hotkeys")]
static TEST_KEY_WAS_DOWN: AtomicBool = AtomicBool::new(false);
#[cfg(feature="test-hotkeys")]
static NEW_HEX_KEY_WAS_DOWN: AtomicBool = AtomicBool::new(false);
#[cfg(feature="test-hotkeys")]
static CHANT_KEY_WAS_DOWN: AtomicBool = AtomicBool::new(false);
// Current development focus: Monster Cage, Crystal Shell, Versatile Arts.
#[cfg(any(feature="test-hotkeys",test))]
const TEST_REWARD: [usize;3] = [14,6,12];
#[cfg(any(feature="test-hotkeys",test))]
const NEW_HEX_REWARD: [usize;3] = [15,16,0];
#[cfg(any(feature="test-hotkeys",test))]
const CHANT_REWARD: [usize;3] = [17,8,0];
#[cfg(feature="test-hotkeys")]
static LAST_SPAWN: AtomicU64 = AtomicU64::new(0);
static CALLBACK_LOCK: AtomicBool = AtomicBool::new(false);
static CONFIG: OnceLock<Config> = OnceLock::new();

#[no_mangle]
pub extern "system" fn NRHex_status()->u32 {ACTIVE.load(Ordering::Acquire) as u32}

/// Inspect the actual on-disk EXE with the same signatures as the live guard.
pub fn verify_executable(path:&std::path::Path)->Result<(),String>{
 let bytes=std::fs::read(path).map_err(|e|e.to_string())?;
 let hash=format!("{:x}",Sha256::digest(&bytes));
 if hash!=EXE_HASH{return Err(format!("Unsupported EXE hash {hash}"));}
 let image=pe::FileImage::parse(&bytes)?;let regions=image.exec_regions();
 for (sig,wanted) in signatures() {
  let hits=pattern::scan_regions(&regions,&pattern::Pattern::parse(sig)?);
  if hits.len()!=1||hits[0].1.rva!=wanted{return Err(format!("Expected {wanted:#x}, found {:?}",hits.iter().map(|v|v.1.rva).collect::<Vec<_>>()));}
 }
 Ok(())
}

const NEW_3DD3D0_PATTERN:&str="48 89 5C 24 10 48 89 74 24 18 57 48 83 EC 50 48 8B D9 33 FF 89 7C 24 30 48 8B 81 B8 01 00 00 C6";
const NEW_3918D0_PATTERN:&str="48 89 5C 24 08 55 56 57 41 54 41 55 41 56 41 57 48 8D 6C 24 B0 48 81 EC 50 01 00 00 48 8B 05 75";
const NEW_519140_PATTERN:&str="48 89 5C 24 08 48 89 54 24 10 56 57 41 56 48 83 EC 20 48 8B FA 48 8B D9 33 F6 48 8B CA E8 6E EA";
const NEW_5197E0_PATTERN:&str="48 89 5C 24 18 55 57 41 56 48 83 EC 20 4C 8D B1 40 9A 00 00 8B DA 49 8B 06 49 8B CE FF 10 33 FF";
const NEW_460870_PATTERN:&str="48 83 EC 28 48 8B 49 10 48 85 C9 74 68 48 8D 44 24 30 48 89 44 24 40 E8 14 5C F2 FF 48 8B C8 8B";
const NEW_47F890_PATTERN:&str="48 83 EC 28 48 8B 49 10 48 85 C9 74 68 48 8D 44 24 30 48 89 44 24 40 E8 A4 0F F1 FF 48 8B C8 8B";
const NEW_3D0040_PATTERN:&str="C7 02 FF FF FF FF 8B 41 40 83 F8 FF 75 03 8B 41 44 89 02 48 8B C2 C3 48 8D 4D 0F E9 E2 31 88 00";
const NEW_3D78D0_PATTERN:&str="40 53 48 83 EC 20 48 8B 41 18 48 8B D9 F6 40 0A 04 75 0B 48 8B 49 60 B2 01 E8 62 56 FC FF 48 8B";
const NEW_3D7490_PATTERN:&str="48 89 5C 24 08 57 48 83 EC 20 48 83 3D 96 7E 83 03 00 8B DA 48 8B F9 75 27 48 8D 0D 73 D1 82 03";
const NEW_3D7130_PATTERN:&str="48 89 5C 24 08 57 48 83 EC 20 0F 28 02 48 8B F9 0F 11 81 80 00 00 00 49 8B D8 0F 28 02 0F 11 81";
const NEW_DF24D0_PATTERN:&str="48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 57 41 56 41 57 48 83 EC 30 48 8B B9 80 00 00 00 41";
const NEW_4EAFE0_PATTERN:&str="48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 30 41 0F B6 F0 48 8B FA 48 8B D9 48 8B CA E8 AF CB FF";
const NEW_4EA470_PATTERN:&str="48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 30 41 8B D8 48 8B FA 48 8B F1 48 8B 05 19 4C 72 03 48";
const NEW_392810_PATTERN:&str="48 89 5C 24 10 48 89 74 24 18 55 57 41 56 48 8D 6C 24 B9 48 81 EC D0 00 00 00 48 8B 05 37 37 75";

const NEW_66C7D0_PATTERN:&str="40 53 48 83 EC 20 48 8B 81 A0 05 00 00 48 85 C0 75 05 E8 D9 00 BA FF 8B 58 08 E8 D1 19 B9 FF 84";

const NEAR_DEATH_STATE_PATTERN:&str="80 B9 10 07 00 00 01 0F 94 C0 C3";
fn signatures()->[(&'static str,u32);94] {
 [(STAMINA_ADD_PATTERN,STAMINA_ADD_RVA as u32),(STAMINA_SET_PATTERN,0x427f00),(WORLD_PATTERN,0x3e0ace),(SPAWN_PATTERN,SPAWN_RVA as u32),
  (APPLY_PATTERN,APPLY_RVA as u32),(APPLY_SELF_PATTERN,APPLY_SELF_RVA as u32),
  (HAS_EFFECT_PATTERN,HAS_EFFECT_RVA as u32),(GAUGE_PATTERN,GAUGE_RVA as u32),
  (NEAR_DEATH_STATE_PATTERN,0x66d700),
  (THROW_ANIM_PATTERN,THROW_ANIM_RVA as u32),(GAUGE_ADD_PERCENT_PATTERN,GAUGE_ADD_PERCENT_RVA as u32),
  (NULLIFY_SITE_PATTERN,NULLIFY_SITE_RVA as u32),(REMOVE_EFFECT_PATTERN,REMOVE_EFFECT_RVA as u32),
  (REBOUND_PATTERN,REBOUND_RVA as u32),
  (HIT_COMPLETE_PATTERN,HIT_COMPLETE_RVA as u32),(HAS_STATE_PATTERN,HAS_STATE_RVA as u32),
  (hero_audio::PATTERN,hero_audio::RVA as u32),(glass_cannon::OBJECT_PATTERN,glass_cannon::OBJECT_RVA as u32),(glass_cannon::PATTERN,glass_cannon::RVA as u32),(DAMAGE_PATTERN,DAMAGE_RVA as u32),(FORWARD_PATTERN,FORWARD_RVA as u32),(POSITION_PATTERN,POSITION_RVA as u32),
  (ACTION_REQUEST_PATTERN,ACTION_REQUEST_RVA as u32),
  (FLASK_COUNT_PATTERN,FLASK_COUNT_RVA as u32),
  (GAME_DATA_PATTERN,0x1fd598),(REWARD_CREATE_PATTERN,REWARD_CREATE_RVA as u32),
  (PLAYER_HERO_PATTERN,0xb61462),
  (status_display::ROUTE_PATTERN,status_display::ROUTE_RVA as u32),
  (STATUS_SORT_PATTERN,STATUS_SORT_RVA as u32),(HUD_SORT_PATTERN,HUD_SORT_RVA as u32),(SPEED_PATTERN,SPEED_RVA as u32),(HIT_BUILD_PATTERN,HIT_BUILD_RVA as u32),(MAGIC_ANIM_PATTERN,0x456870),
  (ATTACK_RATES_PATTERN,ATTACK_RATES_RVA as u32),(EFFECT_FILTER_PATTERN,EFFECT_FILTER_RVA as u32),
  (ability_damage::BASE_PATTERN,ability_damage::BASE_RVA as u32),
  (FALL_HEIGHT_PATTERN,FALL_HEIGHT_RVA as u32),
  (SPRING_HEIGHT_PATTERN,SPRING_HEIGHT_RVA as u32),(SPRING_CONTEXT_PATTERN,0x3fa4c0),
  (CHR_POSITION_PATTERN,CHR_POSITION_RVA as u32),(CHR_ROTATION_PATTERN,CHR_ROTATION_RVA as u32),(MAP_POSITION_PATTERN,MAP_POSITION_RVA as u32),
  (REWARD_HIDE_PATTERN,REWARD_HIDE_RVA as u32),(REWARD_SHOW_PATTERN,REWARD_SHOW_RVA as u32),
  (ARTS_ID_PATTERN,ARTS_ID_RVA as u32),(ARTS_PARAM_PATTERN,ARTS_PARAM_RVA as u32),(ARTS_REF_PATTERN,ARTS_REF_RVA as u32),
  (ARTS_RAW_PATTERN,ARTS_RAW_RVA as u32),
  (arts_network::SNAPSHOT_PATTERN,arts_network::SNAPSHOT_RVA as u32),
  (ARTS_BEHAVIOR_PATTERN,ARTS_BEHAVIOR_RVA as u32),(PC_BEHAVIOR_LOOKUP_PATTERN,0xe6fcb0),
  (ARTS_DROP_PATTERN,ARTS_DROP_RVA as u32),(ARTS_SERIALIZE_PATTERN,ARTS_SERIALIZE_RVA as u32),
  // Return-site allowlist used by the transient equipment export wrapper.
  ("E8 EE 00 34 00",0x3815fd),("E8 73 FF 33 00",0x381778),
  ("E8 DF 4B 4B 00",0x20cb0c),("E8 1D 36 BB FF",0xb0e0ce),("E8 65 75 AD FF",0xbea186),
  (ARTS_REWARD_COPY_PATTERN,0x574fa8),(ARTS_PICKUP_PATTERN,ARTS_PICKUP_RVA as u32),
  (ARTS_ITEM_CREATE_PATTERN,ARTS_ITEM_CREATE_RVA as u32),(ARTS_BULLET_PATTERN,ARTS_BULLET_RVA as u32),
  (SELF_MARK_PATTERN,SELF_MARK_RVA as u32),(LAVA_SFX_CREATE_PATTERN,LAVA_SFX_CREATE_RVA as u32),
  (LAVA_SFX_UPDATE_PATTERN,LAVA_SFX_UPDATE_RVA as u32),(SFX_MATRIX_PATTERN,SFX_MATRIX_RVA as u32),
  (CHR_HANDLE_PATTERN,CHR_HANDLE_RVA as u32),(LAVA_CLEANUP_PATTERN,LAVA_CLEANUP_RVA as u32),
  (NPC_REF_PATTERN,0x4d1ea0),(MUTATION_SOURCE_PATTERN,0x4fff30),(MUTATION_LOOT_PATTERN,0xe95360),
  (MUTATION_REFRESH_PATTERN,0x50ed10),(NEW_3DD3D0_PATTERN,0x3dd3d0),(NEW_3918D0_PATTERN,0x3918d0),(NEW_519140_PATTERN,0x519140),(NEW_5197E0_PATTERN,0x5197e0),(NEW_460870_PATTERN,0x460870),(NEW_47F890_PATTERN,0x47f890),(NEW_3D0040_PATTERN,0x3d0040),(NEW_3D78D0_PATTERN,0x3d78d0),(NEW_3D7490_PATTERN,0x3d7490),(NEW_3D7130_PATTERN,0x3d7130),(NEW_DF24D0_PATTERN,0xdf24d0),(NEW_4EAFE0_PATTERN,0x4eafe0),(NEW_4EA470_PATTERN,0x4ea470),(NEW_392810_PATTERN,0x392810),(NEW_66C7D0_PATTERN,0x66c7d0),(SPIRIT_MODEL_PATTERN,SPIRIT_MODEL_RVA as u32),(SPIRIT_DEAD_PATTERN,SPIRIT_DEAD_RVA as u32),(SPIRIT_ANCHOR_PATTERN,SPIRIT_ANCHOR_RVA as u32),(SPIRIT_POSITION_PATTERN,SPIRIT_POSITION_RVA as u32),(SPIRIT_SWEEP_PATTERN,SPIRIT_SWEEP_RVA as u32),(SPIRIT_NPC_ROW_PATTERN,SPIRIT_NPC_ROW_RVA as u32),(SPIRIT_SLOT_PATTERN,SPIRIT_SLOT_RVA as u32)]
}

#[cfg(feature="test-hotkeys")]
#[link(name="user32")]
extern "system" {
 fn GetAsyncKeyState(key:i32)->i16;
 fn GetForegroundWindow()->*mut c_void;
 fn GetWindowThreadProcessId(window:*mut c_void,pid:*mut u32)->u32;
}
#[link(name="kernel32")]
extern "system" { fn GetTickCount64()->u64; }
#[link(name="bcrypt")]
extern "system" { fn BCryptGenRandom(handle:*mut c_void,buffer:*mut u8,len:u32,flags:u32)->i32; }

#[derive(Debug)]
struct Config { key:i32, test_key:i32, new_hex_key:i32, chant_key:i32, ctrl:bool, cooldown:u64 }
impl Config {
 fn read(text:&str)->Result<Self,String> {
  let mut c=Self{key:0x75,test_key:0x76,new_hex_key:0x77,chant_key:0x78,ctrl:true,cooldown:3000};
  for line in text.lines().map(str::trim).filter(|s|!s.is_empty()&&!s.starts_with([';','#','['])) {
   let (k,v)=line.split_once('=').ok_or("Expected key=value in NRHex.ini")?;
   match k.trim() {
    "VirtualKey"=>{let v=v.trim();c.key=if let Some(s)=v.strip_prefix("0x"){i32::from_str_radix(s,16)}else{v.parse()}.map_err(|_|"Invalid VirtualKey")?;},
    "TestVirtualKey"=>{let v=v.trim();c.test_key=if let Some(s)=v.strip_prefix("0x"){i32::from_str_radix(s,16)}else{v.parse()}.map_err(|_|"Invalid TestVirtualKey")?;},
    "NewHexVirtualKey"=>{let v=v.trim();c.new_hex_key=if let Some(s)=v.strip_prefix("0x"){i32::from_str_radix(s,16)}else{v.parse()}.map_err(|_|"Invalid NewHexVirtualKey")?;},
    "ChantVirtualKey"|"AngelChantVirtualKey"=>{let v=v.trim();c.chant_key=if let Some(s)=v.strip_prefix("0x"){i32::from_str_radix(s,16)}else{v.parse()}.map_err(|_|"Invalid ChantVirtualKey")?;},
    // Older INIs remain loadable, but cannot override the compiled range.
    "SwordNearDistance"=>{},
    "RequireControl"=>c.ctrl=v.trim().parse().map_err(|_|"RequireControl must be true/false")?,
    "CooldownMs"=>c.cooldown=v.trim().parse().map_err(|_|"Invalid CooldownMs")?,
    _=>return Err(format!("Unknown setting {k}"))
   }
  }
  let keys=[c.key,c.test_key,c.new_hex_key,c.chant_key];
  if keys.iter().any(|k|!(1..=254).contains(k))||keys.iter().enumerate().any(|(i,k)|keys[i+1..].contains(k))||c.cooldown<1000 {return Err("Hotkeys must be distinct and 1..254; CooldownMs >=1000".into());}
  Ok(c)
 }
}

unsafe fn module_path(h:*mut c_void)->Result<PathBuf,String> {
 let mut b=[0u16;32768];let n=win::GetModuleFileNameW(h,b.as_mut_ptr(),b.len() as u32);
 if n==0||n as usize>=b.len(){return Err("GetModuleFileNameW failed".into());}
 Ok(PathBuf::from(win::from_wide(&b[..n as usize])))
}

#[no_mangle]
pub unsafe extern "system" fn DllMain(h:*mut c_void,reason:u32,_:*mut c_void)->i32 {
 if reason==win::DLL_PROCESS_ATTACH {
  MODULE.store(h,Ordering::Release);win::DisableThreadLibraryCalls(h);
  let thread=win::CreateThread(ptr::null_mut(),0,worker,ptr::null_mut(),0,ptr::null_mut());
  if !thread.is_null(){win::CloseHandle(thread);}
 }
 win::TRUE
}
unsafe extern "system" fn worker(_:*mut c_void)->u32 {
 let dir=match module_path(MODULE.load(Ordering::Acquire)){Ok(p)=>p.parent().unwrap().to_path_buf(),Err(_)=>return 1};
 log::init(&dir,"NRHex.log",log::Level::Info);
 nlog_info!("NRHex {} initializing; PC attack kind=1; continuous casting guard; pool={} abilities / {} ordered choices",env!("CARGO_PKG_VERSION"),hex_pool::HEX_COUNT,ORDERS.len());
 let init=(||->Result<(),String>{
  let text=std::fs::read_to_string(dir.join("NRHex.ini")).map_err(|e|e.to_string())?;
  let config=Config::read(&text)?;nlog_info!("settings: {config:?}");CONFIG.set(config).map_err(|_|"Already initialized")?;
  nlog_info!("Test reward hotkeys compiled in: {}",cfg!(feature="test-hotkeys"));
  SWORD_BONUSES.set(sword::Bonuses::defaults()).map_err(|_|"Sword bonuses already initialized")?;
  nlog_info!("Sword shared effect percentages initialized from the compiled user balance table");
  SWORD.set(Mutex::new(sword::Healing::default())).map_err(|_|"Sword already initialized")?;
  let base=pe::main_module_base()?;
  let bytes=std::fs::read(module_path(base as *mut c_void)?).map_err(|e|e.to_string())?;
  let hash=format!("{:x}",Sha256::digest(bytes));
  if hash!=EXE_HASH{return Err(format!("Unsupported EXE SHA256 {hash}; no hooks installed"));}
  let image=pe::ModuleImage::from_base(base)?;let regions=image.exec_regions();
  for (pattern,wanted) in signatures() {
   let hits=pattern::scan_regions(&regions,&pattern::Pattern::parse(pattern)?);
   if hits.len()!=1||hits[0].1.rva!=wanted {return Err(format!("Signature mismatch at RVA {wanted:#x}; hits {:?}; no hooks installed",hits.iter().map(|v|v.1.rva).collect::<Vec<_>>()));}
  }
  let update_head=[0x48,0x8b,0xc4,0x48,0x89,0x58,0x10,0x48,0x89,0x78,0x18,0x55,0x48,0x8d,0x68,0xa1];
  if std::slice::from_raw_parts(base.add(UPDATE_RVA),update_head.len())!=update_head {return Err("Character update prologue mismatch".into());}
  // Check the singleton references from their actual RIP-relative instructions.
  for (rva,wanted) in [(0x3e0ace,WORLD_RVA),(0x37f528,ITEMS_RVA),(0x1fd598,GAME_DATA_RVA)] {
   let offset=ptr::read_unaligned(base.add(rva+3) as *const i32);
   if (rva as isize+7+offset as isize) as usize!=wanted {return Err("Singleton reference mismatch".into());}
  }
  BASE.store(base as usize,Ordering::Release);
  JIAHAO.set(Mutex::new(jiahao::Stance::default())).map_err(|_|"Jiahao already initialized")?;
  NOISY.set(Mutex::new(noisy::Lava::default())).map_err(|_|"Noisy already initialized")?;
  ARTS.set(Mutex::new(arts::Arts::default())).map_err(|_|"Arts already initialized")?;
  ESCAPE.set(Mutex::new(escape::Escape::default())).map_err(|_|"Escape already initialized")?;
  RUNTIME.set(Mutex::new(runtime::Runtime::default())).map_err(|_|"Runtime already initialized")?;
  BALLOON.set(Mutex::new(balloon::Balloon::default())).map_err(|_|"Balloon already initialized")?;
  SPIRITS.set(Mutex::new(spirit::Spirits::default())).map_err(|_|"Spirits already initialized")?;
  FLASKS.set(Mutex::new(flasks::Flasks::default())).map_err(|_|"Flasks already initialized")?;
  SHELL.set(Mutex::new(crystal::Shell::default())).map_err(|_|"Shell already initialized")?;
  CAT.set(Mutex::new(cat::Cat::default())).map_err(|_|"Cat already initialized")?;
  CAT_LANDING.set(Mutex::new(cat_jump::Landing::default())).map_err(|_|"Cat landing already initialized")?;
  LEVEL_DELIVERY.set(Mutex::new(level_rewards::Delivery::default())).map_err(|_|"Level delivery already initialized")?;
  REWARD_RECOVERY.set(Mutex::new(reward_recovery::Recovery::default())).map_err(|_|"Reward recovery already initialized")?;
  let speed_target=base.add(SPEED_RVA) as *mut u8;
  let speed_prepared=hook::prepare(speed_target,speed_detour as *const u8,6)?;
  if speed_prepared.stolen!=5{return Err("Speed getter must steal exactly its five-byte MOVSS".into());}
  SPEED_ORIG.store(speed_prepared.trampoline,Ordering::Release);
  let target=base.add(UPDATE_RVA) as *mut u8;
  let prepared=hook::prepare(target,update_detour as *const u8,64)?;
  let apply_target=base.add(APPLY_RVA) as *mut u8;
  let apply_prepared=hook::prepare(apply_target,effect_detour as *const u8,64)?;
  let sort_target=base.add(STATUS_SORT_RVA) as *mut u8;
  let sort_prepared=hook::prepare(sort_target,status_sort_detour as *const u8,64)?;
  STATUS_SORT_ORIG.store(sort_prepared.trampoline,Ordering::Release);
  let hud_sort_target=base.add(HUD_SORT_RVA) as *mut u8;
  let hud_sort_prepared=hook::prepare(hud_sort_target,hud_sort_detour as *const u8,64)?;
  if hud_sort_prepared.stolen!=5{return Err("HUD comparator must relocate its five-byte native jump".into());}
  HUD_SORT_ORIG.store(hud_sort_prepared.trampoline,Ordering::Release);
  let status_route_target=base.add(status_display::ROUTE_RVA) as *mut u8;
  let status_route_prepared=hook::prepare(status_route_target,status_display::route_detour as *const u8,32)?;
  if status_route_prepared.stolen!=7{return Err("Status routing must relocate only MOVD/CMP (7 bytes)".into());}
  status_display::ORIGINAL.store(status_route_prepared.trampoline,Ordering::Release);
  status_display::CONTINUE.store(status_route_target.add(7),Ordering::Release);
  let nullify_target=base.add(NULLIFY_SITE_RVA) as *mut u8;
  let nullify_prepared=hook::prepare(nullify_target,crystal_detour as *const u8,64)?;
  if nullify_prepared.stolen!=7 {return Err("Nullify site must relocate exactly its 7-byte test instruction".into());}
  let balloon_target=base.add(ability_damage::BASE_RVA) as *mut u8;
  let balloon_prepared=hook::prepare(balloon_target,ability_damage::base_detour as *const u8,64)?;
  if balloon_prepared.stolen!=5{return Err("Balloon base site must relocate MOVD XMM8,ECX (5 bytes)".into());}
  ability_damage::ORIGINAL.store(balloon_prepared.trampoline,Ordering::Release);
  let glass_target=base.add(glass_cannon::RVA) as *mut u8;
  let glass_prepared=hook::prepare(glass_target,glass_cannon::detour as *const u8,64)?;
  glass_cannon::ORIGINAL.store(glass_prepared.trampoline,Ordering::Release);
  let hit_target=base.add(HIT_COMPLETE_RVA) as *mut u8;
  let hit_prepared=hook::prepare(hit_target,cat_detour as *const u8,64)?;
  if hit_prepared.stolen!=7 {return Err("Hit completion site must relocate exactly two instructions (7 bytes)".into());}
  let damage_target=base.add(DAMAGE_RVA) as *mut u8;
  let damage_prepared=hook::prepare(damage_target,rear_detour as *const u8,64)?;
  let request_target=base.add(ACTION_REQUEST_RVA) as *mut u8;
  let request_prepared=hook::prepare(request_target,action_request_detour as *const u8,64)?;
  let reward_target=base.add(REWARD_CREATE_RVA) as *mut u8;
  let reward_prepared=hook::prepare(reward_target,reward_create_detour as *const u8,64)?;
  let dispatch_target=base.add(HIT_BUILD_RVA) as *mut u8;
  let dispatch_prepared=hook::prepare(dispatch_target,strength_detour as *const u8,64)?;
  let rates_target=base.add(ATTACK_RATES_RVA) as *mut u8;
  let rates_prepared=hook::prepare(rates_target,sword_rates_detour as *const u8,64)?;
  let filter_target=base.add(EFFECT_FILTER_RVA) as *mut u8;
  let filter_prepared=hook::prepare(filter_target,sword_filter_detour as *const u8,64)?;
  let arts_id_target=base.add(ARTS_ID_RVA) as *mut u8;
  let arts_id_prepared=hook::prepare(arts_id_target,arts_id_detour as *const u8,64)?;
  let arts_param_target=base.add(ARTS_PARAM_RVA) as *mut u8;
  let arts_param_prepared=hook::prepare(arts_param_target,arts_param_detour as *const u8,5)?;
  let arts_raw_target=base.add(ARTS_RAW_RVA) as *mut u8;
  let arts_raw_prepared=hook::prepare(arts_raw_target,arts_raw_detour as *const u8,64)?;
  let arts_behavior_target=base.add(ARTS_BEHAVIOR_RVA) as *mut u8;
  let arts_behavior_prepared=hook::prepare(arts_behavior_target,arts_behavior_detour as *const u8,64)?;
  let arts_transfer_targets=[
   (ARTS_DROP_RVA,arts_drop_detour as *const u8,&ARTS_DROP_ORIG),
   (ARTS_SERIALIZE_RVA,arts_serialize_wrapper as *const u8,&ARTS_SERIALIZE_ORIG),
   (ARTS_REWARD_COPY_RVA,arts_reward_copy_detour as *const u8,&ARTS_REWARD_COPY_ORIG),
   (ARTS_PICKUP_RVA,arts_pickup_detour as *const u8,&ARTS_PICKUP_ORIG),
   (ARTS_ITEM_CREATE_RVA,arts_item_create_detour as *const u8,&ARTS_ITEM_CREATE_ORIG),
   (ARTS_BULLET_RVA,arts_bullet_detour as *const u8,&ARTS_BULLET_ORIG),
   (SELF_MARK_RVA,self_mark_detour as *const u8,&SELF_MARK_ORIG),
   (LAVA_SFX_CREATE_RVA,lava_sfx_create_detour as *const u8,&LAVA_SFX_CREATE_ORIG),
   (LAVA_SFX_UPDATE_RVA,lava_sfx_update_detour as *const u8,&LAVA_SFX_UPDATE_ORIG),
   (SFX_MATRIX_RVA,sfx_matrix_detour as *const u8,&SFX_MATRIX_ORIG),
   (LAVA_CLEANUP_RVA,lava_cleanup_detour as *const u8,&LAVA_CLEANUP_ORIG),
   (glass_cannon::OBJECT_RVA,glass_cannon::object_detour as *const u8,&glass_cannon::OBJECT_ORIGINAL),
   (hero_audio::RVA,hero_audio::detour as *const u8,&hero_audio::ORIGINAL),
   (DEATH_RVA,death_detour as *const u8,&DEATH_ORIG),
   (SPIRIT_MODEL_RVA,spirit_runtime::spirit_model_detour as *const u8,&SPIRIT_MODEL_ORIG)];
  let mut arts_transfer_prepared=Vec::new();
  for (rva,detour,original) in arts_transfer_targets {
   let p=hook::prepare(base.add(rva) as *mut u8,detour,64)?;
   original.store(p.trampoline,Ordering::Release);arts_transfer_prepared.push((rva,p));
  }
  if arts_param_prepared.stolen!=5{return Err("Arts param entry must relocate its five-byte native jump".into());}
  // Publish before patching: the detour always has a valid original target.
  ORIG.store(prepared.trampoline,Ordering::Release);
  APPLY_ORIG.store(apply_prepared.trampoline,Ordering::Release);
  NULLIFY_ORIG.store(nullify_prepared.trampoline,Ordering::Release);
  HIT_COMPLETE_ORIG.store(hit_prepared.trampoline,Ordering::Release);
  DAMAGE_ORIG.store(damage_prepared.trampoline,Ordering::Release);
  ACTION_REQUEST_ORIG.store(request_prepared.trampoline,Ordering::Release);
  REWARD_CREATE_ORIG.store(reward_prepared.trampoline,Ordering::Release);
  HIT_BUILD_ORIG.store(dispatch_prepared.trampoline,Ordering::Release);
  ATTACK_RATES_ORIG.store(rates_prepared.trampoline,Ordering::Release);
  EFFECT_FILTER_ORIG.store(filter_prepared.trampoline,Ordering::Release);
  ARTS_ID_ORIG.store(arts_id_prepared.trampoline,Ordering::Release);
  ARTS_PARAM_ORIG.store(arts_param_prepared.trampoline,Ordering::Release);
  ARTS_RAW_ORIG.store(arts_raw_prepared.trampoline,Ordering::Release);
  ARTS_BEHAVIOR_ORIG.store(arts_behavior_prepared.trampoline,Ordering::Release);
  let mut result=Ok(());
  hook::with_threads_frozen(true,||{
   result=hook::write_code(speed_target,&speed_prepared.patch)
    .and_then(|_|hook::write_code(apply_target,&apply_prepared.patch))
    .and_then(|_|hook::write_code(sort_target,&sort_prepared.patch))
    .and_then(|_|hook::write_code(hud_sort_target,&hud_sort_prepared.patch))
    .and_then(|_|hook::write_code(status_route_target,&status_route_prepared.patch))
    .and_then(|_|hook::write_code(nullify_target,&nullify_prepared.patch))
    .and_then(|_|hook::write_code(balloon_target,&balloon_prepared.patch))
    .and_then(|_|hook::write_code(glass_target,&glass_prepared.patch))
    .and_then(|_|hook::write_code(hit_target,&hit_prepared.patch))
    .and_then(|_|hook::write_code(damage_target,&damage_prepared.patch))
    .and_then(|_|hook::write_code(request_target,&request_prepared.patch))
    .and_then(|_|hook::write_code(reward_target,&reward_prepared.patch))
    .and_then(|_|hook::write_code(dispatch_target,&dispatch_prepared.patch))
    .and_then(|_|hook::write_code(rates_target,&rates_prepared.patch))
    .and_then(|_|hook::write_code(filter_target,&filter_prepared.patch))
    .and_then(|_|hook::write_code(arts_id_target,&arts_id_prepared.patch))
    .and_then(|_|hook::write_code(arts_param_target,&arts_param_prepared.patch))
    .and_then(|_|hook::write_code(arts_raw_target,&arts_raw_prepared.patch))
    .and_then(|_|hook::write_code(arts_behavior_target,&arts_behavior_prepared.patch))
    .and_then(|_|{
     for (rva,p) in &arts_transfer_prepared {hook::write_code(base.add(*rva) as *mut u8,&p.patch)?;}
     Ok(())
    })
    .and_then(|_|hook::write_code(target,&prepared.patch));
  });
  result?;ACTIVE.store(true,Ordering::Release);
  nlog_info!("Installed character-update hook at RVA {UPDATE_RVA:#x}. Random key={:#x}; cage/crystal/arts key={:#x}; waiting for the local player.",CONFIG.get().unwrap().key,CONFIG.get().unwrap().test_key);
  Ok(())
 })();
 if let Err(e)=init {nlog_error!("Disabled: {e}");return 1;}
 0
}

// Preserve the complete entry register state and tail-call the original.
// We do not guess the original function's return type.
#[unsafe(naked)]
unsafe extern "system" fn update_detour() {
 core::arch::naked_asm!(
  "sub rsp, 0xc8",
  "mov [rsp+0x20], rax", "mov [rsp+0x28], rcx", "mov [rsp+0x30], rdx",
  "mov [rsp+0x38], r8", "mov [rsp+0x40], r9", "mov [rsp+0x48], r10", "mov [rsp+0x50], r11",
  "movdqu [rsp+0x60], xmm0", "movdqu [rsp+0x70], xmm1", "movdqu [rsp+0x80], xmm2",
  "movdqu [rsp+0x90], xmm3", "movdqu [rsp+0xa0], xmm4", "movdqu [rsp+0xb0], xmm5",
  "call {callback}",
  "movdqu xmm0, [rsp+0x60]", "movdqu xmm1, [rsp+0x70]", "movdqu xmm2, [rsp+0x80]",
  "movdqu xmm3, [rsp+0x90]", "movdqu xmm4, [rsp+0xa0]", "movdqu xmm5, [rsp+0xb0]",
  "mov rax, [rsp+0x20]", "mov rcx, [rsp+0x28]", "mov rdx, [rsp+0x30]",
  "mov r8, [rsp+0x38]", "mov r9, [rsp+0x40]", "mov r10, [rsp+0x48]", "mov r11, [rsp+0x50]",
  "add rsp, 0xc8", "jmp qword ptr [rip+{original}]",
  callback=sym on_update,original=sym ORIG
 );
}

// Pre-calculation entry: preserve registers and original stack arguments.
#[unsafe(naked)]
unsafe extern "system" fn sword_rates_detour() {
 core::arch::naked_asm!(
  "sub rsp, 0xc8",
  "mov [rsp+0x20], rax", "mov [rsp+0x28], rcx", "mov [rsp+0x30], rdx",
  "mov [rsp+0x38], r8", "mov [rsp+0x40], r9", "mov [rsp+0x48], r10", "mov [rsp+0x50], r11",
  "movdqu [rsp+0x60], xmm0", "movdqu [rsp+0x70], xmm1", "movdqu [rsp+0x80], xmm2",
  "movdqu [rsp+0x90], xmm3", "movdqu [rsp+0xa0], xmm4", "movdqu [rsp+0xb0], xmm5",
  "call {callback}",
  "movdqu xmm0, [rsp+0x60]", "movdqu xmm1, [rsp+0x70]", "movdqu xmm2, [rsp+0x80]",
  "movdqu xmm3, [rsp+0x90]", "movdqu xmm4, [rsp+0xa0]", "movdqu xmm5, [rsp+0xb0]",
  "mov rax, [rsp+0x20]", "mov rcx, [rsp+0x28]", "mov rdx, [rsp+0x30]",
  "mov r8, [rsp+0x38]", "mov r9, [rsp+0x40]", "mov r10, [rsp+0x48]", "mov r11, [rsp+0x50]",
  "add rsp, 0xc8", "jmp qword ptr [rip+{original}]",
  callback=sym on_sword_attack_rates,original=sym ATTACK_RATES_ORIG
 );
}

// 502830 evaluates the attacker's ordinary AttackRate effects and caches them
// in context+90/+a4..b0 before E70350 constructs a hit. Tagging the hit is too late.
// Identify the actual native effect manager; a projectile context may use a
// generic source pointer. Preserve the original spell categories and conditions.
unsafe extern "system" fn on_sword_attack_rates(manager:usize,context:usize){
 if !ACTIVE.load(Ordering::Acquire)||manager==0||context==0{return;}
 let Some(bonuses)=SWORD_BONUSES.get() else{return;};
 if bonuses.for_attack(read_i32(context+8),read_i32(context+0xc)).is_none(){return;}
 let base=BASE.load(Ordering::Acquire);let world=read_ptr(base+WORLD_RVA);
 if world==0{return;}
 let local=read_ptr(world+0x174e8);
 if local==0{return;}
 // Projectile creation can use a copied manager; 502C5D reads its ChrIns owner.
 if manager!=read_ptr(local+0x190)&&read_ptr(manager+0x10)!=local{return;}
 if !has_effect(local,sword::OWNER,base){return;}
 let mut categories=ptr::read_unaligned((context+0x14) as *const [u32;8]);
 sword::add_melee_category(&mut categories);
 ptr::write_unaligned((context+0x14) as *mut [u32;8],categories);
}

// Keep the original magic/prayer evaluation. Only a failed, tagged Sword spell
// may additionally use native weapon evaluation for melee-filtered rows.
// Retry a copy, retaining the original descriptor and every native condition.
unsafe extern "system" fn sword_filter_detour(node:usize,descriptor:usize,owner:usize,weapon:i32,extra:i32)->u8{
 let original:NativeEffectFilter=std::mem::transmute(EFFECT_FILTER_ORIG.load(Ordering::Acquire));
 let result=original(node,descriptor,owner,weapon,extra);
 if result!=0||!ACTIVE.load(Ordering::Acquire)||node==0||descriptor==0{return result;}
 let param=read_ptr(node);if param==0{return result;}
 let mask=ptr::read_unaligned((descriptor+4) as *const [u32;8]);
 let filters=[ptr::read((param+0x13a) as *const u8),ptr::read((param+0x13b) as *const u8),ptr::read((param+0x328) as *const u8)];
 if !sword::extra_melee_application(ptr::read(descriptor as *const u8),&mask,filters){return result;}
 let Some(bonuses)=SWORD_BONUSES.get() else{return result;};
 if bonuses.for_attack(read_i32(descriptor+0x34),read_i32(descriptor+0x38)).is_none(){return result;}
 let base=BASE.load(Ordering::Acquire);
 if !is_local(owner,base)||!has_effect(owner,sword::OWNER,base){return result;}
 sword_retry_as_melee(original,node,descriptor,owner,weapon,extra)
}
type NativeEffectFilter=unsafe extern "system" fn(usize,usize,usize,i32,i32)->u8;
unsafe fn sword_retry_as_melee(original:NativeEffectFilter,node:usize,descriptor:usize,owner:usize,weapon:i32,extra:i32)->u8{
 let mut melee=ptr::read_unaligned(descriptor as *const [u8;0x3c]);melee[0]=1;
 original(node,melee.as_ptr() as usize,owner,weapon,extra)
}

// Mid-function site: RSP is aligned to 16, R14 is the damage module,
// RSI is the hit context and R13B is the original special-hit flag.
// Preserve flags and all volatile GP/XMM registers; the trampoline executes
// the original test after our callback and its original branch consumes flags.
#[unsafe(naked)]
unsafe extern "system" fn crystal_detour() {
 core::arch::naked_asm!(
  "pushfq", "sub rsp, 0xc8",
  "mov [rsp+0x20], rax", "mov [rsp+0x28], rcx", "mov [rsp+0x30], rdx",
  "mov [rsp+0x38], r8", "mov [rsp+0x40], r9", "mov [rsp+0x48], r10", "mov [rsp+0x50], r11",
  "movdqu [rsp+0x60], xmm0", "movdqu [rsp+0x70], xmm1", "movdqu [rsp+0x80], xmm2",
  "movdqu [rsp+0x90], xmm3", "movdqu [rsp+0xa0], xmm4", "movdqu [rsp+0xb0], xmm5",
  "mov rcx, r14", "mov rdx, rsi", "movzx r8d, r13b", "mov r9, r12", "call {callback}",
  "movdqu xmm0, [rsp+0x60]", "movdqu xmm1, [rsp+0x70]", "movdqu xmm2, [rsp+0x80]",
  "movdqu xmm3, [rsp+0x90]", "movdqu xmm4, [rsp+0xa0]", "movdqu xmm5, [rsp+0xb0]",
  "mov rax, [rsp+0x20]", "mov rcx, [rsp+0x28]", "mov rdx, [rsp+0x30]",
  "mov r8, [rsp+0x38]", "mov r9, [rsp+0x40]", "mov r10, [rsp+0x48]", "mov r11, [rsp+0x50]",
  "add rsp, 0xc8", "popfq", "jmp qword ptr [rip+{original}]",
  callback=sym on_crystal_hit,original=sym NULLIFY_ORIG
 );
}

// Same aligned mid-function ABI; RBP retains the native pre-hit HP slot.
#[unsafe(naked)]
unsafe extern "system" fn cat_detour() {
 core::arch::naked_asm!(
  "pushfq", "sub rsp, 0xc8",
  "mov [rsp+0x20], rax", "mov [rsp+0x28], rcx", "mov [rsp+0x30], rdx",
  "mov [rsp+0x38], r8", "mov [rsp+0x40], r9", "mov [rsp+0x48], r10", "mov [rsp+0x50], r11",
  "movdqu [rsp+0x60], xmm0", "movdqu [rsp+0x70], xmm1", "movdqu [rsp+0x80], xmm2",
  "movdqu [rsp+0x90], xmm3", "movdqu [rsp+0xa0], xmm4", "movdqu [rsp+0xb0], xmm5",
  "mov rcx, r14", "mov rdx, rsi", "mov r8d, [rbp-0x59]", "mov r9, r12", "call {callback}",
  "movdqu xmm0, [rsp+0x60]", "movdqu xmm1, [rsp+0x70]", "movdqu xmm2, [rsp+0x80]",
  "movdqu xmm3, [rsp+0x90]", "movdqu xmm4, [rsp+0xa0]", "movdqu xmm5, [rsp+0xb0]",
  "mov rax, [rsp+0x20]", "mov rcx, [rsp+0x28]", "mov rdx, [rsp+0x30]",
  "mov r8, [rsp+0x38]", "mov r9, [rsp+0x40]", "mov r10, [rsp+0x48]", "mov r11, [rsp+0x50]",
  "add rsp, 0xc8", "popfq", "jmp qword ptr [rip+{original}]",
  callback=sym on_cat_hit,original=sym HIT_COMPLETE_ORIG
 );
}

// Calculate first, then adjust the completed per-recipient HP damage. Preserve
// the three native arguments and the original volatile return registers.
// The native calculator overwrites hit+280 for every evaluation/recipient.
#[unsafe(naked)]
unsafe extern "system" fn rear_detour() {
 core::arch::naked_asm!(
  "sub rsp,0xe8",
  "mov [rsp+0x28],rax", "mov [rsp+0x30],rcx", "mov [rsp+0x38],rdx",
  "mov [rsp+0x40],r8", "mov [rsp+0x48],r9", "mov [rsp+0x50],r10", "mov [rsp+0x58],r11",
  "movdqu [rsp+0x60],xmm0", "movdqu [rsp+0x70],xmm1", "movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3", "movdqu [rsp+0xa0],xmm4", "movdqu [rsp+0xb0],xmm5",
  "mov [rsp+0xc0],rcx", "mov [rsp+0xc8],rdx", "mov [rsp+0xd0],r8",
  "call {before}",
  "movdqu xmm0,[rsp+0x60]", "movdqu xmm1,[rsp+0x70]", "movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]", "movdqu xmm4,[rsp+0xa0]", "movdqu xmm5,[rsp+0xb0]",
  "mov rax,[rsp+0x28]", "mov rcx,[rsp+0x30]", "mov rdx,[rsp+0x38]",
  "mov r8,[rsp+0x40]", "mov r9,[rsp+0x48]", "mov r10,[rsp+0x50]", "mov r11,[rsp+0x58]",
  "call qword ptr [rip+{original}]",
  "mov [rsp+0x28],rax", "mov [rsp+0x30],rcx", "mov [rsp+0x38],rdx",
  "mov [rsp+0x40],r8", "mov [rsp+0x48],r9", "mov [rsp+0x50],r10", "mov [rsp+0x58],r11",
  "movdqu [rsp+0x60],xmm0", "movdqu [rsp+0x70],xmm1", "movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3", "movdqu [rsp+0xa0],xmm4", "movdqu [rsp+0xb0],xmm5",
  "mov rcx,[rsp+0xc0]", "mov rdx,[rsp+0xc8]", "mov r8,[rsp+0xd0]", "call {after}",
  "movdqu xmm0,[rsp+0x60]", "movdqu xmm1,[rsp+0x70]", "movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]", "movdqu xmm4,[rsp+0xa0]", "movdqu xmm5,[rsp+0xb0]",
  "mov rax,[rsp+0x28]", "mov rcx,[rsp+0x30]", "mov rdx,[rsp+0x38]",
  "mov r8,[rsp+0x40]", "mov r9,[rsp+0x48]", "mov r10,[rsp+0x50]", "mov r11,[rsp+0x58]",
  "add rsp,0xe8", "ret",
  before=sym on_sword_before_calculation,after=sym on_rear_calculated,original=sym DAMAGE_ORIG
 );
}

// Native hit construction precedes calculation, serialization and cached-hit replay.
// Seven arguments: RCX output, RDX detail, R8 source context, R9B flag,
// stack arg5 int, arg6 pointer and arg7 float; preserve all return registers.
#[unsafe(naked)]
unsafe extern "system" fn strength_detour() {
 core::arch::naked_asm!(
  "sub rsp,0x108",
  "mov [rsp+0x40],rax",
  "mov [rsp+0x48],rcx",
  "mov [rsp+0x50],rdx",
  "mov [rsp+0x58],r8",
  "mov [rsp+0x60],r9",
  "mov [rsp+0x68],r10",
  "mov [rsp+0x70],r11",
  "movdqu [rsp+0x80],xmm0",
  "movdqu [rsp+0x90],xmm1",
  "movdqu [rsp+0xa0],xmm2",
  "movdqu [rsp+0xb0],xmm3",
  "movdqu [rsp+0xc0],xmm4",
  "movdqu [rsp+0xd0],xmm5",
  "mov [rsp+0xe0],rcx", "mov [rsp+0xe8],r8",
  "mov rcx,r8", "call {before}",
  "mov rcx,[rsp+0x48]", "mov rdx,[rsp+0x50]", "mov r8,[rsp+0x58]",
  "mov r9,[rsp+0x60]", "mov r10,[rsp+0x68]", "mov r11,[rsp+0x70]",
  "movdqu xmm0,[rsp+0x80]", "movdqu xmm1,[rsp+0x90]", "movdqu xmm2,[rsp+0xa0]",
  "movdqu xmm3,[rsp+0xb0]", "movdqu xmm4,[rsp+0xc0]", "movdqu xmm5,[rsp+0xd0]",
  "mov rax,[rsp+0x130]", "mov [rsp+0x20],rax",
  "mov rax,[rsp+0x138]", "mov [rsp+0x28],rax",
  "mov rax,[rsp+0x140]", "mov [rsp+0x30],rax",
  "mov rax,[rsp+0x40]", "call qword ptr [rip+{original}]",
  "mov [rsp+0x40],rax",
  "mov [rsp+0x48],rcx",
  "mov [rsp+0x50],rdx",
  "mov [rsp+0x58],r8",
  "mov [rsp+0x60],r9",
  "mov [rsp+0x68],r10",
  "mov [rsp+0x70],r11",
  "movdqu [rsp+0x80],xmm0",
  "movdqu [rsp+0x90],xmm1",
  "movdqu [rsp+0xa0],xmm2",
  "movdqu [rsp+0xb0],xmm3",
  "movdqu [rsp+0xc0],xmm4",
  "movdqu [rsp+0xd0],xmm5",
  "mov rcx,[rsp+0xe0]", "mov rdx,[rsp+0xe8]", "call {after}",
  "movdqu xmm0,[rsp+0x80]",
  "movdqu xmm1,[rsp+0x90]",
  "movdqu xmm2,[rsp+0xa0]",
  "movdqu xmm3,[rsp+0xb0]",
  "movdqu xmm4,[rsp+0xc0]",
  "movdqu xmm5,[rsp+0xd0]",
  "mov rax,[rsp+0x40]",
  "mov rcx,[rsp+0x48]",
  "mov rdx,[rsp+0x50]",
  "mov r8,[rsp+0x58]",
  "mov r9,[rsp+0x60]",
  "mov r10,[rsp+0x68]",
  "mov r11,[rsp+0x70]",
  "add rsp,0x108", "ret",
  before=sym ability_damage::before_construct,after=sym on_strength_constructed,original=sym HIT_BUILD_ORIG
 );
}
unsafe fn strengthen_constructed_hit(hit:usize) {
 let original=ptr::read((hit+0x24) as *const u8);
 ptr::write((hit+0x24) as *mut u8,strength::remap(original));
}
unsafe extern "system" fn on_strength_constructed(hit:usize,context:usize) {
 if !ACTIVE.load(Ordering::Acquire)||hit==0||context==0{return;}
 let base=BASE.load(Ordering::Acquire);let world=read_ptr(base+WORLD_RVA);
 if world==0{return;}
 let local=read_ptr(world+0x174e8);
 // Native E7039B and E70A72 confirm context[0] is the generic source;
 // compare first before treating it as the local player's ChrIns.
 // Deliberately exclude projectile sources, even when their resolved owner
 // is local. Mighty boosts direct character attacks, not ranged projectiles.
 if local==0||read_ptr(context)!=local{return;}
 if has_effect(local,strength::OWNER,base){strengthen_constructed_hit(hit);}
}

unsafe fn spatial_ready(player:usize)->bool {
 if player==0 {return false;}
 let coord=read_ptr(player+0x60);let modules=read_ptr(player+0x1b8);
 coord!=0&&modules!=0&&read_ptr(modules+0x68)!=0
}

unsafe fn position(player:usize,base:usize)->[f32;4] {
 let _=base;
 let physics=read_ptr(read_ptr(player+0x1b8)+0x68);
 ptr::read_unaligned((physics+0x70) as *const [f32;4])
}
// Receiver-side native category calculation also covers bullet/network contexts
// whose original construction did not expose the player's source pointer.
unsafe extern "system" fn on_sword_before_calculation(module:usize,source:usize,hit:usize){
 glass_cannon::enter(module,source,hit);
 if !ACTIVE.load(Ordering::Acquire)||module==0||hit==0{return;}
 let base=BASE.load(Ordering::Acquire);
 if !is_local(source,base)||!has_effect(source,sword::OWNER,base)||spell_percentage(hit).is_none(){return;}
 let mut categories=ptr::read_unaligned((hit+0x124) as *const [u32;8]);
 sword::add_melee_category(&mut categories);
 ptr::write_unaligned((hit+0x124) as *mut [u32;8],categories);
}
unsafe extern "system" fn on_rear_calculated(module:usize,source:usize,hit:usize) {
 glass_cannon::leave();
 if !ACTIVE.load(Ordering::Acquire)||module==0||hit==0 {return;}
 let base=BASE.load(Ordering::Acquire);
 if ptr::read((hit+0x2c1) as *const u8)&8!=0 {
  return; // Native calculator returned without recomputing.
 }
 // Comparing to the local player first avoids interpreting generic attackers
 // (environment/bullets) as ChrIns; only the actual owner receives this bonus.
 let world=read_ptr(base+WORLD_RVA);if world==0{return;}
 let local=read_ptr(world+0x174e8);if local==0{return;}
 let target=read_ptr(module+8);
 let mut damage=read_i32(hit+0x280);
 if target==local&&damage>0&&has_effect(local,sword::OWNER,base)&&sword_casting(local,true,true){
  if let Some(percentage)=SWORD_BONUSES.get().and_then(|values|values.for_magic(SWORD_CAST_MAGIC.load(Ordering::Acquire))){
   damage=sword::reduced(damage,percentage);ptr::write_unaligned((hit+0x280) as *mut i32,damage);
  }
 }
 if source!=local{return;}
 if damage>0&&noisy::lava_attack(read_i32(hit+0x40),read_i32(hit+0x44)){
  // Native calculation already includes eligible fire/ordinary attack buffs.
  // Apply only the self-ping stack factor here, never those buffs a second time.
  let factor=NOISY.get().and_then(|state|state.lock().ok()).map(|l|l.factor(local)).unwrap_or(1.);
  damage=sword::boosted(damage,factor);ptr::write_unaligned((hit+0x280) as *mut i32,damage);
 }
 if damage>0&&source!=target&&has_effect(local,jiahao::OWNER,base)
    &&jiahao::is_backstep_attack(&ptr::read_unaligned((hit+0x124) as *const [u32;8])){
  let modules=read_ptr(local+0x1b8);
  if modules!=0{
   let stats=read_ptr(modules);let gauge_module=read_ptr(modules+0x148);
   if stats!=0&&gauge_module!=0&&read_i32(stats+0x140)>0{
    // Native calculation precedes HP consumption and on_cat_hit's +25% reward.
    let get_gauge:unsafe extern "system" fn(usize)->f32=std::mem::transmute(base+GAUGE_RVA);
    damage=jiahao::boosted(damage,get_gauge(gauge_module));
    ptr::write_unaligned((hit+0x280) as *mut i32,damage);
   }
  }
 }
 if damage>0&&source!=target&&has_effect(local,sword::OWNER,base){
  if let Some(factor)=spell_factor(hit){
   // Ordinary NPCs bypass the player's defensive random-nullify section at
   // 43D53B. Gate here, before generic HP and impact consumers instead.
   if !sword::near(sword_hit_distance(source,hit,base),sword::NEAR_DISTANCE){
     let flags=ptr::read((hit+0x2c3) as *const u8);ptr::write((hit+0x2c3) as *mut u8,flags|2);
     let display=sword::glancing_display(damage);ptr::write_unaligned((hit+0x280) as *mut i32,display);
     return;
   }
   let boosted=sword::boosted(damage,factor);ptr::write_unaligned((hit+0x280) as *mut i32,boosted);
   damage=boosted;
  }
 }
 if !has_effect(local,cat::OWNER,base){return;}
 if source==target{return;}
 if damage<=0||!cat_jump::is_jump(&ptr::read_unaligned((hit+0x124) as *const [u32;8])) {return;}
 let Some(sample)=cat_fall_height(local,base) else{return;};
 if read_i32(sample.stats+0x140)<=0{return;}
 let height=CAT_LANDING.get().and_then(|state|state.lock().ok()).map(|mut state|state.height_frame(sample,GetTickCount64())).unwrap_or(0.);
 let boosted=cat_jump::boosted(damage,height);
 ptr::write_unaligned((hit+0x280) as *mut i32,boosted);
}

unsafe fn cat_fall_height(player:usize,base:usize)->Option<cat_jump::Sample> {
 let modules=read_ptr(player+0x1b8);if modules==0{return None;}
 let physics=read_ptr(modules+0x68);let fall=read_ptr(modules+0x70);let stats=read_ptr(modules);
 if physics==0||fall==0||stats==0{return None;}
 // Exact native getter used by env(224), before its centimetre conversion.
 let getter:unsafe extern "system" fn(usize)->f32=std::mem::transmute(base+FALL_HEIGHT_RVA);
 // 469C94 copies current ground flag +92 to +93 before recomputing +92.
 // Use the CURRENT result, matching the native fall-reference reset at 468DFD.
 // env366 reads this local region context, with metres -> cm conversion
 // performed only by the script dispatcher. Use the value just as a launch
 // hint; it is a configured preset, not the actual flight's descent distance.
 let context=read_ptr(base+SPRING_CONTEXT_RVA);
 let spring=if context==0{0.}else{
  let spring_getter:unsafe extern "system" fn(usize)->f32=std::mem::transmute(base+SPRING_HEIGHT_RVA);
  spring_getter(context)
 };
 // Great Hollow pseudo-streams use 98215 in ExecJump instead of env366.
 Some(cat_jump::Sample{player,stats,airborne:ptr::read((physics+0x92) as *const u8)==0,native_height:getter(fall),world_y:ptr::read_unaligned((physics+0x74) as *const f32),spring_available:cat_jump::spring_launch_available(spring,has_effect(player,cat_jump::CRYSTAL_SPRING_EFFECT,base))})
}

unsafe fn read_ptr(p:usize)->usize {ptr::read_unaligned(p as *const usize)}
unsafe fn read_i32(p:usize)->i32 {ptr::read_unaligned(p as *const i32)}
unsafe fn arts_weapon(reference:usize)->Option<arts::Weapon>{
 if reference==0{return None;}let entry=read_ptr(reference+8);if entry==0{return None;}
 let handle=read_i32(reference) as u32;let base=read_i32(reference+0x10);
 // Native instance manager validates its generation-bearing handle at +8.
 if handle==0||(handle>>28)&7!=0||read_i32(entry+8) as u32!=handle{return None;}
 Some(arts::Weapon{handle,base,original:arts::original(base,read_i32(entry+0x1c))?})
}
unsafe fn arts_overlay(reference:usize)->Option<i32>{
 if !ACTIVE.load(Ordering::Acquire){return None;}
 let base=BASE.load(Ordering::Acquire);let world=read_ptr(base+WORLD_RVA);if world==0{return None;}
 let player=read_ptr(world+0x174e8);if player==0||!has_effect(player,arts::OWNER,base){return None;}
 let w=arts_weapon(reference)?;let equip=read_ptr(player+0x670);if equip==0{return None;}
 // Only this player's six weapon slots use the getter overlay. Native inventory
 // and drops stay original; the serializer separately patches outgoing snapshots.
 if !(0..6).any(|i|read_i32(equip+0x1c+4*i) as u32==w.handle){return None;}
 ARTS.get()?.lock().ok()?.selected(w)
}
unsafe fn arts_shadow(reference:usize,id:i32)->([u64;3],[u64;4]){
 let r=ptr::read_unaligned(reference as *const [u64;3]);let mut entry=ptr::read_unaligned(r[1] as *const [u64;4]);
 // Both native getters only read the entry's +1C override and +10 base ID.
 ptr::write_unaligned((entry.as_mut_ptr() as *mut u8).add(0x1c) as *mut i32,id);
 (r,entry)
}
unsafe extern "system" fn arts_id_detour(reference:usize)->i32{
 let original:unsafe extern "system" fn(usize)->i32=std::mem::transmute(ARTS_ID_ORIG.load(Ordering::Acquire));
 let Some(id)=arts_overlay(reference) else{return original(reference);};
 let (mut r,entry)=arts_shadow(reference,id);r[1]=entry.as_ptr() as u64;original(r.as_ptr() as usize)
}
unsafe extern "system" fn arts_param_detour(reference:usize,out:usize)->usize{
 let original:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(ARTS_PARAM_ORIG.load(Ordering::Acquire));
 let Some(id)=arts_overlay(reference) else{return original(reference,out);};
 let (mut r,entry)=arts_shadow(reference,id);r[1]=entry.as_ptr() as u64;original(r.as_ptr() as usize,out)
}
unsafe extern "system" fn arts_raw_detour(reference:usize)->i32{
 let original:unsafe extern "system" fn(usize)->i32=std::mem::transmute(ARTS_RAW_ORIG.load(Ordering::Acquire));
 let Some(id)=arts_overlay(reference) else{return original(reference);};
 let (mut r,entry)=arts_shadow(reference,id);r[1]=entry.as_ptr() as u64;original(r.as_ptr() as usize)
}
unsafe fn arts_hand_weapon(player:usize,hand:i32,base:usize)->Option<arts::Weapon>{
 arts_source_weapon(player,if hand==0{-2}else{-1},base)
}
unsafe fn arts_source_weapon(player:usize,source:i32,base:usize)->Option<arts::Weapon>{
 let get:unsafe extern "system" fn(usize,*mut u32,i32)->*const u32=std::mem::transmute(read_ptr(read_ptr(player)+0x258));
 let mut handle=0;let p=get(player,&mut handle,source);if p.is_null(){return None;}
 arts_weapon_handle(p,base)
}
unsafe fn arts_weapon_handle(p:*const u32,base:usize)->Option<arts::Weapon>{
 if p.is_null()||ptr::read_unaligned(p)==0{return None;}
 let init:unsafe extern "system" fn(*mut u64,*const u32)->usize=std::mem::transmute(base+ARTS_REF_RVA);
 let release:unsafe extern "system" fn(*mut u64)=std::mem::transmute(base+0x6a1d90);
 let mut r=[0u64;3];init(r.as_mut_ptr(),p);let w=arts_weapon(r.as_ptr() as usize);release(r.as_mut_ptr());w
}
unsafe fn arts_transfers_enabled()->bool{
 if !ACTIVE.load(Ordering::Acquire){return false;}
 let base=BASE.load(Ordering::Acquire);let world=read_ptr(base+WORLD_RVA);
 if world==0{return false;}let player=read_ptr(world+0x174e8);
 player!=0&&has_effect(player,arts::OWNER,base)
}
unsafe extern "system" fn arts_drop_detour(menu:usize,quantity:i32,flag:u8)->usize{
 let original:unsafe extern "system" fn(usize,i32,u8)->usize=std::mem::transmute(ARTS_DROP_ORIG.load(Ordering::Acquire));
 if !arts_transfers_enabled(){return original(menu,quantity,flag);}
 let _scope=ArtsTransferGuard::enter(ArtsTransfer{active:true,..ArtsTransfer::default()});
 original(menu,quantity,flag)
}
#[unsafe(naked)]
unsafe extern "system" fn arts_serialize_wrapper(_payload:usize,_handle:usize)->usize{
 core::arch::naked_asm!("mov r8, [rsp]","jmp {handler}",handler=sym arts_serialize_detour);
}
unsafe extern "system" fn arts_serialize_detour(payload:usize,handle:usize,caller:usize)->usize{
 let original:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(ARTS_SERIALIZE_ORIG.load(Ordering::Acquire));
 let base=BASE.load(Ordering::Acquire);
 let selected=if arts::network_export(caller.wrapping_sub(base),ARTS_TRANSFER.with(|c|c.get().active))&&arts_transfers_enabled()&&handle!=0{
  arts_weapon_handle(handle as *const u32,base).and_then(|w|{
   let world=read_ptr(base+WORLD_RVA);let player=read_ptr(world+0x174e8);let equip=read_ptr(player+0x670);
   if equip==0||!(0..6).any(|i|read_i32(equip+0x1c+4*i) as u32==w.handle){return None;}
   ARTS.get().and_then(|s|s.lock().ok()?.selected(w).map(|skill|(w.base,skill)))
  })
 }else{None};
 let result=arts_serialize_call(original,payload,handle,selected);
 if payload==0||handle==0||!ARTS_TRANSFER.with(|c|c.get().active){return result;}
 let Some(w)=arts_weapon_handle(handle as *const u32,BASE.load(Ordering::Acquire)) else{return result;};
 let selected=ARTS.get().and_then(|s|s.lock().ok()?.selected(w));
 // Capture inside the actual drop (or replacement pickup) call. No matching
 // against unrelated inventory previews and no queue by weapon type.
 let drop=selected.map(|_|ArtsDrop{weapon:w,payload:ptr::read_unaligned(payload as *const [u8;0x48])});
 ARTS_TRANSFER.with(|c|{let mut s=c.get();s.drop=drop;c.set(s);});result
}
unsafe fn arts_serialize_call(original:unsafe extern "system" fn(usize,usize)->usize,payload:usize,handle:usize,selected:Option<(i32,i32)>)->usize{
 let result=original(payload,handle);
 if payload!=0{if let Some((base,skill))=selected{
  let mut data=ptr::read_unaligned(payload as *const [u8;0x48]);arts::snapshot(&mut data,base,skill);
  ptr::write_unaligned(payload as *mut [u8;0x48],data);
 }}result
}
unsafe fn arts_bind_drop(descriptor:usize){
 let scope=ARTS_TRANSFER.with(Cell::get);
 let Some(drop)=scope.drop.filter(|_|scope.active) else{return;};
 if descriptor==0{return;}
 let Some(uid)=arts_drop_uid(drop,descriptor) else{return;};
 if let Some(state)=ARTS.get(){if let Ok(mut s)=state.lock(){s.dropped(uid,drop.weapon);}}
 ARTS_TRANSFER.with(|c|{let mut s=c.get();s.drop=None;c.set(s);});
}
unsafe fn arts_drop_uid(drop:ArtsDrop,descriptor:usize)->Option<u32>{
 if ptr::read_unaligned(descriptor as *const u8)!=1||read_i32(descriptor+0x58)&0xf!=1{return None;}
 // The native 0xC0 descriptor copies the exact serialized 0x48-byte weapon.
 if ptr::read_unaligned((descriptor+8) as *const [u8;0x48])!=drop.payload{return None;}
 let uid=read_i32(descriptor+0x5c) as u32;if uid==0{None}else{Some(uid)}
}
unsafe extern "system" fn arts_reward_copy_detour(out:usize,descriptor:usize)->usize{
 let original:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(ARTS_REWARD_COPY_ORIG.load(Ordering::Acquire));
 // Multiplayer may queue creation; remember its UID before the native scope
 // ends. Direct local creation is covered by reward_create_detour instead.
 arts_bind_drop(descriptor);original(out,descriptor)
}
unsafe extern "system" fn arts_pickup_detour(manager:usize,descriptor:usize,source:usize,out:usize,position:usize,flag:u8,slot:i32)->usize{
 let original:ArtsPickupFn=std::mem::transmute(ARTS_PICKUP_ORIG.load(Ordering::Acquire));
 arts_pickup_call(original,manager,descriptor,source,out,position,flag,slot)
}
unsafe fn arts_pickup_call(original:ArtsPickupFn,manager:usize,descriptor:usize,source:usize,out:usize,position:usize,flag:u8,slot:i32)->usize{
 if descriptor==0||!arts_transfers_enabled(){return original(manager,descriptor,source,out,position,flag,slot);}
 let single=ptr::read_unaligned(descriptor as *const u8)==1;
 let _scope=ArtsTransferGuard::enter(ArtsTransfer{active:true,drop:None,
  pickup_uid:if single{read_i32(descriptor+0x5c) as u32}else{0},
  pickup_payload:if single{descriptor+8}else{0}});
 // This native routine always adds to the local inventory. A replacement
 // pickup may also serialize/drop the previous weapon within this scope.
 original(manager,descriptor,source,out,position,flag,slot)
}
unsafe extern "system" fn arts_item_create_detour(manager:usize,out:usize,payload:usize)->usize{
 let original:unsafe extern "system" fn(usize,usize,usize)->usize=std::mem::transmute(ARTS_ITEM_CREATE_ORIG.load(Ordering::Acquire));
 let result=original(manager,out,payload);let scope=ARTS_TRANSFER.with(Cell::get);
 if !scope.active||scope.pickup_uid==0||payload!=scope.pickup_payload||out==0{return result;}
 // The factory writes a new generation-bearing handle before equip/addition.
 if let Some(w)=arts_weapon_handle(out as *const u32,BASE.load(Ordering::Acquire)){
  if let Some(state)=ARTS.get(){if let Ok(mut s)=state.lock(){s.picked_up(scope.pickup_uid,w);}}
 }
 result
}
unsafe extern "system" fn arts_behavior_detour(player:usize,judge:i32,source:i32,flags:u32,extra:u8)->i32{
 let original:ArtsBehaviorFn=std::mem::transmute(ARTS_BEHAVIOR_ORIG.load(Ordering::Acquire));
 arts_behavior_call(original,player,judge,source,flags,extra)
}
unsafe extern "system" fn arts_bullet_detour(module:usize,behavior:i32,event:i32,descriptor:usize,source:i32,original_source:i32,flag0:u8,flag1:u8,kind:u8,error:usize)->u8{
 let original:ArtsBulletFn=std::mem::transmute(ARTS_BULLET_ORIG.load(Ordering::Acquire));
 let replacement=arts_emitter(module,behavior,descriptor,source);
 arts_bullet_call(original,module,behavior,event,descriptor,source,original_source,flag0,flag1,kind,error,replacement)
}
unsafe fn arts_emitter(module:usize,behavior:i32,descriptor:usize,source:i32)->Option<i32>{
 if !ACTIVE.load(Ordering::Acquire)||module==0||descriptor==0{return None;}
 if !matches!(behavior/1000,303215|303216){return None;}
 let base=BASE.load(Ordering::Acquire);let player=read_ptr(module);
 if player==0||!has_effect(player,arts::OWNER,base){return None;}
 let w=arts_source_weapon(player,source,base)?;
 let selected=arts_actor_selection(player,w,base)?;
 arts::emitter_dummy(selected,w.base,behavior,read_i32(descriptor))
}
unsafe fn arts_actor_selection(player:usize,w:arts::Weapon,base:usize)->Option<i32>{
 if is_local(player,base){return ARTS.get()?.lock().ok()?.selected(w);}
 // Peer instances already carry the transmitted +1C skill. Cross-weapon TAE
 // behavior still needs the same variation lookup as the owning player's overlay.
 (arts::pool(w.base).contains(&w.original)).then_some(w.original)
}
unsafe fn arts_bullet_call(original:ArtsBulletFn,module:usize,behavior:i32,event:i32,descriptor:usize,source:i32,original_source:i32,flag0:u8,flag1:u8,kind:u8,error:usize,replacement:Option<i32>)->u8{
 if let Some(dummy)=replacement{
  // The native creation call consumes this private 0x18-byte descriptor
  // synchronously. Keep TAE data, origin/source flags and all outputs intact.
  let mut shadow=ptr::read_unaligned(descriptor as *const [u8;0x18]);
  shadow[..4].copy_from_slice(&dummy.to_le_bytes());
  original(module,behavior,event,shadow.as_ptr() as usize,source,original_source,flag0,flag1,kind,error)
 }else{original(module,behavior,event,descriptor,source,original_source,flag0,flag1,kind,error)}
}
unsafe fn arts_behavior_call(original:ArtsBehaviorFn,player:usize,judge:i32,source:i32,flags:u32,extra:u8)->i32{
 // Preserve the native five-argument call and its side effects, including miss
 // and common-fallback results. Change only the resulting PC behavior row ID.
 let native=original(player,judge,source,flags,extra);
 if !ACTIVE.load(Ordering::Acquire)||!(3000..4000).contains(&judge){return native;}
 let base=BASE.load(Ordering::Acquire);
 if player==0||!has_effect(player,arts::OWNER,base){return native;}
 let Some(w)=arts_source_weapon(player,source,base) else{return native;};
 arts_actor_selection(player,w,base).and_then(|skill|arts::behavior(judge,skill)).unwrap_or(native)
}
#[cfg(test)]mod arts_behavior_abi_tests{
 use super::*;
 #[test]fn native_serialization_wrapper_preserves_payload_handle_return_and_record_bytes(){unsafe{
  unsafe extern "system" fn fixture(payload:usize,handle:usize)->usize{
   assert_eq!(handle,0x123456789);if payload!=0{let data=&mut *(payload as *mut [u8;0x48]);data.fill(0x67);data[..4].copy_from_slice(&2000000i32.to_le_bytes());}0x87654321
  }
  let mut data=[0u8;0x48];let p=data.as_mut_ptr() as usize;
  assert_eq!(arts_serialize_call(fixture,p,0x123456789,Some((2000000,1173))),0x87654321);
  assert_eq!(read_i32(p+4),1173);assert!(data[8..].iter().all(|b|*b==0x67));
  assert_eq!(arts_serialize_call(fixture,0,0x123456789,Some((2000000,1173))),0x87654321);
  let saved=ARTS_SERIALIZE_ORIG.swap(fixture as *const () as *mut u8,Ordering::AcqRel);
  assert_eq!(arts_serialize_wrapper(p,0x123456789),0x87654321);
  ARTS_SERIALIZE_ORIG.store(saved,Ordering::Release);
  assert_eq!(read_i32(p+4),0x67676767);
  assert_eq!(std::slice::from_raw_parts(arts_serialize_wrapper as *const () as *const u8,4),&[0x4c,0x8b,0x04,0x24]);
 }}
 #[test]fn forwards_five_native_arguments_and_signed_fallback(){
  unsafe extern "system" fn fixture(p:usize,j:i32,s:i32,f:u32,e:u8)->i32{
   assert_eq!((p,j,s,f,e),(0x123456789usize,900,-2,0xaabbccdd,0x81));-1
  }
  assert_eq!(unsafe{arts_behavior_call(fixture,0x123456789,900,-2,0xaabbccdd,0x81)},-1);
 }
 #[test]fn pickup_forwards_all_seven_native_arguments(){
  unsafe extern "system" fn fixture(m:usize,d:usize,s:usize,o:usize,p:usize,f:u8,i:i32)->usize{
   assert_eq!((m,d,s,o,p,f,i),(11,22,33,44,55,0x81,-2));0x123456789
  }
  assert_eq!(unsafe{arts_pickup_call(fixture,11,22,33,44,55,0x81,-2)},0x123456789);
 }
 #[test]fn projectile_shadow_preserves_ten_arguments_native_outputs_and_original_bytes(){unsafe{
  unsafe extern "system" fn fixture(m:usize,b:i32,e:i32,d:usize,s:i32,o:i32,f:u8,g:u8,k:u8,error:usize)->u8{
   assert_eq!((m,b,e,s,o,f,g,k),(11,303215900,33,-1,-2,0x81,0x82,0xff));
   assert_eq!(read_i32(d),58);assert_eq!(ptr::read_unaligned((d+4) as *const [u8;20]),[0x67;20]);
   ptr::write_unaligned(error as *mut i32,123);0x91
  }
  let mut descriptor=[0x67u8;24];descriptor[..4].copy_from_slice(&10141i32.to_le_bytes());let before=descriptor;let mut error=0;
  assert_eq!(arts_bullet_call(fixture,11,303215900,33,descriptor.as_ptr() as usize,-1,-2,0x81,0x82,0xff,&mut error as *mut i32 as usize,Some(58)),0x91);
  assert_eq!(descriptor,before);assert_eq!(error,123);
  unsafe extern "system" fn untouched(_:usize,_:i32,_:i32,d:usize,_:i32,_:i32,_:u8,_:u8,_:u8,_:usize)->u8{assert_eq!(read_i32(d),10141);0x92}
  assert_eq!(arts_bullet_call(untouched,11,303215900,33,descriptor.as_ptr() as usize,-1,-2,0x81,0x82,0xff,0,None),0x92);
 }}
 #[test]fn nested_transfer_scope_restores_incoming_pickup_identity(){
  let outer=ArtsTransferGuard::enter(ArtsTransfer{active:true,pickup_uid:123,pickup_payload:456,drop:None});
  {let _inner=ArtsTransferGuard::enter(ArtsTransfer::default());assert!(!ARTS_TRANSFER.with(|c|c.get().active));}
  assert_eq!(ARTS_TRANSFER.with(|c|(c.get().pickup_uid,c.get().pickup_payload)),(123,456));
  drop(outer);assert!(!ARTS_TRANSFER.with(|c|c.get().active));
 }
 #[test]fn drop_binding_checks_actual_payload_and_preserves_descriptor_bytes(){unsafe{
  let mut descriptor=[0u8;0xc0];descriptor[0]=1;
  let drop=ArtsDrop{weapon:arts::Weapon{handle:123,base:2000000,original:100},payload:[0x67;0x48]};
  descriptor[8..0x50].copy_from_slice(&drop.payload);
  descriptor[0x58..0x5c].copy_from_slice(&0x100001u32.to_le_bytes());
  descriptor[0x5c..0x60].copy_from_slice(&0x28000004u32.to_le_bytes());
  let address=descriptor.as_ptr() as usize;let before=descriptor;
  assert_eq!(arts_drop_uid(drop,address),Some(0x28000004));assert_eq!(descriptor,before);
  descriptor[0x18]^=1;assert_eq!(arts_drop_uid(drop,address),None);descriptor[0x18]^=1;
  descriptor[0]=0;assert_eq!(arts_drop_uid(drop,address),None);descriptor[0]=1;
  descriptor[0x58]=2;assert_eq!(arts_drop_uid(drop,address),None);descriptor[0x58]=1;
  descriptor[0x5c..0x60].fill(0);assert_eq!(arts_drop_uid(drop,address),None);
 }}
}
unsafe fn update_arts(player:usize,base:usize){
 let Some(state)=ARTS.get() else{return;};let owned=has_effect(player,arts::OWNER,base);
 let modules=read_ptr(player+0x1b8);if modules==0{return;}let stats=read_ptr(modules);if stats==0{return;}
 let alive=read_i32(stats+0x140)>0;let context=read_ptr(base+0x3c131c8);
 let pending={let Ok(mut s)=state.lock() else{return;};s.synchronize(context,owned,alive);s.tick()};
 if let Some(w)=pending{reforge_art(w,base);}
 arts_network::update(player,base);
 let used=has_effect(player,arts::USED,base);
 let wanted=owned&&alive&&state.lock().is_ok_and(|s|s.has_pending());
 if wanted&&!used{let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);apply(player,arts::USED,0);}
 else if !wanted&&used{let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);remove(read_ptr(player+0x190),arts::USED);}
 let marker=has_effect(player,arts::MARKER,base);
 if owned&&alive&&!marker{let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);apply(player,arts::MARKER,0);}
 else if (!owned||!alive)&&marker{let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);remove(read_ptr(player+0x190),arts::MARKER);}
}
unsafe fn reforge_art(w:arts::Weapon,_base:usize){
 if let Some(state)=ARTS.get(){
  let previous=state.lock().ok().and_then(|s|s.selected(w)).unwrap_or(w.original);
  let pool=arts::pool(w.base);let mut random=0u32;
  loop{
   if BCryptGenRandom(ptr::null_mut(),&mut random as *mut u32 as *mut u8,4,2)<0{break;}
   let count=pool.len() as u32; if random>=u32::MAX-u32::MAX%count{continue;}
   let id=pool[(random%count) as usize];if id==previous{continue;}
   if let Ok(mut s)=state.lock(){s.reforge(w,id);}break;
  }
 }
}
unsafe fn is_local(player:usize,base:usize)->bool {
 if player==0{return false;}
 let world=read_ptr(base+WORLD_RVA);
 world!=0&&read_ptr(world+0x174e8)==player
}
unsafe fn spell_percentage(hit:usize)->Option<f32>{
 SWORD_BONUSES.get()?.for_attack(read_i32(hit+0x40),read_i32(hit+0x44))
}
unsafe fn spell_factor(hit:usize)->Option<f32>{SWORD_BONUSES.get()?.factor(read_i32(hit+0x40),read_i32(hit+0x44))}
// Native BA186E calls this entry only for the local self-position marker,
// after UI/controller input filtering and the separate item-marker branch.
// Count requests even when the map pin deduplicates the same location.
unsafe extern "system" fn self_mark_detour(manager:usize,out:usize,player:usize)->usize{
 let original:unsafe extern "system" fn(usize,usize,usize)->usize=std::mem::transmute(SELF_MARK_ORIG.load(Ordering::Acquire));
 let base=BASE.load(Ordering::Acquire);
 if ACTIVE.load(Ordering::Acquire)&&is_local(player,base)&&has_effect(player,noisy::OWNER,base){
  let modules=read_ptr(player+0x1b8);let stats=if modules==0{0}else{read_ptr(modules)};
  if stats!=0 {let alive=read_i32(stats+0x140)>0;
   if let Some(state)=NOISY.get(){if let Ok(mut l)=state.lock(){
    l.synchronize(player,stats,true);l.mark(alive);
   }}
  }
 }
 original(manager,out,player)
}
unsafe fn update_noisy(player:usize,base:usize,dt:f32){
 let modules=read_ptr(player+0x1b8);let stats=if modules==0{0}else{read_ptr(modules)};if stats==0{return;}
 let owned=has_effect(player,noisy::OWNER,base);let alive=read_i32(stats+0x140)>0;
 let Some(state)=NOISY.get() else{return;};
 let emit={let Ok(mut l)=state.lock() else{return;};
  l.synchronize(player,stats,owned);let emit=l.tick(dt,alive);
  emit
 };
 if emit{
  let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);
  let ok=apply(player,noisy::EMIT,0)!=0;
  if let Ok(mut l)=state.lock(){
   l.emitted(ok);
  }
 }
}
unsafe fn lava_visual_scale(bullet:usize,advance:bool)->f32{
 if !ACTIVE.load(Ordering::Acquire)||bullet==0{return 1.;}
 let Some(id)=lava_parameter_id(bullet) else{return 1.;};
 let base=BASE.load(Ordering::Acquire);
 let resolve:unsafe extern "system" fn(u64)->usize=std::mem::transmute(base+CHR_HANDLE_RVA);
 let player=resolve(ptr::read_unaligned((bullet+0xb0) as *const u64));
 if !is_local(player,base){return 1.;}
 let Some(state)=NOISY.get() else{return 1.;};let Ok(mut l)=state.lock() else{return 1.;};
 if advance{
  l.created(player,bullet,id);
 }
 noisy::visual_scale(l.factor(player))
}
unsafe fn lava_parameter_id(bullet:usize)->Option<i32>{
 if bullet==0{return None;}
 // Ordinary Bullet uses native kind 2, not the attack-calculator's PC kind 1.
 // Its current ID is +5D0; +600 can already identify the following child.
 let index=read_i32(bullet+0x90);let kinds=read_ptr(bullet+0x88);
 if !(0..=4).contains(&index)||kinds==0||!noisy::ordinary_reference(index,read_i32(kinds+index as usize*4)){return None;}
 let id=read_i32(bullet+noisy::PARAM_ID_OFFSET);noisy::lava_bullet(id).then_some(id)
}
unsafe extern "system" fn lava_sfx_create_detour(bullet:usize,out:usize,sfx:usize,position:usize,rotation:usize)->usize{
 let original:unsafe extern "system" fn(usize,usize,usize,usize,usize)->usize=std::mem::transmute(LAVA_SFX_CREATE_ORIG.load(Ordering::Acquire));
 let _scope=LavaVisualGuard::enter(lava_visual_scale(bullet,true));
 original(bullet,out,sfx,position,rotation)
}
unsafe extern "system" fn lava_sfx_update_detour(bullet:usize){
 let original:unsafe extern "system" fn(usize)=std::mem::transmute(LAVA_SFX_UPDATE_ORIG.load(Ordering::Acquire));
 let _scope=LavaVisualGuard::enter(lava_visual_scale(bullet,false));original(bullet);
}
unsafe extern "system" fn lava_cleanup_detour(bullet:usize){
 let original:unsafe extern "system" fn(usize)=std::mem::transmute(LAVA_CLEANUP_ORIG.load(Ordering::Acquire));
 // Called on expiration, pool reset and destruction before PARAM refs clear.
 let ours=ACTIVE.load(Ordering::Acquire)&&bullet!=0&&noisy::lava_bullet(read_i32(bullet+noisy::PARAM_ID_OFFSET));
 original(bullet);
 if ours{if let Some(state)=NOISY.get(){if let Ok(mut l)=state.lock(){
  l.removed(bullet);
 }}}
}
unsafe extern "system" fn sfx_matrix_detour(handle:usize,matrix:usize){
 let original:unsafe extern "system" fn(usize,usize)=std::mem::transmute(SFX_MATRIX_ORIG.load(Ordering::Acquire));
 let scale=LAVA_VISUAL_SCALE.with(Cell::get);
 if scale>1.&&matrix!=0{
  #[repr(align(16))]struct Matrix([[f32;4];4]);
  let shadow=Matrix(noisy::scale_matrix(ptr::read_unaligned(matrix as *const [[f32;4];4]),scale));
  original(handle,&shadow.0 as *const _ as usize);
 }else{original(handle,matrix);}
}

unsafe fn sword_casting(player:usize,owned:bool,alive:bool)->bool{
 sword::casting(player,owned,alive,SWORD_CAST_PLAYER.load(Ordering::Acquire),SWORD_LAST_CAST.load(Ordering::Acquire),GetTickCount64())
}
unsafe fn active_magic(player:usize)->i32{
 let modules=read_ptr(player+0x1b8);if modules==0{return -1;}
 let action=read_ptr(modules+0x60);if action==0{return -1;}
 let vtable=read_ptr(action);if vtable==0{return -1;}
 // Same current-magic virtual getter used by native env(227) and env(232).
 let address=read_ptr(vtable+0x18);if address==0{return -1;}
 let getter:unsafe extern "system" fn(usize,*mut i32)->*const i32=std::mem::transmute(address);
 let mut id=-1;let result=getter(action,&mut id);if result.is_null(){-1}else{ptr::read_unaligned(result)}
}
unsafe fn sword_hit_distance(source:usize,hit:usize,base:usize)->Option<f32>{
 if hit==0||!spatial_ready(source){return None;}
 // 5363A0 writes the collision point at +1F0; 434FA0 reconstructs the same
 // world-space point from the network packet. +1E0 is the attack origin.
 // Never substitute the target pivot: large bosses can be hit far from it.
 sword::distance(position(source,base),sword_hit_point(hit))
}
unsafe fn sword_hit_point(hit:usize)->[f32;4]{ptr::read_unaligned((hit+0x1f0) as *const [f32;4])}
unsafe fn update_sword(player:usize,base:usize){
 let modules=read_ptr(player+0x1b8);if modules==0{return;}
 let stats=read_ptr(modules);if stats==0{return;}
 let owned=has_effect(player,sword::OWNER,base);let alive=read_i32(stats+0x140)>0;
 let marker=has_effect(player,sword::CAST_OWNER,base);
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);
 if owned&&alive {
  if !marker {apply(player,sword::CAST_OWNER,0);}
 }else{
  for id in [sword::CAST_OWNER,sword::GUARD,sword::POISE]{if has_effect(player,id,base){remove(read_ptr(player+0x190),id);}}
 }
 let casting=sword_casting(player,owned,alive);
 for id in [sword::GUARD,sword::POISE,sword::WEAPON_RIGHT,sword::WEAPON_LEFT]{
  let desired=sword::casting_effect(id,casting,SWORD_CAST_HAND.load(Ordering::Acquire));
  let present=has_effect(player,id,base);
  if desired&&!present {
   apply(player,id,0);
  }else if !desired&&present {
   remove(read_ptr(player+0x190),id);
  }
 }
 let Some(state)=SWORD.get() else{return;};
 let amount={let Ok(mut h)=state.lock() else{return;};h.take(player,stats,alive,owned)};
 if amount==0{return;}
 for bit in 0..sword::HEAL_BITS{if amount&(1<<bit)!=0{apply(player,sword::HEAL_BASE+bit as i32,0);}}
 apply(player,sword::HEAL_VISUAL,0); // 8630000 HP-only visual, without native fixed healing.
}
unsafe fn has_effect(player:usize,effect:i32,base:usize)->bool {
 let manager=read_ptr(player+0x190);if manager==0{return false;}
 let has:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+HAS_EFFECT_RVA);
 has(manager,effect)!=0
}
unsafe fn has_state(player:usize,state:u16,base:usize)->bool {
 let manager=read_ptr(player+0x190);if manager==0{return false;}
 let has:unsafe extern "system" fn(usize,u16)->u8=std::mem::transmute(base+HAS_STATE_RVA);
 has(manager,state)!=0
}
unsafe fn snapshot(player:usize,base:usize)->Option<(usize,runtime::Snapshot)> {
 let modules=read_ptr(player+0x1b8);if modules==0{return None;}
 let stats=read_ptr(modules);if stats==0{return None;}
 let gauge_module=read_ptr(modules+0x148);if gauge_module==0{return None;}
 let get_gauge:unsafe extern "system" fn(usize)->f32=std::mem::transmute(base+GAUGE_RVA);
 let gauge=get_gauge(gauge_module);
 if !gauge.is_finite()||!(0.0..=100.01).contains(&gauge){return None;}
 let owners=(has_effect(player,runtime::ULTIMATE,base) as u8)
  |((has_effect(player,runtime::DRINKING,base) as u8)<<1)
  |((has_effect(player,runtime::REGEN,base) as u8)<<2)
  |((has_effect(player,crystal::OWNER,base) as u8)<<3)
  |((has_effect(player,cat::OWNER,base) as u8)<<4);
 Some((stats,runtime::Snapshot{hp:read_i32(stats+0x140),max_hp:read_i32(stats+0x144),fp:read_i32(stats+0x150),gauge,owners,
  cooldown:has_effect(player,runtime::COOLDOWN,base),beast:has_effect(player,707115,base)}))
}

// Current 3C8885 caller and 3DB410 body verify four register arguments,
// seven stack arguments, and an AL success result. Forward every argument.
type ApplyEffect=unsafe extern "system" fn(usize,i32,usize,usize,usize,u8,u8,u8,u8,u8,i32)->u8;
// HKS env(1106, action) dispatch at 3FA6D5 passes the input module, mode=1,
// and action ID to 3E91A0; its caller consumes AL. 42C120 verifies owner+8.
// Preserve the original result for every request except a blocked raw jump
// during the local Cat owner's wall takeoff; no input fields are written.
type ActionRequest=unsafe extern "system" fn(usize,i32,i32)->u8;
unsafe extern "system" fn action_request_detour(input:usize,mode:i32,action:i32)->u8 {
 let original:ActionRequest=std::mem::transmute(ACTION_REQUEST_ORIG.load(Ordering::Acquire));
 let result=original(input,mode,action);
 if result!=0||!ACTIVE.load(Ordering::Acquire)||input==0||mode!=1||action!=6{return result;}
 let raw=ptr::read_unaligned((input+0x20) as *const u64);
 if raw&64==0{return result;}
 let base=BASE.load(Ordering::Acquire);let player=read_ptr(input+8);
 if !is_local(player,base)||!has_effect(player,cat::OWNER,base){return result;}
 let modules=read_ptr(player+0x1b8);if modules==0||read_ptr(modules+0x80)!=input{return result;}
 let physics=read_ptr(modules+0x68);let stats=read_ptr(modules);
 if physics==0||stats==0{return result;}
 let alive=read_i32(stats+0x140)>0;
 let airborne=ptr::read((physics+0x93) as *const u8)==0;
 let contact=ptr::read((physics+0x2c0) as *const u8)!=0;
 let takeoff_complete=has_effect(player,5031,base);
 if !wall_input::needs_raw_jump(result,mode,action,raw,true,alive,airborne,contact,takeoff_complete){return result;}
 1
}
unsafe extern "system" fn effect_detour(player:usize,mut effect:i32,source:usize,a4:usize,a5:usize,a6:u8,a7:u8,a8:u8,a9:u8,a10:u8,a11:i32)->u8 {
 let original:ApplyEffect=std::mem::transmute(APPLY_ORIG.load(Ordering::Acquire));
 let base=BASE.load(Ordering::Acquire);
 if ACTIVE.load(Ordering::Acquire)&&effect==spirit::NATIVE_DRAIN&&npc_id(player,base).is_some_and(spirit::is_private_npc){effect=spirit::PASSIVE;}
 if ACTIVE.load(Ordering::Acquire)&&effect==102100&&is_local(player,base){on_effect(player,balloon::DOWN,base);}
 if ACTIVE.load(Ordering::Acquire)&&matches!(effect,balloon::DOWN|balloon::REVIVED)&&is_local(player,base){
  on_effect(player,effect,base);
 }
 if ACTIVE.load(Ordering::Acquire)&&effect==chant::PULSE&&is_local(player,base){chant_pulse(player,base);}
 // Capture before native healing can change HP; never hold a lock across it.
 if ACTIVE.load(Ordering::Acquire)&&matches!(effect,arts::BUSY|arts::USE_LEFT|arts::USE_RIGHT)&&is_local(player,base)&&has_effect(player,arts::OWNER,base){
  let w=if effect==arts::BUSY{None}else{arts_hand_weapon(player,if effect==arts::USE_LEFT{0}else{1},base)};
  if let Some(state)=ARTS.get(){if let Ok(mut s)=state.lock(){s.notify(w,effect!=arts::BUSY);}}
 }
 if ACTIVE.load(Ordering::Acquire)&&effect==arts::PREPARE&&is_local(player,base)&&has_effect(player,arts::OWNER,base){
  let pending=ARTS.get().and_then(|state|state.lock().ok()?.prepare());
  if let Some(w)=pending{reforge_art(w,base);}
 }
 let before=if ACTIVE.load(Ordering::Acquire) && matches!(effect,501000|7010220|7010230|runtime::FLASK_TRIGGER) && (effect==runtime::FLASK_TRIGGER || source==player) && is_local(player,base) { snapshot(player,base) } else { None };
 if let Some((stats,s))=before {
  if let Some(state)=RUNTIME.get() {if let Ok(mut r)=state.lock(){r.synchronize(player,stats,s);r.before_flask_heal(s);}}
 }
 if ACTIVE.load(Ordering::Acquire)&&matches!(effect,jiahao::CRITICAL|jiahao::BACK_START|jiahao::ROLL)&&is_local(player,base)&&has_effect(player,jiahao::OWNER,base){
  let stats=read_ptr(read_ptr(player+0x1b8));
  let accepted=effect!=jiahao::CRITICAL||jiahao_front_critical(player,base);
  if accepted&&stats!=0{if let Some(state)=JIAHAO.get(){if let Ok(mut state)=state.lock(){state.synchronize(player,stats,true,read_i32(stats+0x140)>0);state.marker(effect,GetTickCount64());}}}
 }
 let result=original(player,effect,source,a4,a5,a6,a7,a8,a9,a10,a11);
 if ACTIVE.load(Ordering::Acquire)&&matches!(effect,balloon::DOWN|102100)&&is_local(player,base){flush_balloon(player,base);}
 // The event supplies a synchronized persistent marker. Native mutation
 // sources and native death rewards both read this same actor bit.
 if ACTIVE.load(Ordering::Acquire)&&effect==cage_mutation::MARKER {
  promote_cage_mutation(player,base,true);
 }
 if result!=0&&ACTIVE.load(Ordering::Acquire)&&matches!(effect,sword::CAST_PULSE|sword::CAST_RIGHT|sword::CAST_LEFT|sword::RECOVERY_RIGHT|sword::RECOVERY_LEFT)&&is_local(player,base)&&has_effect(player,sword::OWNER,base){
  let now=GetTickCount64();
  let continuous=sword::casting(player,true,true,SWORD_CAST_PLAYER.load(Ordering::Acquire),SWORD_LAST_CAST.load(Ordering::Acquire),now);
  let recovery=matches!(effect,sword::RECOVERY_RIGHT|sword::RECOVERY_LEFT);
  if let Some(magic)=sword::pulse_magic(recovery,continuous,SWORD_CAST_MAGIC.load(Ordering::Acquire),if recovery{-1}else{active_magic(player)}){
   SWORD_CAST_HAND.store(match effect {sword::CAST_RIGHT|sword::RECOVERY_RIGHT=>1,sword::CAST_LEFT|sword::RECOVERY_LEFT=>0,_=>-1},Ordering::Release);
   SWORD_CAST_MAGIC.store(magic,Ordering::Release);
   SWORD_CAST_PLAYER.store(player,Ordering::Release);SWORD_LAST_CAST.store(now,Ordering::Release);
  }
 }
 if result!=0&&ACTIVE.load(Ordering::Acquire) {
  let base=BASE.load(Ordering::Acquire);
  if matches!(effect,flasks::GRANTED|flasks::CAPTURE|flasks::CHURCH|flasks::CHURCH_REQUEST) && is_local(player,base) {
   if let Some(state)=FLASKS.get(){if let Ok(mut f)=state.lock(){f.applied(effect);}}
  }
  if effect==runtime::ULTIMATE_TRIGGER&&is_local(player,base){on_effect(player,effect,base);}
  if matches!(effect,runtime::ULTIMATE_TRIGGER|runtime::FLASK_TRIGGER)&&is_local(player,base) {
   if let Some((stats,s))=snapshot(player,base) {
    if let Some(state)=RUNTIME.get() {
     if let Ok(mut r)=state.lock(){r.synchronize(player,stats,s);r.applied(effect,before.map(|v|v.1).unwrap_or(s));}
    }
   }
  }
 }
 result
}
unsafe fn jiahao_front_critical(player:usize,base:usize)->bool{
 let module=read_ptr(read_ptr(player+0x1b8)+0x88);if module==0{return false;}
 // Same getter as env(273): module+10 -> ChrThrowMan -> typed current ThrowParam.
 let get:unsafe extern "system" fn(usize)->i32=std::mem::transmute(base+THROW_ANIM_RVA);
 let animation=get(module);
 if matches!(animation,31700|31750){return true;}
 if animation!=31720{return false;}
 let current=read_ptr(module+0x10);let data=read_ptr(current+0x58);if data==0{return false;}
 jiahao::is_front_critical(animation,ptr::read_unaligned((data+0x0c) as *const f32),ptr::read_unaligned((data+0x10) as *const f32))
}
unsafe fn update_jiahao(player:usize,base:usize,_dt:f32){
 let stats=read_ptr(read_ptr(player+0x1b8));if stats==0{return;}
 let owned=has_effect(player,jiahao::OWNER,base);let alive=read_i32(stats+0x140)>0&&!has_effect(player,102100,base)&&!has_effect(player,102150,base);
 // Latent powers apply PermanentBuffParam.spEffectId. AttachEffectParam's
 // extra passives do not place the HKS token on the character (live verified).
 // Maintain that exact-f32 token explicitly, as with Sword's casting marker.
 let marker=has_effect(player,jiahao::MARKER,base);
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);
 match jiahao::marker_action(owned,alive,marker){
  Some(true)=>{apply(player,jiahao::MARKER,0);},
  Some(false)=>{remove(read_ptr(player+0x190),jiahao::MARKER);},
  None=>{}
 }
 let Some(state)=JIAHAO.get() else{return;};
 {let Ok(mut state)=state.lock() else{return;};state.synchronize(player,stats,owned,alive);}
 // Marker state 25013 gates native JumpTable 8 directly in the backstep TAE.
 // No wall-clock buff or hit-nullification override controls these frames.
 if !owned||!alive{return;}
 let module=read_ptr(read_ptr(player+0x1b8)+0x148);if module==0{return;}
 let add:unsafe extern "system" fn(usize,f32)=std::mem::transmute(base+GAUGE_ADD_PERCENT_RVA);
 // Never hold a state lock across native gauge mutation. Retry blocked losses.
 for _ in 0..64{
  let delta={let Ok(state)=state.lock() else{return;};state.pending()};let Some(delta)=delta else{break;};
  let current=ptr::read_unaligned((module+0x14) as *const f32);let max=ptr::read_unaligned((module+0x18) as *const f32);
  if !current.is_finite()||!max.is_finite()||max<=0.{break;}
  let expected=(current+max*delta*0.01).clamp(0.,max);add(module,delta);
  let after=ptr::read_unaligned((module+0x14) as *const f32);
  if (after-expected).abs()>max*0.0001{break;}
  if let Ok(mut state)=state.lock(){state.acknowledge();}
 }
}
// Same native speed getter used by the CT's module+1818 field, without writing it.
unsafe extern "system" fn speed_detour(controller:usize)->f32 {
 let original:unsafe extern "system" fn(usize)->f32=std::mem::transmute(SPEED_ORIG.load(Ordering::Acquire));
 let rate=original(controller);let player=ESCAPE_PLAYER.load(Ordering::Acquire);
 if !ACTIVE.load(Ordering::Acquire)||player==0||controller==0||read_ptr(controller+8)!=player{return rate;}
 let modules=read_ptr(player+0x1b8);
 if modules!=0&&read_ptr(modules+0x28).checked_add(0x1800)==Some(controller){2.0}else{rate}
}
unsafe fn update_escape(player:usize,base:usize,dt:f32){
 let modules=read_ptr(player+0x1b8);if modules==0{ESCAPE_PLAYER.store(0,Ordering::Release);return;}
 let stats=read_ptr(modules);let animation=read_ptr(modules+0x28);
 if stats==0||animation==0{ESCAPE_PLAYER.store(0,Ordering::Release);return;}
 let owned=has_effect(player,escape::OWNER,base);
 let Some(state)=ESCAPE.get() else{return;};
 if !owned{
  ESCAPE_PLAYER.store(0,Ordering::Release);
  maintain_status_effect(player,stats,escape::READY,false,base);
  {let Ok(mut e)=state.lock() else{return;};if !e.owned(){return;}*e=escape::Escape::default();}
  let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);
  for id in [escape::BUFF,escape::READY,escape::VISUAL]{if has_effect(player,id,base){remove(read_ptr(player+0x190),id);}}
  return;
 }

 let incapacitated=read_i32(stats+0x140)<=0||has_effect(player,102100,base)||has_effect(player,102150,base);
 let present=has_effect(player,escape::BUFF,base);let cooldown=has_effect(player,escape::COOLDOWN,base);
 let (trigger,active,ready)={let Ok(mut e)=state.lock() else{return;};let (trigger,active)=e.tick(player,stats,owned,!incapacitated,read_i32(stats+0x140),read_i32(stats+0x144),cooldown,present,dt);(trigger,active,owned&&!incapacitated&&!cooldown&&e.ready())};
 // Never hold state lock across native application/removal callbacks.
 if trigger{
  let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);
  apply(player,escape::BUFF,0);apply(player,escape::COOLDOWN,0);apply(player,escape::VISUAL,0);
 }
 if (!owned||incapacitated)&&present{
  let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);remove(read_ptr(player+0x190),escape::BUFF);
 }
 if (!active||incapacitated)&&has_effect(player,escape::VISUAL,base){
  let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);remove(read_ptr(player+0x190),escape::VISUAL);
 }
 maintain_status_effect(player,stats,escape::READY,ready,base);
 ESCAPE_PLAYER.store(if active&&(present||trigger){player}else{0},Ordering::Release);
}
unsafe fn update_runtime(player:usize,base:usize,dt:f32) {
 let Some((stats,s))=snapshot(player,base) else{return;};
 let Some(state)=RUNTIME.get() else{return;};
 let (actions,drinking_reset,drinking_active)={
  let Ok(mut r)=state.lock() else{return;};
  r.synchronize(player,stats,s);
  (r.tick(s,dt),r.take_drinking_reset(),r.drinking_active())
 };
 // Drop the lock before native application: its successful-effect hook can
 // reenter while native replacement/cycle effects are being processed.
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);
 if drinking_reset {
  let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);
  for slot in 0..runtime::DRINK_STACKS {let effect=runtime::drinking_effect(slot);if has_effect(player,effect,base) {remove(read_ptr(player+0x190),effect);}}
  if has_effect(player,runtime::DRINK_GLOW,base) {remove(read_ptr(player+0x190),runtime::DRINK_GLOW);}
 }
 update_flasks(player,stats,base,s);
 for effect in actions {
  if effect==runtime::REFILL {
   let module=read_ptr(read_ptr(player+0x1b8)+0x148);
   // Current 67150A caller places the percentage in XMM1. The native function
   // scales it by .01 * max gauge, clamps, and emits the charge-ready event.
   let add_percent:unsafe extern "system" fn(usize,f32)=std::mem::transmute(base+GAUGE_ADD_PERCENT_RVA);
   add_percent(module,100.0);
   if let Some((_,after))=snapshot(player,base) {
    if let Ok(mut r)=state.lock(){r.note_refill_attempt(after.gauge>=99.9);}
   }
   continue;
  }
  if runtime::is_drinking_effect(effect) {
   // Only recycle this expired slot; never remove another bottle's live row.
   let present=has_effect(player,effect,base);
   if present {
    let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);
    remove(read_ptr(player+0x190),effect);
    let still_present=has_effect(player,effect,base);
    if still_present {continue;}
   }
  }
  apply(player,effect,0);
 }
 let glow=has_effect(player,runtime::DRINK_GLOW,base);
 if drinking_active && !glow {
  apply(player,runtime::DRINK_GLOW,0);
 }else if !drinking_active && glow {
  let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);
  remove(read_ptr(player+0x190),runtime::DRINK_GLOW);
 }
 // Read after pulse application: reaching full HP or spending the last 5 FP
 // must stop the aura immediately. Do not reapply it on every healing tick.
 let regenerating=snapshot(player,base).map(|(current_stats,current)|current_stats==stats && runtime::can_regenerate(current)).unwrap_or(false);
 let regen_glow=has_effect(player,runtime::REGEN_GLOW,base);
 if regenerating && !regen_glow {
  apply(player,runtime::REGEN_GLOW,0);
 }else if !regenerating && regen_glow {
  let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);
  remove(read_ptr(player+0x190),runtime::REGEN_GLOW);
 }
 let ultimate_ready={let Ok(r)=state.lock() else{return;};r.ultimate_ready(s)};
 maintain_status_effect(player,stats,runtime::ULTIMATE_READY,ultimate_ready,base);
 update_crystal(player,stats,base,s.hp,s.owners & 8 != 0,dt);
 update_cat(player,stats,base,s.hp,s.owners & 16 != 0);
}

unsafe fn flask_count(player:usize,base:usize)->Option<i32> {
 let get:unsafe extern "system" fn(usize,*mut i32)->u8=std::mem::transmute(base+FLASK_COUNT_RVA);
 let mut count=-1; if get(player,&mut count)!=0 && (0..=20).contains(&count){Some(count)}else{None}
}
unsafe fn update_flasks(player:usize,stats:usize,base:usize,s:runtime::Snapshot) {
 let Some(state)=FLASKS.get() else{return;};
 let paid=has_effect(player,flasks::PAID,base);
 let (probe,request,target)={
  let Ok(mut f)=state.lock() else{return;};
  f.synchronize((player,stats),s.hp>0,s.owners&2!=0,paid);
  if s.hp<=0 || s.owners&2==0 || paid{return;}
  let probe=f.probe();
  let request=if f.capacity_ready(){flask_count(player,base).and_then(|n|f.request(n))}else{None};
  (probe,request,f.target())
 };
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);
 if let Some(marker)=probe {
  let ok=apply(player,marker,0);
  if ok==0 {if let Ok(mut f)=state.lock(){f.probe_failed();}}
 }
 if let Some(marker)=request {
  let ok=apply(player,marker,0);
  if ok==0 {if let Ok(mut f)=state.lock(){f.request_failed();}}
 }
 if target.is_some() {
  // Both probe and grant refill using native events. All capacity and item
  // mutation stays in the game; the DLL only reads and acknowledges.
  let Some(_capacity)=flask_count(player,base) else{return;};
  {
   let ok=apply(player,flasks::PAID,0);
   if ok!=0 && has_effect(player,flasks::PAID,base) {
    if let Ok(mut f)=state.lock(){if let Some(marker)=f.complete(){
     let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);
     remove(read_ptr(player+0x190),marker);
    }}
   }
  }
 }
}

// Native on-damage dispatch at this site passes hit+280 even when immunity
// prevents HP loss. Sword heals from full hit damage, including overkill and
// locked-HP targets, but excludes nullified hits. Cat includes immune hits.
unsafe extern "system" fn on_cat_hit(module:usize,hit:usize,before:i32,source:usize) {
 if !ACTIVE.load(Ordering::Acquire)||module==0||hit==0{return;}
 let base=BASE.load(Ordering::Acquire);let player=read_ptr(module+8);
 if player==0{return;}
 if source!=player&&is_local(source,base)&&has_effect(source,sword::OWNER,base)&&spell_percentage(hit).is_some()&&sword::near(sword_hit_distance(source,hit,base),sword::NEAR_DISTANCE){
  let caster_modules=read_ptr(source+0x1b8);
  if caster_modules!=0{
   let caster_stats=read_ptr(caster_modules);
   if caster_stats!=0&&read_i32(caster_stats+0x140)>0{
    let amount=sword::heal_amount(read_i32(hit+0x280),spell_percentage(hit).unwrap_or(0.),ptr::read((hit+0x2c3) as *const u8)&2!=0);
    if amount>0{if let Some(state)=SWORD.get(){if let Ok(mut h)=state.lock(){h.queue(source,caster_stats,amount);}}}
   }
  }
 }
 if is_local(player,base)&&has_effect(player,jiahao::OWNER,base){
  let stats=read_ptr(read_ptr(player+0x1b8));if stats!=0{if let Some(state)=JIAHAO.get(){if let Ok(mut state)=state.lock(){state.synchronize(player,stats,true,read_i32(stats+0x140)>0);state.damaged(read_i32(hit+0x280));}}}
 }
 if source!=player&&is_local(source,base)&&has_effect(source,jiahao::OWNER,base){
  let recipient_stats=read_ptr(read_ptr(player+0x1b8));let stats=read_ptr(read_ptr(source+0x1b8));
  if stats!=0&&recipient_stats!=0&&before>read_i32(recipient_stats+0x140)&&ptr::read((hit+0x2c3) as *const u8)&2==0{
   // Native 50/51 expose stateInfo 434 throughout stance/guard break. 36
   // covers the short explicit open-to-critical window; ordinary hit stun
   // has neither condition. Frontal critical recovery is handled separately.
   if let Some(state)=JIAHAO.get(){if let Ok(mut state)=state.lock(){state.synchronize(source,stats,true,read_i32(stats+0x140)>0);state.hit(jiahao::is_backstep_attack(&ptr::read_unaligned((hit+0x124) as *const [u32;8])),read_i32(hit+0x280),has_state(player,434,base)||has_effect(player,36,base));}}
  }
 }
 if !is_local(player,base)||!has_effect(player,cat::OWNER,base){return;}
 let modules=read_ptr(player+0x1b8);if modules==0{return;}
 let stats=read_ptr(modules);if stats==0{return;}
 let after=read_i32(stats+0x140);
 let damage=read_i32(hit+0x280);
 if let Some(state)=CAT.get(){if let Ok(mut c)=state.lock(){c.hit(player,stats,true,damage,after);}}
}
unsafe fn update_cat(player:usize,stats:usize,base:usize,hp:i32,owned:bool) {
 if let Some(state)=CAT_LANDING.get() {if let Ok(mut state)=state.lock() {
  if owned&&hp>0 {if let Some(sample)=cat_fall_height(player,base) {state.observe_frame(sample,true,true,GetTickCount64());}}
  else {state.observe_frame(cat_jump::Sample{player,stats,airborne:false,native_height:0.,world_y:0.,spring_available:false},owned,hp>0,GetTickCount64());}
 }}
 let Some(state)=CAT.get() else{return;};
 let present=has_effect(player,cat::INVISIBLE,base);
 // Do not add/remove the shared native Finale VFX while a separate effect
 // already grants invisibility; in particular never shorten Duchess's Art.
 let other=has_effect(player,703500,base)||(has_state(player,600,base)&&!present);
 let requested={let Ok(mut c)=state.lock() else{return;};c.tick(player,stats,owned,hp,other)};
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);
 if !owned||hp<=0 {if present {remove(read_ptr(player+0x190),cat::INVISIBLE);}return;}
 if requested {
  if present {remove(read_ptr(player+0x190),cat::INVISIBLE);}
  let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);
  apply(player,cat::INVISIBLE,0);
 }
}

type StatusSortFn=unsafe extern "system" fn(*const status_sync::DisplayEntry,*const status_sync::DisplayEntry)->u8;
unsafe extern "system" fn status_sort_detour(left:*const status_sync::DisplayEntry,right:*const status_sync::DisplayEntry)->u8{
 let original:StatusSortFn=std::mem::transmute(STATUS_SORT_ORIG.load(Ordering::Acquire));
 status_sort_call(original,left,right)
}
unsafe extern "system" fn hud_sort_detour(left:*const status_sync::DisplayEntry,right:*const status_sync::DisplayEntry)->u8{
 let original:StatusSortFn=std::mem::transmute(HUD_SORT_ORIG.load(Ordering::Acquire));
 status_sort_call(original,left,right)
}
unsafe fn status_sort_call(original:StatusSortFn,left:*const status_sync::DisplayEntry,right:*const status_sync::DisplayEntry)->u8{
 if !ACTIVE.load(Ordering::Acquire)||left.is_null()||right.is_null(){return original(left,right);}
 status_sync::compare(original,left,right)
}
unsafe fn maintain_status_effect(player:usize,stats:usize,effect:i32,wanted:bool,base:usize) {
 let manager=read_ptr(player+0x190);if manager==0{return;}
 let due={let Ok(mut refresh)=STATUS_REFRESH.get_or_init(||Mutex::new(status_sync::Refresh::default())).lock() else{return;};
  refresh.due((player,stats,manager),effect,wanted,GetTickCount64(),status_sync::READY_INTERVAL_MS)};
 if due {
  let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);
  apply(player,effect,0);
 }else if !wanted&&has_effect(player,effect,base) {
  let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);
  remove(manager,effect);
 }
}
unsafe fn refresh_owned_status(player:usize,base:usize) {
 let modules=read_ptr(player+0x1b8);if modules==0{return;}
 let stats=read_ptr(modules);let manager=read_ptr(player+0x190);if stats==0||manager==0{return;}
 let alive=read_i32(stats+0x140)>0&&!has_effect(player,102100,base)&&!has_effect(player,102150,base);
 let now=GetTickCount64();
 for effect in hex_pool::OWNER_IDS {
  // Confirm current native ownership. Never replay a cached ownership mask.
  let mut wanted=alive&&has_effect(player,effect,base);
  if wanted {
   // An old regulation uses stacking category 10; do not repeatedly apply it.
   let mut node=read_ptr(manager+8);wanted=false;
   for _ in 0..4096 {
    if node==0{break;}
    if read_i32(node+8)==effect&&ptr::read_unaligned((node+0x68) as *const u32)&0x800c0003==0 {
     let row=read_ptr(node);wanted=row!=0&&ptr::read_unaligned((row+0x13e) as *const i16)==20;break;
    }
    node=read_ptr(node+0x38);
   }
  }
  let due={let Ok(mut refresh)=STATUS_REFRESH.get_or_init(||Mutex::new(status_sync::Refresh::default())).lock() else{return;};
   refresh.due((player,stats,manager),effect,wanted,now,status_sync::OWNER_INTERVAL_MS)};
  if due {
   let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);
   apply(player,effect,0);
  }
 }
}
unsafe fn crystal_snapshot(player:usize,base:usize,hp:i32,owned:bool)->crystal::Snapshot {
 crystal::Snapshot { owned,hp,cooldown:has_effect(player,crystal::COOLDOWN,base),visual:has_effect(player,crystal::READY,base),beast:has_effect(player,707115,base) }
}
unsafe extern "system" fn on_crystal_hit(module:usize,hit:usize,special:u8,source:usize) {
 if !ACTIVE.load(Ordering::Acquire)||module==0||hit==0 {return;}
 let base=BASE.load(Ordering::Acquire);
 // Verified 42C120 getter; this is the recipient, not the attacking player.
 let player=read_ptr(module+8);
 if !is_local(player,base) {return;}
 let modules=read_ptr(player+0x1b8);if modules==0{return;}
 let stats=read_ptr(modules);if stats==0{return;}
 let hp=read_i32(stats+0x140);
 let s=crystal_snapshot(player,base,hp,true);
 let nullify=hit+0x2c3;
 let flags=ptr::read(nullify as *const u8);
 let damage=read_i32(hit+0x280);
 let eligible=crystal::Hit { special:special!=0,ignores_nullify:ptr::read((hit+0x160) as *const u8)&0x20!=0,hp_damage:damage,already_nullified:flags&2!=0 };
 if !has_effect(player,crystal::OWNER,base){return;}
 let Some(state)=SHELL.get() else{return;};
 let Ok(mut shell)=state.lock() else{return;};
 if shell.absorb(player,stats,s,eligible) {
   // Same bit the native successful random proc writes at 43D5E7. Consumption
   // and this write happen under the same lock before any next hit can absorb.
  ptr::write(nullify as *mut u8,flags|2);
  // A native callback can re-enter damage handling; release our state lock
  // before asking the attacker's module to play/send its normal recoil.
  drop(shell);
  crystal_rebound(player,source,hit,base);
 }
}
unsafe fn crystal_rebound(player:usize,source:usize,hit:usize,base:usize) {
 // R12 is nonzero only for native ChrIns sources. Also require the actual hit
 // source to be that character: bullets/indirect attacks must not recoil their
 // owner. The damage module getter used by native recoil is module+8.
 if source==0||source==player||read_ptr(hit+0x228)!=source {return;}
 let modules=read_ptr(source+0x1b8);if modules==0{return;}
 let damage_module=read_ptr(modules+DAMAGE_MODULE_SLOT);
 // Both module types have the same owner at +8. Recoil writes +48/+4C/+74,
 // beyond the hit-stop object's 0x30 bytes; ownership alone is not a type check.
 if damage_module==0||read_ptr(damage_module)!=base+DAMAGE_MODULE_VTABLE_RVA||read_ptr(damage_module+8)!=source {return;}
 // Native 439D50 has already resolved attack hardness (including attacker
 // effects) into +2A8. Work on a copy so absorption and later hit consumers
 // retain their original flags, damage, guard state and notification result.
 let original=ptr::read_unaligned(hit as *const [u8;crystal::HIT_SIZE]);
 if let Some(recoil)=crystal::rebound_hit(&original) {
  let dispatch:unsafe extern "system" fn(usize,usize,usize)=std::mem::transmute(base+REBOUND_RVA);
  dispatch(damage_module,player,recoil.as_ptr() as usize);
 }
}
unsafe fn update_crystal(player:usize,stats:usize,base:usize,hp:i32,owned:bool,dt:f32) {
 let Some(state)=SHELL.get() else{return;};
 let s=crystal_snapshot(player,base,hp,owned);
 let (actions,ready)={let Ok(mut shell)=state.lock() else{return;};let actions=shell.tick(player,stats,s,dt);(actions,shell.visual_ready(s))};
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);
 // Current 3CDD40 wrapper passes player+190 and the ID to 504E80. The latter
 // finds the node and calls 50B5B0(manager,node,1) to unlink/free/update flags.
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);
 for action in actions {
  match action {
   crystal::Action::RemoveVisual=>if has_effect(player,crystal::READY,base) {
    remove(read_ptr(player+0x190),crystal::READY);
   },
   crystal::Action::StartCooldown=>{
    apply(player,crystal::COOLDOWN,0);
   },
   crystal::Action::PrepareVisual=>{}, // Shared refresh below reserves each native request.
  }
 }
 maintain_status_effect(player,stats,crystal::READY,ready,base);
}
#[cfg(any(feature="test-hotkeys",test))]
fn pressed_edge(state:&AtomicBool,down:bool)->bool {
 let previous=state.swap(down,Ordering::Relaxed);
 down&&!previous
}
// Current native 5759E0 receives exactly manager/descriptor; its local branch
// returns the reward node from 57D0A0 in RAX. Preserve unrelated calls verbatim.
unsafe extern "system" fn reward_create_detour(manager:usize,descriptor:usize)->usize {
 let original:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(REWARD_CREATE_ORIG.load(Ordering::Acquire));
 arts_bind_drop(descriptor);
 let scope=PERSONAL_REWARD.with(Cell::get);
 if descriptor==0||scope.manager==0||manager!=scope.manager {return original(manager,descriptor);}
 let kind=ptr::read_unaligned((descriptor+0x58) as *const u32);
 let owner=ptr::read_unaligned((descriptor+0xaa) as *const u16);
 let lot=read_i32(descriptor+0xb8);
 match personal_reward::route(scope,manager,lot,kind,owner) {
  personal_reward::Route::Original=>original(manager,descriptor),
  personal_reward::Route::Skip=>0,
  personal_reward::Route::Local(flags)=>{
   ptr::write_unaligned((descriptor+0x58) as *mut u32,flags);
   let node=original(manager,descriptor);
   if node!=0 {PERSONAL_REWARD.with(|c|{let mut s=c.get();s.created+=1;c.set(s);});}
   node
  }
 }
}
unsafe fn owned_hexes(player:usize,base:usize)->u32 {
 hex_pool::OWNER_IDS.iter().enumerate().fold(0,|mask,(i,&effect)|mask|if has_effect(player,effect,base){1<<i}else{0})
}
unsafe fn player_data_hero(game_data:usize)->Option<u8> {
 if game_data==0{return None;}
 let data=read_ptr(game_data+8);if data==0{return None;}
 Some(ptr::read((data+0xc7) as *const u8))
}
unsafe fn local_hero(base:usize)->Option<u8> {
 player_data_hero(read_ptr(base+GAME_DATA_RVA))
}
unsafe fn unavailable_hexes(player:usize,base:usize)->u32 {
 reward_choices::role_exclusions(reward_choices::terrain_exclusions(owned_hexes(player,base),has_effect(player,95001601,base)),local_hero(base))
}
fn random_order(owned:u32)->Option<usize> {
 let candidates=reward_choices::candidates(owned);
 let count=candidates.len() as u32;if count==0{return None;}
 loop {
  let mut random=0u32;
  if unsafe{BCryptGenRandom(ptr::null_mut(),&mut random as *mut u32 as *mut u8,4,2)}<0 {return None;}
  if random<u32::MAX-u32::MAX%count {return Some(candidates[(random%count) as usize]);}
 }
}
unsafe fn unclaimed_hex_ball(base:usize)->bool {
 let manager=read_ptr(base+ITEMS_RVA);if manager==0{return true;}
 let mut node=read_ptr(manager+0x58);
 // Native list belongs to ItemMan and is examined only on character Update.
 for _ in 0..4096 {
  if node==0{return false;}
  let kind=ptr::read_unaligned((node+0x78) as *const u32);
  let owner=ptr::read_unaligned((node+0xca) as *const u16);
  let lot=read_i32(node+0xd8);
  if kind&15==2 && owner&0x40!=0 && (95200000..hex_pool::LEGACY_LOT_END).contains(&lot)
     && reward_choices::is_hex_good(ptr::read_unaligned((node+0x28) as *const u32)) {return true;}
  node=read_ptr(node);
 }
 true // A malformed/oversized list must not issue extra rewards.
}
unsafe fn spawn_personal(player:usize,base:usize,chosen:usize)->bool {
 if unclaimed_hex_ball(base){return false;}
 let created=spawn_personal_lot(player,base,hex_pool::LOT_IDS[chosen]);
 created==3
}
unsafe fn spawn_personal_lot(player:usize,base:usize,lot:i32)->u32 {
 let item_man=read_ptr(base+ITEMS_RVA);let game_data=read_ptr(base+GAME_DATA_RVA);
 if item_man==0||game_data==0{return 0;}
 let data=read_ptr(game_data+8);if data==0{return 0;}
 let slot=read_i32(data+8).max(1);if slot>3{return 0;}
 let scope=personal_reward::Scope{manager:item_man,lot,slot:slot as u8,created:0};
 let previous=PERSONAL_REWARD.with(Cell::get);
 if previous.manager!=0{return 0;}
 PERSONAL_REWARD.with(|c|c.set(scope));
 type Spawn=unsafe extern "system" fn(usize,usize,i32,u8,u8,u8,u8,*const f32)->u8;
 let spawn:Spawn=std::mem::transmute(base+SPAWN_RVA);
 let reward_position=[0f32;4];
 // Keep native lottery, character position and choice aggregation. Only our
 // descriptors are routed locally; other party slots never create nodes.
 spawn(item_man,player,scope.lot,0,1,1,0,reward_position.as_ptr());
 let created=PERSONAL_REWARD.with(|c|{let result=c.get().created;c.set(previous);result});
 created
}
unsafe fn update_reward_recovery(player:usize,base:usize) {
 let Some(recovery)=REWARD_RECOVERY.get() else{return;};
 if !spatial_ready(player){return;}
 let modules=read_ptr(player+0x1b8);let stats=read_ptr(modules);
 if stats==0{return;}
 let expedition=has_effect(player,level_rewards::EXPEDITION,base);
 // The same packed source map used by the native latent-power constructor.
 let mut map=(read_ptr(player+8)>>32) as u32;
 if map==u32::MAX {
  let context=read_ptr(base+MAP_CONTEXT_RVA);
  if context!=0 {map=ptr::read_unaligned((context+0x2c) as *const u32);}
 }
 let hub=matches!(map>>24,10|11);
 let incapacitated=read_i32(stats+0x140)<=0 || (expedition&&(has_effect(player,102100,base)||has_effect(player,102150,base)));
 let physics=read_ptr(modules+0x68);
 let grounded=ptr::read((physics+0x92) as *const u8)!=0;
 let pos=position(player,base);
 let ready={let Ok(mut state)=recovery.lock() else{return;};state.observe(player,expedition,hub,incapacitated,grounded,[pos[0],pos[1],pos[2]])};
 if !ready||map==u32::MAX{return;}
 if CALLBACK_LOCK.swap(true,Ordering::AcqRel){return;}
 let complete=relocate_unclaimed_hexes(player,base,map);
 CALLBACK_LOCK.store(false,Ordering::Release);
 if complete {if let Ok(mut state)=recovery.lock(){state.complete();}}
}
unsafe fn relocate_unclaimed_hexes(player:usize,base:usize,mut map:u32)->bool {
 let manager=read_ptr(base+ITEMS_RVA);let game_data=read_ptr(base+GAME_DATA_RVA);
 if manager==0||game_data==0{return false;}
 let data=read_ptr(game_data+8);if data==0{return false;}
 let slot=read_i32(data+8).max(1);if slot>3{return false;}
 // Gather first. Never retain native pointers across frames or change links.
 let mut nodes=Vec::new();let mut node=read_ptr(manager+0x58);
 for _ in 0..4096 {
  if node==0{break;}
  if reward_recovery::eligible(ptr::read_unaligned((node+0x78) as *const u32),ptr::read_unaligned((node+0xca) as *const u16),read_i32(node+0xd8),ptr::read_unaligned((node+0x28) as *const u32),slot as u8) {
   nodes.push(node);
  }
  node=read_ptr(node);
 }
 if node!=0{return false;} // Malformed list: leave every native node intact.
 if nodes.is_empty(){return true;}
 #[repr(C,align(16))] struct WorldPosition([f32;4]);
 let mut world=WorldPosition([0.;4]);let mut local=[0f32;3];
 let getter:unsafe extern "system" fn(usize,*mut f32)->*const f32=std::mem::transmute(base+CHR_POSITION_RVA);
 let convert:unsafe extern "system" fn(*mut f32,*const f32,*mut u32)->u8=std::mem::transmute(base+MAP_POSITION_RVA);
 // Reuse 576DBF..576E07: world coordinates -> packed map/local coordinates.
 let point=getter(player,world.0.as_mut_ptr());
 if point.is_null() || convert(local.as_mut_ptr(),point,&mut map)==0 || !local.iter().all(|v|v.is_finite()){return false;}
 let hide:unsafe extern "system" fn(usize)=std::mem::transmute(base+REWARD_HIDE_RVA);
 let show:unsafe extern "system" fn(usize)=std::mem::transmute(base+REWARD_SHOW_RVA);
 // Clear the old transform/visual before moving all grouped choice nodes.
 // The original show function displays the group head, skipping its children.
 for &node in &nodes {hide(node);}
 for &node in &nodes {reward_recovery::set_location(node,map,local);}
 for &node in &nodes {show(node);}
 true
}
unsafe fn update_level_rewards(player:usize,base:usize) {
 let game_data=read_ptr(base+GAME_DATA_RVA);if game_data==0{return;}
 let data=read_ptr(game_data+8);if data==0{return;}
 let level=read_i32(data+0x68);
 let Some(delivery)=LEVEL_DELIVERY.get() else{return;};
 let changed={let Ok(mut state)=delivery.lock() else{return;};state.changed(player,level)};
 let now=GetTickCount64();
 let due=NEXT_LEVEL_CHECK.load(Ordering::Relaxed);
 if !changed&&now<due{return;}
 NEXT_LEVEL_CHECK.store(now.saturating_add(250),Ordering::Relaxed);
 let modules=read_ptr(player+0x1b8);if modules==0{return;}
 let stats=read_ptr(modules);if stats==0{return;}
 let alive=read_i32(stats+0x140)>0;
 let expedition=has_effect(player,level_rewards::EXPEDITION,base);
 if !(1..=15).contains(&level){return;}
 // Save eligibility separately from delivery, so queued milestones survive
 // death, object replacement and reloads even when level falls below them.
 let mut receipts=0;let mut reached=level_rewards::reached(level);
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);
 if alive&&expedition {
  for (i,&effect) in level_rewards::REACHED.iter().enumerate() {
   if has_effect(player,effect,base){reached|=1<<i;}
   else if reached&(1<<i)!=0 {apply(player,effect,0);}
  }
 }
 for (i,&effect) in level_rewards::RECEIPTS.iter().enumerate() {if has_effect(player,effect,base){receipts|=1<<i;}}
 let pending_ball=unclaimed_hex_ball(base);
 let collected={let Ok(mut state)=delivery.lock() else{return;};state.just_collected(pending_ball)};
 if collected {NEXT_LEVEL_CHECK.store(now.saturating_add(50),Ordering::Relaxed);return;}
 let Some(index)=level_rewards::queued(reached,receipts,alive,expedition,pending_ball) else{return;};
 if CALLBACK_LOCK.swap(true,Ordering::AcqRel){return;}
 let receipt=level_rewards::RECEIPTS[index];
 let Some(chosen)=random_order(unavailable_hexes(player,base)) else{CALLBACK_LOCK.store(false,Ordering::Release);return;};
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);
 // Native application queues its effect: has_effect is false until the
 // character update processes it. Never reserve a saved receipt before
 // creation and mistake that deferred receipt for a delivered reward.
 // CALLBACK_LOCK prevents reentry and the delay allows the receipt to settle.
 if spawn_personal(player,base,chosen) {
  apply(player,receipt,0);
 }else{
  nlog_error!("Personal level reward creation failed; milestone {} remains unissued",level_rewards::LEVELS[index]);
 }
 NEXT_LEVEL_CHECK.store(now.saturating_add(250),Ordering::Relaxed);
 CALLBACK_LOCK.store(false,Ordering::Release);
}
unsafe fn promote_cage_mutation(actor:usize,base:usize,notified:bool){
 if actor==0{return;}
 let vtable=read_ptr(actor);if vtable==0||read_ptr(vtable+0x1a0)!=base+0x4d1ea0{return;}
 let reference=read_ptr(actor+0x5b8);if reference==0{return;}
 let npc=read_i32(reference+8);
 if !(cage_mutation::FIRST_NPC..cage_mutation::FIRST_NPC+cage_mutation::NPC_COUNT).contains(&npc){return;}
 if !notified{cage_trace::observe(actor,npc,base);}
 let location=(actor+0x1fb) as *mut u8;
 let flags=ptr::read(location);
 let marked=flags&1!=0||notified||has_effect(actor,cage_mutation::MARKER,base);
 let promoted=cage_mutation::promoted_flags(npc,marked,flags);
 if promoted!=flags{ptr::write(location,promoted);}
 // Apply through the existing native source on the normal Update callback,
 // after the marker's effect operation has returned. Source slots record the
 // IDs already attached, so repeated updates never reapply a completed set.
 if notified||promoted&1==0{return;}
 let source=read_ptr(actor+0x5d8);if source==0{return;}
 let source_vtable=read_ptr(source);if source_vtable==0||read_ptr(source_vtable+0x58)!=base+0x4fff30||read_ptr(source_vtable+0x40)!=base+0x50ed10{return;}
 let slots=read_ptr(source+0x18);if slots==0||read_i32(source+0x20)!=4{return;}
 if read_i32(slots)>=0&&read_i32(slots+4)>=0{return;}
 let refresh:unsafe extern "system" fn(usize)=std::mem::transmute(base+0x50ed10);
 refresh(source);
}
unsafe fn combat_alive(player:usize,stats:usize,base:usize)->bool{
 read_i32(stats+0x140)>0&&!has_effect(player,102100,base)&&!has_effect(player,102150,base)
}
unsafe fn update_chant(player:usize,base:usize,dt:f32){
 let modules=read_ptr(player+0x1b8);if modules==0{return;}let stats=read_ptr(modules);if stats==0{return;}
 let alive=combat_alive(player,stats,base);
 let enabled=alive&&has_effect(player,chant::OWNER,base)&&matches!(local_hero(base),Some(4|6|7));
 if let Ok(mut s)=CHANT.get_or_init(||Mutex::new(chant::Chant::default())).lock(){s.frame(player,stats,enabled,dt);}
 let present=has_effect(player,chant::ENABLED,base);
 if enabled&&!present{let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+APPLY_SELF_RVA);apply(player,chant::ENABLED,0);}
 else if !enabled&&present{let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+REMOVE_EFFECT_RVA);remove(read_ptr(player+0x190),chant::ENABLED);}
}
unsafe fn chant_pulse(player:usize,base:usize){
 let modules=read_ptr(player+0x1b8);if modules==0{return;}let stats=read_ptr(modules);if stats==0{return;}
 let active=combat_alive(player,stats,base)&&has_effect(player,chant::OWNER,base)&&has_effect(player,100610,base)&&matches!(local_hero(base),Some(4|6|7));
 let amount=CHANT.get().and_then(|s|s.lock().ok().map(|mut s|s.pulse(player,stats,active,read_i32(stats+0x15c)>=read_i32(stats+0x160)))).unwrap_or(0);
 // Same signed integer addition/clamping API as native HKS act(1001).
 if amount>0{let add:unsafe extern "system" fn(usize,i32,u8)=std::mem::transmute(base+STAMINA_ADD_RVA);add(stats,amount,0);}
}
unsafe extern "system" fn on_update(player:usize,dt:f32) {
 if !ACTIVE.load(Ordering::Acquire)||player==0{return;}
 let base=BASE.load(Ordering::Acquire);
 // Also restore the native mutation state on a newly loaded peer replica.
 promote_cage_mutation(player,base,false);
 let world=read_ptr(base+WORLD_RVA);
 if world==0{return;}
 if read_ptr(world+0x174e8)!=player {return;}
 if !dt.is_finite()||dt<=0.0||dt>1.0{return;}
 update_chant(player,base,dt);
 update_runtime(player,base,dt);
 update_sword(player,base);
 update_escape(player,base,dt);
 update_jiahao(player,base,dt);
 update_arts(player,base);
 update_noisy(player,base,dt);
 actor_lifecycle::update(player,base,dt);
 update_reward_recovery(player,base);
 update_level_rewards(player,base);
 refresh_owned_status(player,base);
 #[cfg(feature="test-hotkeys")]
 update_test_hotkeys(player,base);
}

#[cfg(feature="test-hotkeys")]
unsafe fn update_test_hotkeys(player:usize,base:usize){
 let cfg=CONFIG.get().unwrap();
 let key=GetAsyncKeyState(cfg.key)<0;
 let pressed=pressed_edge(&KEY_WAS_DOWN,key);
 let test_pressed=pressed_edge(&TEST_KEY_WAS_DOWN,GetAsyncKeyState(cfg.test_key)<0);
 let new_hex_pressed=pressed_edge(&NEW_HEX_KEY_WAS_DOWN,GetAsyncKeyState(cfg.new_hex_key)<0);
 let chant_pressed=pressed_edge(&CHANT_KEY_WAS_DOWN,GetAsyncKeyState(cfg.chant_key)<0);
 if !pressed&&!test_pressed&&!new_hex_pressed&&!chant_pressed{return;}
 if cfg.ctrl&&GetAsyncKeyState(0x11)>=0{return;}
 let mut pid=0;GetWindowThreadProcessId(GetForegroundWindow(),&mut pid);
 if pid!=win::GetCurrentProcessId(){return;}
 let now=GetTickCount64();let last=LAST_SPAWN.load(Ordering::Relaxed);
 if last!=0&&now.saturating_sub(last)<cfg.cooldown{return;}
 let modules=read_ptr(player+0x1b8);if modules==0{return;}
 let stats=read_ptr(modules);if stats==0||ptr::read_unaligned((stats+0x140) as *const i32)<=0{return;}
 if unclaimed_hex_ball(base){return;}
 if CALLBACK_LOCK.swap(true,Ordering::AcqRel){return;}
 let owned=unavailable_hexes(player,base);
 let preferred=if chant_pressed {Some(CHANT_REWARD)}else if new_hex_pressed {Some(NEW_HEX_REWARD)}else if test_pressed {Some(TEST_REWARD)}else{None};
 let choice=preferred.and_then(|o|reward_choices::preferred(o,owned)).or_else(||random_order(owned));
 let Some(chosen)=choice else{CALLBACK_LOCK.store(false,Ordering::Release);return;};
 LAST_SPAWN.store(now,Ordering::Relaxed);
 let ok=spawn_personal(player,base,chosen);
 CALLBACK_LOCK.store(false,Ordering::Release);
 if !ok {nlog_error!("Personal test reward spawn failed for order={chosen}");}
}

#[cfg(test)]
mod tests {
 use super::*;
 #[test] fn sword_range_reads_contact_not_attack_origin_or_direction(){unsafe{
  let mut hit=[0u8;0x2d0];
  // Native producer stores attack origin, actual collision point, normal and
  // flight direction consecutively. A giant's contact can be near its feet
  // even when both the projectile origin and the actor pivot are far away.
  let origin=[30f32,1.,0.,1.];let normal=[0f32,1.,0.,0.];let direction=[1f32,0.,0.,0.];
  for (offset,v) in [(0x1e0,origin),(0x200,normal),(0x210,direction)]{
   ptr::write_unaligned(hit.as_mut_ptr().add(offset) as *mut [f32;4],v);
  }
  let player=[0f32,0.,0.,1.];
  for (point,expected) in [([2.,1.,0.,1.],true),([4.,0.,0.,1.],true),([4.001,0.,0.,1.],false),([0.,4.001,0.,1.],false),([f32::NAN,0.,0.,1.],false)]{
   ptr::write_unaligned(hit.as_mut_ptr().add(0x1f0) as *mut [f32;4],point);
   assert_eq!(sword::near(sword::distance(player,sword_hit_point(hit.as_ptr() as usize)),sword::NEAR_DISTANCE),expected);
  }
 }}
 thread_local!{static CRYSTAL_REBOUND_CALL:Cell<(usize,usize,usize)>=const{Cell::new((0,0,0))};}
 unsafe extern "system" fn capture_crystal_rebound(module:usize,recipient:usize,hit:usize){
  assert_eq!(ptr::read((hit+0x2be) as *const u8),1);
  assert_eq!(read_i32(hit+0x2ac),70);
  assert_eq!(ptr::read((hit+0x2c3) as *const u8),2);
  CRYSTAL_REBOUND_CALL.with(|c|{let n=c.get().2;c.set((module,recipient,n+1));});
 }
 #[test] fn shell_rebound_dispatches_only_direct_character_source_with_private_hit(){unsafe{
  let mut actor=[0usize;0x200/8];let mut modules=[0usize;0x200/8];let mut damage=[0usize;0x100/8];
  let mut hit_stop=[0usize;0x30/8];
  let source=actor.as_ptr() as usize;let mut hit=[0u8;crystal::HIT_SIZE];
  actor[0x1b8/8]=modules.as_ptr() as usize;
  modules[0x90/8]=hit_stop.as_ptr() as usize;hit_stop[1]=source;
  modules[0x98/8]=damage.as_ptr() as usize;damage[1]=source;
  hit[..4].copy_from_slice(&100f32.to_le_bytes());hit[0x2c3]=2;
  hit[0x228..0x230].copy_from_slice(&source.to_le_bytes());
  let base=capture_crystal_rebound as *const () as usize-REBOUND_RVA;
  damage[0]=base+DAMAGE_MODULE_VTABLE_RVA;
  hit_stop[0]=base+0x2c12320;
  let input=hit.as_ptr() as usize;let original=hit;
  CRYSTAL_REBOUND_CALL.with(|c|c.set((0,0,0)));
  crystal_rebound(1,0,input,base);crystal_rebound(source,source,input,base);
  crystal_rebound(1,source,input,base);
  CRYSTAL_REBOUND_CALL.with(|c|assert_eq!(c.get(),(damage.as_ptr() as usize,1,1)));
  assert_eq!(hit,original);
  // A same-owner hit-stop module in the damage slot must also be rejected.
  modules[0x98/8]=hit_stop.as_ptr() as usize;crystal_rebound(1,source,input,base);
  modules[0x98/8]=0;crystal_rebound(1,source,input,base);
  modules[0x98/8]=damage.as_ptr() as usize;
  assert_eq!(hit_stop[0],base+0x2c12320);assert_eq!(hit_stop[1],source);
  // Projectile-owned/indirect sources and mismatched modules cannot recoil.
  hit[0x228..0x230].copy_from_slice(&0usize.to_le_bytes());crystal_rebound(1,source,input,base);
  hit[0x228..0x230].copy_from_slice(&source.to_le_bytes());damage[1]=0;crystal_rebound(1,source,input,base);
  damage[1]=source;hit[0x2a8..0x2ac].copy_from_slice(&99i32.to_le_bytes());crystal_rebound(1,source,input,base);
  CRYSTAL_REBOUND_CALL.with(|c|assert_eq!(c.get().2,1));
 }}
 #[test] fn current_player_hero_follows_native_data_and_never_caches_a_previous_class(){unsafe{
  let mut data=[0u8;0x100];let mut manager=[0usize;2];
  assert_eq!(player_data_hero(0),None);
  assert_eq!(player_data_hero(manager.as_ptr() as usize),None);
  manager[1]=data.as_ptr() as usize;
  for hero in [4,6,7,1,9,10,0,255]{
   data[0xc7]=hero;
   assert_eq!(player_data_hero(manager.as_ptr() as usize),Some(hero));
  }
  manager[1]=0;assert_eq!(player_data_hero(manager.as_ptr() as usize),None);
 }}
 #[test]fn focused_reward_keeps_cage_crystal_arts_and_existing_exclusions(){
  assert_eq!(TEST_REWARD.map(|i|hex_pool::OWNER_IDS[i]),[95001600,95000700,95001400]);
  assert_eq!(ORDERS[reward_choices::preferred(TEST_REWARD,0).unwrap()],TEST_REWARD);
  for owned in [1<<14,1<<6,1<<12,(1<<14)|(1<<6)] {
   let order=ORDERS[reward_choices::preferred(TEST_REWARD,owned).unwrap()];
   assert!(reward_choices::eligible(order,owned));
   for i in TEST_REWARD {if owned&(1<<i)==0 {assert!(order.contains(&i));}}
  }
  let unavailable=reward_choices::terrain_exclusions(0,true);
  let order=ORDERS[reward_choices::preferred(TEST_REWARD,unavailable).unwrap()];
  assert_eq!(order,[6,12,0]);
  assert_eq!(reward_choices::preferred(TEST_REWARD,(1<<hex_pool::HEX_COUNT)-1),None);
 }
 #[test]fn native_lava_wrappers_forward_arguments_and_scope_visual_scale(){unsafe{
  unsafe extern "system" fn mark(a:usize,b:usize,c:usize)->usize{assert_eq!((a,b,c),(1,2,3));0xabcd}
  unsafe extern "system" fn create(a:usize,b:usize,c:usize,d:usize,e:usize)->usize{assert_eq!((a,b,c,d,e),(0,2,3,4,5));assert_eq!(LAVA_VISUAL_SCALE.with(Cell::get),1.);0x1234}
  unsafe extern "system" fn update(a:usize){assert_eq!(a,0);assert_eq!(LAVA_VISUAL_SCALE.with(Cell::get),1.);}
  unsafe extern "system" fn matrix(handle:usize,address:usize){assert_eq!(handle,1);assert_eq!(address%16,0);let m=ptr::read(address as *const [[f32;4];4]);assert_eq!(m[0][0],2.);assert_eq!(m[3],[7.,8.,9.,1.]);}
  SELF_MARK_ORIG.store(mark as *mut u8,Ordering::Release);LAVA_SFX_CREATE_ORIG.store(create as *mut u8,Ordering::Release);LAVA_SFX_UPDATE_ORIG.store(update as *mut u8,Ordering::Release);SFX_MATRIX_ORIG.store(matrix as *mut u8,Ordering::Release);
  assert_eq!(self_mark_detour(1,2,3),0xabcd);
  let scope=LavaVisualGuard::enter(2.);assert_eq!(lava_sfx_create_detour(0,2,3,4,5),0x1234);lava_sfx_update_detour(0);assert_eq!(LAVA_VISUAL_SCALE.with(Cell::get),2.);
  let m:[[f32;4];4]=[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[7.,8.,9.,1.]];sfx_matrix_detour(1,&m as *const _ as usize);drop(scope);assert_eq!(LAVA_VISUAL_SCALE.with(Cell::get),1.);
 }}
 #[test]fn recorded_lava_layout_uses_current_kind_two_reference_without_reading_the_child(){unsafe{
  let mut bullet=[0u64;0x630/8];let kinds=[2i32,3,4,4,4];let p=bullet.as_mut_ptr() as usize;
  ptr::write_unaligned((p+0x88) as *mut usize,kinds.as_ptr() as usize);
  ptr::write_unaligned((p+0x90) as *mut i32,0);
  ptr::write_unaligned((p+0x5a0) as *mut i32,-1);
  ptr::write_unaligned((p+0x5d0) as *mut i32,noisy::BULLET_BASE+3);
  ptr::write_unaligned((p+0x600) as *mut i32,noisy::BULLET_BASE+4);
  assert_eq!(lava_parameter_id(p),Some(noisy::BULLET_BASE+3));
  let mut l=noisy::Lava::default();l.synchronize(1,2,true);l.mark(true);assert!(l.tick(0.,true));
  l.created(1,p,lava_parameter_id(p).unwrap());for _ in 0..15{l.mark(true);}
  assert!(!l.tick(4.1,true));assert_eq!(l.factor(1),5.5);
  ptr::write_unaligned((p+0x5d0) as *mut i32,noisy::BULLET_BASE+4);
  l.created(1,p,lava_parameter_id(p).unwrap());l.mark(true);assert_eq!(l.factor(1),5.5);
  ptr::write_unaligned((p+0x90) as *mut i32,1);assert_eq!(lava_parameter_id(p),None);
  ptr::write_unaligned((p+0x90) as *mut i32,4);assert_eq!(lava_parameter_id(p),None);
  ptr::write_unaligned((p+0x88) as *mut usize,0);assert_eq!(lava_parameter_id(p),None);
  assert_eq!(lava_parameter_id(0),None);
 }}
 #[test]fn arts_shadow_preserves_native_drop_data_and_nested_reads(){unsafe{
  let mut entry=[0u64;4];ptr::write_unaligned((entry.as_mut_ptr() as *mut u8).add(8) as *mut u32,0x80800042);ptr::write_unaligned((entry.as_mut_ptr() as *mut u8).add(0x1c) as *mut i32,100);
  let reference=[0x80800042u64,entry.as_ptr() as u64,2000000];let original=entry;
  let (mut a,x)=arts_shadow(reference.as_ptr() as usize,1100);a[1]=x.as_ptr() as u64;
  let (mut b,y)=arts_shadow(reference.as_ptr() as usize,1101);b[1]=y.as_ptr() as u64;
  assert_eq!(read_i32(a[1] as usize+0x1c),1100);assert_eq!(read_i32(b[1] as usize+0x1c),1101);
  assert_eq!(entry,original);assert_eq!(read_i32(reference[1] as usize+0x1c),100);assert_eq!((a[0],a[2]),(reference[0],reference[2]));
  assert_eq!(arts_weapon(reference.as_ptr() as usize),Some(arts::Weapon{handle:0x80800042,original:100,base:2000000}));
 }}
 #[unsafe(naked)]
 unsafe extern "system" fn reward_fixture(_:usize,_:usize)->usize {
  core::arch::naked_asm!(
   "mov [rsp+8],rbx", "push rdi", "sub rsp,0x20", "mov rdi,rdx", "mov rbx,rcx",
   "lea rax,[rbx+rdi]", "mov rbx,[rsp+0x30]", "add rsp,0x20", "pop rdi", "ret"
  );
 }
 #[test]fn active_spell_getter_uses_native_virtual_output_and_tracks_switches(){unsafe{
  unsafe extern "system" fn get_out(action:usize,out:*mut i32)->*const i32{ptr::write(out,read_i32(action+8));out}
  unsafe extern "system" fn get_reference(action:usize,_out:*mut i32)->*const i32{(action+8) as *const i32}
  let mut actor=vec![0u8;0x1c0];let mut modules=vec![0usize;0x80/8];let mut action=[0usize;2];let mut vtable=[0usize;4];
  let actor_ptr=actor.as_mut_ptr() as usize;action[0]=vtable.as_mut_ptr() as usize;action[1]=4000;
  modules[0x60/8]=action.as_mut_ptr() as usize;ptr::write_unaligned((actor_ptr+0x1b8) as *mut usize,modules.as_mut_ptr() as usize);
  vtable[3]=get_out as *const () as usize;assert_eq!(active_magic(actor_ptr),4000);
  action[1]=4021;assert_eq!(active_magic(actor_ptr),4021);
  vtable[3]=get_reference as *const () as usize;action[1]=6400;assert_eq!(active_magic(actor_ptr),6400);
  vtable[3]=0;assert_eq!(active_magic(actor_ptr),-1);modules[0x60/8]=0;assert_eq!(active_magic(actor_ptr),-1);
 }}
 #[test] fn reward_hook_relocates_native_prologue_preserves_return_and_scopes_all_three_choices(){unsafe{
  let p=hook::prepare(reward_fixture as *mut u8,reward_create_detour as *const u8,64).unwrap();
  REWARD_CREATE_ORIG.store(p.trampoline,Ordering::Release);
  let mut descriptor=[0u64;24];let address=descriptor.as_mut_ptr() as usize;
  let flags=(address+0x58) as *mut u32;let owner=(address+0xaa) as *mut u16;let lot=(address+0xb8) as *mut i32;
  ptr::write_unaligned(flags,0xfedcba12);ptr::write_unaligned(owner,0x40c3);ptr::write_unaligned(lot,95200000);
  assert_eq!(reward_create_detour(42,address),42+address);
  assert_eq!(ptr::read_unaligned(flags),0xfedcba12);
  PERSONAL_REWARD.with(|c|c.set(personal_reward::Scope{manager:42,lot:95200000,slot:1,created:0}));
  for _ in 0..3 {ptr::write_unaligned(flags,0xfedcba12);assert_eq!(reward_create_detour(42,address),42+address);assert_eq!(ptr::read_unaligned(flags),0xfedcba02);assert_eq!(ptr::read_unaligned(owner),0x40c3);}
  assert_eq!(PERSONAL_REWARD.with(Cell::get).created,3);
  ptr::write_unaligned(owner,0x4143);ptr::write_unaligned(flags,0xfedcba12);
  assert_eq!(reward_create_detour(42,address),0);assert_eq!(ptr::read_unaligned(flags),0xfedcba12);
  assert_eq!(reward_create_detour(43,address),43+address);
  PERSONAL_REWARD.with(|c|c.set(personal_reward::Scope::default()));
 }}
 #[test] fn reward_pool_samples_every_ordered_distinct_triple_once(){
  let n=hex_pool::HEX_COUNT;
  assert_eq!(ORDERS.len(),n*(n-1)*(n-2));
  let unique:std::collections::HashSet<_>=ORDERS.into_iter().collect();
  assert_eq!(unique.len(),ORDERS.len());
  for order in ORDERS {assert!(order.iter().all(|&i|i<n));assert!(order[0]!=order[1]&&order[0]!=order[2]&&order[1]!=order[2]);}
  for slot in 0..3 {for item in 0..n {assert_eq!(ORDERS.iter().filter(|o|o[slot]==item).count(),(n-1)*(n-2));}}
  assert!(ORDERS.contains(&[3,4,5]));
  assert!(ORDERS.contains(&[6,0,1]));
  assert!(ORDERS.contains(&[7,3,6]));
 }
 #[test] fn key_config_rejects_invalid_input(){assert!(Config::read("VirtualKey=0").is_err());assert!(Config::read("TestVirtualKey=255").is_err());assert!(Config::read("VirtualKey=0x76").is_err());assert!(Config::read("CooldownMs=999").is_err());assert!(Config::read("Unknown=true").is_err());}
 #[test] fn random_and_focused_hotkeys_are_configurable(){
  for c in [Config::read("").unwrap(),Config::read(include_str!("../dist/mod/natives/NRHex.ini")).unwrap()] {
   assert_eq!((c.key,c.test_key,c.new_hex_key,c.chant_key,c.ctrl,c.cooldown),(0x75,0x76,0x77,0x78,true,3000));
  }
  let c=Config::read("VirtualKey=0x70\nTestVirtualKey=0x71\nNewHexVirtualKey=0x72\nAngelChantVirtualKey=0x73").unwrap();
  assert_eq!((c.key,c.test_key,c.new_hex_key,c.chant_key),(0x70,0x71,0x72,0x73));
  for value in ["0","255","0x75","0x76"]{assert!(Config::read(&format!("NewHexVirtualKey={value}")).is_err());}
  for value in ["0","255","0x75","0x76","0x77"]{assert!(Config::read(&format!("AngelChantVirtualKey={value}")).is_err());}
  assert_eq!(CHANT_REWARD.map(|i|hex_pool::OWNER_IDS[i]),[chant::OWNER,sword::OWNER,95000100]);
  for hero in [4,6,7]{
   assert_eq!(ORDERS[reward_choices::preferred(CHANT_REWARD,reward_choices::role_exclusions(0,Some(hero))).unwrap()],CHANT_REWARD);
   let next=ORDERS[reward_choices::preferred(CHANT_REWARD,reward_choices::role_exclusions(1<<17,Some(hero))).unwrap()];assert_eq!(next[0],8);assert!(!next.contains(&17));
   let next=ORDERS[reward_choices::preferred(CHANT_REWARD,reward_choices::role_exclusions(1<<8,Some(hero))).unwrap()];assert_eq!(next[0],17);assert!(!next.contains(&8));
  }
  let excluded=ORDERS[reward_choices::preferred(CHANT_REWARD,reward_choices::role_exclusions(0,Some(1))).unwrap()];assert_eq!(excluded[0],0);assert!(!excluded.contains(&17));assert!(!excluded.contains(&8));
  assert_eq!(NEW_HEX_REWARD.map(|i|hex_pool::OWNER_IDS[i]),[balloon::OWNER,spirit::OWNER,95000100]);
  let both=ORDERS[reward_choices::preferred(NEW_HEX_REWARD,reward_choices::role_exclusions(0,Some(6))).unwrap()];
  assert_eq!(both,NEW_HEX_REWARD);
  let after_balloon=ORDERS[reward_choices::preferred(NEW_HEX_REWARD,reward_choices::role_exclusions(1<<15,Some(6))).unwrap()];
  assert_eq!(after_balloon[0],16);assert!(!after_balloon.contains(&15));
  let other=ORDERS[reward_choices::preferred(NEW_HEX_REWARD,reward_choices::role_exclusions(0,Some(1))).unwrap()];
  assert_eq!(other[0],15);assert!(!other.contains(&16));
  for retired in ["Shell","Cat","Sword","Strength","Escape","Jiahao","Arts","Noisy","Cage"] {
   assert!(Config::read(&format!("{retired}VirtualKey=0x70")).is_err());
  }
 }
 #[test] fn hotkey_rearms_after_release_and_does_not_repeat_while_held(){
  let state=AtomicBool::new(false);
  assert!(!pressed_edge(&state,false));assert!(pressed_edge(&state,true));
  assert!(!pressed_edge(&state,true));assert!(!pressed_edge(&state,false));
  assert!(pressed_edge(&state,true));
 }
 #[test] fn legacy_sword_distance_cannot_override_compiled_range(){
  for value in ["0","5","6","7.5","NaN","inf","101"]{assert!(Config::read(&format!("SwordNearDistance={value}")).is_ok());}
  assert_eq!(sword::NEAR_DISTANCE,4.);
  assert!(sword::near(Some(4.),sword::NEAR_DISTANCE));
  assert!(!sword::near(Some(4.001),sword::NEAR_DISTANCE));
 }
 #[test] fn constructed_impact_stays_boosted_for_shared_and_network_copies(){unsafe{
  for original in [0,1,2,3,8,7] {
   let mut hit=[0u8;0x300];hit[0x24]=original;strengthen_constructed_hit(hit.as_mut_ptr() as usize);
   for _ in 0..3 {let transmitted=hit;assert_eq!(transmitted[0x24],strength::remap(original));}
   // A newly constructed attack starts from its own native input again.
   hit[0x24]=original;strengthen_constructed_hit(hit.as_mut_ptr() as usize);
   assert_eq!(hit[0x24],strength::remap(original));
  }
 }}
 #[inline(never)]
 unsafe extern "system" fn sample(integer:u64,value:f64)->f64 {integer as f64+value*1.25}
 #[inline(never)]
 unsafe extern "system" fn sample_damage(a:usize,b:usize,c:usize)->f64 {
  assert_eq!((a,b,c),(11,22,33));17.25
 }
 #[inline(never)]
 unsafe extern "system" fn sample_damage_integer(a:usize,b:usize,c:usize)->u64 {
  assert_eq!((a,b,c),(11,22,33));0x123456789abcdef0
 }
 #[inline(never)]
 unsafe extern "system" fn sample_dispatch(a:usize,b:usize,c:usize,d:u8,e:i32,f:usize,g:f32)->u64 {
  assert_eq!((a,b,c,d,e,f,g),(11,22,33,173,-249,0x12345678,1.25));0xabcdef0123456789
 }
 #[inline(never)]
 unsafe extern "system" fn sample_dispatch_float(a:usize,b:usize,c:usize,d:u8,e:i32,f:usize,g:f32)->f64 {
  assert_eq!((a,b,c,d,e,f,g),(11,22,33,173,-249,0x12345678,1.25));123.75
 }
 #[test] fn strength_wrapper_preserves_all_seven_native_arguments_and_return_registers(){unsafe{
  ACTIVE.store(false,Ordering::Release);
  let p=hook::prepare(sample_dispatch as *mut u8,strength_detour as *const u8,64).unwrap();
  HIT_BUILD_ORIG.store(p.trampoline,Ordering::Release);
  let call:unsafe extern "system" fn(usize,usize,usize,u8,i32,usize,f32)->u64=std::mem::transmute(strength_detour as *const u8);
  assert_eq!(call(11,22,33,173,-249,0x12345678,1.25),0xabcdef0123456789);
  let p=hook::prepare(sample_dispatch_float as *mut u8,strength_detour as *const u8,64).unwrap();
  HIT_BUILD_ORIG.store(p.trampoline,Ordering::Release);
  let call:unsafe extern "system" fn(usize,usize,usize,u8,i32,usize,f32)->f64=std::mem::transmute(strength_detour as *const u8);
  assert_eq!(call(11,22,33,173,-249,0x12345678,1.25),123.75);
 }}
 #[test] fn sword_attack_rates_entry_preserves_stack_arguments_and_integer_and_float_returns(){unsafe{
  ACTIVE.store(false,Ordering::Release);
  let p=hook::prepare(sample_dispatch as *mut u8,sword_rates_detour as *const u8,64).unwrap();
  ATTACK_RATES_ORIG.store(p.trampoline,Ordering::Release);
  let call:unsafe extern "system" fn(usize,usize,usize,u8,i32,usize,f32)->u64=std::mem::transmute(sword_rates_detour as *const u8);
  assert_eq!(call(11,22,33,173,-249,0x12345678,1.25),0xabcdef0123456789);
  let p=hook::prepare(sample_dispatch_float as *mut u8,sword_rates_detour as *const u8,64).unwrap();
  ATTACK_RATES_ORIG.store(p.trampoline,Ordering::Release);
  let call:unsafe extern "system" fn(usize,usize,usize,u8,i32,usize,f32)->f64=std::mem::transmute(sword_rates_detour as *const u8);
  assert_eq!(call(11,22,33,173,-249,0x12345678,1.25),123.75);
 }}
 #[test] fn extra_melee_filter_forwards_five_arguments_and_does_not_mutate_original_spell_descriptor(){unsafe{
  unsafe extern "system" fn predicate(node:usize,descriptor:usize,owner:usize,weapon:i32,extra:i32)->u8{
   assert_eq!((node,owner,weapon,extra),(11,33,-17,0x76543210));
   let copy=&*(descriptor as *const [u8;0x3c]);assert_eq!(copy[0],1);
   assert!(copy[1..].iter().all(|byte|*byte==0x5a));
   ptr::write(descriptor as *mut u8,7);173
  }
  let mut descriptor=[0x5au8;0x3c];descriptor[0]=3;let before=descriptor;
  assert_eq!(sword_retry_as_melee(predicate,11,descriptor.as_ptr() as usize,33,-17,0x76543210),173);
  assert_eq!(descriptor,before);
 }}
 #[test] fn rear_calculation_wrapper_forwards_three_arguments_and_preserves_integer_and_float_returns(){unsafe{
  ACTIVE.store(false,Ordering::Release);
  let p=hook::prepare(sample_damage as *mut u8,rear_detour as *const u8,64).unwrap();
  DAMAGE_ORIG.store(p.trampoline,Ordering::Release);
  let wrapped:unsafe extern "system" fn(usize,usize,usize)->f64=std::mem::transmute(rear_detour as *const u8);
  assert_eq!(wrapped(11,22,33),17.25);
  let p=hook::prepare(sample_damage_integer as *mut u8,rear_detour as *const u8,64).unwrap();
  DAMAGE_ORIG.store(p.trampoline,Ordering::Release);
  let wrapped:unsafe extern "system" fn(usize,usize,usize)->u64=std::mem::transmute(rear_detour as *const u8);
  assert_eq!(wrapped(11,22,33),0x123456789abcdef0);
 }}
 #[test] fn detour_preserves_integer_float_arguments_and_float_return(){unsafe{
  ACTIVE.store(false,Ordering::Release);
  let prepared=hook::prepare(sample as *mut u8,update_detour as *const u8,64).unwrap();
  ORIG.store(prepared.trampoline,Ordering::Release);
  // Call the real naked wrapper and relocated original without patching live code.
  let wrapped:unsafe extern "system" fn(u64,f64)->f64=std::mem::transmute(update_detour as *const u8);
  for (a,b) in [(7,2.5),(0,-17.25),(1000000,0.125)]{assert_eq!(wrapped(a,b),sample(a,b));}
 }}
 #[inline(never)]
 unsafe extern "system" fn sample_effect(a:usize,b:i32,c:usize,d:usize,e:usize,f:u8,g:u8,h:u8,i:u8,j:u8,k:i32)->u8 {
  assert_eq!((a,b,c,d,e,f,g,h,i,j,k),(11,22,33,44,55,66,77,88,99,111,222));
  173
 }
 #[inline(never)]
 unsafe extern "system" fn sample_action_request(input:usize,mode:i32,action:i32)->u8 {
  assert_eq!(input,0x123456789abcdef0);assert_eq!(mode,-17);action as u8
 }
 #[test] fn action_request_hook_forwards_module_mode_action_and_original_al(){unsafe{
  ACTIVE.store(false,Ordering::Release);
  let p=hook::prepare(sample_action_request as *mut u8,action_request_detour as *const u8,64).unwrap();
  ACTION_REQUEST_ORIG.store(p.trampoline,Ordering::Release);
  for result in [0,1,173,255]{assert_eq!(action_request_detour(0x123456789abcdef0,-17,result),result as u8);}
 }}
 #[test] fn effect_hook_forwards_all_eleven_arguments_and_success_byte(){unsafe{
  ACTIVE.store(false,Ordering::Release);
  let p=hook::prepare(sample_effect as *mut u8,effect_detour as *const u8,64).unwrap();
  APPLY_ORIG.store(p.trampoline,Ordering::Release);
  assert_eq!(effect_detour(11,22,33,44,55,66,77,88,99,111,222),173);
 }}
 // Enter a synthetic mid-function site with exactly the game's stack alignment
 // and nonvolatile registers. Exercise the real wrapper and relocated test,
 // including the original conditional branch and volatile register readback.
 #[unsafe(naked)]
 unsafe extern "system" fn crystal_fixture_entry(_:usize,_:u64,_:f64)->u64 {
  core::arch::naked_asm!(
   "push rsi", "push r14", "push r13", "sub rsp,0x20",
   "mov rsi,rcx", "mov r14,rcx", "mov r13,rdx",
   "mov r8,0x8877", "mov r9,0x9988", "mov r10,0xaa99", "mov r11,0xbbaa",
   "movaps xmm0,xmm2", "movaps xmm1,xmm2", "movaps xmm3,xmm2", "movaps xmm4,xmm2", "movaps xmm5,xmm2",
   "jmp {detour}",detour=sym crystal_detour
  );
 }
 #[unsafe(naked)]
 unsafe extern "system" fn crystal_fixture_site() {
  core::arch::naked_asm!(
   "test byte ptr [rsi+0x2c3],2", "jz 2f", "mov eax,1", "jmp 3f", "2:", "xor eax,eax", "3:",
   "mov [rsi],rcx", "mov [rsi+8],rdx", "mov [rsi+16],r8", "mov [rsi+24],r9",
   "mov [rsi+32],r10", "mov [rsi+40],r11",
   "movq [rsi+48],xmm0", "movq [rsi+56],xmm1", "movq [rsi+64],xmm2",
   "movq [rsi+72],xmm3", "movq [rsi+80],xmm4", "movq [rsi+88],xmm5",
   "add rsp,0x20", "pop r13", "pop r14", "pop rsi", "ret"
  );
 }
 #[test] fn crystal_mid_function_wrapper_preserves_stack_registers_and_original_branch(){unsafe{
  ACTIVE.store(false,Ordering::Release);
  let p=hook::prepare(crystal_fixture_site as *mut u8,crystal_detour as *const u8,64).unwrap();
  assert_eq!(p.stolen,7); NULLIFY_ORIG.store(p.trampoline,Ordering::Release);
  for flag in [0u8,2u8] {
   let mut context=[0u64;100];
   ptr::write((context.as_mut_ptr() as *mut u8).add(0x2c3),flag);
   let address=context.as_mut_ptr() as usize;
   assert_eq!(crystal_fixture_entry(address,0x6655,17.25),(flag!=0) as u64);
   assert_eq!(&context[..6],&[address as u64,0x6655,0x8877,0x9988,0xaa99,0xbbaa]);
   for &v in &context[6..12] {assert_eq!(f64::from_bits(v),17.25);}
  }
 }}
 #[unsafe(naked)]
 unsafe extern "system" fn cat_fixture_entry(_:usize,_:u64,_:f64)->u64 {
  core::arch::naked_asm!(
   "push rsi", "push r14", "push rbp", "sub rsp,0x30",
   "mov rsi,rcx", "mov r14,rcx", "lea rbp,[rcx+0xb9]",
   "mov r8,0x8877", "mov r9,0x9988", "mov r10,0xaa99", "mov r11,0xbbaa",
   "movaps xmm0,xmm2", "movaps xmm1,xmm2", "movaps xmm3,xmm2", "movaps xmm4,xmm2", "movaps xmm5,xmm2",
   "jmp {detour}",detour=sym cat_detour
  );
 }
 #[unsafe(naked)]
 unsafe extern "system" fn cat_fixture_site() {
  core::arch::naked_asm!(
   "mov eax,[rbp-0x59]", "mov [rsp+0x20],eax",
   "mov [rsi],rcx", "mov [rsi+8],rdx", "mov [rsi+16],r8", "mov [rsi+24],r9",
   "mov [rsi+32],r10", "mov [rsi+40],r11",
   "movq [rsi+48],xmm0", "movq [rsi+56],xmm1", "movq [rsi+64],xmm2",
   "movq [rsi+72],xmm3", "movq [rsi+80],xmm4", "movq [rsi+88],xmm5",
   "cmp [rsp+0x20],eax", "sete al", "movzx eax,al",
   "add rsp,0x30", "pop rbp", "pop r14", "pop rsi", "ret"
  );
 }
 #[test] fn cat_mid_function_wrapper_preserves_registers_and_replays_saved_hp_stack_arguments(){unsafe{
  ACTIVE.store(false,Ordering::Release);
  let p=hook::prepare(cat_fixture_site as *mut u8,cat_detour as *const u8,64).unwrap();
  assert_eq!(p.stolen,7);HIT_COMPLETE_ORIG.store(p.trampoline,Ordering::Release);
  let mut context=[0u64;100];context[12]=100;
  let address=context.as_mut_ptr() as usize;
  assert_eq!(cat_fixture_entry(address,0x6655,17.25),1);
  assert_eq!(&context[..6],&[address as u64,0x6655,0x8877,0x9988,0xaa99,0xbbaa]);
  for &v in &context[6..12] {assert_eq!(f64::from_bits(v),17.25);}
 }}
}
