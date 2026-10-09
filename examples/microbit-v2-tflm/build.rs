use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CPU_FLAGS: [&str; 4] = [
    "-mcpu=cortex-m4",
    "-mthumb",
    "-mfloat-abi=hard",
    "-mfpu=fpv4-sp-d16",
];

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    fs::copy("memory.x", out.join("memory.x")).expect("copy memory.x");
    println!("cargo:rustc-link-search={}", out.display());

    compile_tflm();
    link_newlib();

    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=build.rs");
}

/// Compile the TFLite-Micro sources and the shim with arm-none-eabi-g++.
/// The flags match the IREE runtime build: -Os, hard float, no PIC.
fn compile_tflm() {
    let tree = Path::new("tflm-tree");
    assert!(
        tree.is_dir(),
        "tflm-tree/ not found: run scripts/fetch-tflm.sh first"
    );
    let list_path = "scripts/tflm-sources.txt";
    let list = fs::read_to_string(list_path).expect("read the source list");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .cpp_link_stdlib(None)
        .compiler("arm-none-eabi-g++")
        .archiver("arm-none-eabi-ar")
        .target("thumbv7em-none-eabihf")
        .opt_level_str("s")
        .debug(false)
        .warnings(false)
        .include(tree)
        .include(tree.join("third_party/flatbuffers/include"))
        .include("src")
        .define("TF_LITE_STRIP_ERROR_STRINGS", None)
        .define("TF_LITE_DISABLE_X86_NEON", None)
        .define("NDEBUG", None)
        .flag("-std=c++17")
        .flag("-fno-rtti")
        .flag("-fno-exceptions")
        .flag("-fno-threadsafe-statics")
        .flag("-fno-unwind-tables")
        .flag("-fno-asynchronous-unwind-tables")
        .flag("-fno-pic")
        .flag("-fno-pie")
        .flag("-fomit-frame-pointer")
        .flag("-funsigned-char")
        .flag("-ffunction-sections")
        .flag("-fdata-sections");
    for flag in CPU_FLAGS {
        build.flag(flag);
    }
    for line in list.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        build.file(tree.join(line));
        println!("cargo:rerun-if-changed={}", tree.join(line).display());
    }
    build
        .file("src/shim.cc")
        .file("src/platform.cc")
        .file("src/diag.cc");
    build.compile("tflm");

    println!("cargo:rerun-if-changed={list_path}");
    println!("cargo:rerun-if-changed=src/shim.cc");
    println!("cargo:rerun-if-changed=src/shim.h");
    println!("cargo:rerun-if-changed=src/platform.cc");
    println!("cargo:rerun-if-changed=src/diag.cc");
}

/// Ask arm-none-eabi-gcc for a library path; the CPU flags pick the hard-float
/// multilib variant.
fn gcc_path(query: &str) -> PathBuf {
    let output = Command::new("arm-none-eabi-gcc")
        .args(CPU_FLAGS)
        .arg(query)
        .output()
        .expect("arm-none-eabi-gcc not found");
    assert!(output.status.success(), "arm-none-eabi-gcc {query} failed");
    PathBuf::from(String::from_utf8_lossy(&output.stdout).trim())
}

/// TFLite-Micro references newlib C functions (memcpy, memset and similar),
/// and the C++ code references libgcc helpers.
fn link_newlib() {
    for (query, name) in [
        ("-print-file-name=libc.a", "c"),
        ("-print-file-name=libm.a", "m"),
        ("-print-file-name=libnosys.a", "nosys"),
        ("-print-libgcc-file-name", "gcc"),
    ] {
        let library = gcc_path(query);
        let dir = library.parent().expect("library has a parent directory");
        println!("cargo:rustc-link-search=native={}", dir.display());
        println!("cargo:rustc-link-lib=static={name}");
    }
}
