use std::process::Command;
use std::path::Path;
use std::fs::DirEntry;

fn main() {
    let out_dir = Path::new("./allocators/target");
    if !out_dir.exists() {
        std::fs::create_dir_all(out_dir);
    }

    //println!("cargo:rerun-if-changed=allocators/*");
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
        .arg("./allocators/target/")
        .status()
        .unwrap();

    println!("cargo:warning=Creating lrmalloc.rs");
    Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg("allocators/lrmalloc.rs/lrmalloc-rs-global/Cargo.toml")
        .status()
        .unwrap();

}