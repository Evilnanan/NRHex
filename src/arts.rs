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
pub struct Arts {pub chain:crate::arts_cast::Chain,entries:BTreeMap<u32,Entry>,drops:BTreeMap<u32,Entry>,pending:Option<Weapon>,busy:bool,quiet:u8,owned:bool,context:usize,network_dirty:bool,empty_reserved:bool}
impl Arts {
 pub fn observe_cast(&mut self,hand:i32,w:Weapon,request:bool){
  if self.owned&&!self.excluded(w){let skill=self.selected(w).unwrap_or(w.original);if w.base>0&&(0..=0x3fff_fffe).contains(&skill){self.chain.observe(hand,w,skill,request);}}
 }
 pub fn reserve_empty(&mut self,reserved:bool){
  if self.empty_reserved!=reserved{self.empty_reserved=reserved;self.network_dirty=true;}
  if reserved{self.entries.retain(|_,e|e.weapon.base!=110000);if self.pending.is_some_and(|w|w.base==110000){self.pending=None;}}
 }
 fn excluded(&self,w:Weapon)->bool{protected(w.original)||crate::lightning_ram::reserved(self.empty_reserved,w.base)}
 pub fn synchronize(&mut self,context:usize,owned:bool,alive:bool){
  if self.context!=context{self.network_dirty=false;}
  else if !owned&&!self.entries.is_empty(){self.network_dirty=true;}
  if self.context!=context||!owned {self.entries.clear();self.drops.clear();self.pending=None;self.busy=false;self.quiet=0;}
  if self.context!=context||!owned||!alive{self.chain.cast=None;}
  self.context=context;self.owned=owned;
  if !alive{self.pending=None;self.busy=false;self.quiet=0;}
 }
 pub fn notify(&mut self,weapon:Option<Weapon>,used:bool){
  if !self.owned{return;}self.busy=true;self.quiet=0;
  if used&&self.pending.is_none(){if let Some(w)=weapon{if !self.excluded(w){self.pending=Some(w);}}}
 }
 pub fn selected(&self,w:Weapon)->Option<i32>{
  if !self.owned||self.excluded(w){return None;}
  self.entries.get(&w.handle).filter(|e|e.weapon==w).map(|e|e.selected)
 }
 /// Returning from an attack to a fresh preparation is a chain boundary.
 /// The script exits the old stance before asking for a new skill.
 pub fn prepare(&mut self)->Option<Weapon>{self.chain.close();self.pending.take()}
 pub fn has_pending(&self)->bool{self.pending.is_some()}
 /// TAE supplies the exact source weapon even before the first script pulse.
 pub fn behavior(&self,judge:i32,w:Weapon)->Option<i32>{behavior(judge,self.selected(w)?)}
 /// Two updates without a skill node confirm that the whole chain has exited.
 pub fn tick(&mut self)->Option<Weapon>{
  self.chain.tick(self.busy);
  if self.busy{self.busy=false;self.quiet=0;return None;}
  self.quiet=self.quiet.saturating_add(1);
  if self.quiet>=2 {self.pending.take()}else{None}
 }
 pub fn reforge(&mut self,w:Weapon,id:i32){if self.owned&&!self.excluded(w){self.network_dirty=true;self.entries.insert(w.handle,Entry{weapon:w,selected:id});}}
 /// Native world UID includes the producing player and the drop sequence.
 /// It identifies the actual ground item, even for otherwise identical weapons.
 pub fn network_pending(&self)->bool{self.network_dirty}
 pub fn invalidate_network(&mut self){self.network_dirty=true;}
 pub fn network_sent(&mut self){self.network_dirty=false;}
 pub fn dropped(&mut self,uid:u32,w:Weapon){
  if uid==0{return;}if let Some(id)=self.selected(w){self.drops.insert(uid,Entry{weapon:w,selected:id});}
 }
 pub fn picked_up(&mut self,uid:u32,w:Weapon)->bool{
  if !self.owned||self.excluded(w){return false;}
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
  1011|1014|1183=>crate::arts_data::FIRE_WEAPONS.binary_search(&base).is_ok(),
  1024|1184=>crate::arts_data::MAGIC_WEAPONS.binary_search(&base).is_ok(),
  1019=>crate::arts_data::FIRE_WEAPONS.binary_search(&base).is_ok()&&crate::arts_data::MAGIC_WEAPONS.binary_search(&base).is_ok(),
  1022|1187=>crate::arts_data::HOLY_WEAPONS.binary_search(&base).is_ok(),
  _=>true,
 }
}
pub fn bow_ammo_judge(judge:i32)->bool{
 matches!(judge,601|606|610|615|620|625|630|635|641|646|840|845|851|856|870|875|881|886)
 ||crate::arts_data::MMV_AMMO_JUDGES.binary_search(&judge).is_ok()
}
/// Bow TAE resolves these judges through ammunition, not the bow's skill
/// variation. Ordinary arrows lack Through and Through and the rain chain.
/// Keep native greatarrow variants, otherwise use the canonical greatarrow.
pub fn bow_ammo_behavior(judge:i32,skill:i32,weapon:i32,native:i32)->Option<i32>{
 if crate::arts_data::BOW_WEAPONS.binary_search(&weapon).is_err(){return None;}
 if crate::arts_data::MMV_BOW_SKILLS.binary_search(&skill).is_ok(){return behavior(judge,skill);}
 let row=match (skill,judge){
  (400,870|875)|(1169,881|886)|(406,851|856)=>105100000+judge,
  (401,610|615)|(402,601|606)|(404,620|625)|(405,630|635)|(406,641|646)=>105000000+judge,
  // Sky Shot's large-bow TAE uses 840/845, absent from both arrow tables.
  (405,840)=>105000630,(405,845)=>105000635,
  _=>return None,
 };
 let family=native/1000;
 let native_arrow=(105000..105200).contains(&family)&&native%1000==judge;
 let rain_needs_greatarrow=skill==1169&&family<105100;
 if native_arrow&&!rain_needs_greatarrow&&crate::arts_data::BEHAVIORS.binary_search(&native).is_ok(){return None;}
 crate::arts_data::BEHAVIORS.binary_search(&row).is_ok().then_some(row)
}
/// Native 66B2D0 converts a bow's left/right source to -6/-5 (arrows).
/// Recover that exact firing hand; a bow action does not require the stored
/// equipment style to be two-handed. Direct weapon sources remain direct.
pub fn bow_weapon_source(source:i32)->Option<i32>{match source{
 -6=>Some(-2),-5=>Some(-1),-2|-1|0..=5=>Some(source),_=>None,
}}
/// Fire shields fire from a muzzle that most other weapon models lack.
/// Body dummy58 is the native shared arrow/projectile origin; change only the
/// random cross-weapon overlay, leaving original fire shields' muzzles intact.
pub fn emitter_dummy(skill:i32,base:i32,behavior:i32,dummy:i32)->Option<i32>{
 let (family,point)=match skill{1001=>(3215,141),1002=>(3216,142),_=>return None};
 if base/10000==family||behavior/1000!=300000+family{return None;}
 if dummy==10000+point||dummy==20000+point{Some(58)}else{None}
}
pub fn behavior_judge(judge:i32)->bool{
 (3000..4000).contains(&judge)||crate::arts_data::MMV_JUDGES.binary_search(&judge).is_ok()
}
pub fn behavior(judge:i32,skill:i32)->Option<i32>{
 if crate::arts_data::MMV_SKILLS.binary_search(&skill).is_ok(){
  return crate::arts_data::MMV_BEHAVIORS.binary_search_by_key(&(skill,judge),|x|(x.0,x.1)).ok().map(|i|crate::arts_data::MMV_BEHAVIORS[i].2);
 }
 if !(3000..4000).contains(&judge){return None;}
 let target=crate::arts_data::SKILL_VARIATIONS.binary_search_by_key(&skill,|x|x.0).ok().map(|i|crate::arts_data::SKILL_VARIATIONS[i].1)?;
 let id=300000000+target*1000+judge%1000;
 if crate::arts_data::BEHAVIORS.binary_search(&id).is_err(){None}else{Some(id)}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn through_and_through_uses_both_greatarrow_shots_only_for_its_selected_skill(){
  for judge in [870,875]{
   assert!(bow_ammo_judge(judge));
   for weapon in [40000000,41000000,42040000]{
    assert_eq!(bow_ammo_behavior(judge,400,weapon,-1),Some(105100000+judge));
    assert_eq!(bow_ammo_behavior(judge,400,weapon,100000000+judge),Some(105100000+judge));
    for family in [105100,105101,105130]{
     let native=family*1000+judge;
     if crate::arts_data::BEHAVIORS.binary_search(&native).is_ok(){assert_eq!(bow_ammo_behavior(judge,400,weapon,native),None);}
    }
   }
   for skill in [401,402,404,405,406,1169]{assert_eq!(bow_ammo_behavior(judge,skill,40000000,-1),None);}
   assert_eq!(bow_ammo_behavior(judge,400,2000000,-1),None);
  }
  for judge in [610,615,601,606,620,625,630,635,641,646,851,856,881,886]{
   assert_eq!(bow_ammo_behavior(judge,400,40000000,-1),None);
  }
 }
 #[test]fn every_bow_art_has_reviewed_ammunition_and_never_substitutes_another_arts_judge(){
  let entries=[(400,870,105100870),(400,875,105100875),(401,610,105000610),(401,615,105000615),
   (402,601,105000601),(402,606,105000606),(404,620,105000620),(404,625,105000625),
   (405,630,105000630),(405,635,105000635),(405,840,105000630),(405,845,105000635),
   (406,641,105000641),(406,646,105000646),(406,851,105100851),(406,856,105100856),
   (1169,881,105100881),(1169,886,105100886)];
  for (skill,judge,row) in entries{
   assert!(bow_ammo_judge(judge));
   for weapon in [40000000,41000000,42000000]{assert_eq!(bow_ammo_behavior(judge,skill,weapon,-1),Some(row));}
   for &other in &crate::arts_data::BOW_POOL{if other!=skill{assert_eq!(bow_ammo_behavior(judge,other,40000000,-1),None);}}
  }
  for &(skill,judge,row) in crate::arts_data::MMV_BEHAVIORS{
   if crate::arts_data::MMV_BOW_SKILLS.contains(&skill){
    assert!(bow_ammo_judge(judge));assert_eq!(bow_ammo_behavior(judge,skill,40000000,-1),Some(row));
   }
  }
 }
 #[test]fn reforged_rain_uses_rain_ammunition_without_replacing_native_variants(){
  for judge in [881,886]{
   for native in [-1,105000881,100000886,106000881]{
    assert_eq!(bow_ammo_behavior(judge,1169,40000000,native),Some(105100000+judge));
   }
   for family in [105100,105101]{assert_eq!(bow_ammo_behavior(judge,1169,42000000,family*1000+judge),None);}
   for skill in [400,401,402,404,405,406]{assert_eq!(bow_ammo_behavior(judge,skill,40000000,-1),None);}
   assert_eq!(bow_ammo_behavior(judge,1169,2000000,-1),None);
  }
  for judge in [0,800,880,882,885,887,3000]{assert_eq!(bow_ammo_behavior(judge,1169,40000000,-1),None);}
  assert_eq!(bow_weapon_source(-6),Some(-2));assert_eq!(bow_weapon_source(-5),Some(-1));
  for source in [-2,-1,0,1,2,3,4,5]{assert_eq!(bow_weapon_source(source),Some(source));}
  for source in [-7,-4,-3,6,7,8,9,30]{assert_eq!(bow_weapon_source(source),None);}
 }
 fn w(handle:u32,original:i32)->Weapon{Weapon{handle,original,base:2000000}}
 fn state()->Arts{let mut a=Arts::default();a.synchronize(1,true,true);a}
 #[test]fn ram_reserves_all_empty_instances_but_not_real_weapons(){
  let mut a=state();let empty=Weapon{base:110000,..w(3,503)};let real=w(4,504);
  a.reforge(empty,100);a.notify(Some(empty),true);a.reserve_empty(true);
  assert!(!a.has_pending());assert_eq!(a.selected(empty),None);assert!(a.network_pending());a.network_sent();
  for handle in 10..16{let w=Weapon{handle,..empty};a.reforge(w,100);a.notify(Some(w),true);assert_eq!(a.selected(w),None);assert!(!a.has_pending());}
  a.reforge(real,504);assert_eq!(a.selected(real),Some(504));a.notify(Some(real),true);assert!(a.has_pending());
  a.reserve_empty(false);assert!(a.network_pending());a.reforge(empty,100);assert_eq!(a.selected(empty),Some(100));
 }
 #[test]fn target_weapon_catalogue_and_reviewed_behavior_maps_preserve_boundaries(){
  assert!(crate::arts_data::DEFAULTS.windows(2).all(|w|w[0].0<w[1].0));
  for &(base,id) in &crate::arts_data::DEFAULTS{assert_eq!(original(base,-1),Some(id));assert_eq!(original(base,12345),Some(12345));}
  for &base in &crate::arts_data::BOW_WEAPONS{assert_eq!(pool(base),&crate::arts_data::BOW_POOL);}
  assert!(crate::arts_data::MMV_BEHAVIORS.windows(2).all(|w|(w[0].0,w[0].1)<(w[1].0,w[1].1)));
  for &(skill,judge,row) in crate::arts_data::MMV_BEHAVIORS{
   assert!(behavior_judge(judge));assert_eq!(behavior(judge,skill),Some(row));
   assert!(crate::arts_data::BEHAVIORS.binary_search(&row).is_ok());
   assert_eq!(behavior(judge,999999),None);
  }
  for &skill in crate::arts_data::MMV_SKILLS{
   let bow=crate::arts_data::MMV_BOW_SKILLS.contains(&skill);
   assert_eq!(pool(2000000).contains(&skill),!bow);assert_eq!(pool(41000000).contains(&skill),bow);
   assert_eq!(behavior(0,skill),None);assert_eq!(behavior(999999,skill),None);
   let weapon=Weapon{base:if bow{41000000}else{2000000},..w(123,100)};let mut a=state();a.notify(Some(weapon),true);a.tick();a.tick();assert_eq!(a.tick(),Some(weapon));
   a.reforge(weapon,skill);assert!(a.network_pending());assert_eq!(a.selected(weapon),Some(skill));
   let mut data=[0x67;0x48];data[..4].copy_from_slice(&weapon.base.to_le_bytes());let mut expected=data;
   expected[4..8].copy_from_slice(&skill.to_le_bytes());snapshot(&mut data,weapon.base,skill);assert_eq!(data,expected);
   a.dropped(99,weapon);assert!(a.picked_up(99,Weapon{handle:456,..weapon}));
   a.synchronize(1,true,false);assert_eq!(a.selected(weapon),Some(skill));a.synchronize(2,true,true);assert_eq!(a.selected(weapon),None);
  }
  for judge in [-1,0,999,10000,999999]{assert!(!behavior_judge(judge));}
 }
 #[test]fn elemental_projectile_arts_require_the_weapons_matching_attack_component(){
  for (weapon,fire,magic) in [(2000000,false,false),(3140000,true,false),(12200000,true,false),(13020000,false,true),(2150000,false,true),(-1,false,false)]{
   assert_eq!(roll_allowed(weapon,1014),fire);
   assert_eq!(roll_allowed(weapon,1183),fire);
   assert_eq!(roll_allowed(weapon,1184),magic);
   for &id in pool(weapon){if !matches!(id,1011|1014|1019|1022|1024|1183|1184|1187){assert!(roll_allowed(weapon,id));}}
  }
 }
 #[test]fn elemental_filter_never_empties_weapon_pool_or_changes_bow_choices(){
  for &(weapon,_) in crate::arts_data::DEFAULTS.iter(){
   let choices=pool(weapon);assert!(choices.iter().filter(|&&id|roll_allowed(weapon,id)).count()>=choices.len().saturating_sub(8));
   if crate::arts_data::BOW_WEAPONS.binary_search(&weapon).is_ok(){assert!(choices.iter().all(|&id|roll_allowed(weapon,id)));}
  }
 }
 #[test]fn elemental_arts_require_all_of_their_damage_sources(){
  for skill in [1011,1014,1019,1022,1024,1183,1184,1187]{assert!(!roll_allowed(2000000,skill));}
  assert!(roll_allowed(21060000,1011));assert!(!roll_allowed(16120000,1011));
  assert!(roll_allowed(16120000,1024));assert!(!roll_allowed(21060000,1024));
  assert!(roll_allowed(3170000,1022));assert!(roll_allowed(16040000,1187));
  assert!(!roll_allowed(2140000,1022));assert!(!roll_allowed(2140000,1187));
  assert!(roll_allowed(2140000,1019));
  assert!(!roll_allowed(21060000,1019));assert!(!roll_allowed(16120000,1019));
 }
 #[test]fn devourer_uses_the_existing_fire_requirement_without_changing_native_defaults(){
  for &(weapon,_) in crate::arts_data::DEFAULTS.iter(){
   assert_eq!(roll_allowed(weapon,1183),roll_allowed(weapon,1014));
   assert_eq!(roll_allowed(weapon,1183),crate::arts_data::FIRE_WEAPONS.binary_search(&weapon).is_ok());
  }
  assert_eq!(original(12200000,-1),Some(1183));
  assert!(roll_allowed(12200000,1183));
  assert!(pool(2000000).contains(&1183));
  assert!(!pool(41000000).contains(&1183));
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
  let expected:Vec<_>=[400,401,402,404,405,406,1169].into_iter().chain(crate::arts_data::MMV_BOW_SKILLS.iter().copied()).collect();
  assert_eq!(pool(41000000),expected);
  assert_eq!(pool(42000000),pool(41000000));
  let ordinary=pool(2000000);assert_eq!(ordinary.len(),175+crate::arts_data::MMV_SKILLS.len()-crate::arts_data::MMV_BOW_SKILLS.len());
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
