use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.join("../..").canonicalize().unwrap();
    let target = std::env::var("TARGET").unwrap_or_default();
    let is_mcu = target.starts_with("thumbv7em");
    // Bindings differ only by data model: ILP32 on the MCU, LP64 on the host.
    let variant = if is_mcu { "mcu" } else { "host" };

    // The out-of-band IREE runtime build (scripts/build-runtime-{host,mcu}.sh,
    // or an unpacked CI artefact). build.rs only LINKS this. For an artefact,
    // set IREE_RUNTIME_DIR=<unpacked>/build and IREE_SRC_DIR=<unpacked>/src.
    let src = std::env::var("IREE_SRC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join(".iree/src"));
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
    let bindings_rs = out.join("bindings.rs");

    // Pre-generated bindings are committed for the MCU (the deployment target,
    // a fixed thumbv7em ABI) so a consumer needs no libclang: the default build
    // copies them in. The host variant is not committed (it is dev/test only,
    // and libclang is available there); it always regenerates. Refresh the
    // committed copies with `IREE_EMBEDDED_REGENERATE_BINDINGS=1`, e.g. after
    // bumping the pinned IREE version.
    let gen_dir = manifest.join("generated");
    let committed_bindings = gen_dir.join(format!("bindings_{variant}.rs"));
    let committed_extern = gen_dir.join(format!("extern_{variant}.c"));
    let explicit_regen = std::env::var_os("IREE_EMBEDDED_REGENERATE_BINDINGS").is_some();
    let regenerate = explicit_regen || !committed_bindings.exists() || !committed_extern.exists();

    // `brew --prefix llvm` (used for libclang when regenerating, and for the
    // macOS archiver below) answers even when llvm is not installed.
    let llvm_prefix = if cfg!(target_os = "macos") {
        run_capture("brew", &["--prefix", "llvm"])
    } else {
        None
    };

    if regenerate {
        generate_bindings(
            &inc_src,
            &inc_gen,
            &inc_flatcc,
            &bm_config,
            is_mcu,
            &llvm_prefix,
            &bindings_rs,
            &extern_c,
        );
        // Only a deliberate refresh updates the committed copies; an
        // auto-regenerate (host, nothing committed) stays in OUT_DIR.
        if explicit_regen {
            std::fs::create_dir_all(&gen_dir).unwrap();
            std::fs::copy(&bindings_rs, &committed_bindings).expect("save committed bindings");
            std::fs::copy(&extern_c, &committed_extern).expect("save committed extern.c");
        }
    } else {
        std::fs::copy(&committed_bindings, &bindings_rs).expect("use committed bindings");
        std::fs::copy(&committed_extern, &extern_c).expect("use committed extern.c");
    }

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
    } else if cfg!(target_os = "macos") {
        // macOS's newer linker rejects non-8-byte-aligned archive members;
        // llvm-ar pads them, the default `ar` does not. `brew --prefix llvm`
        // answers even when llvm is not installed (CI runners), so only
        // override the archiver when a real llvm-ar exists (brew, then PATH).
        let llvm_ar = llvm_prefix
            .as_ref()
            .map(|p| PathBuf::from(format!("{p}/bin/llvm-ar")))
            .filter(|p| p.exists())
            .or_else(|| run_capture("which", &["llvm-ar"]).map(PathBuf::from));
        if let Some(ar) = llvm_ar {
            wrappers.archiver(ar);
        }
    }
    wrappers.compile("iree_static_wrappers");

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=generated");
    println!("cargo:rerun-if-env-changed=IREE_RUNTIME_DIR");
    println!("cargo:rerun-if-env-changed=IREE_SRC_DIR");
    println!("cargo:rerun-if-env-changed=IREE_EMBEDDED_REGENERATE_BINDINGS");
}

/// Run bindgen against the IREE headers, writing `bindings.rs` and the C
/// wrapper source (`extern.c`) for IREE's `static inline` helpers. Only invoked
/// when regenerating the committed bindings; this is the one path that needs
/// libclang.
#[allow(clippy::too_many_arguments)]
fn generate_bindings(
    inc_src: &std::path::Path,
    inc_gen: &std::path::Path,
    inc_flatcc: &std::path::Path,
    bm_config: &std::path::Path,
    is_mcu: bool,
    llvm_prefix: &Option<String>,
    bindings_rs: &std::path::Path,
    extern_c: &std::path::Path,
) {
    // Clang args for bindgen, matched to the target so struct layouts are
    // correct (the MCU is 32-bit with IREE_DEVICE_SIZE_T=uint32_t).
    let mut clang_args: Vec<String> = vec![
        format!("-I{}", inc_src.display()),
        format!("-I{}", inc_gen.display()),
        format!("-I{}", inc_flatcc.display()),
    ];

    if cfg!(target_os = "macos") && std::env::var_os("LIBCLANG_PATH").is_none() {
        // `brew --prefix llvm` answers even when llvm is not installed, so
        // verify the dylib exists and fall back to the Xcode / CLT copy.
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(p) = llvm_prefix {
            candidates.push(PathBuf::from(format!("{p}/lib")));
        }
        if let Some(xc) = run_capture("xcode-select", &["-p"]) {
            candidates.push(PathBuf::from(format!(
                "{xc}/Toolchains/XcodeDefault.xctoolchain/usr/lib"
            )));
            candidates.push(PathBuf::from(format!("{xc}/usr/lib")));
        }
        if let Some(dir) = candidates
            .into_iter()
            .find(|d| d.join("libclang.dylib").exists())
        {
            // SAFETY: build scripts are single-threaded at this point; no
            // other thread can be reading the environment concurrently.
            unsafe { std::env::set_var("LIBCLANG_PATH", &dir) };
        }
    }

    if is_mcu {
        // Parse the headers as bare-metal Cortex-M (force-include the same
        // config header the runtime was compiled with).
        clang_args.push("--target=thumbv7em-none-eabihf".to_string());
        clang_args.push("-include".to_string());
        clang_args.push(bm_config.display().to_string());
        // newlib system headers (inttypes.h etc.). `-print-sysroot` is empty on
        // some packaged toolchains (Ubuntu's gcc-arm-none-eabi), so use the
        // compiler's actual include search list, reliable everywhere.
        for dir in arm_none_eabi_include_dirs() {
            clang_args.push(format!("-isystem{dir}"));
        }
    } else if cfg!(target_os = "macos")
        && let Some(sdk) = run_capture("xcrun", &["--show-sdk-path"])
    {
        clang_args.push(format!("-isysroot{sdk}"));
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
        // Edition 2024: unsafe fn bodies are no longer implicitly unsafe;
        // make generated code wrap its unsafe operations in unsafe blocks.
        .wrap_unsafe_ops(true)
        // Many IREE helpers are `static inline`; emit C wrappers for them.
        .wrap_static_fns(true)
        .wrap_static_fns_path(extern_c);
    for arg in &clang_args {
        builder = builder.clang_arg(arg);
    }
    let bindings = builder.generate().expect("bindgen failed");
    bindings.write_to_file(bindings_rs).expect("write bindings");
}

/// The C system include directories `arm-none-eabi-gcc` searches, parsed from
/// its own verbose include list. Reliable even when `-print-sysroot` is empty
/// (Ubuntu's packaged toolchain), so bindgen's clang finds the newlib headers.
fn arm_none_eabi_include_dirs() -> Vec<String> {
    let Ok(out) = std::process::Command::new("arm-none-eabi-gcc")
        .args(["-E", "-Wp,-v", "-xc", "/dev/null"])
        .output()
    else {
        return Vec::new();
    };
    // gcc prints the include search list to stderr.
    let text = String::from_utf8_lossy(&out.stderr);
    let mut dirs = Vec::new();
    let mut in_list = false;
    for line in text.lines() {
        if line.contains("#include <...> search starts here:") {
            in_list = true;
        } else if line.contains("End of search list.") {
            break;
        } else if in_list {
            dirs.push(line.trim().to_string());
        }
    }
    dirs
}

/// Run a command and return its trimmed stdout, or None if it fails.
fn run_capture(cmd: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(cmd).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}
