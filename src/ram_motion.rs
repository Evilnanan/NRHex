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
 pub fn plane(self,n:Self)->Self{self.sub(n.scale(self.dot(n)))}
 pub fn array(self)->[f32;4]{[self.0,self.1,self.2,0.]}
 pub fn from(a:[f32;4])->Self{Self(a[0],a[1],a[2])}
}
pub const HIGH:f32=10.;
pub const RELEASE:f32=6.;
pub const SAFETY_SPEED:f32=40.;
#[derive(Default,Debug)]pub struct Motion{pub velocity:V,pub locked:bool,pub airborne:bool}
impl Motion{
 pub fn advance(&mut self,dt:f32,drive:bool,wish:V,ground:Option<V>)->bool{
  if !dt.is_finite()||dt<0.||dt>0.25||!self.velocity.finite(){*self=Self::default();return false;}
  if dt==0.{return true;}
  let steps=(dt*120.).ceil()as usize;let h=dt/steps as f32;
  let ground=ground.filter(|n|n.finite()&&n.1>0.35).map(V::unit);
  self.airborne=ground.is_none();
  for _ in 0..steps{
   let v=self.velocity;let speed=v.len();let mut force=V(0.,-9.81,0.);
   if let Some(n)=ground{
    self.velocity=v.plane(n);let tangent=self.velocity;let speed=tangent.len();
    force=force.plane(n);
    if drive{force=force.add((if speed>0.1{tangent.unit()}else{wish.plane(n).unit()}).scale((160./speed.max(2.)).min(12.)));}
    // Ground traction changes heading without adding kinetic energy.
    let wish=wish.plane(n).unit();if wish.len()>0.5&&speed>0.01{
     let turn=wish.sub(tangent.unit()).scale((5.*h/speed).min(1.));
     self.velocity=tangent.unit().add(turn).unit().scale(speed);
    }
    let loss=(1.2+0.035*speed*speed)*h;
    self.velocity=self.velocity.scale((speed-loss).max(0.)/speed.max(0.0001));
   }else{force=force.sub(v.scale(0.035*speed));}
   self.velocity=self.velocity.add(force.scale(h));
   let speed=self.velocity.len();if speed>SAFETY_SPEED{self.velocity=self.velocity.scale(SAFETY_SPEED/speed);}
  }
  let speed=self.velocity.len();if speed>=HIGH{self.locked=true;}else if speed<=RELEASE{self.locked=false;}
  true
 }
 /// Returns true for a head-on wall impact. Ground impacts bounce only when substantial.
 pub fn collide(&mut self,normal:V)->bool{
  let n=normal.unit();let approach=self.velocity.dot(n);let speed=self.velocity.len();
  if !n.finite()||n.len()<0.9||approach>=0.{return false;}
  if n.1>0.45{
   let bounce=if approach< -3.{0.45}else{0.};
   self.velocity=self.velocity.plane(n).scale(0.9).sub(n.scale(approach*bounce));self.airborne=bounce>0.;false
  }else{
   let head_on= -approach/speed.max(0.001)>=0.7;
   self.velocity=self.velocity.plane(n).scale(if head_on{0.25}else{0.8}).sub(n.scale(approach*0.3));
   if head_on{self.locked=false;}head_on
  }
 }
 pub fn recoil(&mut self){self.velocity=V(-self.velocity.0*0.3,2.,-self.velocity.2*0.3);self.locked=false;self.airborne=true;}
}
#[cfg(test)]mod tests{
 use super::*;
 fn run(hz:f32,drive:bool,normal:Option<V>)->Motion{let mut m=Motion{velocity:V(0.,0.,-4.),..Default::default()};for _ in 0..(hz*10.)as usize{assert!(m.advance(1./hz,drive,V(0.,0.,-1.),normal));}m}
 #[test]fn power_drag_and_frame_rate(){let a=run(30.,true,Some(V(0.,1.,0.)));let b=run(144.,true,Some(V(0.,1.,0.)));assert!((a.velocity.len()-b.velocity.len()).abs()<0.1);assert!((12. ..20.).contains(&a.velocity.len()));assert!(a.locked);assert!(run(60.,false,Some(V(0.,1.,0.))).velocity.len()<0.01);}
 #[test]fn downhill_gains_energy_uphill_loses_it(){let a=run(60.,true,Some(V(0.,0.95,-0.31)));let b=run(60.,true,Some(V(0.,0.95,0.31)));assert!(a.velocity.len()>b.velocity.len());}
 #[test]fn flight_has_no_drive_and_landing_loses_energy(){let mut a=Motion{velocity:V(8.,2.,0.),..Default::default()};let mut b=Motion{velocity:a.velocity,..Default::default()};a.advance(0.1,true,V(1.,0.,0.),None);b.advance(0.1,false,V(-1.,0.,0.),None);assert_eq!(a.velocity,b.velocity);a.velocity=V(10.,-12.,0.);let old=a.velocity.len();assert!(!a.collide(V(0.,1.,0.)));assert!(a.velocity.1>0.&&a.velocity.len()<old);}
 #[test]fn glancing_and_head_on_contacts(){let mut a=Motion{velocity:V(10.,0.,-2.),..Default::default()};assert!(!a.collide(V(0.,0.,1.)));assert!(a.velocity.0>0.&&a.velocity.2>0.);a.velocity=V(0.,0.,-15.);assert!(a.collide(V(0.,0.,1.)));assert!(a.velocity.2>0.);}
 #[test]fn release_hysteresis_and_invalid_time(){let mut m=Motion{velocity:V(0.,0.,-12.),..Default::default()};m.advance(0.01,false,V::default(),Some(V(0.,1.,0.)));assert!(m.locked);for _ in 0..600{m.advance(1./60.,false,V::default(),Some(V(0.,1.,0.)));}assert!(!m.locked);assert!(!m.advance(f32::NAN,true,V::default(),None));assert_eq!(m.velocity,V::default());}
}
