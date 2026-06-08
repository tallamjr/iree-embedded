use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.join("../..").canonicalize().unwrap();

    // The out-of-band IREE runtime build (see `.iree/setup.sh` / `just
    // build-runtime-host`). build.rs only LINKS this; it never builds IREE.
    let build_dir = std::env::var("IREE_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join(".iree/build/host"));
    let src = root.join(".iree/src");

    let inc_src = src.join("runtime/src");
    let inc_gen = build_dir.join("runtime/src");
    let inc_flatcc = src.join("third_party/flatcc/include");

    // Link exactly the three archives the runtime build produces. The unified
    // archive merges the runtime's transitive deps; the driver and loader are
    // opt-in and linked explicitly (mirrors IREE's static_library sample).
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
    ] {
        println!(
            "cargo:rustc-link-search=native={}",
            build_dir.join(dir).display()
        );
        println!("cargo:rustc-link-lib=static={lib}");
    }

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let extern_c = out.join("extern.c");

    // On macOS, point bindgen's clang at the SDK headers and (if the caller did
    // not already) at Homebrew's libclang, so plain `cargo build`/`test` works
    // without manual environment setup.
    let mut extra_clang_args: Vec<String> = Vec::new();
    if cfg!(target_os = "macos") {
        if let Some(sdk) = run_capture("xcrun", &["--show-sdk-path"]) {
            extra_clang_args.push(format!("-isysroot{sdk}"));
        }
        if std::env::var_os("LIBCLANG_PATH").is_none() {
            if let Some(prefix) = run_capture("brew", &["--prefix", "llvm"]) {
                std::env::set_var("LIBCLANG_PATH", format!("{prefix}/lib"));
            }
        }
    }

    let mut builder = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", inc_src.display()))
        .clang_arg(format!("-I{}", inc_gen.display()))
        .clang_arg(format!("-I{}", inc_flatcc.display()))
        .use_core()
        .ctypes_prefix("core::ffi")
        // bindgen's generated size/align assertions misfire on IREE's vtable
        // and opaque types; the bindings themselves are correct.
        .layout_tests(false)
        .allowlist_function("iree_.*")
        .allowlist_type("iree_.*")
        .allowlist_var("IREE_.*")
        // Many IREE helpers are `static inline`; emit C wrappers for them.
        .wrap_static_fns(true)
        .wrap_static_fns_path(&extern_c);
    for arg in &extra_clang_args {
        builder = builder.clang_arg(arg);
    }
    let bindings = builder.generate().expect("bindgen failed");
    bindings
        .write_to_file(out.join("bindings.rs"))
        .expect("write bindings");

    // Compile the generated wrappers for the static-inline helpers.
    cc::Build::new()
        .file(&extern_c)
        .include(&manifest)
        .include(&inc_src)
        .include(&inc_gen)
        .include(&inc_flatcc)
        .compile("iree_static_wrappers");

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
