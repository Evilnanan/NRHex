//! A minimal, self-contained x86-64 inline-hook engine.
//!
//! Strategy (same shape as the upstream C++ mod, but implemented in Rust):
//!
//! 1. decode whole instructions at the target until at least 5 bytes are
//!    covered (a `E9 rel32` needs 5 bytes),
//! 2. allocate a trampoline within ±2 GiB of the target and let `iced-x86`'s
//!    block encoder relocate the stolen instructions there (RIP-relative
//!    operands and direct branches are rewritten),
//! 3. append an absolute `jmp [rip+0]` back to `target + stolen`,
//! 4. suspend every other thread in the process and write `E9 rel32` (plus NOP
//!    padding) over the target, restoring the original page protection.
//!
//! The trampoline is a drop-in replacement for the original function: calling
//! it runs the stolen prologue and then continues inside the original body.

#![allow(dead_code)]

use core::arch::asm;
use core::ffi::c_void;
use core::ptr;

use iced_x86::{
    BlockEncoder, BlockEncoderOptions, Decoder, DecoderOptions, Instruction, InstructionBlock,
    Mnemonic,
};

use crate::win::*;
use crate::{nlog_debug as debug, nlog_error as error, nlog_warn as warn};

pub const JMP_SIZE: usize = 5;
/// How many bytes we are willing to decode while looking for an instruction
/// boundary that covers `JMP_SIZE`.
const MAX_PROLOGUE_SCAN: usize = 64;
/// Public alias used by callers that want to bound the read to the end of an
/// executable section.
pub const DEFAULT_MAX_PROLOGUE: usize = MAX_PROLOGUE_SCAN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hook {
    /// Address of the patched game function.
    pub target: *mut u8,
    /// Executable thunk holding the relocated prologue; call it to reach the
    /// original function.
    pub trampoline: *mut u8,
    /// Number of bytes overwritten at `target`.
    pub stolen: usize,
    /// Set when a relay thunk was needed because the detour was out of `rel32`
    /// range (only possible for very large address-space gaps).
    pub relay: *mut u8,
}

/// Everything the installer needs to know before it touches game memory.
pub struct PreparedHook {
    pub instrs: Vec<Instruction>,
    pub stolen: usize,
    pub trampoline: *mut u8,
    pub code: Vec<u8>,
    pub patch: Vec<u8>,
    pub relay: *mut u8,
}

unsafe fn alloc_exec(size: usize) -> *mut u8 {
    VirtualAlloc(
        ptr::null_mut(),
        size,
        MEM_COMMIT | MEM_RESERVE,
        PAGE_EXECUTE_READWRITE,
    ) as *mut u8
}

/// Allocate executable memory within ±2 GiB of `near`, so that `rel32`
/// branches inside the trampoline stay encodable.
unsafe fn alloc_exec_near(near: usize, size: usize) -> *mut u8 {
    const GRANULARITY: usize = 0x1_0000;
    const MAX_DISTANCE: usize = 0x400_0000; // 64 MiB
    let base = near & !(GRANULARITY - 1);
    let steps = MAX_DISTANCE / GRANULARITY;

    for i in 1..=steps {
        let up = base.saturating_add(i * GRANULARITY);
        if up > 0x0000_7FFF_FFFF_0000 {
            break;
        }
        let p = VirtualAlloc(
            up as *mut c_void,
            size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );
        if !p.is_null() {
            return p as *mut u8;
        }
        if let Some(down) = base.checked_sub(i * GRANULARITY) {
            if down > 0x1_0000 {
                let p = VirtualAlloc(
                    down as *mut c_void,
                    size,
                    MEM_COMMIT | MEM_RESERVE,
                    PAGE_EXECUTE_READWRITE,
                );
                if !p.is_null() {
                    return p as *mut u8;
                }
            }
        }
    }

    // Last resort: anywhere the OS is willing to put it.  The trampoline's
    // final jump is absolute, so only intra-trampoline branches could suffer.
    alloc_exec(size)
}

/// Decode instructions from `target` until at least `JMP_SIZE` bytes are
/// covered.  `max_bytes` bounds the read so a function sitting at the very end
/// of an executable section cannot walk off the mapping.
unsafe fn decode_prologue(target: *const u8, max_bytes: usize) -> Result<Vec<Instruction>, String> {
    let max_bytes = max_bytes.min(MAX_PROLOGUE_SCAN);
    if max_bytes < JMP_SIZE {
        return Err(format!(
            "only {max_bytes} bytes available at the target; need at least {JMP_SIZE}"
        ));
    }
    let bytes = core::slice::from_raw_parts(target, max_bytes);
    let mut decoder = Decoder::with_ip(64, bytes, target as u64, DecoderOptions::NONE);
    let mut out: Vec<Instruction> = Vec::new();
    let mut total = 0usize;
    while total < JMP_SIZE {
        if !decoder.can_decode() {
            return Err("ran out of bytes while decoding the prologue".into());
        }
        let ins = decoder.decode();
        if ins.is_invalid() {
            return Err(format!("invalid instruction at +{total:#x}"));
        }
        match ins.mnemonic() {
            Mnemonic::Ret | Mnemonic::Retf | Mnemonic::Int3 => {
                return Err(format!(
                    "hit a {:?} at +{total:#x} before covering {JMP_SIZE} bytes",
                    ins.mnemonic()
                ));
            }
            _ => {}
        }
        total += ins.len();
        out.push(ins);
    }
    Ok(out)
}

fn fits_rel32(from: usize, to: usize) -> bool {
    let delta = (to as i64) - (from as i64) - JMP_SIZE as i64;
    delta >= i32::MIN as i64 && delta <= i32::MAX as i64
}

unsafe fn make_relay(near: usize, detour: *const u8) -> Result<*mut u8, String> {
    let mem = alloc_exec_near(near, 16);
    if mem.is_null() {
        return Err("VirtualAlloc failed for the relay thunk".into());
    }
    let mut code = [0u8; 14];
    code[0] = 0xFF;
    code[1] = 0x25; // jmp qword ptr [rip+0]
    code[6..14].copy_from_slice(&(detour as u64).to_le_bytes());
    ptr::copy_nonoverlapping(code.as_ptr(), mem, code.len());
    FlushInstructionCache(GetCurrentProcess(), mem as *const c_void, code.len());
    Ok(mem)
}

/// Build everything that does *not* modify the target.  Doing the allocation,
/// decoding and encoding up front keeps the suspended-thread window tiny.
pub unsafe fn prepare(
    target: *mut u8,
    detour: *const u8,
    max_bytes: usize,
) -> Result<PreparedHook, String> {
    if target.is_null() || detour.is_null() {
        return Err("null target/detour".into());
    }

    let instrs = decode_prologue(target, max_bytes)?;
    let stolen: usize = instrs.iter().map(|i| i.len()).sum();

    let trampoline = alloc_exec_near(target as usize, stolen + 32);
    if trampoline.is_null() {
        return Err("VirtualAlloc failed for the trampoline".into());
    }

    let block = InstructionBlock::new(&instrs, trampoline as u64);
    let result = BlockEncoder::encode(64, block, BlockEncoderOptions::NONE)
        .map_err(|e| format!("trampoline block encode failed: {e}"))?;
    let mut code = result.code_buffer;

    // Absolute jump back to the instruction following the stolen region.
    let return_addr = (target as u64).wrapping_add(stolen as u64);
    code.extend_from_slice(&[0xFF, 0x25, 0x00, 0x00, 0x00, 0x00]);
    code.extend_from_slice(&return_addr.to_le_bytes());

    ptr::copy_nonoverlapping(code.as_ptr(), trampoline, code.len());
    FlushInstructionCache(GetCurrentProcess(), trampoline as *const c_void, code.len());

    // Build the patch bytes, adding a relay thunk when the detour is more than
    // ±2 GiB away (ASLR can place the mod DLL anywhere).
    let (jump_dest, relay) = if fits_rel32(target as usize, detour as usize) {
        (detour, ptr::null_mut())
    } else {
        let r = make_relay(target as usize, detour)?;
        debug!("detour {detour:p} out of rel32 range of {target:p}; using relay {r:p}");
        (r as *const u8, r)
    };

    let mut patch = vec![0x90u8; stolen];
    patch[0] = 0xE9;
    let rel = (jump_dest as i64) - (target as i64) - JMP_SIZE as i64;
    patch[1..5].copy_from_slice(&(rel as i32).to_le_bytes());

    Ok(PreparedHook {
        instrs,
        stolen,
        trampoline,
        code,
        patch,
        relay,
    })
}

struct SuspendedThreads(Vec<HANDLE>);

impl Drop for SuspendedThreads {
    fn drop(&mut self) {
        unsafe {
            for &h in &self.0 {
                ResumeThread(h);
                CloseHandle(h);
            }
        }
    }
}

/// Freeze every thread of this process except the caller.
unsafe fn suspend_other_threads() -> SuspendedThreads {
    let mut handles = Vec::new();
    let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
    if snapshot == INVALID_HANDLE_VALUE || snapshot.is_null() {
        warn!("CreateToolhelp32Snapshot failed; patching without freezing threads");
        return SuspendedThreads(handles);
    }

    let pid = GetCurrentProcessId();
    let me = GetCurrentThreadId();
    let mut entry = THREADENTRY32::default();
    let mut ok = Thread32First(snapshot, &mut entry) != 0;
    while ok {
        if entry.th32OwnerProcessID == pid && entry.th32ThreadID != me {
            let h = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID);
            if !h.is_null() && h != INVALID_HANDLE_VALUE {
                handles.push(h);
            }
        }
        ok = Thread32Next(snapshot, &mut entry) != 0;
    }
    CloseHandle(snapshot);
    handles.retain(|&handle| {
        if SuspendThread(handle) != u32::MAX {
            true
        } else {
            CloseHandle(handle);
            false
        }
    });
    SuspendedThreads(handles)
}

pub unsafe fn write_code(target: *mut u8, bytes: &[u8]) -> Result<(), String> {
    let mut old_protect = 0u32;
    if VirtualProtect(
        target as *mut c_void,
        bytes.len(),
        PAGE_EXECUTE_READWRITE,
        &mut old_protect,
    ) == 0
    {
        return Err("VirtualProtect(RWX) failed".into());
    }
    ptr::copy_nonoverlapping(bytes.as_ptr(), target, bytes.len());
    FlushInstructionCache(GetCurrentProcess(), target as *const c_void, bytes.len());
    let mut tmp = 0u32;
    VirtualProtect(target as *mut c_void, bytes.len(), old_protect, &mut tmp);
    Ok(())
}

/// Install a hook; returns the trampoline pointer (call it to reach the
/// original).
pub unsafe fn install(
    target: *mut u8,
    detour: *const u8,
    suspend_threads: bool,
    max_bytes: usize,
) -> Result<Hook, String> {
    let prepared = prepare(target, detour, max_bytes)?;

    let _guard = if suspend_threads {
        Some(suspend_other_threads())
    } else {
        None
    };
    write_code(target, &prepared.patch)?;
    drop(_guard);

    debug!(
        "hooked {target:p} -> {detour:p} ({} bytes stolen, trampoline {trampoline:p}, {} trampoline bytes)",
        prepared.stolen,
        prepared.code.len(),
        trampoline = prepared.trampoline
    );

    Ok(Hook {
        target,
        trampoline: prepared.trampoline,
        stolen: prepared.stolen,
        relay: prepared.relay,
    })
}

/// Compile-time assertion helper used by callers that only have a `fn` item.
pub fn fn_addr(f: unsafe extern "system" fn()) -> *const u8 {
    f as *const () as *const u8
}

/// Serialise the process' threads around an arbitrary critical section.
pub fn with_threads_frozen<F: FnOnce()>(enable: bool, f: F) {
    let _guard = if enable {
        Some(unsafe { suspend_other_threads() })
    } else {
        None
    };
    f();
}

#[inline(always)]
pub fn read_i32(addr: *const u8) -> i32 {
    unsafe { ptr::read_unaligned(addr as *const i32) }
}

/// Small helper so the detours can be tested without a live game process.
#[inline(always)]
pub fn spin_hint() {
    unsafe { asm!("pause", options(nomem, nostack, preserves_flags)) };
}

pub fn report_failure(what: &str, err: &str) {
    error!("{what}: {err}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}};

    #[test]
    #[ignore = "suspends this test process; run alone"]
    fn native_thread_freeze_resumes_large_thread_set() {
        let stop = Arc::new(AtomicBool::new(false));
        let ready = Arc::new(AtomicUsize::new(0));
        let workers: Vec<_> = (0..136).map(|_| {
            let stop = Arc::clone(&stop);
            let ready = Arc::clone(&ready);
            std::thread::spawn(move || {
                ready.fetch_add(1, Ordering::Release);
                while !stop.load(Ordering::Acquire) {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            })
        }).collect();
        while ready.load(Ordering::Acquire) != workers.len() {
            std::thread::yield_now();
        }
        let visits = AtomicUsize::new(0);
        for _ in 0..3 {
            with_threads_frozen(true, || { visits.fetch_add(1, Ordering::Relaxed); });
        }
        stop.store(true, Ordering::Release);
        for worker in workers { worker.join().unwrap(); }
        assert_eq!(visits.load(Ordering::Relaxed), 3);
    }
}
