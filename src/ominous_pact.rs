//! Landing an attack requests one synchronous native HP cost. HP changes and
//! hit-driven effect replacement are separate native APIs; both are needed.
use crate::{has_effect, is_local, native_actor, read_i32, read_ptr};
use std::cell::Cell;
use std::sync::Mutex;

pub const OWNER: i32 = 95002600;
pub const PULSE: i32 = 95002601;
const COOLDOWN_SECONDS: f64 = 10.0;
static COOLDOWN: Mutex<HitCooldown> = Mutex::new(HitCooldown::new());

// One local-player cooldown, shared by all native hit notifications (including
// simultaneous targets). Never hold this lock across a reentrant native call.
struct HitCooldown { object: (usize, usize), time: f64, ready_at: f64 }
impl HitCooldown {
    const fn new() -> Self { Self { object: (0, 0), time: 0., ready_at: 0. } }
    fn sync(&mut self, object: (usize, usize)) {
        if self.object != object { *self = Self { object, ..Self::new() }; }
    }
    fn advance(&mut self, object: (usize, usize), owned: bool, dt: f32) {
        if !owned || object.1 == 0 { *self = Self::new(); return; }
        self.sync(object);
        if dt.is_finite() && dt > 0. && dt <= 1. { self.time += f64::from(dt); }
    }
    fn claim(&mut self, object: (usize, usize)) -> bool {
        self.sync(object);
        if self.time < self.ready_at { return false; }
        self.ready_at = self.time + COOLDOWN_SECONDS;
        true
    }
}

pub unsafe fn update(player: usize, base: usize, dt: f32) {
    let stats = native_actor::stats(player);
    let owned = stats != 0 && has_effect(player, OWNER, base);
    if let Ok(mut cooldown) = COOLDOWN.lock() {
        cooldown.advance((player, stats), owned, dt);
    }
}
// CSPlayerDamageModule is derived from the NPC/common damage module. Its
// vtable differs even though the owner accessor and HP-cost API are shared.
pub const PLAYER_DAMAGE_VTABLE_RVA: usize = 0x2c11d00;
pub const HP_RVA: usize = 0x431090;
pub const HP_GUARD_RVA: usize = 0x4311ae;
pub const HP_PATTERN: &str = "8B D6 F7 DA C6 44 24 30 01 F3 0F 10 05 DD 24 EC 02 F3 0F 11 44 24 28 0F 57 C9 F3 0F 11 4C 24 20 45 33 C9";
pub const NOTIFY_RVA: usize = 0x504d30;
pub const NOTIFY_PATTERN: &str = "0F 57 C0 0F 2F C8 0F 86 BB 00 00 00 48 8B 51 08 48 85 D2 0F 84 AE 00 00 00 4C 8D 15 B0 B2 AF FF 8B 42 68 A9";
thread_local! {static IN_COST: Cell<bool> = const { Cell::new(false) };}
struct CostScope;
impl Drop for CostScope {fn drop(&mut self){IN_COST.with(|s|s.set(false));}}

pub unsafe fn pulse(player: usize, effect: i32, base: usize) -> bool {
    if effect != PULSE || !is_local(player, base) {return false;}
    // Consume the marker instead of installing another delayed HP effect.
    if !has_effect(player, OWNER, base) || IN_COST.with(|s|s.get()) {return true;}
    let modules=read_ptr(player+0x1b8);
    if modules==0 {return true;}
    let stats=read_ptr(modules);
    if stats==0 || native_actor::incapacitated(player,stats) {return true;}
    let damage=read_ptr(modules+crate::DAMAGE_MODULE_SLOT);
    let effects=read_ptr(player+0x190);
    if damage==0 || effects==0 || read_ptr(damage)!=base+PLAYER_DAMAGE_VTABLE_RVA || read_ptr(damage+8)!=player {return true;}
    let cost=read_i32(stats+0x144)/20;
    if cost<=0 {return true;}
    // The original stateInfo=2103 on-hit selector owns hit eligibility. Only
    // throttle its accepted requests here; misses cannot start the cooldown.
    // Reserve before HP cost/notifications, even if an invulnerability effect
    // later prevents HP loss. Extra hits never extend the first deadline.
    if !COOLDOWN.lock().map(|mut s|s.claim((player,stats))).unwrap_or(false) {return true;}
    IN_COST.with(|s|s.set(true));let _scope=CostScope;
    let before=read_i32(stats+0x140);
    let hurt: unsafe extern "system" fn(usize,i32)=std::mem::transmute(base+HP_RVA);
    hurt(damage,cost);
    // Re-resolve after a native call; never retain an object across frames.
    let current=native_actor::stats(player);
    if current==0 || current!=stats {return true;}
    let after=read_i32(current+0x140);
    let lost=before.saturating_sub(after).max(0);
    if lost>0 {
        let current_effects=read_ptr(player+0x190);
        if current_effects!=0 {
            let notify: unsafe extern "system" fn(usize,f32,u8,u8,u8)=std::mem::transmute(base+NOTIFY_RVA);
            // Unclassified HP cost, no elemental impact, not a periodic hit.
            // The native dispatcher selects all matching damage criteria,
            // including criterion 12 used by madness/sleep relics.
            notify(current_effects,lost as f32,0xfe,0,0);
        }
        crate::on_contract_hp_damage(player,current,base,lost,after);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::win::*;
    use core::ptr;
    thread_local! {
        static OWNED:Cell<bool>=const{Cell::new(true)};
        static BLOCK:Cell<bool>=const{Cell::new(false)};
        static REENTER:Cell<bool>=const{Cell::new(false)};
        static REPLACE:Cell<bool>=const{Cell::new(false)};
        static BASE:Cell<usize>=const{Cell::new(0)};
        static COST:Cell<(usize,i32,usize)>=const{Cell::new((0,0,0))};
        static NOTIFIED:Cell<(usize,f32,u8,u8,u8,usize)>=const{Cell::new((0,0.,0,0,0,0))};
    }
    unsafe extern "system" fn owned(_:usize,id:i32)->u8{(id==OWNER&&OWNED.with(|s|s.get())) as u8}
    unsafe extern "system" fn hurt(module:usize,cost:i32){
        let player=read_ptr(module+8);let stats=native_actor::stats(player);
        COST.with(|s|{let old=s.get();s.set((module,cost,old.2+1));});
        if REENTER.with(|s|s.replace(false)){pulse(player,PULSE,BASE.with(|s|s.get()));}
        if !BLOCK.with(|s|s.get()){ptr::write_unaligned((stats+0x140) as *mut i32,read_i32(stats+0x140).saturating_sub(cost).max(0));}
        if REPLACE.with(|s|s.get()){ptr::write_unaligned(read_ptr(player+0x1b8) as *mut usize,0);}
    }
    unsafe extern "system" fn notify(manager:usize,damage:f32,attr:u8,special:u8,periodic:u8){
        NOTIFIED.with(|s|{let old=s.get();s.set((manager,damage,attr,special,periodic,old.5+1));});
    }
    unsafe fn jump(address:usize,target:usize){
        let mut bytes=[0u8;12];bytes[..2].copy_from_slice(&[0x48,0xb8]);bytes[2..10].copy_from_slice(&target.to_le_bytes());bytes[10..].copy_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(bytes.as_ptr(),address as *mut u8,12);
    }
    #[test]
    fn native_cost_is_local_once_per_ten_seconds_and_notifies_only_actual_loss(){unsafe{
        let memory=VirtualAlloc(ptr::null_mut(),0x3d00000,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE);
        assert!(!memory.is_null());let base=memory as usize;BASE.with(|s|s.set(base));
        jump(base+crate::HAS_EFFECT_RVA,owned as *const () as usize);jump(base+HP_RVA,hurt as *const () as usize);jump(base+NOTIFY_RVA,notify as *const () as usize);
        let mut world=vec![0u8;0x174f0];let mut player=vec![0u8;0x718];let mut stats=vec![0u8;0x170];let mut modules=vec![0usize;0x200/8];let mut damage=vec![0usize;0x90/8];let manager=vec![0u8;0x30];
        let actor=player.as_mut_ptr() as usize;let statistics=stats.as_mut_ptr() as usize;
        ptr::write_unaligned((base+crate::WORLD_RVA) as *mut usize,world.as_mut_ptr() as usize);
        ptr::write_unaligned(world.as_mut_ptr().add(0x174e8) as *mut usize,actor);
        ptr::write_unaligned(player.as_mut_ptr().add(0x1b8) as *mut usize,modules.as_mut_ptr() as usize);
        ptr::write_unaligned(player.as_mut_ptr().add(0x190) as *mut usize,manager.as_ptr() as usize);
        modules[0]=statistics;modules[crate::DAMAGE_MODULE_SLOT/8]=damage.as_mut_ptr() as usize;
        damage[0]=base+PLAYER_DAMAGE_VTABLE_RVA;damage[1]=actor;
        for (hp,max_hp,blocked,reentrant) in [(1000,1000,false,false),(10,1000,false,false),(1000,1000,true,false),(1000,1000,false,true),(100,10,false,false)]{
            *COOLDOWN.lock().unwrap()=HitCooldown::new();
            ptr::write_unaligned(stats.as_mut_ptr().add(0x140) as *mut i32,hp);ptr::write_unaligned(stats.as_mut_ptr().add(0x144) as *mut i32,max_hp);
            BLOCK.with(|s|s.set(blocked));REENTER.with(|s|s.set(reentrant));COST.with(|s|s.set((0,0,0)));NOTIFIED.with(|s|s.set((0,0.,0,0,0,0)));
            assert!(pulse(actor,PULSE,base));let cost=max_hp/20;let lost=if blocked {0}else{cost.min(hp)};
            assert_eq!(read_i32(statistics+0x140),hp-lost);
            assert_eq!(COST.with(|s|s.get().2),usize::from(cost>0));
            if lost>0{assert_eq!(NOTIFIED.with(|s|s.get()),(manager.as_ptr() as usize,lost as f32,0xfe,0,0,1));}else{assert_eq!(NOTIFIED.with(|s|s.get().5),0);}
        }
        BLOCK.with(|s|s.set(false));ptr::write_unaligned(stats.as_mut_ptr().add(0x140) as *mut i32,1000);ptr::write_unaligned(stats.as_mut_ptr().add(0x144) as *mut i32,1000);
        COST.with(|s|s.set((0,0,0)));NOTIFIED.with(|s|s.set((0,0.,0,0,0,0)));
        *COOLDOWN.lock().unwrap()=HitCooldown::new();
        for _ in 0..20{pulse(actor,PULSE,base);}
        assert_eq!(read_i32(statistics+0x140),950);assert_eq!(COST.with(|s|s.get().2),1);assert_eq!(NOTIFIED.with(|s|s.get().5),1);
        for _ in 0..9{update(actor,base,1.);pulse(actor,PULSE,base);assert_eq!(COST.with(|s|s.get().2),1);}
        update(actor,base,0.5);update(actor,base,0.499);pulse(actor,PULSE,base);
        assert_eq!(COST.with(|s|s.get().2),1);
        update(actor,base,0.001);pulse(actor,PULSE,base);
        assert_eq!(read_i32(statistics+0x140),900);assert_eq!(COST.with(|s|s.get().2),2);assert_eq!(NOTIFIED.with(|s|s.get().5),2);
        for mode in 0..7{
            *COOLDOWN.lock().unwrap()=HitCooldown::new();
            COST.with(|s|s.set((0,0,0)));NOTIFIED.with(|s|s.set((0,0.,0,0,0,0)));OWNED.with(|s|s.set(mode!=0));
            player[0x710]=if mode==1{1}else if mode==2{2}else{0};damage[0]=base+if mode==6{crate::DAMAGE_MODULE_VTABLE_RVA}else{PLAYER_DAMAGE_VTABLE_RVA+usize::from(mode==3)};damage[1]=if mode==4{0}else{actor};modules[0]=if mode==5{0}else{statistics};
            assert!(pulse(actor,PULSE,base));assert_eq!(COST.with(|s|s.get().2),0);assert_eq!(NOTIFIED.with(|s|s.get().5),0);
        }
        assert!(!pulse(actor,OWNER,base));assert!(!pulse(actor+1,PULSE,base));
        OWNED.with(|s|s.set(true));player[0x710]=0;damage[0]=base+PLAYER_DAMAGE_VTABLE_RVA;damage[1]=actor;modules[0]=statistics;
        REPLACE.with(|s|s.set(true));COST.with(|s|s.set((0,0,0)));NOTIFIED.with(|s|s.set((0,0.,0,0,0,0)));assert!(pulse(actor,PULSE,base));assert_eq!(COST.with(|s|s.get().2),1);assert_eq!(NOTIFIED.with(|s|s.get().5),0);REPLACE.with(|s|s.set(false));
        assert_eq!(VirtualFree(memory,0,MEM_RELEASE),1);
    }}

    #[test]
    fn native_hit_burst_has_one_deadline_and_rejected_hits_do_not_extend_it(){
        let mut s=HitCooldown::new();let actor=(1,2);assert!(s.claim(actor));
        for _ in 0..999{s.advance(actor,true,0.01);for _ in 0..10{assert!(!s.claim(actor));}}
        s.advance(actor,true,0.011);assert!(s.claim(actor));assert!(!s.claim(actor));
        for _ in 0..9{s.advance(actor,true,1.);assert!(!s.claim(actor));}
        s.advance(actor,true,0.5);assert!(!s.claim(actor));s.advance(actor,true,0.5);assert!(s.claim(actor));
    }
    #[test]
    fn paused_or_invalid_time_cannot_expire_cooldown(){
        let mut s=HitCooldown::new();let actor=(1,2);assert!(s.claim(actor));
        for dt in [0.,-1.,f32::NAN,f32::INFINITY,1.01]{s.advance(actor,true,dt);assert!(!s.claim(actor));}
        for _ in 0..9{s.advance(actor,true,1.);assert!(!s.claim(actor));}
        s.advance(actor,true,1.);assert!(s.claim(actor));
    }
    #[test]
    fn owner_loss_or_recreated_actor_cannot_inherit_another_cooldown(){
        let mut s=HitCooldown::new();let actor=(1,2);assert!(s.claim(actor));
        s.advance(actor,false,0.);assert!(s.claim(actor));assert!(!s.claim(actor));
        assert!(s.claim((1,3)));assert!(!s.claim((1,3)));assert!(s.claim((4,5)));
        s.advance((4,0),true,0.);assert!(s.claim((4,5)));
    }
    #[test]
    fn simultaneous_native_notifications_reserve_once_across_threads(){
        use std::sync::{Arc,Barrier};
        let gate=Arc::new(Mutex::new(HitCooldown::new()));let barrier=Arc::new(Barrier::new(8));
        let workers:Vec<_>=(0..8).map(|_|{let gate=gate.clone();let barrier=barrier.clone();std::thread::spawn(move||{barrier.wait();gate.lock().unwrap().claim((1,2))})}).collect();
        assert_eq!(workers.into_iter().filter_map(|w|w.join().ok()).filter(|v|*v).count(),1);
    }
}
