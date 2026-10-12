fn main(){
 let path=std::env::args_os().nth(1).map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from(r"D:\Program Files (x86)\Steam\steamapps\common\ELDEN RING NIGHTREIGN\Game\nightreign.exe"));
 NRHex::verify_executable(&path).expect("EXE guard failed");
 println!("PASS: current EXE hash and runtime signatures are unique at the researched RVAs.");
}
