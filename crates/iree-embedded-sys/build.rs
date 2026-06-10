use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.join("../..").canonicalize().unwrap();
    let target = std::env::var("TARGET").unwrap_or_default();
    let is_mcu = target.starts_with("thumbv7em");

    let src = root.join(".iree/src");
    // The out-of-band IREE runtime build (see `.iree/setup.sh` for host,
    // `.iree/setup-mcu.sh` for the board). build.rs only LINKS this.
    let build_dir = std::env::var("IREE_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            root.join(".iree/build")
                .join(if is_mcu { "mcu" } else { "host" })
        });

    let inc_src = src.join("runtime/src");
    let inc_gen = build_dir.join("runtime/src");
    let inc_flatcc = src.join("third_party/flatcc/include");
    let bm_config = root.join("toolchains/iree_bm_config.h");

    // Link exactly the archives the runtime build produces (unified merges the
    // first-party deps; the driver, loader, and third-party libs are explicit).
    for (dir, lib) in [
        ("runtime/src/iree/runtime", "iree_runtime_unified"),
        (
            "runtime/src/iree/hal/drivers/local_sync",
            "iree_hal_drivers_local_sync_sync_driver",
        ),
        (
            "runtime/src/iree/hal/local/loaders",
            "iree_hal_local_loaders_embedded_elf_loader",
        ),
        (
            "runtime/src/iree/hal/local/loaders",
            "iree_hal_local_loaders_static_library_loader",
        ),
        ("build_tools/third_party/flatcc", "flatcc_parsing"),
        ("build_tools/third_party/printf", "printf_printf"),
    ] {
        println!(
            "cargo:rustc-link-search=native={}",
            build_dir.join(dir).display()
        );
        println!("cargo:rustc-link-lib=static={lib}");
    }

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let extern_c = out.join("extern.c");

    // Clang args for bindgen, matched to the target so struct layouts are
    // correct (the MCU is 32-bit with IREE_DEVICE_SIZE_T=uint32_t).
    let mut clang_args: Vec<String> = vec![
        format!("-I{}", inc_src.display()),
        format!("-I{}", inc_gen.display()),
        format!("-I{}", inc_flatcc.display()),
    ];

    let llvm_prefix = if cfg!(target_os = "macos") {
        run_capture("brew", &["--prefix", "llvm"])
    } else {
        None
    };
    if cfg!(target_os = "macos") && std::env::var_os("LIBCLANG_PATH").is_none() {
        if let Some(p) = &llvm_prefix {
            std::env::set_var("LIBCLANG_PATH", format!("{p}/lib"));
        }
    }

    if is_mcu {
        // Parse the headers as bare-metal Cortex-M (force-include the same
        // config header the runtime was compiled with).
        clang_args.push("--target=thumbv7em-none-eabihf".to_string());
        clang_args.push("-include".to_string());
        clang_args.push(bm_config.display().to_string());
        if let Some(sysroot) = run_capture("arm-none-eabi-gcc", &["-print-sysroot"]) {
            clang_args.push(format!("-isystem{sysroot}/include"));
        }
    } else if cfg!(target_os = "macos") {
        if let Some(sdk) = run_capture("xcrun", &["--show-sdk-path"]) {
            clang_args.push(format!("-isysroot{sdk}"));
        }
    }

    let mut builder = bindgen::Builder::default()
        .header("wrapper.h")
        .use_core()
        .ctypes_prefix("core::ffi")
        // Emit enum constants as `IREE_FOO` rather than `iree_foo_e_IREE_FOO`.
        .prepend_enum_name(false)
        // bindgen's generated size/align assertions misfire on IREE's vtable
        // and opaque types; the bindings themselves are correct.
        .layout_tests(false)
        .allowlist_function("iree_.*")
        .allowlist_type("iree_.*")
        .allowlist_var("IREE_.*")
        // bindgen renders iree_allocator_t opaque (its `self` field); define the
        // real layout by hand in src/lib.rs instead.
        .blocklist_type("iree_allocator_t")
        // Many IREE helpers are `static inline`; emit C wrappers for them.
        .wrap_static_fns(true)
        .wrap_static_fns_path(&extern_c);
    for arg in &clang_args {
        builder = builder.clang_arg(arg);
    }
    let bindings = builder.generate().expect("bindgen failed");
    bindings
        .write_to_file(out.join("bindings.rs"))
        .expect("write bindings");

    // Compile the generated wrappers for the static-inline helpers.
    let mut wrappers = cc::Build::new();
    wrappers
        .file(&extern_c)
        .include(&manifest)
        .include(&inc_src)
        .include(&inc_gen)
        .include(&inc_flatcc);
    if is_mcu {
        // Force the cross compiler (cc-rs otherwise picks Homebrew gcc), then
        // force-include the config and M4F flags so the wrapper ABI matches.
        // cc-rs adds -fPIC by default; that emits GOT relocations the cortex-m
        // linker script rejects. Force non-PIC.
        wrappers.pic(false);
        wrappers
            .compiler("arm-none-eabi-gcc")
            .flag("-include")
            .flag(bm_config.to_str().unwrap())
            .flag("-mcpu=cortex-m4")
            .flag("-mthumb")
            .flag("-mfloat-abi=hard")
            .flag("-mfpu=fpv4-sp-d16")
            .flag("-fno-pic");
    } else if let Some(p) = &llvm_prefix {
        // macOS's newer linker rejects non-8-byte-aligned archive members;
        // llvm-ar pads them, the default `ar` does not.
        wrappers.archiver(format!("{p}/bin/llvm-ar"));
    }
    wrappers.compile("iree_static_wrappers");

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=IREE_RUNTIME_DIR");
}

/// Run a command and return its trimmed stdout, or None if it fails.
fn run_capture(cmd: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(cmd).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
