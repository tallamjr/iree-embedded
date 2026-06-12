# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-06-12

### Fixed

- docs.rs builds no longer fail: the `iree-embedded-sys` build script
  short-circuits under `DOCS_RS` (docs.rs builds in a network-isolated
  sandbox, so the runtime download could never succeed there) and emits the
  committed Cortex-M bindings instead. Both crates now declare
  `thumbv7em-none-eabihf` as their docs.rs target so the documented ABI
  matches those bindings.

### Added

- `scripts/test-docsrs-sim.sh` and a `docsrs-sim` CI job that simulate the
  docs.rs sandbox (packaged crate, no network, `DOCS_RS=1`) so the failure
  mode cannot regress.

## [0.1.0] - 2026-06-11

### Added

- Safe `no_std` API (`iree-embedded`) over IREE's bare-metal runtime: RAII
  `Arena`, `Instance`, `Device`, `Context`, `Tensor`, `Error`.
- Static-library executable loader (`Device::local_sync_static`) so model
  kernels execute in place from flash.
- `iree-embedded-sys` build script resolves the IREE runtime in three tiers:
  `IREE_RUNTIME_DIR`/`IREE_SRC_DIR`, then an in-repo `.iree/` build, then a
  checksum-verified download of the prebuilt artefact for the target from the
  matching GitHub release, so a plain crates.io consumer needs no local build.
- Committed Cortex-M bindings, so building for `thumbv7em-none-eabihf` needs no
  `libclang`.
- `singleton!`, `link_kernels!` and `libc_stubs!` macros encapsulating the
  bare-metal plumbing (static buffers, the kernel query symbol, libc stubs),
  so firmware using the crate compiles under `#![forbid(unsafe_code)]`; the
  micro:bit example now does.
- `kws-frontend`: a pure-Rust, byte-exact port of the TFLite-Micro audio front
  end (bundled in the micro:bit example).
- Live keyword-spotting firmware example for the BBC micro:bit v2, using the
  `embassy-nrf` SAADC HAL, verified end to end on hardware.
- CI that builds the IREE runtime per target (host, cortex-m4f/m7/m33) and
  publishes prebuilt artefacts on tagged releases.
- Dual MIT / Apache-2.0 licensing and crates.io metadata.

[0.1.1]: https://github.com/tallamjr/iree-embedded/releases/tag/v0.1.1
[0.1.0]: https://github.com/tallamjr/iree-embedded/releases/tag/v0.1.0
