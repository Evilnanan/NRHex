//! Back to Basics: native skill cooldown multipliers and source-specific damage.
use crate::{has_effect,local_hero,native_actor,ptr,read_i32,read_ptr};
pub const OWNER:i32=95004400;
pub const RESTAGE_RVA:usize=0x449ac7;
pub const RESTAGE_PATTERN:&str="F3 0F 59 4F 3C 66 41 0F 6E C6 0F 5B C0 F3 0F 58 C8 F3 44 0F 2C F1 48 8D 8D 90 00 00";
pub static RESTAGE_ORIGINAL:std::sync::atomic::AtomicPtr<u8>=std::sync::atomic::AtomicPtr::new(ptr::null_mut());
pub const ULTIMATE_QUERY:i32=990045;
pub const READY_QUERY:i32=990046;
pub const CONFLICTS:[i32;5]=[95000400,95003800,95003900,95004000,95001800];
pub const COOLDOWNS:[i32;4]=[OWNER+1,OWNER+2,OWNER+3,OWNER+7];
pub const FAMILY_IDLE:[i32;3]=[705061,705071,705081];
pub const FAMILY_RELOAD:[i32;3]=[OWNER+4,OWNER+5,OWNER+6];
pub fn cooldown_effect(hero:Option<u8>)->Option<i32>{match hero{Some(1)=>Some(OWNER+7),Some(2|9|10)=>Some(OWNER+3),Some(3)=>Some(OWNER+1),Some(4|5)=>Some(OWNER+2),_=>None}}
pub fn multiplier(hero:Option<u8>,attack:i32,table:i32)->f32{
 if table!=1{return 1.;}
 match (hero,attack){
  (Some(1),600000000|600000005|600000010|600000015|600000016|600000030|600000035)=>1.5,
  (Some(2),600000050|600000055|600000080..=600000087|600000089..=600000091)=>1.5,
  (Some(3),600000110|600000120)=>1.5,
  (Some(5),600000200|600000210|600000220|600000230|600000231|600000240|600000250)=>3.,
  (Some(7),80000|80002|80100|80101|80200|80201|81000|81005|81100|81110|81300|81310|81500|81510|82000|82010|82011|82012|82100|82200|82201|82202|82210|82300|82310)=>1.75,
  (Some(8),600000350)=>1.5,
  (Some(8),960200..=960591|600000355)=>1.15,
  _=>1.,
 }
}
pub unsafe fn sealed(actor:usize,base:usize)->bool{
 actor!=0&&crate::is_local(actor,base)&&has_effect(actor,OWNER,base)&&!has_effect(actor,crate::reward_cycle_runtime::RESET,base)
}
pub unsafe fn update(actor:usize,base:usize){
 let stats=native_actor::stats(actor);if stats==0{return;}
 let wanted=if sealed(actor,base){cooldown_effect(local_hero(base))}else{None};
 for id in COOLDOWNS{crate::maintain_status_effect(actor,stats,id,wanted==Some(id),base);}
 update_family(actor,base);
}
unsafe fn family_handles(owner:usize)->Option<[u64;3]>{
 if owner==0{return None;}
 let modules=read_ptr(owner+0x1b8);if modules==0{return None;}
 let wrapper=read_ptr(modules+0x160);if wrapper==0{return None;}
 let family=read_ptr(wrapper+0x10);if family==0{return None;}
 Some(std::array::from_fn(|slot|ptr::read_unaligned((family+0x10+slot*0x60)as*const u64)))
}
unsafe fn family_slot(owner:usize,source:usize,base:usize)->Option<usize>{
 if source==0||source==owner{return None;}
 family_handles(owner)?.iter().position(|&handle|native_actor::resolve(base,handle)==source)
}
unsafe fn family_source(owner:usize,source:usize,base:usize)->bool{family_slot(owner,source,base).is_some()}
pub unsafe fn route_effect(target:usize,effect:i32,base:usize)->i32{
 let Some(slot)=FAMILY_IDLE.iter().position(|&v|v==effect).or_else(||FAMILY_RELOAD.iter().position(|&v|v==effect))else{return effect;};
 let world=read_ptr(base+crate::WORLD_RVA);if world==0{return effect;}
 let owner=read_ptr(world+0x174e8);if family_slot(owner,target,base)!=Some(slot){return effect;}
 if local_hero(base)==Some(6)&&sealed(owner,base){FAMILY_RELOAD[slot]}else{FAMILY_IDLE[slot]}
}
unsafe fn update_family(owner:usize,base:usize){
 let Some(handles)=family_handles(owner)else{return;};
 let enabled=local_hero(base)==Some(6)&&sealed(owner,base);
 for (slot,handle)in handles.into_iter().enumerate(){
  let actor=native_actor::resolve(base,handle);if actor==0||actor==owner{continue;}
  // Swap only an existing inactive effect. Never apply regeneration to a
  // deployed family member or periodically refresh its healing tick timer.
  let (old,new)=if enabled{(FAMILY_IDLE[slot],FAMILY_RELOAD[slot])}else{(FAMILY_RELOAD[slot],FAMILY_IDLE[slot])};
  if has_effect(actor,old,base)&&!has_effect(actor,new,base){
   let apply:unsafe extern "system" fn(usize,i32,u8)->u8=std::mem::transmute(base+crate::APPLY_SELF_RVA);
   apply(actor,new,0);
  }
 }
}
pub unsafe fn calculated(local:usize,target:usize,source:usize,hit:usize,base:usize){
 if hit==0||source==0||source==target||read_i32(hit+0x280)<=0||!sealed(local,base){return;}
 let hero=local_hero(base);
 let factor=if source==local{multiplier(hero,read_i32(hit+0x40),read_i32(hit+0x44))}
  else if hero==Some(6)&&family_source(local,source,base){3.}else{1.};
 if factor!=1.{ptr::write_unaligned((hit+0x280)as*mut i32,crate::sword::boosted(read_i32(hit+0x280),factor));}
}
// Restage replays recorded damage, with its own proportion and network payload.
// Scale each native record before MULSS/truncation; do not boost the visual hit.
unsafe extern "system" fn restage_amount(module:usize,amount:f32)->f32{
 use std::sync::atomic::Ordering;
 if !crate::ACTIVE.load(Ordering::Acquire)||module==0{return amount;}
 let base=crate::BASE.load(Ordering::Acquire);
 if local_hero(base)==Some(4)&&sealed(read_ptr(module+8),base){amount*1.2}else{amount}
}
#[unsafe(naked)]
pub unsafe extern "system" fn restage_detour(){
 core::arch::naked_asm!(
  "pushfq", "sub rsp,0xc8",
  "mov [rsp+0x20],rax", "mov [rsp+0x28],rcx", "mov [rsp+0x30],rdx", "mov [rsp+0x38],r8",
  "mov [rsp+0x40],r9", "mov [rsp+0x48],r10", "mov [rsp+0x50],r11",
  "movdqu [rsp+0x60],xmm0", "movdqu [rsp+0x70],xmm1", "movdqu [rsp+0x80],xmm2",
  "movdqu [rsp+0x90],xmm3", "movdqu [rsp+0xa0],xmm4", "movdqu [rsp+0xb0],xmm5",
  "mov rcx,rdi", "call {callback}",
  "movss [rsp+0x70],xmm0", "mov rax,[rsp+0x20]", "mov rcx,[rsp+0x28]", "mov rdx,[rsp+0x30]", "mov r8,[rsp+0x38]",
  "mov r9,[rsp+0x40]", "mov r10,[rsp+0x48]", "mov r11,[rsp+0x50]",
  "movdqu xmm0,[rsp+0x60]", "movdqu xmm1,[rsp+0x70]", "movdqu xmm2,[rsp+0x80]",
  "movdqu xmm3,[rsp+0x90]", "movdqu xmm4,[rsp+0xa0]", "movdqu xmm5,[rsp+0xb0]",
  "add rsp,0xc8", "popfq", "jmp qword ptr [rip+{original}]",
  callback=sym restage_amount,original=sym RESTAGE_ORIGINAL
 );
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn role_values_do_not_leak_to_other_attacks_or_roles(){
  for (hero,attack,factor)in [(1,600000000,1.5),(2,600000050,1.5),(3,600000110,1.5),(5,600000210,3.),(7,82310,1.75),(8,960200,1.15),(8,600000350,1.5),(8,600000355,1.15)]{
   assert_eq!(multiplier(Some(hero),attack,1),factor);
   assert_eq!(multiplier(Some(hero),attack,0),1.);
   for other in 1..=10{if other!=hero{assert_eq!(multiplier(Some(other),attack,1),1.);}}
  }
  for hero in 0..=10{for attack in [0,12064105,300000290,600000400,81200,81201,82111]{assert_eq!(multiplier(Some(hero),attack,1),1.);}}
  assert_eq!(cooldown_effect(Some(1)),Some(OWNER+7));assert_eq!(cooldown_effect(Some(2)),Some(OWNER+3));assert_eq!(cooldown_effect(Some(3)),Some(OWNER+1));
  for hero in [4,5]{assert_eq!(cooldown_effect(Some(hero)),Some(OWNER+2));}
  for hero in [9,10]{assert_eq!(cooldown_effect(Some(hero)),Some(OWNER+3));}
  for hero in [None,Some(0),Some(6),Some(7),Some(8)]{assert_eq!(cooldown_effect(hero),None);}
 }
 #[test]fn ultimate_and_necromancy_conflicts_use_ownership_in_every_draw_route(){
  use crate::reward_choices::*;
  let index=|id|crate::hex_pool::OWNER_IDS.iter().position(|&v|v==id).unwrap();
  let basics=index(OWNER);let conflicts=CONFLICTS.map(index);
  for bits in 0..(1usize<<(CONFLICTS.len()+1)){
   let owned=mask(std::iter::once(basics).chain(conflicts).enumerate().filter_map(|(i,id)|(bits&(1<<i)!=0).then_some(id)));
   let unavailable=ownership_exclusions(owned.clone());
   assert_eq!(unavailable.contains(&basics),bits!=0);
   for id in conflicts{assert_eq!(unavailable.contains(&id),owned.contains(&id)||bits&1!=0);}
   for seen in [Default::default(),all(crate::hex_pool::HEX_COUNT)]{
    let round=crate::reward_cycle::Round::new(&unavailable,&seen);
    let focused=round.preferred([basics,conflicts[0],conflicts[1]]).unwrap();
    let refresh=crate::reward_cycle::Single::new(&unavailable,&seen,&mask([0,1]));
    for id in std::iter::once(basics).chain(conflicts){assert_eq!(round.candidates.contains(&id),!unavailable.contains(&id));assert_eq!(refresh.candidates.contains(&id),!unavailable.contains(&id));if unavailable.contains(&id){assert!(!focused.contains(&id));}}
   }
  }
  assert_eq!(crate::hex_pool::HISTORY_BITS[basics],41);
 }
 #[test]fn family_records_preserve_native_reference_handles(){unsafe{
  let mut owner=vec![0usize;0x1c0/8];let mut modules=vec![0usize;0x168/8];let mut wrapper=[0usize;3];let mut family=vec![0u64;0x140/8];
  owner[0x1b8/8]=modules.as_mut_ptr()as usize;modules[0x160/8]=wrapper.as_mut_ptr()as usize;wrapper[2]=family.as_mut_ptr()as usize;
  let owner=owner.as_ptr()as usize;
  let handles=[0x701effffffff,0x701fffffffff,0x7020ffffffff];
  for (slot,handle)in handles.into_iter().enumerate(){family[(0x10+slot*0x60)/8]=handle;}
  assert_eq!(family_handles(owner),Some(handles));assert_eq!(family_handles(0),None);
  assert!(!family_source(owner,owner,0));assert!(!family_source(owner,0,0));
 }}
}
#[cfg(test)]#[path="basics_native_tests.rs"]mod native_tests;
