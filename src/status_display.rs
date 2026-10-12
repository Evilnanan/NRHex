//! Detailed status routing, independent of the native row's payload kind.
//! Type 0 is a live SpEffect; type 1 is an equipment AttachEffect. The game
//! normally puts those in Special Effects and Passive Effects respectively.
//! Keep type 0 intact so the common native row renderer still uses its own
//! effect name/info/count; only select a different destination for our states.
use std::sync::atomic::AtomicPtr;

pub const ROUTE_RVA: usize = 0xa89a93;
pub const ROUTE_PATTERN: &str = "66 0F 7E D0 83 F8 01 75 74 48 8D 54 24 20 48 8D 4C 24 50 E8 ? ? ? ? 4C 8B C0 48 8B 56 10";
pub static ORIGINAL: AtomicPtr<u8> = AtomicPtr::new(core::ptr::null_mut());
pub static CONTINUE: AtomicPtr<u8> = AtomicPtr::new(core::ptr::null_mut());

#[unsafe(naked)]
pub unsafe extern "system" fn route_detour() {
 core::arch::naked_asm!(
  "movd eax, xmm2", "test eax, eax", "jnz 4f",
  "push r10", "mov r10d, [rbx+4]",
  "cmp r10d, 95000403", "je 3f",
  "cmp r10d, 95000701", "je 3f",
  "cmp r10d, 95001203", "je 3f",
  "cmp r10d, 95000501", "je 3f",
  "cmp r10d, 95000801", "je 3f",
  "cmp r10d, 95001201", "je 3f",
  "cmp r10d, 95001202", "je 3f",
  "cmp r10d, 95005001", "jl 2f",
  "cmp r10d, 95005079", "jle 3f",
  "2:", "pop r10",
  "4:", "jmp qword ptr [rip+{original}]",
  // The following native JNE chooses the vector. Preserve EAX=0 and the
  // entire copied row; the constructor must still see a genuine SpEffect.
  "3:", "pop r10", "cmp eax, eax", "jmp qword ptr [rip+{continuation}]",
  original=sym ORIGINAL, continuation=sym CONTINUE
 );
}

#[cfg(test)]
mod tests {
 use super::*;
 use crate::win::*;
 use std::sync::atomic::Ordering;
 #[unsafe(naked)]
 unsafe extern "system" fn probe(_record:*const i32,_output:*mut u64) {
  core::arch::naked_asm!(
   "push rbx", "sub rsp, 0x20", "mov rbx, rcx",
   "movdqu xmm2, [rcx]", "mov r10, 0x123456789abcdef0",
   "call {route}", "mov [rdx], rax", "mov [rdx+8], r10",
   "pushfq", "pop rax", "mov [rdx+16], rax",
   "add rsp, 0x20", "pop rbx", "ret", route=sym route_detour
  );
 }
 #[test]
 fn live_native_branch_routes_only_states_and_preserves_payloads() {unsafe{
  // Exact 1.3.3 selector instructions, followed by a private RET. Use our
  // real relocation engine and real naked detour, without touching the game.
  let native=include_bytes!("../data/reference/ui/status-route.bin");
  assert_eq!(native,&[0x66,0x0f,0x7e,0xd0,0x83,0xf8,0x01]);
  let memory=VirtualAlloc(core::ptr::null_mut(),1024,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE) as *mut u8;
  assert!(!memory.is_null());core::ptr::copy_nonoverlapping(native.as_ptr(),memory,7);*memory.add(7)=0xc3;
  let prepared=crate::hook::prepare(memory,route_detour as *const u8,8).unwrap();
  assert_eq!(prepared.stolen,7);
  ORIGINAL.store(prepared.trampoline,Ordering::Release);CONTINUE.store(memory.add(7),Ordering::Release);
  let states=[95000403,95000701,95001203,95000501,95000801,95001201,95001202];
  let mut ids=states.to_vec();ids.extend(95005001..=95005079);
  ids.extend([95005000,95005080,95000500,95000700,95000400,95001200,95000800,8610700,8610702,9700,700600,-1]);
  for kind in [0,1,2,-1] {for &effect in &ids {
   let row=[kind,effect,20408,0,0x3f800000,7,123,456,789,0,8];let before=row;
   let mut output=[0u64;3];probe(row.as_ptr(),output.as_mut_ptr());
   let state=states.contains(&effect)||(95005001..=95005079).contains(&effect);
   let additional=kind==1||(kind==0&&state);
   assert_eq!(output[0],kind as u32 as u64,"row kind changed: {kind}/{effect}");
   assert_eq!(output[1],0x123456789abcdef0,"scratch register changed");
   assert_eq!(output[2]&0x40!=0,additional,"wrong destination: {kind}/{effect}");
   assert_eq!(row,before,"source record changed");
  }}
  assert_eq!(VirtualFree(memory as *mut _,0,MEM_RELEASE),1);
  assert_eq!(VirtualFree(prepared.trampoline as *mut _,0,MEM_RELEASE),1);
  if !prepared.relay.is_null(){assert_eq!(VirtualFree(prepared.relay as *mut _,0,MEM_RELEASE),1);}
 }}
}
