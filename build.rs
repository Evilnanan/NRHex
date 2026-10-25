use std::{env, fs, path::PathBuf};

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    // Compatibility targets supply generated data, never a second runtime implementation.
    println!("cargo:rerun-if-env-changed=NRHEX_PARRY_DATA");
    let parry = env::var_os("NRHEX_PARRY_DATA").map(PathBuf::from)
        .unwrap_or_else(|| root.join("src/parry_data.rs"));
    println!("cargo:rerun-if-changed={}", parry.display());
    fs::copy(&parry, PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("parry_data.rs"))
        .expect("Cannot stage generated parry data");
    println!("cargo:rerun-if-env-changed=NRHEX_SWORD_DATA");
    let sword = env::var_os("NRHEX_SWORD_DATA").map(PathBuf::from)
        .unwrap_or_else(|| root.join("src/sword_map.rs"));
    println!("cargo:rerun-if-changed={}", sword.display());
    fs::copy(&sword, PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("sword_map.rs"))
        .expect("Cannot stage automatically estimated Sword data");
    println!("cargo:rerun-if-env-changed=NRHEX_ECHO_DATA");
    let echo = env::var_os("NRHEX_ECHO_DATA").map(PathBuf::from)
        .unwrap_or_else(|| root.join("src/echo_cast_data.rs"));
    println!("cargo:rerun-if-changed={}", echo.display());
    fs::copy(&echo, PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("echo_cast_data.rs"))
        .expect("Cannot stage reviewed Echo data");
}
