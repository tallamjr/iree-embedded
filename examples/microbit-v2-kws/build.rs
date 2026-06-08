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

    // Non-PIC build, so any .got input sections (empty placeholders from
    // newlib) carry no entries; discard them so the linker need not place one.
    File::create(out.join("got.x"))
        .unwrap()
        .write_all(b"SECTIONS {\n  /DISCARD/ : { *(.got) *(.got.plt) }\n} INSERT AFTER .text;\n")
        .unwrap();

    // IREE's runtime references newlib C functions (abort, strtol, fprintf,
    // memchr, ...). Locate the hard-float multilib newlib and link it. The CPU
    // flags select the correct (cortex-m4f, hard-float) multilib variant.
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
    // Order matters: IREE libs (from the sys crate) reference these.
    println!("cargo:rustc-link-lib=static=c");
    println!("cargo:rustc-link-lib=static=m");
    println!("cargo:rustc-link-lib=static=nosys");

    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=build.rs");
}
