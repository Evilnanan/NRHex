use super::*;
#[test]fn work_fee_is_fractional_and_independent_of_animation_cycles(){
 for hz in [30,60,144]{
  let mut fee=MotorFee::default();assert_eq!(fee.action(18,110000),18);
  let mut paid=0;
  for i in 0..hz*10{
   if i%3==0{assert_eq!(fee.action(18,110000),0);}
   paid+=fee.charge(160./hz as f32,1.);
  }
  assert!((199..=200).contains(&paid),"{hz}: {paid}");
  assert!((f64::from(paid)+fee.remainder-200.).abs()<0.001);
 }
 let mut fee=MotorFee::default();assert_eq!(fee.action(0,110000),0);
 assert_eq!(fee.action(18,110000),0,"free startup cannot defer a later startup charge");
 for _ in 0..100{assert_eq!(fee.charge(160.,0.),0);assert_eq!(fee.charge(0.,1.),0);}
 assert_eq!(fee.charge(160.,0.5),10);assert_eq!(fee.charge(80.,1.),10);
 for (work,rate)in[(f32::NAN,1.),(-1.,1.),(1.,f32::INFINITY),(1.,-1.)]{assert_eq!(fee.charge(work,rate),0);}
}
#[test]fn motor_work_excludes_gravity_drag_and_passive_rolling(){
 for hz in [30.,60.,144.]{
  for ground in [Some(V(0.,1.,0.)),Some(V(0.6,0.8,0.)),None]{
   let mut passive=Motion{velocity:V(15.,0.,0.),..Default::default()};
   for _ in 0..hz as usize{assert!(passive.advance(1./hz,false,V(1.,0.,0.),ground));assert_eq!(passive.motor_work,0.);}
  }
  let mut ground=Motion{velocity:V(16.,0.,0.),..Default::default()};let mut air=Motion{velocity:V(20.,0.,0.),..Default::default()};
  let(mut ground_fee,mut air_fee)=(MotorFee::default(),MotorFee::default());let(mut paid_ground,mut paid_air)=(0,0);
  for _ in 0..hz as usize{
   ground.advance(1./hz,true,V(1.,0.,0.),Some(V(0.,1.,0.)));
   air.advance(1./hz,true,V(1.,0.,0.),None);
   paid_ground+=ground_fee.charge(ground.motor_work,1.);paid_air+=air_fee.charge(air.motor_work,1.);
  }
  assert!((f64::from(paid_ground)+ground_fee.remainder-20.).abs()<0.002);
  assert!((f64::from(paid_air)+air_fee.remainder-10.).abs()<0.002);
 }
 let mut low=Motion::default();low.advance(1./60.,true,V(1.,0.,0.),Some(V(0.,1.,0.)));
 assert!(low.motor_work>0.&&low.motor_work<160./60.,"low-speed torque limit reduces output");
}
#[test]fn scarce_stamina_limits_motor_work_and_preserves_momentum(){
 let mut motion=Motion{velocity:V(16.,0.,0.),..Default::default()};
 let mut fee=MotorFee::default();fee.remainder=0.25;
 // One point remains, of which 0.25 is already owed. A long frame would cost 5.
 let budget=(1.-fee.remainder)as f32/0.125;
 motion.advance_with_budget(0.25,true,V(1.,0.,0.),Some(V(0.,1.,0.)),budget);
 assert!(motion.motor_work<=budget+1e-5);assert_eq!(fee.charge(motion.motor_work,1.),1);
 assert!(motion.velocity.len()>10.);
 motion.advance_with_budget(0.25,true,V(1.,0.,0.),Some(V(0.,1.,0.)),0.);
 assert_eq!(motion.motor_work,0.);assert!(motion.velocity.len()>10.);
}
#[test]fn level_bonus_never_changes_speed_tiers_and_roll_identity(){
 for generation in [1,2,0xffffff,0x1000000,0xdead_beef_abcd_1234,u64::MAX]{for level in 1..=15{
  let encoded=encode_roll(generation,level);assert!(encoded.iter().all(|x|x.is_finite()&&x.fract()==0.));
  assert_eq!(decode_roll(encoded),Some((generation,level)));
 }}
 assert_eq!(decode_roll([1.;3]),None);assert_eq!(decode_roll([f32::NAN;3]),None);
 assert_eq!(decode_roll(crate::soul_immunity::encode(123)),Some((123,0)));
 for speed in [6.,9.99,10.,14.99,15.,20.,40.]{
  let raw=speed*speed*1.5;let tier=attack_for_damage(raw);
  for level in [0,1,15]{assert_eq!(contact_damage(raw,level),raw+if speed>=15.{30.*level as f32}else{0.});assert_eq!(attack_for_damage(raw),tier);}
 }
 assert_eq!(contact_damage(337.5,15),787.5);
}
