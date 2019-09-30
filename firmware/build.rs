extern crate walkdir;

use std::env;
use std::io::Write;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

use walkdir::WalkDir;

fn linker_script() {
    // Put the linker script somewhere the linker can find it
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    println!("cargo:rerun-if-changed=memory.x");
}

fn main() {
    linker_script();
}
