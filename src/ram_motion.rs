//! Metres / seconds. Independent velocity state, native capsule remains authoritative.
#[derive(Clone,Copy,Debug,Default,PartialEq)]pub struct V(pub f32,pub f32,pub f32);
impl V{
 pub fn dot(self,b:Self)->f32{self.0*b.0+self.1*b.1+self.2*b.2}
 pub fn len(self)->f32{self.dot(self).sqrt()}
 pub fn scale(self,k:f32)->Self{Self(self.0*k,self.1*k,self.2*k)}
 pub fn add(self,b:Self)->Self{Self(self.0+b.0,self.1+b.1,self.2+b.2)}
 pub fn sub(self,b:Self)->Self{self.add(b.scale(-1.))}
 pub fn unit(self)->Self{let n=self.len();if n>0.0001{self.scale(1./n)}else{Self::default()}}
 pub fn finite(self)->bool{self.0.is_finite()&&self.1.is_finite()&&self.2.is_finite()}
 pub fn cross(self,b:Self)->Self{Self(self.1*b.2-self.2*b.1,self.2*b.0-self.0*b.2,self.0*b.1-self.1*b.0)}
 pub fn plane(self,n:Self)->Self{self.sub(n.scale(self.dot(n)))}
 pub fn array(self)->[f32;4]{[self.0,self.1,self.2,0.]}
 pub fn from(a:[f32;4])->Self{Self(a[0],a[1],a[2])}
}
pub const HIGH:f32=10.;
pub const GRAVITY:f32=12.;
pub const RELEASE:f32=6.;
pub const CRASH_SPEED:f32=15.;
pub const SAFETY_SPEED:f32=40.;
pub const SUPPORT_Y:f32=0.55;
pub fn wall_normal(n:V)->bool{n.1<SUPPORT_Y&&n.0.hypot(n.2)>=0.45}
/// Short collision compliance only for verified breakable assets / enemies
/// that actually accepted knockback. Native displacement remains authoritative.
#[derive(Default,Debug)]pub struct Yielding{pub body:Option<u32>,pub age:f32,pub knocked:f32}
impl Yielding{
 pub fn step(&mut self,body:Option<u32>,hard:bool,dt:f32)->Option<f32>{
  let enemy=self.knocked>0.;self.knocked=(self.knocked-dt).max(0.);
  if body!=self.body{self.body=body;self.age=0.;}
  if body.is_some(){self.age+=dt;}
  if hard{return None;}
  if body.is_some()&&self.age<=0.12{Some(0.10)}else if body.is_none()&&enemy{Some(0.12)}else{None}
 }
}
pub fn compliant_velocity(incoming:V,accepted:V,dt:f32,tau:f32)->V{
 // Relax the horizontal impulse over physical time, not per callback. Do not
 // retain rejected vertical motion (stairs, ground, landings remain native).
 let keep=(-dt/tau).exp();
 let v=V(accepted.0+(incoming.0-accepted.0)*keep,accepted.1,accepted.2+(incoming.2-accepted.2)*keep);
 let limit=incoming.len().min(SAFETY_SPEED);v.scale((limit/v.len().max(0.00001)).min(1.))
}
pub fn frontal_speed(incoming:V,normal:V)->Option<f32>{
 let closing=(-incoming.dot(normal)).max(0.);
 (closing>=incoming.len()*0.7&&closing>0.).then_some(closing)
}
pub fn wall_impact(incoming:V,rejected:V,normal:V)->Option<f32>{
 frontal_speed(incoming,normal)?;
 // A floor correcting vertical fall is not a wall. Require rejected lateral
 // travel into this surface as well as the full 3D normal impulse; tilted
 // walls still use the 3D incoming speed for impact severity.
 let lateral=-(rejected.0*normal.0+rejected.2*normal.2);
 // Angle decides whether rolling ends. Once stopped, all incoming motion is
 // discarded, so severity uses the pre-collision total speed.
 // Fast grazing contact must keep the tangent accepted by the native capsule.
 (lateral>0.5&& -rejected.dot(normal)>0.5).then_some(incoming.len())
}
/// Trade forward kinetic energy for clearance over a verified low ledge.
/// Geometry must prove a reachable top and clear torso path before calling.
pub fn step_launch(incoming:V,accepted:V,normal:V,rise:f32)->Option<V>{
 if !incoming.finite()||!accepted.finite()||!normal.finite()||!rise.is_finite()
  ||!(0.01..=0.7).contains(&rise)||incoming.1< -1.{return None;}
 let forward=V(incoming.0,0.,incoming.2);let speed=incoming.len();
 let rejected=incoming.sub(accepted);
 if forward.len()<6.|| -forward.dot(normal)<3.|| -rejected.dot(normal)<=0.5{return None;}
 let up=(2.*GRAVITY*(rise+0.12)).sqrt().max(incoming.1);
 if up>=speed*0.7{return None;}
 let horizontal=(speed*speed-up*up).max(0.).sqrt()*0.94;
 Some(forward.unit().scale(horizontal).add(V(0.,up,0.)))
}
/// Small continuous changes of contact can redirect tangent momentum. An
/// abrupt corner cannot turn a frontal impact into a lossless wall climb.
pub fn continuous_surface(previous:Option<V>,velocity:V,normal:V)->bool{
 normal.1>=0.&&previous.is_some_and(|old|old.dot(normal)>0.94)
  && -velocity.dot(normal)<3f32.max(velocity.len()*0.2)
}
pub fn support_surface(previous:Option<V>,velocity:V,normal:V)->bool{
 normal.1>=SUPPORT_Y||continuous_surface(previous,velocity,normal)
}
pub fn separates(velocity:V,normal:V,dt:f32)->bool{
 // Gravity may bend travel onto a gentle descending surface, but support
 // cannot pull the body around a convex crest faster than gravity permits.
 velocity.dot(normal)>GRAVITY*normal.1.max(0.)*dt+0.05
}
/// Keep ballistic velocity where the scene is clear. Native support snapping
/// is not an airborne impulse. Large unexplained blocks still consume speed;
/// a short, bounded takeoff allowance only bridges the native ground transition.
pub fn flight_feedback(incoming:V,accepted:V,takeoff:bool,side_block:bool,ceiling:bool)->V{
 let mut v=accepted;
 let loss=V(incoming.0-accepted.0,0.,incoming.2-accepted.2).len();
 if !side_block&&(takeoff||loss<=0.5f32.max(incoming.0.hypot(incoming.2)*0.15)){
  v.0=incoming.0;v.2=incoming.2;
 }
 if !ceiling{v.1=incoming.1;}
 let limit=incoming.len().min(SAFETY_SPEED);v.scale((limit/v.len().max(0.00001)).min(1.))
}
/// One submission, then the position resolved by the native capsule on the
/// following update. Rejected motion is never carried into the next step.
#[derive(Default,Debug)]pub struct Travel{pub previous:Option<(V,V,f32)>,pub blocked:f32}
#[derive(Debug)]pub struct Resolved{pub velocity:V,pub incoming:V,pub rejected:V,pub stopped:bool}
impl Travel{
 pub fn submit(&mut self,position:V,velocity:V,dt:f32){self.previous=Some((position,velocity,dt));}
 pub fn resolve(&mut self,position:V)->Option<Resolved>{
  let (from,incoming,dt)=self.previous.take()?;
  if !dt.is_finite()||dt<=0.||dt>0.25{return None;}
  let displacement=position.sub(from);
  if !displacement.finite()||displacement.len()>10.{self.blocked=0.;return Some(Resolved{velocity:V::default(),incoming:V::default(),rejected:V::default(),stopped:true});}
  let measured=displacement.scale(1./dt);let speed=measured.len();
  // Native corrections/platform motion may move us farther, but must not
  // manufacture rolling energy. Keep the accepted direction, cap the energy.
  let velocity=measured.scale(incoming.len().min(speed)/speed.max(0.00001));
  let rejected=incoming.sub(velocity);
  if velocity.len()<0.5&&incoming.len()>0.05{self.blocked+=dt;}else{self.blocked=0.;}
  Some(Resolved{velocity,incoming,rejected,stopped:self.blocked>=0.18})
 }
}
// Parallel transport on a smooth support surface does no work: only gravity,
// drag and actual impact impulses may change kinetic energy.
fn transport(v:V,from:V,to:V)->V{
 let axis=from.cross(to);let c=from.dot(to).clamp(-1.,1.);
 if c< -0.99{return v.plane(to);}
 v.add(axis.cross(v)).add(axis.cross(axis.cross(v)).scale(1./(1.+c)))
}
fn steer(v:V,wish:V,n:V,dt:f32)->V{
 let speed=v.len();let desired=wish.plane(n).unit();
 if speed<0.01||desired.len()<0.5{return v;}
 let forward=v.scale(1./speed);let angle=n.dot(forward.cross(desired)).atan2(forward.dot(desired).clamp(-1.,1.));
 let angle=angle.clamp(-5.*dt/speed,5.*dt/speed);
 forward.scale(angle.cos()).add(n.cross(forward).scale(angle.sin())).scale(speed)
}
fn impact_tangent(tangent:V,normal_impulse:f32,friction:f32)->V{
 let speed=tangent.len();tangent.scale((speed-normal_impulse*friction).max(0.)/speed.max(0.0001))
}
const INERTIA:f32=0.4; // I/(m*r^2); spin is expressed as surface speed r*omega.
const STATIC_GRIP:f32=4.5;
const SLIDING_GRIP:f32=3.375;
fn traction(velocity:V,spin:V,n:V,normal_impulse:f32)->(V,V){
 let slip=spin.plane(n).sub(velocity.plane(n));
 let impulse=slip.scale(INERTIA/(1.+INERTIA));
 // Static grip keeps ordinary rolling matched to travel. Once that required
 // impulse exceeds static grip, kinetic friction consumes the slip gradually.
 let load=normal_impulse.max(0.);
 let impulse=if impulse.len()<=STATIC_GRIP*load{impulse}else{impulse.unit().scale(SLIDING_GRIP*load)};
 (velocity.add(impulse),spin.sub(impulse.scale(1./INERTIA)))
}
fn spin_motor(spin:V,heading:V,dt:f32,drive:bool,power:f32,acceleration:f32)->(V,f32){
 if !drive{return (spin,0.);}
 let added=(power/(INERTIA*spin.len().max(4.))).min(acceleration)*dt;
 let v=spin.add(heading.scale(added));let v=v.scale((SAFETY_SPEED/v.len().max(0.00001)).min(1.));
 // Bill only positive motor work, before grip, gravity and drag. Bound the
 // discrete energy increment by the configured output power.
 (v,(0.5*INERTIA*(v.dot(v)-spin.dot(spin))).clamp(0.,power*dt))
}
#[derive(Default,Debug)]pub struct Motion{pub motor_work:f32,pub velocity:V,pub spin:V,pub initialized:bool,pub locked:bool,pub airborne:bool,pub support:Option<V>}
impl Motion{
 pub fn advance(&mut self,dt:f32,drive:bool,wish:V,ground:Option<V>)->bool{self.advance_with_budget(dt,drive,wish,ground,f32::INFINITY)}
 pub fn advance_with_budget(&mut self,dt:f32,drive:bool,wish:V,ground:Option<V>,work_budget:f32)->bool{
  self.motor_work=0.;
  if !dt.is_finite()||dt<0.||dt>0.25||!self.velocity.finite()||!self.spin.finite(){*self=Self::default();return false;}
  if dt==0.{return true;}
  if !self.initialized{
   self.spin=if self.airborne||ground.is_none(){V(self.velocity.0,0.,self.velocity.2)}else{self.velocity};
   self.initialized=true;
  }
  let previous=self.support;
  let mut ground=ground.filter(|n|n.finite()&&n.len()>0.5).map(V::unit).filter(|n|support_surface(previous,self.velocity,*n));
  if let Some(n)=ground{
   let approach=self.velocity.dot(n);
   // Do not glue a rolling body to the downward side of a crest, nor catch
   // an ascending rebound merely because the short support probe sees floor.
   if separates(self.velocity,n,dt){ground=None;}
   else if self.airborne&&approach<0.{
    self.collide(n);
    if self.velocity.dot(n)>0.5{ground=None;}
   }
   else if let Some(old)=previous.filter(|_|continuous_surface(previous,self.velocity,n)){
    // Accepted travel may already have been projected by the capsule. Do not
    // rotate that vector a second time (which would bias a banked surface).
    self.velocity=self.velocity.plane(n);
    self.spin=transport(self.spin.plane(old),old,n);
   }else{self.velocity=self.velocity.plane(n);}
  }
  self.support=ground;self.airborne=ground.is_none();
  let steps=(dt*120.).ceil()as usize;let h=dt/steps as f32;
  for _ in 0..steps{
   let remaining=(work_budget-self.motor_work).max(0.)/h;
   if let Some(n)=ground{
    let before=self.velocity.unit();
    self.velocity=steer(self.velocity,wish,n,h);
    let speed=self.velocity.len();let heading=if speed>0.1{self.velocity.unit()}else{wish.plane(n).unit()};
    if before.len()>0.5&&speed>0.1{
     let c=before.dot(heading).clamp(-1.,1.);let s=n.dot(before.cross(heading));
     self.spin=self.spin.scale(c).add(n.cross(self.spin).scale(s));
    }
    let (spin,work)=spin_motor(self.spin.plane(n),heading,h,drive,160f32.min(remaining),30.);self.spin=spin;self.motor_work+=work;
    (self.velocity,self.spin)=traction(self.velocity,self.spin,n,GRAVITY*n.1*h);
    let speed=self.velocity.len();let next=(speed-(1.2+0.035*speed*speed)*h).max(0.);
    self.velocity=self.velocity.unit().scale(next).add(V(0.,-GRAVITY,0.).plane(n).scale(h));
   }else{
    let heading=if self.spin.len()>0.1{self.spin.unit()}else{V(wish.0,0.,wish.2).unit()};
    let (spin,work)=spin_motor(self.spin,heading,h,drive,80f32.min(remaining),10.);self.spin=spin;self.motor_work+=work;
    self.spin=self.spin.scale((-(0.15+0.015*self.spin.len())*h).exp());
    let v=self.velocity;self.velocity=v.add(V(0.,-GRAVITY,0.).sub(v.scale(0.01*v.len())).scale(h));
   }
   let speed=self.velocity.len();if speed>SAFETY_SPEED{self.velocity=self.velocity.scale(SAFETY_SPEED/speed);}
  }
  let speed=self.velocity.len();if speed>=HIGH{self.locked=true;}else if speed<=RELEASE{self.locked=false;}
  true
 }
 /// Collision losses depend on the normal impulse, never on callback count.
 pub fn collide(&mut self,normal:V)->bool{
  let n=normal.unit();let approach=self.velocity.dot(n);let speed=self.velocity.len();
  if !n.finite()||n.len()<0.9||approach>=0.{return false;}
  let tangent=self.velocity.plane(n);
  if !self.initialized{self.spin=tangent;self.initialized=true;}
  if n.1>=SUPPORT_Y{
   let rebound=if approach< -6.{(-approach*0.18).min(2.2)}else{0.};
   let tangent=if approach< -1.{let(t,spin)=traction(tangent,self.spin,n,-approach+rebound);self.spin=spin;t}else{tangent};
   if rebound>0.{
    self.velocity=tangent.add(n.scale(rebound));
    self.airborne=true;self.support=None;
   }else{
    // Resting/sliding contact removes penetration only. No repeated 10% tax.
    self.velocity=tangent;
   }
   false
  }else{
   let head_on= -approach/speed.max(0.001)>=0.7;
   self.velocity=impact_tangent(tangent,-approach*1.3,0.2).sub(n.scale(approach*0.3));
   if head_on{self.locked=false;}head_on
  }
 }
 pub fn recoil(&mut self){self.velocity=V(-self.velocity.0*0.3,2.,-self.velocity.2*0.3);self.locked=false;self.airborne=true;self.support=None;}
}
#[cfg(test)]mod tests{
 #[test]fn verified_step_trades_speed_for_lift_without_creating_energy(){
  let n=V(0.12084629,-0.1304672,-0.9840602);
  for speed in [6.,10.,13.5,20.,30.]{for rise in [0.05,0.3,0.6,0.7]{
   let incoming=V(0.,1.,speed);let launch=step_launch(incoming,V(0.,2.,1.),n,rise);
   // With stronger gravity, 6m/s cannot clear 0.7m plus the safety margin
   // while retaining the existing forward-energy reserve. Do not invent lift.
   if speed==6.&&rise==0.7{assert!(launch.is_none());continue;}
   let v=launch.unwrap();
   assert!(v.1>1.&&v.2>0.&&v.len()<incoming.len());
   assert!(v.1*v.1/(2.*GRAVITY)>=rise+0.119);
  }}
  for v in [V(0.,0.,4.),V(0.,-5.,20.),V(20.,0.,0.),V(f32::NAN,0.,20.)]{assert!(step_launch(v,V::default(),n,0.4).is_none());}
  assert!(step_launch(V(0.,0.,15.),V(0.,0.,15.),n,0.4).is_none());
  for rise in [-0.1,0.,0.71,f32::NAN]{assert!(step_launch(V(0.,0.,15.),V::default(),n,rise).is_none());}
 }
 use super::*;
 #[test]fn tangent_vertical_climb_and_downhill_collision_are_directional(){
  let vertical=V(-1.,0.,0.);let steep=V(-0.995,0.1,0.).unit();
  assert!(continuous_surface(Some(steep),V(0.,12.,0.),vertical));
  assert!(!continuous_surface(Some(V(0.,1.,0.)),V(15.,0.,0.),vertical));
  let ramp=V(-0.6,0.8,0.);let downhill=V(8.,-12.,0.);
  assert!(wall_impact(downhill,ramp.scale(downhill.dot(ramp)),ramp).unwrap()>14.);
  for hz in [30.,60.,144.]{
   let mut m=Motion{velocity:V(0.,12.,0.),spin:V(0.,12.,0.),initialized:true,support:Some(steep),..Default::default()};
   let mut travel=Travel::default();let mut pos=V::default();
   for _ in 0..(hz*0.5)as usize{
    m.advance(1./hz,true,V(1.,0.,0.),Some(vertical));
    travel.submit(pos,m.velocity,1./hz);pos=pos.add(m.velocity.scale(1./hz));
    assert!(!travel.resolve(pos).unwrap().stopped);assert!(!m.airborne);
   }
   assert!(m.velocity.1>4.&&m.velocity.1<8.);assert!(m.velocity.0.abs()<0.001);
  }
 }
 #[test]fn fast_convex_departure_does_not_require_a_large_normal_velocity(){
  let n=V(0.,1.,-0.02).unit();let v=V(0.,0.,-20.);
  assert!(v.dot(n)<0.75);assert!(separates(v,n,1./60.));
  let mut m=Motion{velocity:v,support:Some(V(0.,1.,0.)),..Default::default()};
  m.advance(1./60.,true,V(0.,0.,-1.),Some(n));
  assert!(m.airborne);assert!(m.velocity.1> -0.2);
 }
 fn energy(v:V,spin:V)->f32{v.dot(v)+INERTIA*spin.dot(spin)}
 #[test]fn ground_grip_recovers_large_slip_while_powered(){
  for hz in [30.,60.,144.]{
   let mut m=Motion{velocity:V(8.,0.,0.),spin:V(30.,0.,0.),initialized:true,support:Some(V(0.,1.,0.)),..Default::default()};
   for _ in 0..(hz*1.2)as usize{m.advance(1./hz,true,V(1.,0.,0.),Some(V(0.,1.,0.)));}
   assert!(m.spin.sub(m.velocity).len()<0.3,"{hz} Hz: velocity {:?}, spin {:?}",m.velocity,m.spin);
   assert!(m.velocity.0>8.);
  }
 }
 #[test]fn airborne_spin_has_a_drag_limited_terminal_rate_and_decays_when_released(){
  for hz in [30.,60.,144.]{
   let mut m=Motion{velocity:V(12.,0.,0.),spin:V(12.,0.,0.),initialized:true,airborne:true,..Default::default()};
   for _ in 0..(hz*10.)as usize{m.advance(1./hz,true,V(1.,0.,0.),None);}
   assert!((19. ..22.).contains(&m.spin.len()),"{hz} Hz: {:?}",m.spin);
   let before=m.spin.len();
   for _ in 0..hz as usize{m.advance(1./hz,false,V(1.,0.,0.),None);}
   assert!(m.spin.len()<before*0.75);
  }
 }
 #[test]fn airborne_motor_changes_spin_not_the_flight_path(){
  for hz in [30.,60.,144.]{
   let mut powered=Motion{velocity:V(12.,4.,0.),airborne:true,..Default::default()};
   let mut free=Motion{velocity:powered.velocity,airborne:true,..Default::default()};
   for _ in 0..hz as usize{
    powered.advance(1./hz,true,V(1.,0.,0.),None);free.advance(1./hz,false,V(-1.,0.,0.),None);
    assert_eq!(powered.velocity,free.velocity);
   }
   assert!(powered.spin.len()>free.spin.len()+2.);assert!(powered.spin.len()<22.);
  }
 }
 #[test]fn ground_friction_transfers_spin_without_creating_energy_or_overshooting_slip(){
  let n=V(0.,1.,0.);
  for speed in [0.,5.,15.,30.]{for spin in [0.,10.,25.,40.]{for load in [0.01,1.,20.]{
   let v=V(speed,0.,0.);let w=V(spin,0.,0.);let(a,b)=traction(v,w,n,load);
   assert!(energy(a,b)<=energy(v,w)+0.001);
   assert!(a.sub(v).len()<=STATIC_GRIP*load+0.00001);
   assert!((a.0-b.0).abs()<=(speed-spin).abs()+0.00001);
   assert!((a.0-b.0)*(speed-spin)>= -0.001);
  }}}
  let(a,b)=traction(V(5.,0.,0.),V(30.,0.,0.),n,20.);
  assert!(a.0>5.&&b.0<30.);
 }
 #[test]fn powered_flat_rolling_keeps_spin_near_travel_instead_of_permanent_wheelspin(){
  for hz in [30.,60.,144.]{
   let m=run(hz,true,Some(V(0.,1.,0.)));
   assert!(m.spin.sub(m.velocity).len()<0.2);
  }
 }
 #[test]fn high_fall_has_one_small_bounce_and_second_contact_settles(){
  let n=V(0.,1.,0.);
  for fall in [7.,12.,20.,35.]{
   let mut m=Motion{velocity:V(10.,-fall,0.),spin:V(30.,0.,0.),initialized:true,airborne:true,..Default::default()};
   let before=energy(m.velocity,m.spin);m.collide(n);
   assert!(m.velocity.1>0.&&m.velocity.1<=2.2);assert!(m.velocity.1.powi(2)/(2.*GRAVITY)<0.25);
   assert!(m.velocity.0>10.);assert!(energy(m.velocity,m.spin)<before);
   m.velocity.1= -m.velocity.1;m.advance(1./60.,false,V(1.,0.,0.),Some(n));
   assert!(!m.airborne);assert!(m.velocity.1.abs()<0.001);
  }
 }
 #[test]fn flight_feedback_bridges_takeoff_but_never_ignores_wall_or_persistent_block(){
  let incoming=V(20.,0.,0.);let snap=V(2.,-5.,0.);
  assert_eq!(flight_feedback(incoming,snap,true,false,false),incoming);
  let wall=flight_feedback(incoming,snap,true,true,false);assert_eq!(wall,V(2.,0.,0.));
  assert_eq!(flight_feedback(incoming,snap,false,false,false),wall);
  let falling=V(12.,-10.,0.);assert_eq!(flight_feedback(falling,V(12.,0.,0.),false,false,false),falling);
  assert_eq!(flight_feedback(V(0.,5.,0.),V::default(),false,false,true),V::default());
 }
 #[test]fn fast_grazing_contact_does_not_force_recovery(){
  assert_eq!(wall_impact(V(30.,0.,-16.),V(0.,0.,-16.),V(0.,0.,1.)),None);
  assert_eq!(wall_impact(V(30.,0.,-3.),V(0.,0.,-3.),V(0.,0.,1.)),None);
  // Same angle at increasing speed cannot cross a hidden absolute-speed gate.
  for speed in [4.,10.,16.,30.,40.]{
   let v=V(speed*0.8660254,0.,-speed*0.5);
   assert_eq!(wall_impact(v,V(0.,0.,v.2),V(0.,0.,1.)),None);
  }
  // The angular boundary remains independent of the 6/15 severity tiers.
  for speed in [4.,10.,20.]{for ratio in [0.69f32,0.71]{
   let v=V(speed*(1.-ratio*ratio).sqrt(),0.,-speed*ratio);
   assert_eq!(wall_impact(v,V(0.,0.,v.2),V(0.,0.,1.)).is_some(),ratio>0.7);
  }}
 }
 #[test]fn floor_rejection_does_not_become_a_wall_crash(){
  let slope=V(-0.173633,0.7306475,-0.66030735);
  // Captured uphill travel: horizontal movement was fully accepted. Only
  // modelled falling was rejected by the native floor.
  assert_eq!(wall_impact(V(1.1253592,-9.837262,0.96821654),V(0.,-10.979669,0.),slope),None);
 }
 fn run(hz:f32,drive:bool,normal:Option<V>)->Motion{let mut m=Motion{velocity:V(0.,0.,-4.),..Default::default()};for _ in 0..(hz*10.)as usize{assert!(m.advance(1./hz,drive,V(0.,0.,-1.),normal));}m}
 #[test]fn collision_severity_uses_total_speed_after_frontal_gate(){
  let incoming=V(10.,0.,-12.);
  assert_eq!(wall_impact(incoming,V(0.,0.,-12.),V(0.,0.,1.)),Some(incoming.len()));
  assert!(wall_impact(incoming,V(0.,0.,-12.),V(0.,0.,1.)).unwrap()>=CRASH_SPEED);
  assert_eq!(wall_impact(V(16.,0.,-3.),V(0.,0.,-3.),V(0.,0.,1.)),None);
  assert_eq!(wall_impact(V(0.,0.,-16.),V(0.,0.,-16.),V(0.,0.,1.)),Some(16.));
  assert_eq!(wall_impact(V(0.,0.,-16.),V::default(),V(0.,0.,1.)),None);
 }
 #[test]fn compliance_is_time_based_and_never_adds_energy(){
  for hz in [30.,60.,120.]{let mut v=V(0.,0.,20.);for _ in 0..(hz/10.)as usize{v=compliant_velocity(v,V::default(),1./hz,0.1);}
   assert!((v.len()-20.*(-1f32).exp()).abs()<0.001);
  }
  let a=V(3.2084,-1.505,15.4975);let b=V(-0.453,-0.158,1.797);
  let soft=compliant_velocity(a,b,0.0217,0.1);assert!(soft.len()>12.&&soft.len()<a.len());assert!(soft.0>0.&&soft.0<a.0);assert_eq!(soft.1,b.1);
  let enemy=compliant_velocity(a,b,0.0217,0.06);assert!(enemy.len()<soft.len());
  let boosted=compliant_velocity(V(3.,0.,0.),V(50.,0.,0.),0.02,0.1);assert!(boosted.len()<=3.);
 }
 #[test]fn yielding_has_a_hard_deadline_and_cannot_override_a_real_wall(){
  for hz in [30.,60.,144.]{let mut c=Yielding::default();let mut elapsed=0.;
   while c.step(Some(42),false,1./hz).is_some(){elapsed+=1./hz;assert!(elapsed<=0.121);}
   for _ in 0..50{assert!(c.step(Some(42),false,1./hz).is_none());}
  }
  let mut c=Yielding{knocked:0.12,..Default::default()};assert!(c.step(None,true,0.02).is_none());
  assert!(c.step(None,false,0.02).is_some());assert!(c.step(None,false,0.2).is_some());assert!(c.step(None,false,0.02).is_none());
 }
 #[test]fn native_block_erases_momentum_before_turning_out(){
  let mut t=Travel::default();t.submit(V::default(),V(0.,0.,-20.),1./60.);
  let r=t.resolve(V::default()).unwrap();assert_eq!(r.velocity,V::default());assert_eq!(r.incoming.len(),20.);
  let mut m=Motion{velocity:r.velocity,..Default::default()};
  m.advance(1./60.,true,V(1.,0.,0.),Some(V(0.,1.,0.)));
  assert!(m.velocity.len()<0.2&&m.velocity.0>0.);assert!(!m.locked);
 }
 #[test]fn native_wall_slide_retains_allowed_motion_without_restitution(){
  let mut t=Travel::default();t.submit(V::default(),V(12.,0.,-8.),0.1);
  let r=t.resolve(V(1.2,0.,0.)).unwrap();assert!((r.velocity.0-12.).abs()<0.0001);assert_eq!(r.velocity.2,0.);
  t.submit(V(1.2,0.,0.),r.velocity,0.1);
  let r=t.resolve(V(1.2,0.,0.)).unwrap();assert_eq!(r.velocity,V::default());
 }
 #[test]fn sustained_constraint_stops_consistently_without_storing_power(){
  for hz in [30.,60.,144.]{let mut t=Travel::default();let mut elapsed=0.;
   loop{t.submit(V::default(),V(0.1,0.,0.),1./hz);elapsed+=1./hz;
    let r=t.resolve(V::default()).unwrap();assert_eq!(r.velocity,V::default());if r.stopped{break;}assert!(elapsed<0.25);
   }assert!((0.18..0.22).contains(&elapsed));
  }
 }
 #[test]fn free_travel_feedback_uses_previous_dt_and_never_adds_energy(){
  let mut t=Travel::default();let mut position=V::default();let v=V(3.,2.,-12.);
  for dt in [1./30.,1./144.,1./60.,0.1]{t.submit(position,v,dt);position=position.add(v.scale(dt));let r=t.resolve(position).unwrap();assert!((r.velocity.len()-v.len()).abs()<0.0001);assert!(!r.stopped);}
  t.submit(position,v,0.1);let r=t.resolve(position.add(v.scale(0.2))).unwrap();assert!((r.velocity.len()-v.len()).abs()<0.0001);
  t.submit(position,v,0.1);assert!(t.resolve(position.add(V(20.,0.,0.))).unwrap().stopped);
 }
 #[test]fn light_floor_contacts_do_not_tax_tangential_speed(){
  let mut m=Motion{velocity:V(15.,-0.2,0.),..Default::default()};
  for _ in 0..144{m.velocity.1= -0.2;assert!(!m.collide(V(0.,1.,0.)));}
  assert_eq!(m.velocity,V(15.,0.,0.));
 }
 #[test]fn shallow_wall_losses_scale_with_impulse_and_do_not_repeat_after_separation(){
  let mut m=Motion{velocity:V(15.,0.,-0.11),..Default::default()};m.collide(V(0.,0.,1.));
  assert!(m.velocity.0>14.97);let once=m.velocity;
  for _ in 0..100{m.collide(V(0.,0.,1.));}assert_eq!(m.velocity,once);
 }
 #[test]fn steering_preserves_energy_turns_both_ways_and_can_turn_around(){
  let n=V(0.,1.,0.);let v=V(0.,0.,-12.);
  let l=steer(v,V(-1.,0.,0.),n,0.1);let r=steer(v,V(1.,0.,0.),n,0.1);
  assert!(l.0<0.&&r.0>0.);assert!((l.len()-12.).abs()<0.00001);assert!((r.len()-12.).abs()<0.00001);
  let back=steer(v,V(0.,0.,1.),n,0.1);assert!(back.0.abs()>0.1);assert!(back.2< -11.);
  let slow=steer(V(0.,0.,-3.),V(1.,0.,0.),n,0.1);
  assert!(slow.0/slow.len()>r.0/r.len());
 }
 #[test]fn support_transport_preserves_speed_and_crests_allow_flight(){
  let a=V(0.,1.,0.);let b=V(0.,0.98,0.2).unit();let v=V(0.,0.,-12.);
  let along=transport(v,a,b);assert!(along.dot(b).abs()<0.00001);assert!((along.len()-v.len()).abs()<0.00001);
  let mut m=Motion{velocity:along,support:Some(b),..Default::default()};m.advance(1./60.,true,V(0.,0.,-1.),Some(a));assert!(m.airborne&&m.support.is_none());assert!(m.velocity.1>0.);
 }
 #[test]fn seam_normal_noise_does_not_accumulate_frame_based_drag(){
  fn noisy(hz:f32)->f32{
   let mut m=Motion{velocity:V(0.,0.,-12.),support:Some(V(0.,1.,0.)),..Default::default()};
   for i in 0..(hz*3.)as usize{let tilt=(i as f32/hz*20.).sin()*0.01;let n=V(0.,1.,tilt).unit();m.advance(1./hz,true,V(0.,0.,-1.),Some(n));m.collide(n);}
   m.velocity.len()
  }
  let a=noisy(30.);let b=noisy(144.);assert!((a-b).abs()<0.1,"30 Hz {a}, 144 Hz {b}");assert!(a>14.);
 }
 #[test]fn power_drag_and_frame_rate(){let a=run(30.,true,Some(V(0.,1.,0.)));let b=run(144.,true,Some(V(0.,1.,0.)));assert!((a.velocity.len()-b.velocity.len()).abs()<0.1);assert!((12. ..20.).contains(&a.velocity.len()));assert!(a.locked);assert!(run(60.,false,Some(V(0.,1.,0.))).velocity.len()<0.01);}
 #[test]fn downhill_gains_energy_uphill_loses_it(){let a=run(60.,true,Some(V(0.,0.95,-0.31)));let b=run(60.,true,Some(V(0.,0.95,0.31)));assert!(a.velocity.len()>b.velocity.len());}
 #[test]fn flight_has_no_drive_and_landing_loses_energy(){let mut a=Motion{velocity:V(8.,2.,0.),..Default::default()};let mut b=Motion{velocity:a.velocity,..Default::default()};a.advance(0.1,true,V(1.,0.,0.),None);b.advance(0.1,false,V(-1.,0.,0.),None);assert_eq!(a.velocity,b.velocity);a.velocity=V(10.,-12.,0.);let old=a.velocity.len();assert!(!a.collide(V(0.,1.,0.)));assert!(a.velocity.1>0.&&a.velocity.len()<old);}
 #[test]fn glancing_and_head_on_contacts(){let mut a=Motion{velocity:V(10.,0.,-2.),..Default::default()};assert!(!a.collide(V(0.,0.,1.)));assert!(a.velocity.0>0.&&a.velocity.2>0.);a.velocity=V(0.,0.,-15.);assert!(a.collide(V(0.,0.,1.)));assert!(a.velocity.2>0.);}
 #[test]fn release_hysteresis_and_invalid_time(){let mut m=Motion{velocity:V(0.,0.,-12.),..Default::default()};m.advance(0.01,false,V::default(),Some(V(0.,1.,0.)));assert!(m.locked);for _ in 0..600{m.advance(1./60.,false,V::default(),Some(V(0.,1.,0.)));}assert!(!m.locked);assert!(!m.advance(f32::NAN,true,V::default(),None));assert_eq!(m.velocity,V::default());}
}
