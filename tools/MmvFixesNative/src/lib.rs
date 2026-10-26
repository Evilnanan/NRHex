//! Independent MMV fixes. No dependency on the NRHex DLL or its gameplay state.
#![allow(non_snake_case)]
use std::{ffi::c_void, io::Write, path::PathBuf, ptr, sync::{OnceLock, atomic::{AtomicUsize, Ordering}}};
#[path = "../../../src/win.rs"] mod win;
#[path = "../../../src/hook.rs"] mod hook;
use win::*;

static LOG: OnceLock<PathBuf> = OnceLock::new();
static BASE: AtomicUsize = AtomicUsize::new(0);
static RECOVERED: AtomicUsize = AtomicUsize::new(0);
fn log(s: &str) {
    if let Some(path) = LOG.get() {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) { let _ = writeln!(f, "{s}"); }
    }
}
#[macro_export] macro_rules! nlog_debug { ($($x:tt)*) => { $crate::log(&format!($($x)*)) }; }
#[macro_export] macro_rules! nlog_warn { ($($x:tt)*) => { $crate::log(&format!($($x)*)) }; }
#[macro_export] macro_rules! nlog_error { ($($x:tt)*) => { $crate::log(&format!($($x)*)) }; }

const SELECT: usize = 0x6bde90;
const GET_DEFINE: usize = 0xe80650;
const CALL: usize = 0x6bd123;
const EXPECTED: &[u8] = &[0xe8,0x68,0x0d,0,0,0x84,0xc0,0x0f,0x84,0xef,0,0,0];
type Select = unsafe extern "system" fn(*mut i32, i32, *const u8, *const u8) -> u8;
#[repr(C)] struct Param { id: i32, pad: u32, data: *const u8 }
type GetDefine = unsafe extern "system" fn(*mut Param, i32);

fn candidates(map: u16) -> Option<([i32; 2], i16)> {
    match map { 5000..=5006 => Some(([115,2115],77)), 5010..=5011 => Some(([116,2116],78)), _ => None }
}
fn matches_definition(data: &[u8], map: u16, icon: i16) -> bool {
    data.len() >= 0x2e && i16::from_le_bytes([data[0x2c],data[0x2d]]) == icon &&
        data[4..40].chunks_exact(2).any(|v| u16::from_le_bytes([v[0],v[1]]) == map)
}
unsafe extern "system" fn select_map_icon(out: *mut i32, pattern: i32, cache: *const u8, entry: *const u8) -> u8 {
    let base = BASE.load(Ordering::Acquire);
    let original: Select = std::mem::transmute(base + SELECT);
    let result = original(out, pattern, cache, entry);
    if result != 0 || out.is_null() || entry.is_null() || cache.is_null() { return result; }
    let map = ptr::read_unaligned(entry.add(4).cast::<u16>());
    let Some((ids, icon)) = candidates(map) else { return result; };
    let get: GetDefine = std::mem::transmute(base + GET_DEFINE);
    for id in ids {
        let mut row = Param { id: -1, pad: 0, data: ptr::null() };
        get(&mut row, id);
        if !row.data.is_null() && row.id == id && matches_definition(std::slice::from_raw_parts(row.data,0x2e),map,icon) {
            *out = id;
            // One startup diagnostic, not per-marker or per-frame logging.
            if RECOVERED.fetch_add(1,Ordering::Relaxed) == 0 { log(&format!("Recovered DLC map icon definition: pattern={pattern} map={map} definition={id}")); }
            return 1;
        }
    }
    result
}

unsafe fn install() -> Result<(), String> {
    let base = GetModuleHandleW(ptr::null()) as usize;
    if base == 0 { return Err("No game module".into()); }
    let pe = base + ptr::read_unaligned((base+0x3c) as *const u32) as usize;
    if ptr::read_unaligned((pe+8) as *const u32) != 0x69e0dc2a || ptr::read_unaligned((pe+0x50) as *const u32) != 0x7c5d600 {
        return Err("Unsupported EXE; map fix disabled".into());
    }
    if std::slice::from_raw_parts((base+CALL) as *const u8,EXPECTED.len()) != EXPECTED ||
       std::slice::from_raw_parts((base+GET_DEFINE) as *const u8,4) != [0x85,0xd2,0x0f,0x88] {
        return Err("Native signature changed; map fix disabled".into());
    }
    // Redirect only the icon-cache builder's CALL. The selector's other users,
    // slot permissions, placement rules and successful native results are untouched.
    let target = base+CALL;
    let mut relay = ptr::null_mut();
    for step in 1..0x4000usize {
        for candidate in [(target & !0xffff).checked_add(step*0x10000),(target & !0xffff).checked_sub(step*0x10000)].into_iter().flatten() {
            relay = VirtualAlloc(candidate as *mut c_void,4096,MEM_COMMIT|MEM_RESERVE,PAGE_READWRITE);
            if !relay.is_null() { break; }
        }
        if !relay.is_null() { break; }
    }
    if relay.is_null() { return Err("Cannot allocate nearby relay".into()); }
    let mut code = vec![0xff,0x25,0,0,0,0];
    code.extend_from_slice(&(select_map_icon as *const () as usize).to_le_bytes());
    ptr::copy_nonoverlapping(code.as_ptr(),relay.cast(),code.len());
    let mut old=0;
    if VirtualProtect(relay,4096,PAGE_EXECUTE_READ,&mut old)==0 { VirtualFree(relay,0,MEM_RELEASE); return Err("Relay protection failed".into()); }
    FlushInstructionCache(GetCurrentProcess(),relay,code.len());
    let displacement = i32::try_from(relay as isize - (target+5) as isize).map_err(|_| "Relay out of range")?;
    let mut patch=vec![0xe8];patch.extend_from_slice(&displacement.to_le_bytes());
    BASE.store(base,Ordering::Release);
    let mut result=Err("Patch not attempted".into());
    hook::with_threads_frozen(true, || {
        if std::slice::from_raw_parts(target as *const u8,EXPECTED.len()) == EXPECTED {
            result=hook::write_code(target as *mut u8,&patch);
        } else { result=Err("Call changed during installation".into()); }
    });
    if result.is_err() { VirtualFree(relay,0,MEM_RELEASE); }
    result?;
    log("MMV-Fixes map icon fallback ACTIVE; icon-cache call only");
    Ok(())
}

unsafe extern "system" fn init(module: *mut c_void) -> u32 {
    let mut path=[0u16;32768];let n=GetModuleFileNameW(module,path.as_mut_ptr(),path.len() as u32) as usize;
    if n>0 && n<path.len() { let _=LOG.set(PathBuf::from(String::from_utf16_lossy(&path[..n])).with_extension("log")); }
    if let Err(e)=install() { log(&e); }
    0
}
#[no_mangle]
pub unsafe extern "system" fn DllMain(module: HMODULE, reason: u32, _: *mut c_void) -> i32 {
    if reason==DLL_PROCESS_ATTACH {
        DisableThreadLibraryCalls(module);
        let thread=CreateThread(ptr::null_mut(),0,init,module,0,ptr::null_mut());
        if !thread.is_null() { CloseHandle(thread); }
    }
    1
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn only_matching_dlc_definition_is_accepted() {
        let mut row=[0u8;0x64];row[4..6].copy_from_slice(&5002u16.to_le_bytes());row[0x2c..0x2e].copy_from_slice(&77i16.to_le_bytes());
        assert!(matches_definition(&row,5002,77));
        assert!(!matches_definition(&row,5011,77));
        assert!(!matches_definition(&row,5002,78));
        for map in [3000,4999,5007,5008,5009,5012,65535] { assert!(candidates(map).is_none()); }
        assert_eq!(candidates(5002),Some(([115,2115],77)));
        assert_eq!(candidates(5011),Some(([116,2116],78)));
    }
}
