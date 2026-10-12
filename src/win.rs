//! Minimal hand-rolled Win32 bindings.
//!
//! Deliberately dependency free: a game mod DLL should not drag a large crate
//! tree (and its own copies of kernel32 wrappers) into the game process.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use core::ffi::c_void;

pub type BOOL = i32;
pub type HANDLE = *mut c_void;
pub type HMODULE = *mut c_void;
pub type HINSTANCE = *mut c_void;
pub type DWORD = u32;
pub type LPVOID = *mut c_void;
pub type SIZE_T = usize;

pub const DLL_PROCESS_ATTACH: u32 = 1;
pub const DLL_PROCESS_DETACH: u32 = 0;
pub const DLL_THREAD_ATTACH: u32 = 2;
pub const DLL_THREAD_DETACH: u32 = 3;

pub const MEM_COMMIT: u32 = 0x0000_1000;
pub const MEM_RESERVE: u32 = 0x0000_2000;
pub const MEM_RELEASE: u32 = 0x0000_8000;
pub const PAGE_NOACCESS: u32 = 0x01;
pub const PAGE_READWRITE: u32 = 0x04;
pub const PAGE_EXECUTE_READ: u32 = 0x20;
pub const PAGE_EXECUTE_READWRITE: u32 = 0x40;

pub const TH32CS_SNAPTHREAD: u32 = 0x0000_0004;
pub const THREAD_SUSPEND_RESUME: u32 = 0x0002;

pub const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: u32 = 0x0000_0004;
pub const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: u32 = 0x0000_0002;

pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;
pub const TRUE: BOOL = 1;
pub const FALSE: BOOL = 0;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct THREADENTRY32 {
    pub dwSize: DWORD,
    pub cntUsage: DWORD,
    pub th32ThreadID: DWORD,
    pub th32OwnerProcessID: DWORD,
    pub tpBasePri: i32,
    pub tpDeltaPri: i32,
    pub dwFlags: DWORD,
}

impl Default for THREADENTRY32 {
    fn default() -> Self {
        Self {
            dwSize: core::mem::size_of::<THREADENTRY32>() as DWORD,
            cntUsage: 0,
            th32ThreadID: 0,
            th32OwnerProcessID: 0,
            tpBasePri: 0,
            tpDeltaPri: 0,
            dwFlags: 0,
        }
    }
}

pub type LPTHREAD_START_ROUTINE = unsafe extern "system" fn(LPVOID) -> DWORD;

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(lpModuleName: *const u16) -> HMODULE;
    pub fn LoadLibraryW(lpLibFileName: *const u16) -> HMODULE;
    pub fn FreeLibrary(hLibModule: HMODULE) -> BOOL;
    pub fn GetModuleHandleExW(
        dwFlags: DWORD,
        lpModuleName: *const u16,
        phModule: *mut HMODULE,
    ) -> BOOL;
    pub fn GetModuleFileNameW(hModule: HMODULE, lpFilename: *mut u16, nSize: DWORD) -> DWORD;
    pub fn GetProcAddress(hModule: HMODULE, lpProcName: *const u8) -> *mut c_void;

    pub fn VirtualAlloc(
        lpAddress: LPVOID,
        dwSize: SIZE_T,
        flAllocationType: DWORD,
        flProtect: DWORD,
    ) -> LPVOID;
    pub fn VirtualFree(lpAddress: LPVOID, dwSize: SIZE_T, dwFreeType: DWORD) -> BOOL;
    pub fn VirtualProtect(
        lpAddress: LPVOID,
        dwSize: SIZE_T,
        flNewProtect: DWORD,
        lpflOldProtect: *mut DWORD,
    ) -> BOOL;
    pub fn FlushInstructionCache(
        hProcess: HANDLE,
        lpBaseAddress: *const c_void,
        dwSize: SIZE_T,
    ) -> BOOL;

    pub fn GetCurrentProcess() -> HANDLE;
    pub fn GetCurrentProcessId() -> DWORD;
    pub fn GetCurrentThreadId() -> DWORD;

    pub fn CreateThread(
        lpThreadAttributes: *mut c_void,
        dwStackSize: SIZE_T,
        lpStartAddress: LPTHREAD_START_ROUTINE,
        lpParameter: LPVOID,
        dwCreationFlags: DWORD,
        lpThreadId: *mut DWORD,
    ) -> HANDLE;

    pub fn Sleep(dwMilliseconds: DWORD);
    pub fn DisableThreadLibraryCalls(hLibModule: HMODULE) -> BOOL;

    pub fn CreateToolhelp32Snapshot(dwFlags: DWORD, th32ProcessID: DWORD) -> HANDLE;
    pub fn Thread32First(hSnapshot: HANDLE, lpte: *mut THREADENTRY32) -> BOOL;
    pub fn Thread32Next(hSnapshot: HANDLE, lpte: *mut THREADENTRY32) -> BOOL;
    pub fn OpenThread(dwDesiredAccess: DWORD, bInheritHandle: BOOL, dwThreadId: DWORD) -> HANDLE;
    pub fn SuspendThread(hThread: HANDLE) -> DWORD;
    pub fn ResumeThread(hThread: HANDLE) -> DWORD;
    pub fn CloseHandle(hObject: HANDLE) -> BOOL;

}

/// Encode a Rust string as a NUL-terminated UTF-16 buffer.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(core::iter::once(0)).collect()
}

/// Read a NUL-terminated UTF-16 string from a fixed buffer into a `String`.
pub fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}
