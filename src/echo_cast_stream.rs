use super::*;
use std::cell::RefCell;
use std::sync::atomic::AtomicUsize;

pub(super) struct Shared{address:AtomicUsize,base:usize}
impl Drop for Shared{fn drop(&mut self){unsafe{
 let mut descriptor=Descriptor([0;0x110]);ptr::write_unaligned(descriptor.0.as_mut_ptr().add(0x98)as*mut usize,self.address.load(Ordering::Acquire));
 let release:unsafe extern "system" fn(usize)=std::mem::transmute(self.base+RELEASE_RVA);release(descriptor.0.as_mut_ptr()as usize);
}}}
pub(super) unsafe fn seed(mut ticket:Ticket)->Arc<Ticket>{
 let mut descriptor=Descriptor([0;0x110]);descriptor.0[0x98..0xa0].copy_from_slice(&ticket.descriptor.0[0x98..0xa0]);
 let release:unsafe extern "system" fn(usize)=std::mem::transmute(ticket.base+RELEASE_RVA);release(descriptor.0.as_mut_ptr()as usize);
 ticket.descriptor.0[0x98..0xa0].fill(0);
 ticket.shared=Some(Arc::new(Shared{address:AtomicUsize::new(0),base:ticket.base}));Arc::new(ticket)
}
pub(super) unsafe fn prepare(ticket:&mut Ticket){
 let Some(shared)=ticket.shared.as_ref()else{return;};let address=shared.address.load(Ordering::Acquire);if address==0{return;}
 retain_counter(address,0x18,0x3fffffff);ptr::write_unaligned(ticket.descriptor.0.as_mut_ptr().add(0x98)as*mut usize,address);
}
pub(super) unsafe fn created(ticket:&Ticket,group:usize){
 let Some(shared)=ticket.shared.as_ref()else{return;};let address=read_ptr(group+0x528);
 if address!=0&&shared.address.compare_exchange(0,address,Ordering::AcqRel,Ordering::Acquire).is_ok(){retain_counter(address,0x18,0x3fffffff);}
}

struct Group{address:usize,id:i32,seed:Arc<Ticket>,latest:Option<Shots>}
static GROUPS:Mutex<Vec<Group>>=Mutex::new(Vec::new());
thread_local!{static SEED:RefCell<Option<Arc<Ticket>>>=const{RefCell::new(None)};}
pub(super) struct Guard(Option<Arc<Ticket>>);
impl Drop for Guard{fn drop(&mut self){SEED.with(|seed|*seed.borrow_mut()=self.0.take());}}
pub(super) fn factory(seed:Option<Arc<Ticket>>)->Guard{Guard(SEED.with(|state|state.replace(seed)))}
pub(super) unsafe fn eligible(ticket:&Ticket,payload:usize)->bool{
 let descriptor=ticket.descriptor.0.as_ptr()as usize;
 root_magic(read_i32(descriptor+0x18),read_i32(descriptor+0x14)).is_some_and(|magic|crate::echo_cast_data::STREAMS.binary_search(&magic).is_ok())&&ptr::read((payload+0x164)as*const u8)==0
}
pub(super) fn volley(source:usize,owner:Owner)->Option<Arc<Volley>>{
 GROUPS.lock().ok()?.iter().filter(|group|group.seed.owner==owner).filter_map(|group|group.seed.volley.as_ref()).find(|volley|volley.source==source).cloned()
}
pub(super) unsafe fn group(address:usize,seed:Option<Arc<Ticket>>){
 if address==0{return;}
 let old=if let Ok(mut groups)=GROUPS.lock(){
  let old=groups.iter().position(|group|group.address==address).map(|index|groups.remove(index));
  if let Some(seed)=seed{groups.push(Group{address,id:read_i32(address),seed,latest:None});}old
 }else{None};drop(old);
}
pub(super) unsafe fn source(address:usize)->Option<Arc<Ticket>>{
 if !ACTIVE.load(Ordering::Acquire)||REPLAY.with(Cell::get).ticket!=0||address==0{return None;}
 GROUPS.lock().ok().and_then(|groups|groups.iter().find(|group|group.address==address&&group.id==read_i32(address)).map(|group|Arc::clone(&group.seed)))
  .or_else(||SEED.with(|seed|seed.borrow().clone()))
}
unsafe fn copy(seed:&Ticket,transform:usize)->Ticket{
 let mut descriptor=Box::new(Descriptor(seed.descriptor.0));
 descriptor.0[0xc0..0xc8].fill(0);
 let target_copy:unsafe extern "system" fn(usize,usize)->usize=std::mem::transmute(seed.base+TARGET_COPY_RVA);
 target_copy(descriptor.0.as_mut_ptr().add(0xb0)as usize,seed.descriptor.0.as_ptr().add(0xb0)as usize);
 retain_counter(read_ptr(descriptor.0.as_ptr()as usize+0x98),0x18,0x3fffffff);
 retain_counter(read_ptr(descriptor.0.as_ptr()as usize+0xa0),0x28,0x7fffffff);
 let matrix=ptr::read_unaligned(transform as*const[u8;0x30]);let mut pose=[0u8;0x40];
 for axis in 0..3{for component in 0..4{pose[component*16+axis*4..component*16+axis*4+4].copy_from_slice(&matrix[axis*16+component*4..axis*16+component*4+4]);}}
 pose[0x3c..0x40].copy_from_slice(&1f32.to_le_bytes());
 descriptor.0[0x50..0x90].copy_from_slice(&pose);
 let shots=Arc::new(Mutex::new(BTreeMap::from([(0,Transform(matrix))])));
 Ticket{descriptor,pose,base:seed.base,wait:0.5,area:false,shots,owner:seed.owner,volley:seed.volley.clone(),shared:seed.shared.clone()}
}
pub(super) unsafe fn initialized(address:usize,seed:Option<Arc<Ticket>>,transform:usize,index:u32){
 let Some(seed)=seed else{return;};if owner(seed.owner.actor,seed.base)!=Some(seed.owner){return;}
 if index!=0{
  let shots=GROUPS.lock().ok().and_then(|groups|groups.iter().find(|group|group.address==address&&group.id==read_i32(address)).and_then(|group|group.latest.clone()));
  if let Some(shots)=shots{if let Ok(mut shots)=shots.lock(){shots.insert(index,Transform(ptr::read_unaligned(transform as*const[u8;0x30])));}}return;
 }
 let ticket=copy(&seed,transform);
 if let Ok(mut groups)=GROUPS.lock(){if let Some(group)=groups.iter_mut().find(|group|group.address==address&&group.id==read_i32(address)){group.latest=Some(Arc::clone(&ticket.shots));}}
 enqueue(ticket,None);
}
pub(super) fn finish(address:usize){
 let removed=GROUPS.lock().ok().and_then(|mut groups|groups.iter().position(|group|group.address==address).map(|index|groups.remove(index)));drop(removed);
}
pub(super) fn update(owner:Option<Owner>){
 let removed=if let Ok(mut groups)=GROUPS.lock(){let(mut keep,mut removed)=(Vec::new(),Vec::new());for group in std::mem::take(&mut*groups){if Some(group.seed.owner)==owner{keep.push(group);}else{removed.push(group);}}*groups=keep;removed}else{Vec::new()};drop(removed);
}
