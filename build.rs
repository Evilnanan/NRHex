use std::{env, fs, path::PathBuf};

#[path = "src/sword_csv.rs"]
mod sword_csv;
#[allow(dead_code)]
#[path = "src/sword_map.rs"]
mod sword_map;

fn main() {
    for path in ["data/balance/sword.csv", "src/sword_csv.rs", "src/sword_map.rs"] {
        println!("cargo:rerun-if-changed={path}");
    }
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let csv = fs::read_to_string(root.join("data/balance/sword.csv"))
        .expect("Cannot read build input data/balance/sword.csv");
    let bonuses = sword_csv::read(&csv, sword_map::DEFAULT_BONUSES.iter().map(|&(id, _)| id))
        .unwrap_or_else(|error| panic!("Invalid build input data/balance/sword.csv: {error}"));
    let mut source = String::from("// Generated at compile time from the preserved user sword.csv.\nconst COMPILED_BONUSES: &[(i32, f32)] = &[\n");
    for (id, percentage) in bonuses {
        // Round-trip the parsed f32 exactly; no build-time change to user values.
        source.push_str(&format!("    ({id}, {percentage:?}f32),\n"));
    }
    source.push_str("];\n");
    fs::write(PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("sword_bonuses.rs"), source)
        .expect("Cannot write compiled sword bonuses");
}
