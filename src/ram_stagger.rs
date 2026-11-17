//! Confirm native ultimate stagger after HKS has selected an actual graph node.
use crate::{read_ptr,read_i32,has_effect};
pub const NODE_RVA:usize=0x16b03c0;
pub const NODE_PATTERN:&str="48 89 5C 24 08 48 89 74 24 10 48 89 7C 24 18 41 56 48 83 EC 20 48 8B F2 48 85 C9 74 64";
#[derive(Clone,Copy,Debug,PartialEq)]pub struct Expected{pub graph:usize,pub kind:u8}
const NAMES:[&[u8];6]=[b"DamageExtraLarge_CMSG\0",b"DamageSpecial_CMSG\0",b"Event20002_CMSG\0",b"Event20000_CMSG\0",b"Event20005_CMSG\0",b"DamageSpecial_CMSG\0"];
// Same graph used by hkbIsNodeActive: character+98, graph+E0 active nodes.
// Re-resolve through the current animation module on every observation.
pub(crate) unsafe fn graph(actor:usize,base:usize)->Option<usize>{
 if actor==0{return None;}let modules=read_ptr(actor+0x1b8);if modules==0{return None;}
 let anim=read_ptr(modules+0x28);if anim==0||read_ptr(anim+8)!=actor{return None;}
 let instance=read_ptr(anim+0x10);if instance==0{return None;}
 let character=read_ptr(instance+0x30);if character==0||read_ptr(character)!=base+0x2e1a2b8{return None;}
 let graph=read_ptr(character+0x98);
 (graph!=0&&read_ptr(graph)==base+0x2df1098).then_some(graph)
}
unsafe fn active(expected:Expected,base:usize)->bool{
 let query:unsafe extern "system" fn(usize,*const u8)->u8=std::mem::transmute(base+NODE_RVA);
 query(expected.graph,NAMES[expected.kind as usize].as_ptr())!=0
}
pub unsafe fn prepare(actor:usize,base:usize)->Option<Expected>{
 // Native c9997 ExecSpDamage / IsSpDamageAllowed. Current invulnerability
 // and model-specific branches still have the final say when HKS runs.
 if (5750..=5752).any(|id|has_effect(actor,id,base)){return None;}
 let kind=if (has_effect(actor,45640,base)||has_effect(actor,45641,base))&&!has_effect(actor,45643,base){2}
 else if has_effect(actor,45866,base){1}else if has_effect(actor,46108,base){3}
 else if has_effect(actor,42055,base){4}else if has_effect(actor,46361,base){5}else{0};
 let expected=Expected{graph:graph(actor,base)?,kind};
 // Already staggering is not a new response to this hit. Unknown/custom
 // graph states fail closed to deflection, never manufacture self knockback.
 (!active(expected,base)).then_some(expected)
}
pub unsafe fn confirmed(actor:usize,expected:Expected,base:usize)->bool{
 let modules=read_ptr(actor+0x1b8);if modules==0{return false;}let stats=read_ptr(modules);
 stats!=0&&read_i32(stats+0x140)>0&&graph(actor,base)==Some(expected.graph)&&active(expected,base)
}
// Observe the native branch consuming behavior 12, not a coincidental later
// stagger from another attack. env's arguments are native call descriptors.
pub unsafe fn ultimate_query(context:usize,vm:usize,result:i32,base:usize){
 if context==0||vm==0||result==0{return;}
 let actor=read_ptr(context);
 if !crate::lightning_ram::awaiting_stagger(actor){return;}
 let number:unsafe extern "system" fn(usize,i32)->f32=std::mem::transmute(base+crate::ambulance::NUMBER_RVA);
 if number(vm,2)==12.{crate::lightning_ram::stagger_consumed(actor,read_ptr(actor+8)as u64);}
}
