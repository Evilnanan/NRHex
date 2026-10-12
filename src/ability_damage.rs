//! Native outgoing modifiers for the HP-based physical explosion.
//! Query before death cleanup, never enumerate/multiply effect rows ourselves.
use crate::{balloon, ptr, read_ptr, ATTACK_RATES_ORIG};
use std::sync::atomic::Ordering;

pub const CONTEXT_RVA:usize=0x33e360;
pub const CONTEXT_PATTERN:&str="48 89 5C 24 08 57 48 83 EC 20 33 FF C7 41 08 FF FF FF FF 48 89 39 48 8B D9 89 79 0C 40 88 79 10";
type Init=unsafe extern "system" fn(usize)->usize;
type Rates=unsafe extern "system" fn(usize,usize,i32,i32);

#[derive(Clone,Copy,Debug)]
pub struct PhysicalBuffs {pub power:f32,pub flat:f32,pub damage:f32}
impl Default for PhysicalBuffs {fn default()->Self{Self{power:1.,flat:0.,damage:1.}}}
impl PhysicalBuffs {
 pub fn apply(self,hp:i32)->i32 {
  if hp<=0{return 0;}
  if !self.power.is_finite()||!self.flat.is_finite()||!self.damage.is_finite()||self.power<0.||self.damage<0.{return hp;}
  ((hp as f32*self.power+self.flat).max(0.)*self.damage).max(0.) as i32
 }
}

pub unsafe fn balloon_buffs(player:usize,base:usize)->PhysicalBuffs {
 if player==0{return PhysicalBuffs::default();}
 let manager=read_ptr(player+0x190);let rates=ATTACK_RATES_ORIG.load(Ordering::Acquire);
 if manager==0||rates.is_null(){return PhysicalBuffs::default();}
 query(std::mem::transmute(base+CONTEXT_RVA),std::mem::transmute(rates),manager,player)
}
unsafe fn query(init:Init,rates:Rates,manager:usize,player:usize)->PhysicalBuffs {
 // 33E360 initializes through +111; 50F2A0 reads classification and source
 // fields through +10D. Keep the native neutral/no-weapon defaults.
 #[repr(align(16))]struct Context([u8;0x120]);
 let mut context=Context([0;0x120]);let p=context.0.as_mut_ptr() as usize;
 init(p);
 ptr::write_unaligned(p as *mut usize,player);
 ptr::write_unaligned((p+8) as *mut i32,balloon::ATTACK);
 ptr::write_unaligned((p+0xc) as *mut i32,1); // AtkParam_Pc
 let mut categories=[0u32;8];categories[110/32]|=1<<(110%32);
 ptr::write_unaligned((p+0x14) as *mut [u32;8],categories);
 // Native 502830 retains conditions, category priority, attack-type filters
 // and the attack row's three SpEffect correction rates.
 rates(manager,p,-1,0);
 let get=|offset|ptr::read_unaligned((p+offset) as *const f32);
 // 6C2EB0: physical base * +6C + +48. E70270/E71170: neutral
 // physical final multiplier is +90 * +A0 (AtkParam.atkAttribute = 3).
 PhysicalBuffs{power:get(0x6c),flat:get(0x48),damage:get(0x90)*get(0xa0)}
}

#[cfg(test)]mod tests{
 use super::*;
 unsafe extern "system" fn init(p:usize)->usize {
  ptr::write_unaligned((p+0xe4) as *mut i32,16);
  ptr::write_unaligned((p+0xe8) as *mut i32,-1);p
 }
 unsafe extern "system" fn rates(manager:usize,p:usize,weapon:i32,extra:i32){
  assert_eq!((manager,weapon,extra),(123,-1,0));assert_eq!(read_ptr(p),456);
  assert_eq!(ptr::read_unaligned((p+8) as *const [i32;2]),[balloon::ATTACK,1]);
  assert_eq!(ptr::read((p+0x10) as *const u8),0);
  let mut expected=[0u32;8];expected[3]=1<<14;
  assert_eq!(ptr::read_unaligned((p+0x14) as *const [u32;8]),expected);
  assert_eq!(ptr::read_unaligned((p+0xe4) as *const i32),16);
  assert_eq!(ptr::read_unaligned((p+0xe8) as *const i32),-1);
  for (offset,value) in [(0x6c,1.25f32),(0x48,50.),(0x90,1.5),(0xa0,1.2)]{ptr::write_unaligned((p+offset) as *mut f32,value);}
 }
 #[test]fn native_query_keeps_category_and_filters_and_extracts_physical_modifiers(){unsafe{
  let b=query(init,rates,123,456);assert_eq!(b.apply(1000),2340);
 }}
 #[test]fn damage_preserves_neutral_reductions_zero_and_saturates(){
  assert_eq!(PhysicalBuffs::default().apply(1500),1500);
  assert_eq!(PhysicalBuffs{damage:1.4,..Default::default()}.apply(1000),1400);
  assert_eq!(PhysicalBuffs{damage:0.75,..Default::default()}.apply(1000),750);
  assert_eq!(PhysicalBuffs{damage:0.,..Default::default()}.apply(1000),0);
  assert_eq!(PhysicalBuffs{damage:f32::NAN,..Default::default()}.apply(1000),1000);
  assert_eq!(PhysicalBuffs{damage:f32::MAX,..Default::default()}.apply(i32::MAX),i32::MAX);
 }
}
