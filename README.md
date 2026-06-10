# iree-embedded

An embedded `no_std` Rust runtime for machine-learning inference on Cortex-M
microcontrollers, built on [IREE](https://iree.dev)'s bare-metal C runtime.

## Why this crate

IREE is an ahead-of-time compiler plus a small runtime: a model is compiled
once on a host into a self-contained artefact, and the device only loads and
executes it through a thin executor. `iree-embedded` provides safe, idiomatic
Rust bindings over the _runtime_ half of that split, for constrained `no_std`
embedded systems, a niche not currently served by mature Rust crates (`ort`,
`tract`, and `candle` are all host/`std`).

Three reasons to want it:

- **A safe API over a hostile FFI.** IREE's C API is hundreds of refcounted
  objects with manual release discipline. The crate wraps them in six RAII
  types (`Arena`, `Instance`, `Device`, `Context`, `Tensor`, `Error`), so
  leaks and double-frees become compile-time impossibilities and every
  fallible call returns a `Result` carrying the real IREE status message.
  Running a model is ~15 lines of safe Rust:

  ```rust
  let arena = unsafe { Arena::new(&mut HEAP) };      // any static buffer
  let instance = Instance::new(&arena)?;
  let device = Device::local_sync_static(&arena, &[my_model_library_query])?;
  let ctx = Context::new(&instance, &device, VMFB, &arena)?;
  let out = ctx.invoke(ctx.resolve("module.main")?, &[&input], &arena)?;
  ```

- **The hard embedded answers are baked in.** Model kernels execute in place
  from flash via IREE's static-library loader (zero RAM for weights and code);
  `include_vmfb!` enforces the 64-byte alignment without which IREE silently
  falls back to a staging copy that deadlocks a single-threaded bare-metal
  HAL; transient blocks are MCU-sized; `iree_allocator_t` is bridged onto any
  static Rust buffer; out-of-memory failures report the allocation size that
  did not fit. Each of these was found the hard way on real hardware.

- **The IREE compiler as the front door.** TFLite-Micro interprets ops from a
  fixed C++ kernel library; IREE ingests models from any framework (TFLite,
  ONNX, PyTorch via Torch-MLIR, JAX) and ahead-of-time compiles kernels for
  your exact CPU. This crate makes that pipeline reachable from Rust firmware.

### Why IREE and not ONNX Runtime or TFLite-Micro?

The practical benefits this project leans on:

1. **It fits at all.** ONNX Runtime simply cannot run on an nRF52833 with
   128 KB RAM and no OS. IREE's runtime half is designed for exactly that.
2. **Target-specific codegen.** The kernels in `model.o` are compiled with
   `-mcpu=cortex-m4`, hard-float ABI, for the exact tensor shapes in the
   model: no dynamic dispatch, no unused ops.
3. **Zero-RAM model.** Because the output is an ordinary object file, it is
   linked into the firmware and the CPU executes it straight from flash. An
   interpreted runtime needs its graph, and often its weights, staged in RAM.
4. **Framework-agnostic front door.** One pipeline ingests
   TFLite/ONNX/PyTorch/JAX, versus being tied to one format.

The trade-off, to be fair: with IREE you take on a host-side compile step per
model per target ("phase 1" in the example README), the toolchain is younger
and rougher than ONNX Runtime's, and on big servers ORT's mature hand-tuned
kernels and ecosystem (execution providers for CUDA/TensorRT and so on) are
often the pragmatic choice. IREE's sweet spot is where the deployment target
is constrained or unusual, which is exactly the embedded niche this crate
serves. The closer comparison for this use case is not really ORT at all but
TFLite-Micro, where the same logic applies: TFLM interprets against a fixed
kernel library, IREE compiles.

## Status

The end-to-end demo works on real hardware: live keyword spotting
("yes"/"no") from the BBC micro:bit v2's onboard analog microphone, with the
full pipeline on the nRF52833 (Cortex-M4F, 128 KB RAM): SAADC capture, the
TFLite-Micro audio front end, and the micro_speech model under IREE,
re-classified four times per second. See
[`examples/microbit-v2-kws`](examples/microbit-v2-kws/README.md) for the
demo, the model-compilation workflow, and a documented RAM map.

Stack: `cortex-m-rt`, `nrf52833-pac`, `defmt` over RTT, `probe-rs run`.

## Workspace

| Crate                      | Purpose                                                                   |
| -------------------------- | ------------------------------------------------------------------------- |
| `crates/iree-embedded-sys` | Raw `bindgen` FFI to the prebuilt IREE runtime (the only `unsafe` crate). |
| `crates/iree-embedded`     | Safe `no_std` public API.                                                 |
| `examples/microbit-v2-kws` | Live keyword-spotting demo binary.                                        |

## Building

Two options for the IREE runtime the crates link against:

- **Prebuilt artefact (recommended).** CI builds the runtime per target and
  attaches tarballs to GitHub releases. Unpack one and point the build at it:

  ```sh
  export IREE_RUNTIME_DIR=<unpacked>/build
  export IREE_SRC_DIR=<unpacked>/src
  cargo build
  ```

- **From source.** `scripts/build-runtime-host.sh` (host, for `cargo test`)
  and `scripts/build-runtime-mcu.sh [cortex-m4f|cortex-m7|cortex-m33]` fetch
  the pinned IREE commit (`scripts/iree-version.env`) and build into
  `.iree/build/`, where the build script finds them by default.

Models are compiled with `iree-compile` (the pinned pip release, see
`scripts/iree-version.env`) into a static-library `.o` plus a `.vmfb`, which
the firmware links and embeds; the exact flags are in the example README.

The word "toolchain" appears in three distinct senses here, so to be precise
about what is installed versus configured:

| Thing                          | What it is                                        | Where it comes from        |
| ------------------------------ | ------------------------------------------------- | -------------------------- |
| Rust + `thumbv7em-none-eabihf` | compiles the firmware and crates                  | `rustup`                   |
| `arm-none-eabi-gcc`            | compiles the IREE C runtime and vendored C        | package manager            |
| `iree-compile`                 | compiles the *model* to Cortex-M kernels          | `pip` (pinned version)     |
| `toolchains/*.cmake`           | NOT compilers: per-CPU flag presets (M4F/M7/M33)  | this repo                  |

Only the cross-compiled bare-metal IREE runtime cannot be downloaded from
anywhere upstream (the IREE project publishes host pip packages only); that
gap is what the CI artefacts exist to fill.

## Continuous integration

`.github/workflows/runtime.yml` builds the runtime for every target on each
push (host runtimes also run the crate test suite; the firmware job builds
the example against the freshly packaged cortex-m4f artefact, proving the
artefact layout end to end). Tagged `v*` pushes publish all artefacts as a
GitHub release. cortex-m4f is validated on hardware; cortex-m7 and cortex-m33
artefacts are CI-build-only until someone runs them on silicon.

## Supporting another board

The crates are board-agnostic: anything Cortex-M4F-class reuses the prebuilt
runtime and even the same compiled model artefacts unchanged. A new board
(say, an Arduino Nano 33 BLE Sense, nRF52840) needs only a new example:

1. `memory.x` for the chip's flash/RAM (and app offset if a bootloader stays
   resident),
2. the chip's PAC and its audio-capture peripheral (the Nano's mic is PDM,
   which the nRF52840 decodes in hardware, simpler than the micro:bit's
   analog SAADC path),
3. a flash/log transport: an SWD probe for `probe-rs` + `defmt`, or the
   stock bootloader plus UART logging,
4. arena and buffer sizes for the RAM budget (256 KB on the nRF52840 is
   comfortable).

A different CPU class (M0/M7/M33) additionally needs an IREE runtime
cross-build and model recompile for that target triple, both mechanical.

## Licence

To be decided before first release (Apache-2.0 or MIT/Apache dual, to match
the Rust and IREE ecosystems).
