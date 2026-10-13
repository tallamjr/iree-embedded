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

    emit_arena_bytes();
    link_models(&out);
    link_newlib();

    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=build.rs");
}

/// Pass the arena size to the crate. `IREE_ARENA_BYTES` sets it at build time;
/// the default is 56 KiB.
fn emit_arena_bytes() {
    println!("cargo:rerun-if-env-changed=IREE_ARENA_BYTES");
    let arena_bytes = match env::var("IREE_ARENA_BYTES") {
        Ok(text) => text
            .parse::<usize>()
            .unwrap_or_else(|error| panic!("IREE_ARENA_BYTES {text:?}: {error}")),
        Err(env::VarError::NotPresent) => 56 * 1024,
        Err(error) => panic!("IREE_ARENA_BYTES: {error}"),
    };
    println!("cargo:rustc-env=IREE_ARENA_BYTES_RESOLVED={arena_bytes}");
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
        .arg("models/simple_mul.o")
        .arg("models/micro_speech.o")
        .arg("models/micro_speech_int8.o")
        .status()
        .expect("arm-none-eabi-ar not found");
    assert!(status.success(), "archiving model objects failed");
    println!("cargo:rustc-link-lib=static=kws_models");
    println!("cargo:rerun-if-changed=models/simple_mul.o");
    println!("cargo:rerun-if-changed=models/micro_speech.o");
    println!("cargo:rerun-if-changed=models/micro_speech_int8.o");
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
