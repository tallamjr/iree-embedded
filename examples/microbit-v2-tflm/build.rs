use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Read a list file: one path per line, `#` starts a comment line.
fn read_list(path: &str) -> Vec<String> {
    println!("cargo:rerun-if-changed={path}");
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read {path}: {error}"))
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

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

    emit_arena_bytes();
    compile_tflm();
    link_newlib();

    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=build.rs");
}

/// Pass the arena size to the crate. `TFLM_ARENA_BYTES` sets it at build time;
/// the default is 16 KiB.
fn emit_arena_bytes() {
    println!("cargo:rerun-if-env-changed=TFLM_ARENA_BYTES");
    let arena_bytes = match env::var("TFLM_ARENA_BYTES") {
        Ok(text) => text
            .parse::<usize>()
            .unwrap_or_else(|error| panic!("TFLM_ARENA_BYTES {text:?}: {error}")),
        Err(env::VarError::NotPresent) => 16 * 1024,
        Err(error) => panic!("TFLM_ARENA_BYTES: {error}"),
    };
    println!("cargo:rustc-env=TFLM_ARENA_BYTES_RESOLVED={arena_bytes}");
}

/// Compile the TFLite-Micro sources and the shim with arm-none-eabi-g++.
/// The flags match the IREE runtime build: -Os, hard float, no PIC.
/// The `cmsis-nn` feature swaps the depthwise and fully connected kernels for
/// the CMSIS-NN ones and adds the CMSIS-NN C files.
fn compile_tflm() {
    let cmsis_nn = env::var_os("CARGO_FEATURE_CMSIS_NN").is_some();
    let tree = Path::new(if cmsis_nn {
        "tflm-tree-cmsis-nn"
    } else {
        "tflm-tree"
    });
    assert!(
        tree.is_dir(),
        "{} not found: run scripts/fetch-tflm.sh first",
        tree.display()
    );
    let kernel_list = if cmsis_nn {
        "scripts/tflm-kernels-cmsis-nn.txt"
    } else {
        "scripts/tflm-kernels-reference.txt"
    };

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
    if cmsis_nn {
        build
            .define("CMSIS_NN", None)
            .define("ARM_NN_ENABLE_F16", "0")
            .define("ARM_NN_ENABLE_F32", "0")
            .include(tree.join("third_party/cmsis_nn"))
            .include(tree.join("third_party/cmsis_nn/Include"))
            .include(tree.join("third_party/cmsis"))
            .include(tree.join("third_party/cmsis/CMSIS/Core/Include"));
    }
    for list in ["scripts/tflm-sources.txt", kernel_list] {
        for line in read_list(list) {
            build.file(tree.join(&line));
            println!("cargo:rerun-if-changed={}", tree.join(&line).display());
        }
    }
    build
        .file("src/shim.cc")
        .file("src/platform.cc")
        .file("src/diag.cc");
    build.compile("tflm");
    if cmsis_nn {
        compile_cmsis_nn(tree);
    }

    println!("cargo:rerun-if-changed=src/shim.cc");
    println!("cargo:rerun-if-changed=src/shim.h");
    println!("cargo:rerun-if-changed=src/platform.cc");
    println!("cargo:rerun-if-changed=src/diag.cc");
}

/// Compile the CMSIS-NN C files with arm-none-eabi-gcc. `-mcpu=cortex-m4`
/// makes the compiler define `__ARM_FEATURE_DSP`, so CMSIS-NN selects its DSP
/// (SIMD) code.
fn compile_cmsis_nn(tree: &Path) {
    let source_dir = tree.join("third_party/cmsis_nn/Source");
    let mut build = cc::Build::new();
    build
        .compiler("arm-none-eabi-gcc")
        .archiver("arm-none-eabi-ar")
        .target("thumbv7em-none-eabihf")
        .opt_level_str("s")
        .debug(false)
        .warnings(false)
        .include(tree)
        .include(tree.join("third_party/cmsis_nn"))
        .include(tree.join("third_party/cmsis_nn/Include"))
        .include(tree.join("third_party/cmsis"))
        .include(tree.join("third_party/cmsis/CMSIS/Core/Include"))
        .define("ARM_NN_ENABLE_F16", "0")
        .define("ARM_NN_ENABLE_F32", "0")
        .define("NDEBUG", None)
        .flag("-fno-pic")
        .flag("-fno-pie")
        .flag("-fomit-frame-pointer")
        .flag("-funsigned-char")
        .flag("-ffunction-sections")
        .flag("-fdata-sections");
    for flag in CPU_FLAGS {
        build.flag(flag);
    }
    for line in read_list("scripts/cmsis-nn-sources.txt") {
        build.file(source_dir.join(&line));
        println!(
            "cargo:rerun-if-changed={}",
            source_dir.join(&line).display()
        );
    }
    build.compile("cmsis_nn");
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
