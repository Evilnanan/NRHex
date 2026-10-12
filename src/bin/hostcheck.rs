//! Load the actual DLL in a disposable process to check fail-closed bootstrap.
use std::{fs,path::PathBuf,time::{Instant,Duration},thread};
use NRHex::win;
fn main(){
 let root=PathBuf::from(env!("CARGO_MANIFEST_DIR"));
 let dir=root.join("research/hostcheck");fs::create_dir_all(&dir).unwrap();
 fs::copy(root.join("target/release/NRHex.dll"),dir.join("NRHex.dll")).unwrap();
 fs::copy(root.join("dist/mod/natives/NRHex.ini"),dir.join("NRHex.ini")).unwrap();
 fs::write(dir.join("NRHex.log"),"").unwrap();
 let dll=unsafe{win::LoadLibraryW(win::wide(&dir.join("NRHex.dll").to_string_lossy()).as_ptr())};
 assert!(!dll.is_null(),"DLL failed to load");
 let start=Instant::now();
 loop {
  let log=fs::read_to_string(dir.join("NRHex.log")).unwrap_or_default();
  if log.contains("Unsupported EXE SHA256") {break;}
  assert!(start.elapsed()<Duration::from_secs(10),"Expected unsupported-host guard was not reached: {log}");
  thread::sleep(Duration::from_millis(50));
 }
 let symbol=unsafe{win::GetProcAddress(dll,b"NRHex_status\0".as_ptr())};assert!(!symbol.is_null());
 let status:extern "system" fn()->u32=unsafe{std::mem::transmute(symbol)};
 assert_eq!(status(),0,"Unsupported host installed a hook");
 println!("PASS: actual DLL loaded, config parsed, unsupported EXE rejected, no hook installed.");
 // Process exit owns cleanup; do not unload while the bootstrap thread is returning.
}
