use std::process::Command;
use std::path::{Path, PathBuf};

fn main() {
    let mut out_dir = PathBuf::from(env!("OUT_DIR"));
    out_dir.push("allocators");
    if !out_dir.exists() {
        std::fs::create_dir_all(out_dir).unwrap();
    }


    println!("cargo:rerun-if-changed=allocators/*");
    println!("cargo:warning=Creating jemalloc");
    if !Path::new("./allocators/jemalloc/Makefile").exists() {
        println!("cargo:warning=Configuring jemalloc...");
        Command::new("./configure")
            .current_dir("./allocators/jemalloc")
            .arg("--without-export")
            .arg("--disable-zone-allocator")
            .spawn()
            .unwrap();
    }
    Command::new("make")
        .current_dir("./allocators/jemalloc")
        .arg("build_lib_static")
        .status()
        .unwrap();
    Command::new("cp")
        .arg("./allocators/jemalloc/lib/libjemalloc.a")
        .arg(out_dir.to_str().unwrap())
        .status()
        .unwrap();

    println!("cargo:warning=Creating lrmalloc.rs");

    Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg("allocators/lrmalloc.rs/lrmalloc-rs-global/Cargo.toml")
        .status()
        .unwrap();
    Command::new("cp")
        .arg("allocators/lrmalloc.rs/target/debug/liblrmalloc_rs_global.a")
        .arg(out_dir.to_str().unwrap())
        .status()
        .unwrap();

}