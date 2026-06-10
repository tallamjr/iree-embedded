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

    compile_frontend();
    link_models(&out);
    link_newlib();

    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=csrc/kws_frontend.c");
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
        .status()
        .expect("arm-none-eabi-ar not found");
    assert!(status.success(), "archiving model objects failed");
    println!("cargo:rustc-link-lib=static=kws_models");
    println!("cargo:rerun-if-changed=models/simple_mul.o");
    println!("cargo:rerun-if-changed=models/micro_speech.o");
}

/// Compile the vendored TFLite-Micro audio front end (+ int16 kissfft) and the
/// C shim. C files via gcc, C++ files via g++.
fn compile_frontend() {
    let mf = "vendor/tensorflow/lite/experimental/microfrontend/lib";
    let cpu = [
        "-mcpu=cortex-m4",
        "-mthumb",
        "-mfloat-abi=hard",
        "-mfpu=fpv4-sp-d16",
    ];

    let mut c = cc::Build::new();
    c.compiler("arm-none-eabi-gcc");
    for f in [
        "frontend",
        "frontend_util",
        "window",
        "window_util",
        "filterbank",
        "filterbank_util",
        "noise_reduction",
        "noise_reduction_util",
        "pcan_gain_control",
        "pcan_gain_control_util",
        "log_scale",
        "log_scale_util",
        "log_lut",
    ] {
        c.file(format!("{mf}/{f}.c"));
    }
    c.file("csrc/kws_frontend.c");
    c.include("vendor").include("vendor/kissfft");
    for flag in cpu {
        c.flag(flag);
    }
    // The profile's -Oz makes the FFT-heavy front end ~2x slower; speed is
    // what keeps the streaming loop inside its 250 ms real-time budget.
    c.opt_level(3);
    c.pic(false).warnings(false);
    c.compile("kws_frontend_c");

    let mut cxx = cc::Build::new();
    cxx.cpp(true).compiler("arm-none-eabi-g++");
    for f in ["fft", "fft_util", "kiss_fft_int16"] {
        cxx.file(format!("{mf}/{f}.cc"));
    }
    cxx.include("vendor").include("vendor/kissfft");
    for flag in cpu {
        cxx.flag(flag);
    }
    cxx.opt_level(3);
    cxx.flag("-fno-exceptions")
        .flag("-fno-rtti")
        .pic(false)
        .warnings(false);
    cxx.compile("kws_frontend_cxx");

    println!("cargo:rerun-if-changed=vendor");
}

/// IREE and the front end reference newlib C functions; link the hard-float
/// multilib newlib (the CPU flags select the correct variant).
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
