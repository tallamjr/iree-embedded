# iree-embedded-sys

[![crates.io](https://img.shields.io/crates/v/iree-embedded-sys.svg)](https://crates.io/crates/iree-embedded-sys)
[![docs.rs](https://docs.rs/iree-embedded-sys/badge.svg)](https://docs.rs/iree-embedded-sys)
[![license](https://img.shields.io/crates/l/iree-embedded-sys.svg)](#licence)

Raw `bindgen` FFI bindings to the [IREE](https://iree.dev) bare-metal runtime,
for `no_std` Cortex-M targets. This is the unsafe `-sys` layer; most users
want the safe wrapper, [`iree-embedded`](https://crates.io/crates/iree-embedded).

## Runtime dependency

The build script links the IREE C runtime, which `cargo` does not build itself.
It resolves the runtime in this order:

1. `IREE_RUNTIME_DIR` (archives) and `IREE_SRC_DIR` (headers), if set, a local
   build, used for development.
2. Otherwise it downloads the prebuilt runtime artefact matching your target
   from the project's GitHub releases and verifies its checksum.

Bindings for the Cortex-M target are committed, so building for
`thumbv7em-none-eabihf` (the deployment target) needs no `libclang`. Host-target
builds, used for `cargo test`, regenerate their bindings and so do require
`libclang`. Force a refresh of the committed bindings with
`IREE_EMBEDDED_REGENERATE_BINDINGS=1`.

## Licence

Licensed under either of
[Apache-2.0](https://github.com/tallamjr/iree-embedded/blob/master/LICENSE-APACHE)
or [MIT](https://github.com/tallamjr/iree-embedded/blob/master/LICENSE-MIT) at
your option.
