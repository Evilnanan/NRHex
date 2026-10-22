use super::*;
use std::cell::RefCell;

pub(super) struct Seed{descriptor:Descriptor,owner:Owner,base:usize,magic:i32}
impl Drop for Seed{fn drop(&mut self){unsafe{
 let release:unsafe extern "system" fn(usize)=std::mem::transmute(self.base+RELEASE_RVA);
 release(self.descriptor.0.as_mut_ptr()as usize);
}}}
struct Group{address:usize,id:i32,seed:Arc<Seed>}
#[derive(Default)]struct State{groups:Vec<Group>,instances:BTreeMap<usize,(u64,Arc<Seed>)>}
static STATE:Mutex<State>=Mutex::new(State{groups:Vec::new(),instances:BTreeMap::new()});
thread_local!{
 static SEED:RefCell<Option<Arc<Seed>>>=const{RefCell::new(None)};
 static CHILD:RefCell<Option<Arc<Seed>>>=const{RefCell::new(None)};
}
pub(super) struct SeedGuard(Option<Arc<Seed>>);
impl Drop for SeedGuard{fn drop(&mut self){SEED.with(|state|*state.borrow_mut()=self.0.take());}}
pub(crate) struct ChildGuard(Option<Arc<Seed>>);
impl Drop for ChildGuard{fn drop(&mut self){CHILD.with(|state|*state.borrow_mut()=self.0.take());}}
pub(super) fn magic(magic:i32,bullet:i32)->Option<i32>{
 crate::echo_cast_data::RELEASES.iter().find(|&&(spell,held,_)|bullet==held&&(magic==-1||magic==spell)).map(|row|row.0)
}
pub(super) fn released(magic:i32,bullet:i32)->Option<i32>{
 crate::echo_cast_data::RELEASES.iter().find(|&&(spell,_,flight)|bullet==flight&&(magic==-1||magic==spell)).map(|row|row.0)
}
pub(super) unsafe fn capture(scope:Scope,payload:usize,base:usize)->Option<Arc<Seed>>{
 if scope.descriptor==0||scope.consumed||payload==0||read_ptr(payload+0x10)as u64!=scope.owner.handle{return None;}
 let magic=magic(read_i32(scope.descriptor+0x18),read_i32(scope.descriptor+0x14))?;
 let mut descriptor=Descriptor(ptr::read_unaligned(scope.descriptor as*const[u8;0x110]));
 descriptor.0[0x90..0x98].fill(0);descriptor.0[0xc0..0xc8].fill(0);
 let copy:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(base+TARGET_COPY_RVA);
 copy(descriptor.0.as_mut_ptr().add(0xb0)as usize,scope.descriptor+0xb0);
 retain_counter(read_ptr(scope.descriptor+0x98),0x18,0x3fffffff);
 retain_counter(read_ptr(scope.descriptor+0xa0),0x28,0x7fffffff);
 Some(Arc::new(Seed{descriptor,owner:scope.owner,base,magic}))
}
pub(super) fn factory(seed:Option<Arc<Seed>>)->SeedGuard{SeedGuard(SEED.with(|state|state.replace(seed)))}
pub(super) unsafe fn group(address:usize,seed:Option<Arc<Seed>>){
 if address==0{return;}
 if let Ok(mut state)=STATE.lock(){
  state.groups.retain(|group|group.address!=address);
  if let Some(seed)=seed{state.groups.push(Group{address,id:read_i32(address),seed});}
 }
}
pub(super) unsafe fn initialize(address:usize)->SeedGuard{
 let seed=STATE.lock().ok().and_then(|state|state.groups.iter().find(|group|group.address==address&&group.id==read_i32(address)).map(|group|Arc::clone(&group.seed)))
  .or_else(||SEED.with(|state|state.borrow().clone()));
 factory(seed)
}
pub(crate) unsafe fn bound(instance:usize){
 if instance==0{return;}
 let seed=SEED.with(|state|state.borrow().clone());
 if let Ok(mut state)=STATE.lock(){
  state.instances.remove(&instance);
  if let Some(seed)=seed{state.instances.insert(instance,(ptr::read_unaligned((instance+8)as*const u64),seed));}
 }
}
pub(crate) unsafe fn child_scope(instance:usize,bullet:i32)->ChildGuard{
 let seed=if ACTIVE.load(Ordering::Acquire)&&instance!=0{
  STATE.lock().ok().and_then(|state|state.instances.get(&instance).filter(|(handle,seed)|*handle==ptr::read_unaligned((instance+8)as*const u64)&&released(seed.magic,bullet).is_some()).map(|(_,seed)|Arc::clone(seed)))
 }else{None};
 ChildGuard(CHILD.with(|state|state.replace(seed)))
}
pub(super) unsafe fn snapshot(payload:usize,base:usize)->Option<Ticket>{
 if payload==0{return None;}
 let seed=CHILD.with(|state|state.borrow().clone())?;
 if seed.base!=base||owner(seed.owner.actor,base)!=Some(seed.owner){return None;}
 let bullet=read_i32(payload+4);released(seed.magic,bullet)?;
 let mut descriptor=Descriptor(seed.descriptor.0);
 descriptor.0[8..12].copy_from_slice(&(-1i32).to_le_bytes());
 descriptor.0[0x14..0x18].copy_from_slice(&bullet.to_le_bytes());
 ptr::copy_nonoverlapping((payload+0x190)as*const u8,descriptor.0.as_mut_ptr().add(0x50),0x40);
 for (source,target)in [(0x1f8,0x98),(0x200,0xa0)]{ptr::write_unaligned(descriptor.0.as_mut_ptr().add(target)as*mut usize,read_ptr(payload+source));}
 super::snapshot(Scope{descriptor:descriptor.0.as_ptr()as usize,owner:seed.owner,consumed:false},payload,base)
}
pub(crate) fn reset(instance:usize){if let Ok(mut state)=STATE.lock(){state.instances.remove(&instance);}}
pub(super) fn finish(address:usize){if let Ok(mut state)=STATE.lock(){state.groups.retain(|group|group.address!=address);}}
pub(super) fn update(owner:Option<Owner>){if let Ok(mut state)=STATE.lock(){state.groups.retain(|group|Some(group.seed.owner)==owner);state.instances.retain(|_,(_,seed)|Some(seed.owner)==owner);}}
