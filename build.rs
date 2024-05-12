use std::{fs, path::Path};

fn main() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let cores = std::thread::available_parallelism().unwrap().get();

    fs::write(Path::new(&out_dir).join("core_count"), cores.to_le_bytes()).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}
