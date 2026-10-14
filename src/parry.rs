//! Native TAE guard windows, selected from unbuffed per-attack stance damage.
//! Native direction/guardability gates are retained. Our attack guard alone
//! waives HP/stamina loss and uses at least ordinary greatshield recoil strength.
use crate::{has_effect,native_actor,read_ptr,ACTIVE,BASE,HAS_STATE_RVA};
use std::sync::atomic::{AtomicPtr,Ordering};
use std::ptr;
use std::cell::RefCell;
use crate::parry_data as data;
pub const OWNER:i32=95002400;
pub const CHAIN:i32=95002401;
pub const SPARK_RIGHT:i32=95002402;
pub const SPARK_LEFT:i32=95002403;
pub const COUNTER:i32=95002404;
pub const REACTION:i32=95002405;
// Native HKS env(207) -> PlayerIns vtable +278 -> 66B060.
// Read the current equipment style, not attack-hand or input/animation guesses.
pub const HAND_STYLE_RVA:usize=0x66b060;
pub const HAND_STYLE_PATTERN:&str="48 8B 81 70 06 00 00 8B 00 C3";
// Native airborne query: current ground contact plus jump-controller and
// action flags. Unlike jump-attack categories, this also covers aerial skills.
pub const AIRBORNE_RVA:usize=0x4678e0;
pub const AIRBORNE_PATTERN:&str="40 57 48 83 EC 20 80 B9 92 00 00 00 00 48 8B F9 74 43 48 81 C1 00 04 00 00";
pub unsafe fn grounded(player:usize)->bool{
 let Some(base)=active_base() else{return false;};
 if player==0{return false;}
 let modules=read_ptr(player+0x1b8);if modules==0||read_ptr(modules+8)==0{return false;}
 let physics=read_ptr(modules+0x68);if physics==0{return false;}
 let airborne:unsafe extern "system" fn(usize)->u8=std::mem::transmute(base+AIRBORNE_RVA);
 airborne(physics)==0
}
pub unsafe fn two_handed(player:usize)->bool{
 if player==0{return false;}
 let equipment=read_ptr(player+0x670);
 equipment!=0&&matches!(crate::read_i32(equipment),2|3)
}
pub const FEEDBACK_RIGHT:u16=24000;
pub const FEEDBACK_LEFT:u16=24001;
pub const FEEDBACK_SOUND:u16=24002;
pub const FEEDBACK_WEAPON_RIGHT:u16=24003;
pub const FEEDBACK_WEAPON_LEFT:u16=24004;
// Native PlayerIns vtable +4D0 counts actual dummy points on the resolved
// model (+4A0), without the FFX transform provider's body-model fallback.
pub const DUMMY_COUNT_RVA:usize=0x3c9660;
pub const DUMMY_COUNT_PATTERN:&str="40 53 56 57 48 83 EC 50 49 8B D8 48 8B F2 48 8D 44 24 70 48 89 44 24 78 8B 02";
unsafe fn feedback_spark_state(player:usize)->Option<u16>{
 if player==0{return None;}
 let equipment=read_ptr(player+0x670);if equipment==0{return None;}
 let (dummy,weapon,hand)=match crate::read_i32(equipment){
  2=>(20110,FEEDBACK_WEAPON_RIGHT,FEEDBACK_RIGHT),
  3=>(10110,FEEDBACK_WEAPON_LEFT,FEEDBACK_LEFT),
  _=>return None,
 };
 // Query the current model each time: no inventory-slot, actor or weapon
 // pointer cache to become stale on hand/weapon swaps or network replicas.
 let vt=read_ptr(player);
 if vt==0||read_ptr(player+0x680)==0{return Some(hand);}
 let address=read_ptr(vt+0x4d0);if address==0{return Some(hand);}
 let count:unsafe extern "system" fn(usize,*const i32,*const i32)->i32=std::mem::transmute(address);
 Some(if count(player,&dummy,&dummy)>0{weapon}else{hand})
}
pub const NOTIFY_RVA:usize=0x4381f0;
pub const NOTIFY_PATTERN:&str="48 89 5C 24 08 55 56 57 41 56 41 57 48 83 EC 40 45 8B F0 48 8B FA 48 8B E9 4C 89 4C 24 20 F6 82 C1 02 00 00 08";
pub static NOTIFY_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub const RVA:usize=HAS_STATE_RVA;
// TAE stores this tag in the per-frame guard behavior field. Resolve it to 460
// before normal behavior lookup; never cache a window in wall time.
pub const GUARD_JUDGE:i32=95002400;
pub const COST_RVA:usize=0x43a0fd;
pub const COST_RESUME_RVA:usize=0x43a18e;
pub const COST_PATTERN:&str="F3 41 0F 10 B6 4C 02 00 00 F3 41 0F 59 76 28 49 8B CF E8 0C 20 FF FF";
pub const REPEL_RVA:usize=0x66b910;
pub const REPEL_PATTERN:&str="48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 57 41 56 41 57 48 83 EC 40 48 8B 81 70 06 00 00 44 8B FA";
pub static COST_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static COST_RESUME:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
pub static REPEL_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
// Weapon-behavior execution, immediately after the native stamina calculator.
// RSI is the attack module; [RSP+58] is its resolved BehaviorParam row.
pub const COUNTER_COST_RVA:usize=0x434768;
pub const COUNTER_COST_PATTERN:&str="48 8B CE 8B D8 E8 AE 79 FF FF F7 DB 45 33 C0 8B D3 48 8B 88 B8 01 00 00 48 8B 09 E8 48 1A FF FF";
pub static COUNTER_COST_ORIGINAL:AtomicPtr<u8>=AtomicPtr::new(ptr::null_mut());
const COUNTER_CATEGORIES:[usize;5]=[0x4e,0x4f,0x1c8,0x1c9,0x1ca];
#[repr(C)]struct CounterAttackRef{id:i32,cache:u32,kind:i32,padding:u32,row:usize}
type CounterLookup=unsafe extern "system" fn(*mut CounterAttackRef,i32,i32);
unsafe fn counter_cost_with(module:usize,behavior:usize,cost:i32,lookup:CounterLookup)->i32{
 let Some(base)=active_base() else{return cost;};
 if cost<=0||module==0||behavior==0||ptr::read((behavior+9) as *const u8)!=0{return cost;}
 let player=read_ptr(module+8);
 if owned_action(player).is_none()||!has_effect(player,COUNTER,base){return cost;}
 let id=crate::read_i32(behavior+0xc);if id<0{return cost;}
 let mut attack=CounterAttackRef{id:-1,cache:0,kind:1,padding:0,row:0};
 lookup(&mut attack,1,id); // native table kind 1 = PC, 0 = NPC
 if attack.row!=0&&COUNTER_CATEGORIES.iter().any(|offset|ptr::read((attack.row+offset) as *const u8)==103){0}else{cost}
}
unsafe extern "system" fn counter_cost(module:usize,behavior:usize,cost:i32)->i32{
 let Some(base)=active_base() else{return cost;};
 counter_cost_with(module,behavior,cost,std::mem::transmute(base+crate::devils_gaze_hit::LOOKUP_RVA))
}
// Preserve the full surrounding native frame. Only the final positive debit
// changes; no refill, shared parameter mutation, actor flag or cached pointer.
#[unsafe(naked)]pub unsafe extern "system" fn counter_cost_detour(){
 core::arch::naked_asm!(
  "pushfq","sub rsp,0xc8",
  "mov [rsp+0x20],rax","mov [rsp+0x28],rcx","mov [rsp+0x30],rdx",
  "mov [rsp+0x38],r8","mov [rsp+0x40],r9","mov [rsp+0x48],r10","mov [rsp+0x50],r11",
  "movdqu [rsp+0x60],xmm0","movdqu [rsp+0x70],xmm1","movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3","movdqu [rsp+0xa0],xmm4","movdqu [rsp+0xb0],xmm5",
  "mov r8d,eax","mov rcx,rsi","mov rdx,[rsp+0x128]","call {callback}","mov [rsp+0x20],eax",
  "movdqu xmm0,[rsp+0x60]","movdqu xmm1,[rsp+0x70]","movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]","movdqu xmm4,[rsp+0xa0]","movdqu xmm5,[rsp+0xb0]",
  "mov rax,[rsp+0x20]","mov rcx,[rsp+0x28]","mov rdx,[rsp+0x30]",
  "mov r8,[rsp+0x38]","mov r9,[rsp+0x40]","mov r10,[rsp+0x48]","mov r11,[rsp+0x50]",
  "add rsp,0xc8","popfq","jmp qword ptr [rip+{original}]",
  callback=sym counter_cost,original=sym COUNTER_COST_ORIGINAL
 );
}
pub fn native_guard_judge(judge:i32)->i32{if judge==GUARD_JUDGE{460}else{judge}}
thread_local!{static GUARD_SCOPES:RefCell<Vec<Option<(usize,u8,i32)>>>=const{RefCell::new(Vec::new())};}
#[cfg(test)]thread_local!{static TEST_CONTEXT:std::cell::Cell<Option<(bool,usize)>>=const{std::cell::Cell::new(None)};}
fn active_base()->Option<usize>{
 #[cfg(test)]if let Some((active,base))=TEST_CONTEXT.with(|s|s.get()){return active.then_some(base);}
 ACTIVE.load(Ordering::Acquire).then(||BASE.load(Ordering::Acquire))
}

unsafe fn owned_action(player:usize)->Option<usize>{
 let base=active_base()?;if player==0{return None;}
 let vt=read_ptr(player);
 if vt==0||read_ptr(vt+0x118)!=base+0x4d2f00{return None;}
 let modules=read_ptr(player+0x1b8);if modules==0{return None;}
 let action=read_ptr(modules+8);let stats=read_ptr(modules);
 if action==0||stats==0||native_actor::incapacitated(player,stats)||!has_effect(player,OWNER,base){return None;}
 Some(action)
}
unsafe fn attack_guard(player:usize)->bool{
 let Some(action)=owned_action(player) else{return false;};
 if !two_handed(player)||!grounded(player){return false;}
 (ptr::read((action+0x58) as *const u8)&0x20!=0&&crate::read_i32(action+0x24c)==GUARD_JUDGE)||
 followup(player,active_base().unwrap_or(0))
}
unsafe fn followup(player:usize,base:usize)->bool{
 // REACTION belongs only to guard-recoil TAE clips, throughout their native
 // guard span. Manual GuardStart/GuardOn/GuardEnd never supply it. COUNTER
 // alone is deliberately insufficient: holding guard cannot sustain a chain.
 two_handed(player)&&grounded(player)&&(has_effect(player,CHAIN,base)||(has_effect(player,COUNTER,base)&&has_effect(player,REACTION,base)))
}
pub unsafe fn update_turn(player:usize){
 let eligible=owned_action(player).is_some()&&two_handed(player)&&grounded(player);
 crate::parry_turn::update(player,eligible);
}
// The five input attack powers survive guard reduction. Body movement and
// zero-damage utility contacts must not arm attack guard or produce sparks.
unsafe fn damaging_contact(hit:usize)->bool{
 hit!=0&&(0..5).any(|i|{let v=ptr::read_unaligned((hit+i*4) as *const f32);v.is_finite()&&v>0.})
}
// Native timed SpEffect expires in game time. Borrow the guard flag only while
// evaluating a new incoming hit; no persistent actor flags or saved pointers.
pub unsafe fn enter(module:usize,hit:usize){
 let saved=(||{
  let base=active_base()?;
  if module==0||hit==0||ptr::read((hit+0x2c1) as *const u8)&8!=0{return None;}
  let player=read_ptr(module+8);let action=owned_action(player)?;
  let flags=ptr::read((action+0x58) as *const u8);let judge=crate::read_i32(action+0x24c);
  // A hand-style change or takeoff can leave this frame's private TAE guard
  // bit behind. Suppress only that bit, never an ordinary native guard.
  if !two_handed(player)||!grounded(player)||!damaging_contact(hit){
   if flags&0x20==0||judge!=GUARD_JUDGE{return None;}
   ptr::write((action+0x58) as *mut u8,flags&!0x20);
   return Some((action,flags&0x20,judge));
  }
  if !followup(player,base){return None;}
  ptr::write((action+0x58) as *mut u8,flags|0x20);
  ptr::write_unaligned((action+0x24c) as *mut i32,GUARD_JUDGE);
  Some((action,flags&0x20,judge))
 })();GUARD_SCOPES.with(|s|s.borrow_mut().push(saved));
}
pub unsafe fn leave(){
 if let Some(Some((action,guard,judge)))=GUARD_SCOPES.with(|s|s.borrow_mut().pop()){
  let flags=ptr::read((action+0x58) as *const u8);
  ptr::write((action+0x58) as *mut u8,(flags&!0x20)|guard);
  ptr::write_unaligned((action+0x24c) as *mut i32,judge);
 }
}
// Native final reaction notification: kind 3 is GUARD, after cancellation,
// death/nullification and guard-break decisions. Never use the crystal proc site.
unsafe fn on_guard_notification(module:usize,hit:usize,kind:i32)->(usize,bool){
 let Some(base)=active_base() else{return (0,false);};
 if kind!=3||module==0||hit==0||ptr::read((hit+0x2b4) as *const u8)==0||
  ptr::read((hit+0x2c1) as *const u8)&(2|8)!=0||ptr::read((hit+0x2c3) as *const u8)&2!=0{return (0,false);}
 let player=read_ptr(module+8);if player==0{return (0,false);}
 let enabled=damaging_contact(hit)&&attack_guard(player);
 let continuing=enabled&&followup(player,base);
 let manager=read_ptr(player+0x190);
 let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);
 let remove:unsafe extern "system" fn(usize,i32)->u8=std::mem::transmute(base+crate::REMOVE_EFFECT_RVA);
 // Native HKS allows guard counters only after a guard (173/174). Remember
 // which effect supplied that guard, and replace that provenance on EVERY
 // accepted guard. No arbitrary timer: slow/multi-hit counters retain the
 // native category-103 multiplier for their whole attack, never stacking it.
 let present=has_effect(player,COUNTER,base);
 if enabled&&!present{apply(player,COUNTER,0);}
 else if !enabled&&present{remove(manager,COUNTER);}
 if !enabled{return (0,false);}
 // Re-applying an existing SpEffect does not reliably restart its init FFX.
 // Native reaction TAE events now own the one-shot spark and sound.
 if has_effect(player,CHAIN,base){remove(manager,CHAIN);}apply(player,CHAIN,0);
 (read_ptr(read_ptr(player+0x1b8)+8),continuing)
}
unsafe fn feedback_query(manager:usize,state:u16)->Option<u8>{
 if !(FEEDBACK_RIGHT..=FEEDBACK_WEAPON_LEFT).contains(&state){return None;}
 let Some(base)=active_base() else{return Some(0);};
 if manager==0{return Some(0);}
 let player=read_ptr(manager+0x10);
 if player==0||read_ptr(player+0x190)!=manager||owned_action(player).is_none()||!two_handed(player)||!has_effect(player,COUNTER,base){return Some(0);}
 if state==FEEDBACK_SOUND{return Some(1);}
 Some(u8::from(feedback_spark_state(player)==Some(state)))
}
// HKS env(237) maps action +1C (resolved impact) and +20 (weapon guard
// motion class) through native 6C8970. Impact zero maps to no guard reaction,
// even for a damaging, successfully blocked hit. That bypasses the reaction
// TAE and its follow-up window. Repair only this accepted custom guard's
// notification AFTER native impact/poise resolution, not the incoming attack.
unsafe fn finish_notification(action:usize){
 if action==0||crate::read_i32(action+0x38)!=3{return;}
 let class=crate::read_i32(action+0x20);
 if (1..=4).contains(&class)&&ptr::read((action+0x1c) as *const u8)==0{
  ptr::write((action+0x1c) as *mut u8,1);
 }
}
// Runs after native facing/unblockable/cancellation tests, before the stamina
// comparison. Skip that comparison even at zero stamina.
unsafe extern "system" fn free_guard(module:usize,hit:usize)->u8{
 if active_base().is_none()||module==0||!damaging_contact(hit)||!attack_guard(read_ptr(module+8)){return 0;}
 ptr::write_unaligned((hit+0x284) as *mut i32,0);1
}
pub unsafe fn finish_guard(module:usize,hit:usize){
 if active_base().is_none()||module==0||!damaging_contact(hit)||ptr::read((hit+0x2c1) as *const u8)&8!=0||
  ptr::read((hit+0x2b4) as *const u8)==0||!attack_guard(read_ptr(module+8)){return;}
 // Per-recipient computed damage, before HP consumers and crystal-shell proc.
 // Leave buildup, hit identity and native guard/recoil flags untouched.
 for offset in [0x280,0x284,0x28c,0x290,0x294,0x298]{ptr::write_unaligned((hit+offset) as *mut i32,0);}
}
pub unsafe extern "system" fn repel(player:usize,behavior:i32)->i32{
 let original:unsafe extern "system" fn(usize,i32)->i32=std::mem::transmute(REPEL_ORIGINAL.load(Ordering::Acquire));
 let native=original(player,behavior);
 if attack_guard(player){native.max(crate::crystal::REBOUND_DEFENCE)}else{native}
}

// Verified four-register native signature. Keep all four argument bit patterns
// and the original RAX result; no deferred actor or hit pointer is retained.
pub unsafe extern "system" fn notify_detour(module:usize,hit:usize,kind:i32,direction:u64)->u64{
 let (action,continuing)=on_guard_notification(module,hit,kind);
 let turn=if action!=0{crate::parry_turn::accepted(module,hit,continuing)}else{None};
 let original:unsafe extern "system" fn(usize,usize,i32,u64)->u64=std::mem::transmute(NOTIFY_ORIGINAL.load(Ordering::Acquire));
 let result=original(module,hit,kind,direction);
 finish_notification(action);
 if let Some(turn)=turn{crate::parry_turn::apply(turn);}
 result
}

#[unsafe(naked)]
pub unsafe extern "system" fn cost_detour(){
 core::arch::naked_asm!(
  "pushfq", "sub rsp,0xc8",
  "mov [rsp+0x20],rax", "mov [rsp+0x28],rcx", "mov [rsp+0x30],rdx",
  "mov [rsp+0x38],r8", "mov [rsp+0x40],r9", "mov [rsp+0x48],r10", "mov [rsp+0x50],r11",
  "movdqu [rsp+0x60],xmm0", "movdqu [rsp+0x70],xmm1", "movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3", "movdqu [rsp+0xa0],xmm4", "movdqu [rsp+0xb0],xmm5",
  "mov rcx,r15", "mov rdx,r14", "call {callback}", "mov [rsp+0xc0],al",
  "movdqu xmm0,[rsp+0x60]", "movdqu xmm1,[rsp+0x70]", "movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]", "movdqu xmm4,[rsp+0xa0]", "movdqu xmm5,[rsp+0xb0]",
  "mov rax,[rsp+0x20]", "mov rcx,[rsp+0x28]", "mov rdx,[rsp+0x30]",
  "mov r8,[rsp+0x38]", "mov r9,[rsp+0x40]", "mov r10,[rsp+0x48]", "mov r11,[rsp+0x50]",
  "cmp byte ptr [rsp+0xc0],0", "je 2f",
  "add rsp,0xc8", "popfq", "jmp qword ptr [rip+{resume}]",
  "2:", "add rsp,0xc8", "popfq", "jmp qword ptr [rip+{original}]",
  callback=sym free_guard,resume=sym COST_RESUME,original=sym COST_ORIGINAL
 );
}
type Query=unsafe extern "system" fn(usize,u16)->u8;
const TIERS:usize=data::TIERS;
fn condition(state:u16)->Option<((i32,u8,u16,u8),u8)>{
 let index=state.checked_sub(data::FIRST)? as usize;
 data::CONDITIONS.get(index/TIERS).copied().map(|v|(v,(index%TIERS+data::MIN_TIER) as u8))
}
fn lookup(table:&[(i32,f32)],id:i32)->Option<f32>{table.binary_search_by_key(&id,|v|v.0).ok().map(|i|table[i].1)}
fn tier(weapon:f32,correction:f32)->Option<u8>{
 let poise=weapon*(correction*0.01);
 if !weapon.is_finite()||weapon<=0.||!correction.is_finite()||correction<0.||!poise.is_finite(){return None;}
 Some((poise/2.).ceil().clamp(data::MIN_TIER as f32,(data::MIN_TIER+TIERS-1) as f32) as u8)
}
// TAE uses 0=current attack hand, 1=right, 2=left. Equipment sources -1/-2
// select the currently equipped slots; using fixed slots 0/1 would be wrong.
fn source(forced:u8,current:i32)->Option<i32>{match forced{0=>Some(current),1=>Some(-1),2=>Some(-2),_=>None}}
pub unsafe extern "system" fn detour(manager:usize,state:u16)->u8{
 if let Some(value)=feedback_query(manager,state){return value;}
 let original:Query=std::mem::transmute(ORIGINAL.load(Ordering::Acquire));
 let Some(((judge,forced,required,extra),wanted))=condition(state) else{return original(manager,state);};
 if !ACTIVE.load(Ordering::Acquire)||manager==0{return 0;}
 let base=BASE.load(Ordering::Acquire);
 // 502C5D reads the owning ChrIns from manager+10. TAE must use its canonical
 // manager, not an attack/bullet copy. This works for local and peer PlayerIns.
 let player=read_ptr(manager+0x10);if player==0||read_ptr(player+0x190)!=manager{return 0;}
 let vt=read_ptr(player);if vt==0||read_ptr(vt+0x118)!=base+0x4d2f00{return 0;}
 let stats=native_actor::stats(player);
 if stats==0||native_actor::incapacitated(player,stats)||!has_effect(player,OWNER,base)||!two_handed(player)||!grounded(player){return 0;}
 if required!=0&&original(manager,required)==0{return 0;}
 let get_source:unsafe extern "system" fn(usize)->i32=std::mem::transmute(read_ptr(vt+0x260));
 let Some(source)=source(forced,if forced==0{get_source(player)}else{0}) else{return 0;};
 let weapon_id:unsafe extern "system" fn(usize,i32)->i32=std::mem::transmute(read_ptr(vt+0x250));
 let Some(weight)=lookup(data::WEAPONS,weapon_id(player,source)) else{return 0;};
 // Use the game's resolver, including the existing cross-weapon skill overlay.
 // Its fallback and selected source are identical to weapon attack behaviors.
 let behavior:crate::ArtsBehaviorFn=std::mem::transmute(read_ptr(vt+0x328));
 let id=behavior(player,judge,source,0,extra);
 let Some(correction)=lookup(data::BEHAVIORS,id) else{return 0;};
 u8::from(tier(weight,correction)==Some(wanted))
}
#[cfg(test)]mod tests{
 use super::*;
 static NOTIFY_TEST:std::sync::Mutex<()>=std::sync::Mutex::new(());
 thread_local!{static COUNTER_ROW:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};}
 unsafe extern "system" fn counter_lookup(out:*mut CounterAttackRef,kind:i32,id:i32){
  assert_eq!(kind,1);assert_eq!((*out).id,-1);assert_eq!((*out).cache,0);
  (*out).row=if id==99{0}else{COUNTER_ROW.with(|r|r.get())};
 }
 unsafe extern "system" fn counter_owns(manager:usize,id:i32)->u8{
  u8::from((manager==1||manager==2)&&id==OWNER||manager==2&&id==COUNTER)
 }
 // Enter with the real call-site alignment, no extra return address atop its
 // stack locals. The continuation checks preserved flags/registers/SIMD.
 #[unsafe(naked)]unsafe extern "system" fn run_counter(_module:usize,_behavior:usize,_cost:i32)->i32{
  core::arch::naked_asm!("push rsi","push rbx","sub rsp,0x88","mov rsi,rcx",
   "mov [rsp+0x58],rdx","mov eax,r8d","mov edx,0x12345678","movd xmm0,edx","stc",
   "jmp {detour}",detour=sym counter_cost_detour);
 }
 #[unsafe(naked)]unsafe extern "system" fn counter_continuation(){
  core::arch::naked_asm!("jnc 2f","cmp edx,0x12345678","jne 2f","movd r10d,xmm0","cmp r10d,edx","jne 2f",
   "mov rcx,rsi","mov ebx,eax","jmp 3f","2:","mov eax,0xdeadbeef",
   "3:","add rsp,0x88","pop rbx","pop rsi","ret");
 }
 #[test]fn counter_stamina_is_scoped_to_native_counter_category_and_earned_guard(){unsafe{
  let base=crate::win::VirtualAlloc(ptr::null_mut(),0xe70000,0x3000,0x40) as usize;assert_ne!(base,0);
  for(rva,address)in[(crate::HAS_EFFECT_RVA,counter_owns as *const () as usize),(crate::devils_gaze_hit::LOOKUP_RVA,counter_lookup as *const () as usize)]{
   let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&address.to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(jump.as_ptr(),(base+rva) as *mut u8,jump.len());
  }
  TEST_CONTEXT.with(|s|s.set(Some((true,base))));
  let previous=COUNTER_COST_ORIGINAL.swap(counter_continuation as *const () as *mut u8,Ordering::AcqRel);
  let mut actor=vec![0u8;0x718];let mut vt=vec![0usize;0x120/8];let mut stats=vec![0u8;0x168];let mut action=[0u8;0x260];
  let modules=[stats.as_mut_ptr() as usize,action.as_mut_ptr() as usize];let player=actor.as_mut_ptr() as usize;
  vt[0x118/8]=base+0x4d2f00;ptr::write_unaligned(player as *mut usize,vt.as_ptr() as usize);
  ptr::write_unaligned((player+0x1b8) as *mut usize,modules.as_ptr() as usize);
  stats[0x140..0x144].copy_from_slice(&100i32.to_le_bytes());stats[0x15c..0x160].copy_from_slice(&37i32.to_le_bytes());
  let module=[0usize,player];let m=module.as_ptr() as usize;
  let mut behavior=[0u8;0x20];behavior[0xc..0x10].copy_from_slice(&123i32.to_le_bytes());let b=behavior.as_ptr() as usize;
  let mut row=[0u8;0x1d0];COUNTER_ROW.with(|r|r.set(row.as_mut_ptr() as usize));
  for manager in [0usize,1,2]{ptr::write_unaligned((player+0x190) as *mut usize,manager);
   for category in [0u8,101,102,103,129]{for offset in COUNTER_CATEGORIES{
    row.fill(0);row[offset]=category;
    for cost in [-20,0,1,35,200]{assert_eq!(run_counter(m,b,cost),if manager==2&&category==103&&cost>0{0}else{cost});}
   }}
  }
  row.fill(0);row[0x4e]=103;
  for kind in [1u8,2,255]{behavior[9]=kind;assert_eq!(run_counter(m,b,35),35);}behavior[9]=0;
  for id in [-1i32,99]{behavior[0xc..0x10].copy_from_slice(&id.to_le_bytes());assert_eq!(run_counter(m,b,35),35);}
  behavior[0xc..0x10].copy_from_slice(&123i32.to_le_bytes());
  actor[0x710]=1;assert_eq!(run_counter(m,b,35),35);actor[0x710]=0;
  stats[0x140..0x144].copy_from_slice(&0i32.to_le_bytes());assert_eq!(run_counter(m,b,35),35);
  stats[0x140..0x144].copy_from_slice(&100i32.to_le_bytes());
  assert_eq!(run_counter(0,b,35),35);assert_eq!(run_counter(m,0,35),35);
  TEST_CONTEXT.with(|s|s.set(Some((false,base))));assert_eq!(run_counter(m,b,35),35);
  assert_eq!(crate::read_i32(stats.as_ptr() as usize+0x15c),37); // no refill or mutation
  COUNTER_COST_ORIGINAL.store(previous,Ordering::Release);COUNTER_ROW.with(|r|r.set(0));TEST_CONTEXT.with(|s|s.set(None));
  assert_ne!(crate::win::VirtualFree(base as *mut _,0,0x8000),0);
 }}
 unsafe extern "system" fn fixture_airborne(physics:usize)->u8{u8::from(ptr::read((physics+0x92) as *const u8)==0)}
 // Exercise the real accepted-hit callback with isolated native API stubs.
 #[test]fn counter_arms_only_on_accepted_attack_guards_and_ordinary_guard_clears_it(){unsafe{
  let _lock=NOTIFY_TEST.lock().unwrap();
  #[repr(C)]#[derive(Default)]struct Effects{reserved:[usize;2],owner:usize,ids:Vec<i32>,counter_applies:usize}
  unsafe extern "system" fn has(m:usize,id:i32)->u8{u8::from((*(m as *const Effects)).ids.contains(&id))}
  unsafe extern "system" fn add(p:usize,id:i32,_:u8)->u8{let e=&mut *(read_ptr(p+0x190) as *mut Effects);if id==COUNTER{e.counter_applies+=1;}e.ids.push(id);1}
  unsafe extern "system" fn remove(m:usize,id:i32)->u8{(*(m as *mut Effects)).ids.retain(|i|*i!=id);1}
  unsafe extern "system" fn source(_:usize)->i32{-1}
  unsafe extern "system" fn notify(m:usize,_:usize,k:i32,d:u64)->u64{
   assert_eq!(d,0xabcdef1234567890);
   let action=read_ptr(read_ptr(read_ptr(m+8)+0x1b8)+8);
   ptr::write((action+0x1c) as *mut u8,0);
   ptr::write_unaligned((action+0x20) as *mut i32,1);
   ptr::write_unaligned((action+0x38) as *mut i32,k);
   0xfedcba9876543210
  }
  let base=crate::win::VirtualAlloc(ptr::null_mut(),0x510000,0x3000,0x40) as usize;assert_ne!(base,0);
  for(rva,address)in[(AIRBORNE_RVA,fixture_airborne as *const () as usize),(crate::HAS_EFFECT_RVA,has as *const () as usize),(crate::APPLY_SELF_RVA,add as *const () as usize),(crate::REMOVE_EFFECT_RVA,remove as *const () as usize)]{
   let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&address.to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(jump.as_ptr(),(base+rva) as *mut u8,jump.len());
  }
  TEST_CONTEXT.with(|s|s.set(Some((true,base))));
  let mut effects=Effects{ids:vec![OWNER],..Effects::default()};
  let mut actor=vec![0u8;0x718];let mut vt=vec![0usize;0x268/8];
  let mut stats=vec![0u8;0x168];let mut action=vec![0u8;0x260];
  let mut physics=[0u8;0x98];physics[0x92]=1;
  let mut modules=[0usize;0x70/8];modules[0]=stats.as_mut_ptr() as usize;modules[1]=action.as_mut_ptr() as usize;modules[0x68/8]=physics.as_mut_ptr() as usize;
  let a=actor.as_mut_ptr() as usize;vt[0x118/8]=base+0x4d2f00;vt[0x260/8]=source as *const () as usize;
  let mut equipment=2i32;ptr::write_unaligned((a+0x670) as *mut usize,&equipment as *const i32 as usize);
  effects.owner=a;
  ptr::write_unaligned(a as *mut usize,vt.as_ptr() as usize);
  ptr::write_unaligned((a+0x190) as *mut usize,&mut effects as *mut Effects as usize);
  ptr::write_unaligned((a+0x1b8) as *mut usize,modules.as_ptr() as usize);
  stats[0x140..0x144].copy_from_slice(&100i32.to_le_bytes());
  action[0x58]=0x20;action[0x24c..0x250].copy_from_slice(&GUARD_JUDGE.to_le_bytes());
  let damage=[0usize,a];let mut hit=vec![0u8;crate::crystal::HIT_SIZE];let h=hit.as_mut_ptr() as usize;let m=damage.as_ptr() as usize;hit[..4].copy_from_slice(&100f32.to_le_bytes());
  // Unguarded hits and preview/cancel/replay/nullify never grant the bonus.
  on_guard_notification(m,h,3);assert_eq!(effects.counter_applies,0);hit[0x2b4]=1;
  hit[..20].fill(0);on_guard_notification(m,h,3);assert!(effects.ids==[OWNER]);
  hit[..4].copy_from_slice(&100f32.to_le_bytes());
  physics[0x92]=0;on_guard_notification(m,h,3);
  assert_eq!(effects.ids,vec![OWNER]);assert_eq!(effects.counter_applies,0);
  physics[0x92]=1;
  for kind in [0,2,7,1001,99999]{on_guard_notification(m,h,kind);assert!(effects.ids==[OWNER]);}
  for(offset,value,special)in[(0x2c1,2,0),(0x2c1,8,0),(0x2c3,2,0),(0x2c1,0,1)]{
   hit[offset]=value;on_guard_notification(m,h,if special==0{3}else{99999});assert_eq!(effects.counter_applies,0);hit[offset]=0;
  }
  let manager=&mut effects as *mut Effects as usize;
  assert_eq!(feedback_query(manager,FEEDBACK_SOUND),Some(0));
  on_guard_notification(m,h,3);assert!(effects.ids.contains(&COUNTER));assert!(!effects.ids.contains(&SPARK_RIGHT));
  assert_eq!(feedback_query(manager,FEEDBACK_RIGHT),Some(1));assert_eq!(feedback_query(manager,FEEDBACK_LEFT),Some(0));
  assert_eq!(feedback_query(manager,FEEDBACK_SOUND),Some(1));
  unsafe extern "system" fn left_source(_:usize)->i32{-2}
  vt[0x260/8]=left_source as *const () as usize;
  ptr::write_volatile(&mut equipment,3);
  assert_eq!(feedback_query(manager,FEEDBACK_LEFT),Some(1));assert_eq!(feedback_query(manager,FEEDBACK_RIGHT),Some(0));
  assert_eq!(feedback_query(0,FEEDBACK_SOUND),Some(0));assert_eq!(feedback_query(manager,606),None);
  for _ in 0..5{on_guard_notification(m,h,3);}assert_eq!(effects.counter_applies,1);
  // Continuation guards also qualify, without stacking the counter buff.
  action[0x58]=0;on_guard_notification(m,h,3);assert_eq!(effects.counter_applies,1);
  // A paused/long reaction remains eligible throughout its native TAE span,
  // after the short hit-time bridge expires. No wall-clock timeout is used.
  effects.ids.retain(|id|*id!=CHAIN);effects.ids.push(REACTION);
  assert!(attack_guard(a));on_guard_notification(m,h,3);assert_eq!(effects.counter_applies,1);
  effects.ids.retain(|id|*id!=REACTION);
  effects.ids.retain(|id|*id!=CHAIN);action[0x58]=0x20;
  action[0x24c..0x250].copy_from_slice(&460i32.to_le_bytes());
  // Switching to held manual guard removes the reaction clip, while keeping
  // the guard-counter bonus until the next accepted ordinary block. That
  // bonus must not promote manual guard back into free guard.
  assert!(effects.ids.contains(&COUNTER));assert!(!attack_guard(a));
  hit[0x284..0x288].copy_from_slice(&37i32.to_le_bytes());
  assert_eq!(free_guard(m,h),0);assert_eq!(crate::read_i32(h+0x284),37);
  enter(m,h);assert_eq!(crate::read_i32(action.as_ptr() as usize+0x24c),460);leave();
  hit[0x2c1]=8;on_guard_notification(m,h,3);assert!(effects.ids.contains(&COUNTER));
  hit[0x2c1]=0;on_guard_notification(m,h,3);assert!(!effects.ids.contains(&COUNTER));
  // The ordinary block can play exactly the same recoil animation, but its
  // provenance has been cleared, so its full-duration marker grants nothing.
  effects.ids.push(REACTION);assert!(!attack_guard(a));assert_eq!(free_guard(m,h),0);
  effects.ids.retain(|id|*id!=REACTION);
  assert_eq!(feedback_query(manager,FEEDBACK_SOUND),Some(0));
  action[0x24c..0x250].copy_from_slice(&GUARD_JUDGE.to_le_bytes());on_guard_notification(m,h,3);
  assert_eq!(effects.counter_applies,2);assert!(effects.ids.contains(&COUNTER));
  effects.ids.retain(|id|*id!=OWNER);on_guard_notification(m,h,3);assert!(!effects.ids.contains(&COUNTER));
  assert_eq!(feedback_query(manager,FEEDBACK_SOUND),Some(0));
  // Native writes the zero-impact notification AFTER our pre-callback. The
  // wrapper must repair it afterward, and only for this accepted custom hit.
  let old_notify=NOTIFY_ORIGINAL.swap(notify as *const () as *mut u8,Ordering::AcqRel);
  effects.ids.push(OWNER);action[0x58]=0x20;
  for (kind,power,cancel,wanted) in [(3,100f32,0,1),(3,0.,0,0),(3,100.,2,0),(99999,100.,0,0)]{
   hit[..4].copy_from_slice(&power.to_le_bytes());hit[0x2c1]=cancel;
   assert_eq!(notify_detour(m,h,kind,0xabcdef1234567890),0xfedcba9876543210);
   assert_eq!(action[0x1c],wanted);
  }
  effects.ids.retain(|id|*id!=OWNER);hit[0x2c1]=0;
  notify_detour(m,h,3,0xabcdef1234567890);assert_eq!(action[0x1c],0);
  NOTIFY_ORIGINAL.store(old_notify,Ordering::Release);
  TEST_CONTEXT.with(|s|s.set(None));assert_ne!(crate::win::VirtualFree(base as *mut _,0,0x8000),0);
 }}
 #[test]fn minimum_guard_reaction_changes_only_zero_impact_of_a_valid_guard(){unsafe{
  finish_notification(0);
  for kind in [3i32,0,99999,1001]{for class in [-1i32,0,1,2,3,4,5]{for impact in 0u8..=12{
   let mut action=[0x55u8;0x260];action[0x1c]=impact;
   action[0x20..0x24].copy_from_slice(&class.to_le_bytes());
   action[0x38..0x3c].copy_from_slice(&kind.to_le_bytes());
   let before=action;finish_notification(action.as_mut_ptr() as usize);
   for i in 0..action.len(){assert_eq!(action[i],if i==0x1c&&kind==3&&(1..=4).contains(&class)&&impact==0{1}else{before[i]});}
  }}}
 }}
 // Exercise the actual mid-function wrapper on a private actor fixture. The
 // two continuation stubs also check flags/RAX survived the Rust callback.
 #[unsafe(naked)]unsafe extern "system" fn run_cost(_module:usize,_hit:usize)->u32{
  core::arch::naked_asm!("push r14","push r15","sub rsp,0x20","mov r15,rcx","mov r14,rdx",
   "mov eax,0x12345678","stc","call {cost}","add rsp,0x20","pop r15","pop r14","ret",cost=sym cost_detour);
 }
 #[unsafe(naked)]unsafe extern "system" fn accepted(){core::arch::naked_asm!(
  "jnc 2f","cmp eax,0x12345678","jne 2f","mov eax,1","ret","2:","xor eax,eax","ret");}
 #[unsafe(naked)]unsafe extern "system" fn unchanged(){core::arch::naked_asm!(
  "jnc 2f","cmp eax,0x12345678","jne 2f","mov eax,2","ret","2:","xor eax,eax","ret");}
 unsafe extern "system" fn owns(manager:usize,id:i32)->u8{u8::from((manager==1||manager==2)&&id==OWNER||manager==2&&id==CHAIN)}
 unsafe extern "system" fn native_repel(_:usize,behavior:i32)->i32{behavior}
 #[test]fn attack_guard_cost_hp_and_recoil_are_scoped_to_live_native_window(){unsafe{
  let base=crate::win::VirtualAlloc(ptr::null_mut(),0x510000,0x3000,0x40) as usize;assert_ne!(base,0);
  for(rva,address)in[(crate::HAS_EFFECT_RVA,owns as *const () as usize),(AIRBORNE_RVA,fixture_airborne as *const () as usize)]{
   let mut jump=vec![0x48,0xb8];jump.extend_from_slice(&address.to_le_bytes());jump.extend_from_slice(&[0xff,0xe0]);ptr::copy_nonoverlapping(jump.as_ptr(),(base+rva) as *mut u8,jump.len());
  }
  TEST_CONTEXT.with(|s|s.set(Some((true,base))));
  let old_cost=COST_ORIGINAL.swap(unchanged as *const () as *mut u8,Ordering::AcqRel);
  let old_resume=COST_RESUME.swap(accepted as *const () as *mut u8,Ordering::AcqRel);
  let old_repel=REPEL_ORIGINAL.swap(native_repel as *const () as *mut u8,Ordering::AcqRel);
  let mut actor=vec![0u8;0x718];let mut vt=vec![0usize;0x120/8];
  let mut stats=vec![0u8;0x168];let mut action=vec![0u8;0x260];
  let mut physics=[0u8;0x98];physics[0x92]=1;
  let mut modules=[0usize;0x70/8];modules[0x68/8]=physics.as_mut_ptr() as usize;
  let mut damage=[0usize;2];let mut h=[0u8;crate::crystal::HIT_SIZE];
  let a=actor.as_mut_ptr() as usize;let hit=h.as_mut_ptr() as usize;
  let mut equipment=2i32;ptr::write_unaligned((a+0x670) as *mut usize,&mut equipment as *mut i32 as usize);
  vt[0x118/8]=base+0x4d2f00;ptr::write_unaligned(a as *mut usize,vt.as_ptr() as usize);
  modules[0]=stats.as_mut_ptr() as usize;modules[1]=action.as_mut_ptr() as usize;
  ptr::write_unaligned((a+0x1b8) as *mut usize,modules.as_ptr() as usize);
  ptr::write_unaligned((a+0x190) as *mut usize,1);damage[1]=a;
  stats[0x140..0x144].copy_from_slice(&100i32.to_le_bytes());
  action[0x58]=0x20;action[0x24c..0x250].copy_from_slice(&GUARD_JUDGE.to_le_bytes());
  let m=damage.as_ptr() as usize;h[..4].copy_from_slice(&100f32.to_le_bytes());
  for stamina in [0i32,1,100]{
   stats[0x15c..0x160].copy_from_slice(&stamina.to_le_bytes());
   h[0x284..0x288].copy_from_slice(&200i32.to_le_bytes());
   assert_eq!(run_cost(m,hit),1);assert_eq!(crate::read_i32(hit+0x284),0);
   assert_eq!(crate::read_i32(stats.as_ptr() as usize+0x15c),stamina);
  }
  for (kind,expected) in [(30,70),(70,70),(80,80)]{assert_eq!(repel(a,kind),expected);}
  for blocked in [false,true]{for replay in [false,true]{
   h.fill(0x55);h[0x2b4]=u8::from(blocked);h[0x2c1]=if replay{8}else{0};let before=h;
   finish_guard(m,hit);
   for i in 0..h.len(){let field=[0x280,0x284,0x28c,0x290,0x294,0x298].iter().any(|o|(*o..*o+4).contains(&i));
    assert_eq!(h[i],if blocked&&!replay&&field{0}else{before[i]},"offset {i:x}");}
  }}
  for disabled in 0..6{
   h.fill(0);h[..4].copy_from_slice(&100f32.to_le_bytes());h[0x2b4]=1;h[0x280..0x284].copy_from_slice(&100i32.to_le_bytes());
   match disabled{0=>action[0x58]=0,1=>action[0x24c..0x250].copy_from_slice(&460i32.to_le_bytes()),
    2=>actor[0x710]=1,3=>stats[0x140..0x144].copy_from_slice(&0i32.to_le_bytes()),
    4=>ptr::write_unaligned((a+0x190) as *mut usize,0),_=>TEST_CONTEXT.with(|s|s.set(Some((false,base))))}
   assert_eq!(run_cost(m,hit),2);finish_guard(m,hit);assert_eq!(crate::read_i32(hit+0x280),100);assert_eq!(repel(a,30),30);
   action[0x58]=0x20;action[0x24c..0x250].copy_from_slice(&GUARD_JUDGE.to_le_bytes());actor[0x710]=0;
   stats[0x140..0x144].copy_from_slice(&100i32.to_le_bytes());ptr::write_unaligned((a+0x190) as *mut usize,1);TEST_CONTEXT.with(|s|s.set(Some((true,base))));
  }
  for judge in [-1,460,1234,GUARD_JUDGE]{assert_eq!(native_guard_judge(judge),if judge==GUARD_JUDGE{460}else{judge});}
  // Native chain lifetime authorizes the following hit after the original
  // attack animation has ended. Nested calculations restore flags exactly.
  action[0x58]=0x80;action[0x24c..0x250].copy_from_slice(&123i32.to_le_bytes());
  ptr::write_unaligned((a+0x190) as *mut usize,2);h.fill(0);h[..4].copy_from_slice(&100f32.to_le_bytes());
  enter(m,hit);assert_eq!(action[0x58],0xa0);assert_eq!(crate::read_i32(action.as_ptr() as usize+0x24c),GUARD_JUDGE);
  enter(m,hit);assert_eq!(run_cost(m,hit),1);action[0x58]|=0x40;
  leave();assert_eq!(action[0x58],0xe0);leave();assert_eq!(action[0x58],0xc0);
  assert_eq!(crate::read_i32(action.as_ptr() as usize+0x24c),123);
  // Existing chain markers cannot authorize single-hand/dual-wield guard.
  // Both right and left native two-hand styles retain free guard and recoil.
  for style in [-1,0,1,2,3,4]{
   ptr::write_volatile(&mut equipment,style);
   action[0x58]=0xa0;action[0x24c..0x250].copy_from_slice(&GUARD_JUDGE.to_le_bytes());
   let valid=matches!(style,2|3);
   assert_eq!(attack_guard(a),valid);assert_eq!(followup(a,base),valid);
   enter(m,hit);assert_eq!(action[0x58]&0x20!=0,valid);
   assert_eq!(run_cost(m,hit),if valid{1}else{2});assert_eq!(repel(a,30),if valid{70}else{30});
   leave();assert_eq!(action[0x58],0xa0);
   // A native ordinary guard is unaffected by the two-hand restriction.
   action[0x24c..0x250].copy_from_slice(&460i32.to_le_bytes());
   enter(m,hit);assert_ne!(action[0x58]&0x20,0);leave();
  }
  ptr::write_volatile(&mut equipment,2);
  action[0x58]=0xc0;action[0x24c..0x250].copy_from_slice(&123i32.to_le_bytes());
  ptr::write_unaligned((a+0x190) as *mut usize,1); // chain expired
  enter(m,hit);assert_eq!(run_cost(m,hit),2);leave();assert_eq!(action[0x58],0xc0);
  // Body movement with no attack power cannot borrow our guard, including
  // when the TAE attack window is active. Native flags are restored afterward.
  action[0x58]=0xa0;action[0x24c..0x250].copy_from_slice(&GUARD_JUDGE.to_le_bytes());h.fill(0);
  enter(m,hit);assert_eq!(action[0x58],0x80);assert_eq!(run_cost(m,hit),2);
  leave();assert_eq!(action[0x58],0xa0);
  assert!(GUARD_SCOPES.with(|s|s.borrow().is_empty()));
  // Takeoff rejects both attack windows and existing continuous-guard tokens.
  // Suppress only our private guard bit; ordinary manual guard stays intact.
  for contact in [0u8,1]{for manager in [1usize,2]{for judge in [GUARD_JUDGE,460]{
   physics[0x92]=contact;ptr::write_unaligned((a+0x190) as *mut usize,manager);
   action[0x58]=0xa0;action[0x24c..0x250].copy_from_slice(&judge.to_le_bytes());
   h.fill(0);h[..4].copy_from_slice(&100f32.to_le_bytes());h[0x2b4]=1;
   h[0x280..0x284].copy_from_slice(&100i32.to_le_bytes());h[0x284..0x288].copy_from_slice(&50i32.to_le_bytes());
   let custom=contact!=0&&(judge==GUARD_JUDGE||manager==2);
   assert_eq!(grounded(a),contact!=0);assert_eq!(attack_guard(a),custom);
   assert_eq!(followup(a,base),contact!=0&&manager==2);
   enter(m,hit);
   assert_eq!(action[0x58]&0x20,if contact==0&&judge==GUARD_JUDGE{0}else{0x20});
   assert_eq!(run_cost(m,hit),if custom{1}else{2});finish_guard(m,hit);
   assert_eq!(crate::read_i32(hit+0x280),if custom{0}else{100});
   assert_eq!(crate::read_i32(hit+0x284),if custom{0}else{50});
   assert_eq!(repel(a,30),if custom{70}else{30});leave();
   assert_eq!(action[0x58],0xa0);assert_eq!(crate::read_i32(action.as_ptr() as usize+0x24c),judge);
  }}}
  ptr::write_volatile(modules.as_mut_ptr().add(0x68/8),0);assert!(!grounded(a));assert!(!grounded(0));
  COST_ORIGINAL.store(old_cost,Ordering::Release);COST_RESUME.store(old_resume,Ordering::Release);
  REPEL_ORIGINAL.store(old_repel,Ordering::Release);TEST_CONTEXT.with(|s|s.set(None));
  assert_ne!(crate::win::VirtualFree(base as *mut _,0,0x8000),0);
 }}
 #[test]fn input_attack_power_rejects_empty_nonfinite_and_negative_contacts(){unsafe{
  let mut hit=[0u8;crate::crystal::HIT_SIZE];let h=hit.as_mut_ptr() as usize;
  for value in [0.,-1.,f32::NAN,f32::INFINITY]{hit[..4].copy_from_slice(&value.to_le_bytes());assert!(!damaging_contact(h));}
  for index in 0..5{hit.fill(0);hit[index*4..index*4+4].copy_from_slice(&1f32.to_le_bytes());assert!(damaging_contact(h));}
 }}
 #[test]fn final_notification_wrapper_preserves_native_arguments_and_return(){unsafe{
  let _lock=NOTIFY_TEST.lock().unwrap();
  unsafe extern "system" fn native(m:usize,h:usize,k:i32,d:u64)->u64{
   assert_eq!((m,h,k,d),(0x1234,0x5678,3,0xabcdef1234567890));0xfedcba9876543210
  }
  TEST_CONTEXT.with(|s|s.set(Some((false,0))));
  let old=NOTIFY_ORIGINAL.swap(native as *const () as *mut u8,Ordering::AcqRel);
  let call:unsafe extern "system" fn(usize,usize,i32,u64)->u64=std::mem::transmute(notify_detour as *const ());
  assert_eq!(call(0x1234,0x5678,3,0xabcdef1234567890),0xfedcba9876543210);
  NOTIFY_ORIGINAL.store(old,Ordering::Release);TEST_CONTEXT.with(|s|s.set(None));
 }}
 #[test]fn onset_window_selects_seven_millisecond_tiers(){
  assert_eq!((data::MIN_TIER,TIERS),(2,7));
  assert_eq!(tier(3.,110.),Some(2)); // two-hand dagger: 3.3 poise -> 67 ms
  assert_eq!(tier(5.,110.),Some(3)); // two-hand straight sword: 5.5 -> 100 ms
  assert_eq!(tier(5.5,203.),Some(6)); // two-hand greatsword: 11.165 -> 200 ms
  assert_eq!(tier(6.5,100.),Some(4));assert_eq!(tier(5.,300.),Some(8));
  assert_eq!(tier(6.5,600.),Some(8));assert_eq!(tier(6.5,10000.),Some(8));
  assert_eq!(tier(5.,0.),Some(2));
  for boundary in [4f32,6.,8.,10.,12.,14.]{
   assert_eq!(tier(boundary,100.),Some((boundary/2.) as u8));
   assert_eq!(tier(boundary+0.01,100.),Some((boundary/2.) as u8+1));
  }
  for (a,b) in [(0.,100.),(5.,-100.),(f32::NAN,1.),(5.,f32::INFINITY)]{assert_eq!(tier(a,b),None);}
 }
 #[test]fn spark_follows_actual_two_hand_model_and_missing_point_falls_back(){unsafe{
  unsafe extern "system" fn count(p:usize,first:*const i32,last:*const i32)->i32{
   assert_eq!(*first,*last);
   let style=crate::read_i32(read_ptr(p+0x670));
   assert_eq!(*first,if style==2{20110}else{10110});
   crate::read_i32(read_ptr(p+0x680))
  }
  let mut actor=vec![0usize;0x688/8];let p=actor.as_mut_ptr() as usize;
  let mut vt=vec![0usize;0x4d8/8];vt[0x4d0/8]=count as *const () as usize;actor[0]=vt.as_ptr() as usize;
  let mut style=2i32;let mut points=1i32;
  ptr::write_unaligned((p+0x670) as *mut usize,&mut style as *mut i32 as usize);
  ptr::write_unaligned((p+0x680) as *mut usize,&mut points as *mut i32 as usize);
  for (hand,weapon,fallback) in [(2,FEEDBACK_WEAPON_RIGHT,FEEDBACK_RIGHT),(3,FEEDBACK_WEAPON_LEFT,FEEDBACK_LEFT)]{
   ptr::write_volatile(&mut style,hand);
   for n in [1,0,2,0,1]{ptr::write_volatile(&mut points,n);assert_eq!(feedback_spark_state(p),Some(if n>0{weapon}else{fallback}));}
   ptr::write_unaligned((p+0x680) as *mut usize,0);assert_eq!(feedback_spark_state(p),Some(fallback));
   ptr::write_unaligned((p+0x680) as *mut usize,&mut points as *mut i32 as usize);
  }
  for s in [0,1,4,-1]{ptr::write_volatile(&mut style,s);assert_eq!(feedback_spark_state(p),None);}
  ptr::write_unaligned((p+0x670) as *mut usize,0);assert_eq!(feedback_spark_state(p),None);
  assert_eq!(feedback_spark_state(0),None);
 }}
 #[test]fn source_tracks_equipped_and_current_attack_hand(){
  assert_eq!(source(0,-2),Some(-2));assert_eq!(source(0,5),Some(5));
  assert_eq!(source(1,-2),Some(-1));assert_eq!(source(2,-1),Some(-2));assert_eq!(source(3,0),None);
 }
 #[test]fn native_equipment_style_requires_right_or_left_two_handing(){unsafe{
  assert!(!two_handed(0));let mut actor=[0usize;0x678/8];let a=actor.as_mut_ptr() as usize;
  assert!(!two_handed(a));let mut style=0i32;
  ptr::write_unaligned((a+0x670) as *mut usize,&mut style as *mut i32 as usize);
  for value in [-1,0,1,2,3,4,5,i32::MAX]{ptr::write_volatile(&mut style,value);assert_eq!(two_handed(a),matches!(value,2|3));}
 }}
 #[test]fn only_generated_states_are_intercepted(){
  assert!(condition(data::FIRST-1).is_none());
  for (i,expected) in data::CONDITIONS.iter().enumerate(){for index in 0..TIERS{
   assert_eq!(condition(data::FIRST+(i*TIERS+index) as u16),Some((*expected,(index+data::MIN_TIER) as u8)));
  }}
  assert!(condition(data::FIRST+(data::CONDITIONS.len()*TIERS) as u16).is_none());
 }
 #[test]fn compiled_tables_have_unique_sorted_keys_and_native_stance_samples(){
  for table in [data::WEAPONS,data::BEHAVIORS]{assert!(table.windows(2).all(|p|p[0].0<p[1].0));}
  assert_eq!(lookup(data::WEAPONS,1000000),Some(3.));
  assert_eq!(lookup(data::WEAPONS,2000000),Some(5.));
  assert_eq!(lookup(data::WEAPONS,-1),None);assert_eq!(lookup(data::BEHAVIORS,-1),None);
 }
 #[test]fn native_states_forward_full_manager_and_state_unchanged(){unsafe{
  unsafe extern "system" fn fixture(manager:usize,state:u16)->u8{assert_eq!(manager,0x12345678);assert_eq!(state,434);0x7f}
  let old=ORIGINAL.swap(fixture as *const () as *mut u8,Ordering::AcqRel);
  assert_eq!(detour(0x12345678,434),0x7f);
  ORIGINAL.store(old,Ordering::Release);
 }}
}
