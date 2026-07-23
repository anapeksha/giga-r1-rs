use std::{env, fs, path::PathBuf};

fn main() {
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo always sets OUT_DIR"));
    fs::copy("memory.x", output.join("memory.x")).expect("copy M7 linker memory map");
    println!("cargo:rustc-link-search={}", output.display());
    println!("cargo:rerun-if-changed=memory.x");
}
