//! Instance-specific skill overlays: native inventory/drop data stays original.
use std::collections::BTreeMap;
pub const OWNER:i32=95001400;
pub const MARKER:i32=6999800;
pub const BUSY:i32=6999801;
pub const USE_LEFT:i32=6999802;
pub const USE_RIGHT:i32=6999803;
pub const USED:i32=6999804;
pub const PREPARE:i32=6999805;
pub const STORM:i32=1200;
pub const BUCKLER_PARRY:i32=303;
fn protected(original:i32)->bool{matches!(original,STORM|BUCKLER_PARRY)}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Weapon {pub handle:u32,pub original:i32,pub base:i32}
#[derive(Clone,Copy,Debug)]
struct Entry {weapon:Weapon,selected:i32}
#[derive(Default)]
pub struct Arts {entries:BTreeMap<u32,Entry>,drops:BTreeMap<u32,Entry>,pending:Option<Weapon>,busy:bool,quiet:u8,owned:bool,context:usize,network_dirty:bool}
impl Arts {
 pub fn synchronize(&mut self,context:usize,owned:bool,alive:bool){
  if self.context!=context{self.network_dirty=false;}
  else if !owned&&!self.entries.is_empty(){self.network_dirty=true;}
  if self.context!=context||!owned {self.entries.clear();self.drops.clear();self.pending=None;self.busy=false;self.quiet=0;}
  self.context=context;self.owned=owned;
  if !alive{self.pending=None;self.busy=false;self.quiet=0;}
 }
 pub fn notify(&mut self,weapon:Option<Weapon>,used:bool){
  if !self.owned{return;}self.busy=true;self.quiet=0;
  if used&&self.pending.is_none(){if let Some(w)=weapon{if !protected(w.original){self.pending=Some(w);}}}
 }
 pub fn selected(&self,w:Weapon)->Option<i32>{
  if !self.owned||protected(w.original){return None;}
  self.entries.get(&w.handle).filter(|e|e.weapon==w).map(|e|e.selected)
 }
 /// Returning from an attack to a fresh preparation is a chain boundary.
 /// The script exits the old stance before asking for a new skill.
 pub fn prepare(&mut self)->Option<Weapon>{self.pending.take()}
 pub fn has_pending(&self)->bool{self.pending.is_some()}
 /// TAE supplies the exact source weapon even before the first script pulse.
 pub fn behavior(&self,judge:i32,w:Weapon)->Option<i32>{behavior(judge,self.selected(w)?)}
 /// Two updates without a skill node confirm that the whole chain has exited.
 pub fn tick(&mut self)->Option<Weapon>{
  if self.busy{self.busy=false;self.quiet=0;return None;}
  self.quiet=self.quiet.saturating_add(1);
  if self.quiet>=2 {self.pending.take()}else{None}
 }
 pub fn reforge(&mut self,w:Weapon,id:i32){if self.owned&&!protected(w.original){self.network_dirty=true;self.entries.insert(w.handle,Entry{weapon:w,selected:id});}}
 /// Native world UID includes the producing player and the drop sequence.
 /// It identifies the actual ground item, even for otherwise identical weapons.
 pub fn network_pending(&self)->bool{self.network_dirty}
 pub fn network_sent(&mut self){self.network_dirty=false;}
 pub fn dropped(&mut self,uid:u32,w:Weapon){
  if uid==0{return;}if let Some(id)=self.selected(w){self.drops.insert(uid,Entry{weapon:w,selected:id});}
 }
 pub fn picked_up(&mut self,uid:u32,w:Weapon)->bool{
  if !self.owned||protected(w.original){return false;}
  let Some(e)=self.drops.get(&uid).copied() else{return false;};
  if e.weapon.base!=w.base||e.weapon.original!=w.original{return false;}
  self.reforge(w,e.selected);true
 }
}
/// Native 6C16F0 callers for full network equipment, PlayerGameData network
/// state, public character equipment and the peer equipment preview.
/// Inventory menus, event item creation, comparisons and drop records stay original.
pub fn network_export(return_rva:usize,transfer_active:bool)->bool{
 !transfer_active&&matches!(return_rva,0x381602|0x38177d|0x20cb11|0xb0e0d3|0xbea18b)
}
/// +4 in the transient 0x48-byte record restores the peer instance's +1C.
pub fn snapshot(data:&mut [u8;0x48],base:i32,skill:i32){
 if i32::from_le_bytes(data[..4].try_into().unwrap())==base&&pool(base).contains(&skill){
  data[4..8].copy_from_slice(&skill.to_le_bytes());
 }
}
pub fn original(base:i32,override_id:i32)->Option<i32>{
 if override_id>=0{return Some(override_id);}
 crate::arts_data::DEFAULTS.binary_search_by_key(&base,|x|x.0).ok().map(|i|crate::arts_data::DEFAULTS[i].1)
}
pub fn pool(base:i32)->&'static [i32]{
 if crate::arts_data::BOW_WEAPONS.binary_search(&base).is_ok(){&crate::arts_data::BOW_POOL}else{&crate::arts_data::POOL}
}
/// Projectile attacks use the weapon's corresponding elemental attack power.
/// Check the actual weapon variant, not its name, original art or AR sum.
pub fn roll_allowed(base:i32,skill:i32)->bool{
 match skill {
  1014=>crate::arts_data::FIRE_WEAPONS.binary_search(&base).is_ok(),
  1184=>crate::arts_data::MAGIC_WEAPONS.binary_search(&base).is_ok(),
  _=>true,
 }
}
/// Fire shields fire from a muzzle that most other weapon models lack.
/// Body dummy58 is the native shared arrow/projectile origin; change only the
/// random cross-weapon overlay, leaving original fire shields' muzzles intact.
pub fn emitter_dummy(skill:i32,base:i32,behavior:i32,dummy:i32)->Option<i32>{
 let (family,point)=match skill{1001=>(3215,141),1002=>(3216,142),_=>return None};
 if base/10000==family||behavior/1000!=300000+family{return None;}
 if dummy==10000+point||dummy==20000+point{Some(58)}else{None}
}
pub fn behavior(judge:i32,skill:i32)->Option<i32>{
 if !(3000..4000).contains(&judge){return None;}
 let target=crate::arts_data::SKILL_VARIATIONS.binary_search_by_key(&skill,|x|x.0).ok().map(|i|crate::arts_data::SKILL_VARIATIONS[i].1)?;
 let id=300000000+target*1000+judge%1000;
 if crate::arts_data::BEHAVIORS.binary_search(&id).is_err(){None}else{Some(id)}
}
#[cfg(test)]mod tests{
 use super::*;
 fn w(handle:u32,original:i32)->Weapon{Weapon{handle,original,base:2000000}}
 fn state()->Arts{let mut a=Arts::default();a.synchronize(1,true,true);a}
 #[test]fn elemental_projectile_arts_require_the_weapons_matching_attack_component(){
  for (weapon,fire,magic) in [(2000000,false,false),(3140000,true,false),(13020000,false,true),(2150000,false,true),(-1,false,false)]{
   assert_eq!(roll_allowed(weapon,1014),fire);
   assert_eq!(roll_allowed(weapon,1184),magic);
   for &id in pool(weapon){if !matches!(id,1014|1184){assert!(roll_allowed(weapon,id));}}
  }
 }
 #[test]fn elemental_filter_never_empties_weapon_pool_or_changes_bow_choices(){
  for &(weapon,_) in crate::arts_data::DEFAULTS.iter(){
   let choices=pool(weapon);assert!(choices.iter().filter(|&&id|roll_allowed(weapon,id)).count()>=choices.len()-2);
   if crate::arts_data::BOW_WEAPONS.binary_search(&weapon).is_ok(){assert!(choices.iter().all(|&id|roll_allowed(weapon,id)));}
  }
 }
 #[test]fn peer_snapshot_changes_only_selected_instance_skill_fields(){
  let mut data=[0x67;0x48];data[..4].copy_from_slice(&2000000i32.to_le_bytes());
  let before=data;let mut expected=before;
  expected[4..8].copy_from_slice(&1173i32.to_le_bytes());snapshot(&mut data,2000000,1173);assert_eq!(data,expected);
  let mut other=before;snapshot(&mut other,3000000,1173);snapshot(&mut other,2000000,9999);assert_eq!(other,before);
  let mut second=before;snapshot(&mut second,2000000,1047);assert_ne!(second,data);assert_eq!(&second[8..],&before[8..]);
  let mut bow=before;bow[..4].copy_from_slice(&41000000i32.to_le_bytes());let old=bow;
  snapshot(&mut bow,41000000,1173);assert_eq!(bow,old);
  snapshot(&mut bow,41000000,400);assert_eq!(i32::from_le_bytes(bow[4..8].try_into().unwrap()),400);
 }
 #[test]fn only_native_network_exports_can_use_reforged_skill(){
  for address in [0x381602,0x38177d,0x20cb11,0xb0e0d3,0xbea18b]{assert!(network_export(address,false));assert!(!network_export(address,true));}
  for address in [0,0x6c1282,0x5789ef,0x57a60e,0x8b7c15,0x8bc956,0xb1200a,0xb140d0]{assert!(!network_export(address,false));}
 }
 #[test]fn stance_only_never_reforges(){let mut a=state();for _ in 0..80{a.notify(None,false);assert_eq!(a.tick(),None);}assert_eq!(a.tick(),None);assert_eq!(a.tick(),None);}
 #[test]fn bows_and_other_weapons_have_disjoint_usable_pools(){
  assert_eq!(pool(41000000),&[400,401,402,404,405,406,1169]);
  assert_eq!(pool(42000000),pool(41000000));
  let ordinary=pool(2000000);assert_eq!(ordinary.len(),175);
  for id in [0,1,2,10,117,1007,9999,400,401,402,404,405,406,1169]{assert!(!ordinary.contains(&id));}
  for id in pool(41000000){assert!(!ordinary.contains(id));}
  assert!(ordinary.contains(&1001)&&ordinary.contains(&1002));
  assert_eq!(original(30200000,-1),Some(1007)); // Native Coil Shield remains intact.
 }
 #[test]fn fire_shield_muzzles_use_body_origin_only_for_random_cross_weapon_skill(){
  for (id,family,point) in [(1001,3215,141),(1002,3216,142)]{
   let row=300000000+family*1000+900;
   for hand in [10000,20000]{assert_eq!(emitter_dummy(id,2000000,row,hand+point),Some(58));}
   assert_eq!(emitter_dummy(id,family*10000,row,10000+point),None);
   assert_eq!(emitter_dummy(id,family*10000+25,row,10000+point),None);
   assert_eq!(emitter_dummy(id,2000000,row,58),None);
   assert_eq!(emitter_dummy(id,2000000,300707900,10000+point),None);
   assert_eq!(emitter_dummy(1173,2000000,row,10000+point),None);
  }
 }
 #[test]fn continuous_combo_uses_original_until_exit(){let mut a=state();let w=w(3,100);for _ in 0..120{a.notify(Some(w),true);assert_eq!(a.tick(),None);assert_eq!(a.selected(w),None);}assert_eq!(a.tick(),None);assert_eq!(a.tick(),Some(w));assert_eq!(a.tick(),None);a.reforge(w,1177);assert_eq!(a.selected(w),Some(1177));}
 #[test]fn returning_to_preparation_consumes_use_without_waiting_for_quiet_frames(){
  let mut a=state();let w=w(3,100);a.notify(Some(w),true);a.tick();
  a.notify(None,false);assert_eq!(a.prepare(),Some(w));assert!(!a.has_pending());
  assert_eq!(a.prepare(),None);a.reforge(w,1100);assert_eq!(a.selected(w),Some(1100));
  for _ in 0..80{a.notify(None,false);assert_eq!(a.prepare(),None);assert_eq!(a.tick(),None);}
 }
 #[test]fn separate_identical_weapons_keep_independent_results(){let mut a=state();a.reforge(w(3,100),1100);a.reforge(w(4,100),1101);assert_eq!(a.selected(w(3,100)),Some(1100));assert_eq!(a.selected(w(4,100)),Some(1101));assert_eq!(a.selected(w(3,101)),None);}
 #[test]fn identical_drops_restored_in_reverse_order_by_native_uid(){
  let mut a=state();a.reforge(w(3,100),1100);a.reforge(w(4,100),1101);
  a.dropped(0x28000001,w(3,100));a.dropped(0x28000002,w(4,100));
  assert!(a.picked_up(0x28000002,w(10,100)));assert!(a.picked_up(0x28000001,w(11,100)));
  assert_eq!(a.selected(w(10,100)),Some(1101));assert_eq!(a.selected(w(11,100)),Some(1100));
 }
 #[test]fn repeated_drop_pickup_tracks_latest_reforge_and_survives_death(){
  let mut a=state();a.reforge(w(3,100),1100);a.dropped(1,w(3,100));
  a.synchronize(1,true,false);assert!(a.picked_up(1,w(4,100)));
  a.reforge(w(4,100),1173);a.dropped(2,w(4,100));assert!(a.picked_up(2,w(5,100)));
  assert_eq!(a.selected(w(5,100)),Some(1173));
 }
 #[test]fn unknown_drops_and_mismatched_weapon_never_receive_overlay(){
  let mut a=state();a.reforge(w(3,100),1100);a.dropped(1,w(3,100));
  assert!(!a.picked_up(2,w(4,100)));assert!(!a.picked_up(1,w(4,101)));
  assert!(!a.picked_up(1,Weapon{base:3000000,..w(4,100)}));
  assert!(!a.picked_up(1,w(4,STORM)));assert!(!a.picked_up(1,w(4,BUCKLER_PARRY)));
  a.dropped(0,w(3,100));assert!(!a.picked_up(0,w(4,100)));
  assert_eq!(a.selected(w(4,100)),None);
 }
 #[test]fn drop_links_clear_on_owner_loss_and_new_expedition(){
  for (context,owned) in [(1,false),(2,true)]{
   let mut a=state();a.reforge(w(3,100),1100);a.dropped(1,w(3,100));
   a.synchronize(context,owned,true);a.synchronize(context,true,true);
   assert!(!a.picked_up(1,w(4,100)));
  }
 }
 #[test]fn death_cancels_pending_but_preserves_weapon_result(){let mut a=state();a.reforge(w(3,100),1100);a.notify(Some(w(3,100)),true);a.synchronize(1,true,false);assert_eq!(a.tick(),None);assert_eq!(a.tick(),None);assert_eq!(a.selected(w(3,100)),Some(1100));}
 #[test]fn owner_loss_or_new_expedition_clears_overlay(){let mut a=state();a.reforge(w(3,100),1100);a.synchronize(1,false,true);assert_eq!(a.selected(w(3,100)),None);a.synchronize(1,true,true);a.reforge(w(3,100),1100);a.synchronize(2,true,true);assert_eq!(a.selected(w(3,100)),None);}
 #[test]fn protected_storm_weapon_never_changes(){let mut a=state();let s=w(3,STORM);a.notify(Some(s),true);a.tick();a.tick();assert_eq!(a.tick(),None);a.reforge(s,100);assert_eq!(a.selected(s),None);}
 #[test]fn original_buckler_parry_never_changes(){let mut a=state();let b=w(3,BUCKLER_PARRY);for _ in 0..3{a.notify(Some(b),true);a.tick();a.tick();assert_eq!(a.tick(),None);}a.reforge(b,100);assert_eq!(a.selected(b),None);}
 #[test]fn randomized_buckler_parry_can_still_reforge(){let mut a=state();let ordinary=w(3,100);a.reforge(ordinary,BUCKLER_PARRY);assert_eq!(a.selected(ordinary),Some(BUCKLER_PARRY));a.notify(Some(ordinary),true);a.tick();a.tick();assert_eq!(a.tick(),Some(ordinary));a.reforge(ordinary,101);assert_eq!(a.selected(ordinary),Some(101));}
 #[test]fn both_nebulas_use_their_own_native_attack_and_bullet_rows(){
  assert_eq!(behavior(3900,1173),Some(300707900));
  assert_eq!(behavior(3901,1047),Some(301301901));
  assert_eq!(behavior(3900,1047),Some(301301900));
  assert_eq!(behavior(3905,1173),Some(300707905));
 }
 #[test]fn behavior_scope_preserves_normal_attacks_and_missing_rows(){
  for id in [900,2900,4900,-1]{assert_eq!(behavior(id,1173),None);}
  assert_eq!(behavior(3999,1173),None);
  assert_eq!(behavior(3900,9999),None);
 }
 #[test]fn behavior_binding_uses_exact_source_instance_without_pending_pulse(){
  let mut a=state();let right=Weapon{handle:3,original:100,base:3000000};let left=Weapon{handle:4,..right};
  a.reforge(right,1173);a.reforge(left,1047);
  assert_eq!(a.behavior(3900,right),Some(300707900));
  assert_eq!(a.behavior(3901,left),Some(301301901));
  assert_eq!(a.behavior(3900,Weapon{handle:5,..right}),None);
  a.notify(Some(left),true);assert_eq!(a.behavior(3900,right),Some(300707900));
  a.synchronize(1,false,true);assert_eq!(a.behavior(3900,left),None);
 }
}
