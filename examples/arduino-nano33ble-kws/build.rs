use std::env;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // Make memory.x available to the cortex-m-rt linker script.
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    link_models(&out);
    link_newlib();

    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=build.rs");
}

/// Archive the iree-compile static-library kernel objects (real Cortex-M
/// machine code; see README phase 1) so the linker pulls in just the model the
/// firmware references via its `*_library_query` symbol.
fn link_models(out: &Path) {
    let lib = out.join("libkws_models.a");
    let _ = std::fs::remove_file(&lib);
    let status = Command::new("arm-none-eabi-ar")
        .arg("crs")
        .arg(&lib)
        .arg("models/micro_speech.o")
        .status()
        .expect("arm-none-eabi-ar not found");
    assert!(status.success(), "archiving model objects failed");
    println!("cargo:rustc-link-lib=static=kws_models");
    println!("cargo:rerun-if-changed=models/micro_speech.o");
}

/// IREE references newlib C functions (memcpy, malloc and friends); link the
/// hard-float multilib newlib (the CPU flags select the correct variant).
fn link_newlib() {
    let cpu = [
        "-mcpu=cortex-m4",
        "-mthumb",
        "-mfloat-abi=hard",
        "-mfpu=fpv4-sp-d16",
    ];
    let mut args: Vec<&str> = cpu.to_vec();
    args.push("-print-file-name=libc.a");
    let libc = Command::new("arm-none-eabi-gcc")
        .args(&args)
        .output()
        .expect("arm-none-eabi-gcc not found");
    let libc_path = String::from_utf8_lossy(&libc.stdout).trim().to_string();
    let lib_dir = Path::new(&libc_path)
        .parent()
        .expect("libc.a has no parent dir");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=c");
    println!("cargo:rustc-link-lib=static=m");
    println!("cargo:rustc-link-lib=static=nosys");
}
