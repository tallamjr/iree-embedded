use std::path::{Path, PathBuf};

// Pinned IREE runtime release the committed bindings match. This mirrors
// scripts/iree-version.env (the dev/CI source of truth) but is embedded here
// because the published crate does not carry repo-root files: a crates.io
// consumer's build.rs only sees what lives under this crate directory.
const IREE_SHA: &str = "e4a3b0405d7d23554da26403658d0e8c3c5ecf25";
const RELEASE_TAG: &str = "v0.1.0";
const RELEASE_BASE: &str = "https://github.com/tallamjr/iree-embedded/releases/download";

// Pinned sha256 of each downloadable artefact. Compiled in so a missing file is
// a build error, never a silent skip of verification.
const CHECKSUMS: &str = include_str!("runtime-checksums.txt");

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.join("../..").canonicalize().unwrap();
    let target = std::env::var("TARGET").unwrap_or_default();
    let is_mcu = target.starts_with("thumbv7em");
    // Bindings differ only by data model: ILP32 on the MCU, LP64 on the host.
    let variant = if is_mcu { "mcu" } else { "host" };

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    // Locate the out-of-band IREE runtime (built by scripts/build-runtime-*.sh,
    // or an unpacked CI artefact). build.rs only LINKS this; it never builds it.
    // Resolution order:
    //   1. IREE_RUNTIME_DIR / IREE_SRC_DIR env vars (explicit override; each
    //      falls back independently to the in-repo .iree path).
    //   2. In-repo .iree/build/{host,mcu} + .iree/src (the dev and CI layout).
    //   3. Download the prebuilt artefact for this target from the GitHub
    //      release and unpack it into OUT_DIR (the crates.io consumer path).
    let env_src = std::env::var_os("IREE_SRC_DIR").map(PathBuf::from);
    let env_build = std::env::var_os("IREE_RUNTIME_DIR").map(PathBuf::from);
    let in_repo_src = root.join(".iree/src");
    let in_repo_build = root
        .join(".iree/build")
        .join(if is_mcu { "mcu" } else { "host" });

    let (src, build_dir) = if env_src.is_some() || env_build.is_some() {
        (
            env_src.unwrap_or_else(|| in_repo_src.clone()),
            env_build.unwrap_or_else(|| in_repo_build.clone()),
        )
    } else if in_repo_build.is_dir() && in_repo_src.is_dir() {
        (in_repo_src, in_repo_build)
    } else {
        let unpacked = download_runtime(&target, is_mcu, &out);
        (unpacked.join("src"), unpacked.join("build"))
    };

    let inc_src = src.join("runtime/src");
    let inc_gen = build_dir.join("runtime/src");
    let inc_flatcc = src.join("third_party/flatcc/include");
    // Committed inside the crate (a copy of repo-root toolchains/iree_bm_config.h)
    // so it is reachable when building from a crates.io checkout, where the repo
    // root is absent.
    let bm_config = manifest.join("iree_bm_config.h");

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
    println!("cargo:rerun-if-changed=iree_bm_config.h");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=generated");
    println!("cargo:rerun-if-changed=runtime-checksums.txt");
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
    } else if cfg!(target_os = "macos") {
        // Not a let-chain (`&& let`), to keep the MSRV at edition 2024's 1.85.
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

/// Download the prebuilt runtime artefact for `target` from the GitHub release,
/// verify it against the pinned sha256, unpack it into `OUT_DIR`, and return the
/// unpacked directory (which holds `build/` and `src/`). Taken only when neither
/// IREE_RUNTIME_DIR/IREE_SRC_DIR nor an in-repo .iree/ is present, i.e. a plain
/// crates.io consumer. Uses system `curl`, `tar`, and `sha256sum`/`shasum`
/// rather than pulling a TLS + crypto stack into every consumer's build.
fn download_runtime(target: &str, is_mcu: bool, out: &Path) -> PathBuf {
    let artefact_target = artefact_target(target, is_mcu).unwrap_or_else(|| {
        panic!(
            "iree-embedded-sys: no prebuilt IREE runtime is published for target `{target}`. \
             Build one with scripts/build-runtime-*.sh and point IREE_RUNTIME_DIR (build dir) \
             and IREE_SRC_DIR (IREE source) at it."
        )
    });
    let sha9 = &IREE_SHA[..9];
    let name = format!("iree-runtime-{artefact_target}-{sha9}");
    let file = format!("{name}.tar.gz");

    let unpacked = out.join(&name);
    // Reuse a prior download on incremental builds.
    if unpacked.join("build").is_dir() && unpacked.join("src").is_dir() {
        return unpacked;
    }

    let expected = expected_sha256(CHECKSUMS, &file).unwrap_or_else(|| {
        panic!(
            "iree-embedded-sys: no pinned sha256 for `{file}` in runtime-checksums.txt; \
             the download cannot be verified. Set IREE_RUNTIME_DIR and IREE_SRC_DIR instead."
        )
    });

    let url = format!("{RELEASE_BASE}/{RELEASE_TAG}/{file}");
    let tarball = out.join(&file);
    // --proto =https / --tlsv1.2: refuse to be redirected onto plaintext.
    let status = std::process::Command::new("curl")
        .args(["--proto", "=https", "--tlsv1.2", "-fsSL", "-o"])
        .arg(&tarball)
        .arg(&url)
        .status()
        .unwrap_or_else(|e| {
            panic!("iree-embedded-sys: failed to run `curl` to download {url}: {e}")
        });
    assert!(
        status.success(),
        "iree-embedded-sys: `curl` failed ({status}) downloading {url}"
    );

    let actual = sha256_file(&tarball);
    assert!(
        actual == expected,
        "iree-embedded-sys: sha256 mismatch for {file}\n  expected {expected}\n  got      {actual}\n\
         Refusing to use a runtime that does not match the pinned checksum."
    );

    let status = std::process::Command::new("tar")
        .arg("-xzf")
        .arg(&tarball)
        .arg("-C")
        .arg(out)
        .status()
        .unwrap_or_else(|e| panic!("iree-embedded-sys: failed to run `tar` on {file}: {e}"));
    assert!(
        status.success(),
        "iree-embedded-sys: `tar` failed ({status}) extracting {file}"
    );

    assert!(
        unpacked.join("build").is_dir() && unpacked.join("src").is_dir(),
        "iree-embedded-sys: unpacked {file} but {} is missing build/ or src/",
        unpacked.display()
    );
    unpacked
}

/// Map a Rust target triple to the artefact target name used in the release
/// filenames (see scripts/package-runtime.sh). Returns None for platforms the
/// v0.1.0 release does not publish a runtime for.
fn artefact_target(target: &str, is_mcu: bool) -> Option<String> {
    if is_mcu {
        // thumbv7em-none-eabihf is the hardware-validated Cortex-M4F build.
        return Some("cortex-m4f".to_string());
    }
    let arch = if target.starts_with("x86_64") {
        "x86_64"
    } else if target.starts_with("aarch64") {
        "arm64"
    } else {
        return None;
    };
    let os = if target.contains("apple") || target.contains("darwin") {
        "darwin"
    } else if target.contains("linux") {
        "linux"
    } else {
        return None;
    };
    Some(format!("host-{os}-{arch}"))
}

/// Look up the pinned sha256 for `filename` in the checksums file. Lines are
/// `<hex>  <filename>`; blank lines and `#` comments are ignored.
fn expected_sha256(checksums: &str, filename: &str) -> Option<String> {
    for line in checksums.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?;
        if name == filename {
            return Some(hash.to_lowercase());
        }
    }
    None
}

/// Compute the sha256 of a file using GNU coreutils `sha256sum` (Linux) or the
/// BSD/perl `shasum -a 256` (macOS), returning the lowercase hex digest.
fn sha256_file(path: &Path) -> String {
    let out = std::process::Command::new("sha256sum")
        .arg(path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .or_else(|| {
            std::process::Command::new("shasum")
                .args(["-a", "256"])
                .arg(path)
                .output()
                .ok()
                .filter(|o| o.status.success())
        })
        .unwrap_or_else(|| {
            panic!(
                "iree-embedded-sys: need `sha256sum` or `shasum` on PATH to verify the \
                 runtime download. Set IREE_RUNTIME_DIR and IREE_SRC_DIR to skip the download."
            )
        });
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .expect("sha256 tool produced no output")
        .to_lowercase()
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
