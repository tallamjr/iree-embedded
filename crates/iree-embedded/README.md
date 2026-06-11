# iree-embedded

[![CI](https://github.com/tallamjr/iree-embedded/actions/workflows/runtime.yml/badge.svg)](https://github.com/tallamjr/iree-embedded/actions/workflows/runtime.yml)
[![crates.io](https://img.shields.io/crates/v/iree-embedded.svg)](https://crates.io/crates/iree-embedded)
[![docs.rs](https://docs.rs/iree-embedded/badge.svg)](https://docs.rs/iree-embedded)
[![license](https://img.shields.io/crates/l/iree-embedded.svg)](#licence)

A safe, `no_std` Rust API for machine-learning inference on Cortex-M
microcontrollers, built on [IREE](https://iree.dev)'s bare-metal C runtime.

IREE is an ahead-of-time compiler plus a small runtime: a model is compiled
once on a host into a self-contained artefact, and the device only loads and
executes it. This crate wraps the _runtime_ half in six RAII types
(`Arena`, `Instance`, `Device`, `Context`, `Tensor`, `Error`) so leaks and
double-frees become compile-time impossibilities and every fallible call
returns a `Result` carrying the real IREE status message.

```rust,no_run
use iree_embedded::{Arena, Context, Device, Instance, Tensor};

// HEAP is any static byte buffer; VMFB is the embedded compiled model.
let arena = unsafe { Arena::new(&mut HEAP) };
let instance = Instance::new(&arena)?;
let device = Device::local_sync_static(&arena, &[my_model_library_query])?;
let ctx = Context::new(&instance, &device, VMFB, &arena)?;
let out = ctx.invoke(ctx.resolve("module.main")?, &[&input], &arena)?;
```

## Runtime dependency

This crate links the IREE C runtime, which is **not** built by `cargo` itself.
`iree-embedded-sys`'s build script downloads the matching prebuilt runtime for
your target from the project's GitHub releases, or uses a local build when
`IREE_RUNTIME_DIR` / `IREE_SRC_DIR` are set. See the
[repository](https://github.com/tallamjr/iree-embedded) for details and a
complete `no_std` firmware example (live keyword spotting on a BBC micro:bit
v2).

## Licence

Licensed under either of [Apache-2.0](../../LICENSE-APACHE) or
[MIT](../../LICENSE-MIT) at your option.
