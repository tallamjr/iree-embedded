<h1 align="center">IREE: Embedded</h1>
<p align="center">
  <img src="assets/logo.png" width="180">
</p>

<h4 align="center"><em>An embedded <code>no_std</code> Rust runtime for AI inference on microcontrollers, built on <a href="https://iree.dev">IREE</a>.</em></h4>

<p align="center">
  <a href="https://crates.io/crates/iree-embedded"><img src="https://img.shields.io/crates/v/iree-embedded"></a>
  <a href="https://docs.rs/iree-embedded"><img src="https://img.shields.io/docsrs/iree-embedded"></a>
  <a href="https://github.com/tallamjr/iree-embedded/actions/workflows/runtime.yml"><img src="https://img.shields.io/github/actions/workflow/status/tallamjr/iree-embedded/runtime.yml?branch=master"></a>
  <a href="#licence"><img src="https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue"></a>
</p>

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
  Running a model is ~15 lines of Rust, with no `unsafe` anywhere in user code:

  ```rust
  let arena = Arena::new(singleton!([u8; 56 * 1024] = [0; 56 * 1024]));
  let instance = Instance::new(&arena)?;
  let device = Device::local_sync_static(&arena, &[link_kernels!(my_model_library_query)])?;
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

### Why IREE and not ONNX Runtime, TFLite-Micro, TVM, or MicroFlow?

The practical benefits this project leans on:

1. **Does it even fit.** ONNX Runtime simply cannot run on an nRF52833 with
   128 KB RAM and no OS. IREE's runtime half is designed for exactly that.
2. **Target-specific codegen.** The kernels in `model.o` are compiled with
   `-mcpu=cortex-m4`, hard-float ABI, for the exact tensor shapes in the
   model: no dynamic dispatch, no unused ops.
3. **Zero-RAM model.** Because the output is an ordinary object file, it is
   linked into the firmware and the CPU executes it straight from flash. An
   interpreted runtime needs its graph, and often its weights, staged in RAM.
4. **Framework-agnostic front door.** One pipeline ingests
   TFLite/ONNX/PyTorch/JAX, versus being tied to one format.

**Where each alternative wins and loses:**

|                 | Fits a 128 KB MCU?        | Model formats              | Kernels                                     | Update a model by           | Maturity              |
| --------------- | ------------------------- | -------------------------- | ------------------------------------------- | --------------------------- | --------------------- |
| ONNX Runtime    | No                        | ONNX                       | mature, hand-tuned, GPU execution providers | loading a file at runtime   | very mature           |
| TFLite-Micro    | Yes                       | TFLite only                | fixed library, interpreted                  | swapping the `.tflite` blob | mature, battle-tested |
| microTVM        | Yes, but dormant upstream | many                       | compiled                                    | reflashing                  | stagnant              |
| MicroFlow       | Yes, even 8-bit AVR       | TFLite subset              | generated portable Rust                     | recompiling firmware        | young                 |
| `iree-embedded` | Yes                       | TFLite, ONNX, PyTorch, JAX | compiled for your exact CPU                 | reflashing                  | young                 |

**The trade-offs.**

- For `iree-embedded`, every model must be compiled on a host, per target,
  before it can run so a model is fixed at build time, which means updating
  requires reflashing firmware rather than loading a new file.
- The toolchain is younger and rougher than ONNX Runtime's. That is why, on a
  server with an OS and gigabytes of RAM, ORT's ecosystem usually wins and IREE is
  not the pragmatic choice.
- On a microcontroller the real comparison is TFLite-Micro, and the split is
  **interpretation** i.e at runtime vs. **compilation**: TFLM carries a fixed
  kernel library and interprets your graph against it, with the flexibility to
  swap models at runtime; IREE compiles the graph into target-specific machine
  code **ahead-of-time**, smaller and faster, at the price of that host-side
  compile step.

IREE's sweet spot is exactly the constrained target this crate serves.

> [!NOTE]
> **More on TVM.** Architecturally the closest relative: Apache TVM is
> also an ahead-of-time compiler, and its microTVM project targeted this same
> bare-metal Cortex-M niche with a C runtime and AoT executor. The difference is
> momentum and the runtime half. microTVM has stagnated upstream (its
> maintainers moved on as the TVM project's focus shifted towards LLM serving),
> its bare-metal story leans on generated C glue per project, and there are no
> Rust bindings. IREE's runtime is a small, actively developed C library with a
> HAL built for single-threaded bare-metal targets, which is precisely the piece
> this crate binds.

> [!NOTE]
> **More on [MicroFlow](https://github.com/matteocarnelos/microflow-rs).** A genuinely
> elegant alternative, and the right tool for some jobs: it is pure Rust end to
> end (no C toolchain at all), its procedural macro parses the `.tflite`
> flatbuffer at compile time into static Rust code, and it runs on parts far
> smaller than this crate targets (down to 8-bit AVR). The trade-offs are a
> small fixed operator set, a TFLite-only front door, and portable Rust kernels
> rather than per-CPU compiled ones. If a small quantised TFLite model fits
> MicroFlow's ops, it is the simpler choice; this crate is for when you want the
> full compiler front door (ONNX, PyTorch, JAX), broad op coverage, and kernels
> code-generated for your exact core.

## Status

The end-to-end demo works on real hardware, on two boards: live keyword
spotting ("yes"/"no") re-classified four times per second, with the full
pipeline (mic capture, a pure-Rust audio front end, and the micro_speech
model under IREE) on the device.

- **BBC micro:bit v2** (nRF52833, 128 KB RAM): analog mic over `embassy-nrf`
  SAADC, flashed with probe-rs, logs over defmt/RTT. See
  [`examples/microbit-v2-kws`](examples/microbit-v2-kws/README.md) for the
  demo, the model-compilation workflow, and a documented RAM map.
- **Arduino Nano 33 BLE Sense** (nRF52840, 256 KB RAM): digital PDM mic,
  flashed over plain USB through the stock Arduino bootloader (no debug
  probe needed), detections shown on the onboard RGB LED. See
  [`examples/arduino-nano33ble-kws`](examples/arduino-nano33ble-kws/README.md).

Both examples are **pure Rust end to end** with `#![forbid(unsafe_code)]`
firmware: the only non-Rust artefacts are the IREE runtime (vendored C, bound
by `iree-embedded`) and the model itself (`iree-compile` output, i.e. machine
code). The audio front end is a byte-exact Rust port of the TFLite-Micro
reference, bundled inside each example.

Stack: `embassy-executor`, `embassy-nrf` (the nRF device HAL), `cortex-m-rt`,
`defmt` over RTT, `probe-rs run` (micro:bit) or `bossac` over USB (Nano).

## Workspace

| Crate                      | Purpose                                                                                                                  |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `crates/iree-embedded-sys` | Raw `bindgen` FFI to the prebuilt IREE runtime (the only FFI crate).                                                     |
| `crates/iree-embedded`     | Safe `no_std` public API.                                                                                                |
| `examples/microbit-v2-kws` | Live keyword-spotting demo; a self-contained workspace that bundles its own pure-Rust audio front end (`kws-frontend/`). |
| `examples/arduino-nano33ble-kws` | The same demo on the Nano 33 BLE Sense (PDM mic, USB flashing, LED feedback); equally self-contained. |

## Building

Consuming the crates from crates.io needs none of this: `iree-embedded-sys`'s
build script downloads the matching prebuilt runtime for your target and
verifies its checksum. The options below are for building from this repository.

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

| Thing                          | What it is                                       | Where it comes from    |
| ------------------------------ | ------------------------------------------------ | ---------------------- |
| Rust + `thumbv7em-none-eabihf` | compiles the firmware and crates                 | `rustup`               |
| `arm-none-eabi-gcc`            | cross-compiles the IREE C runtime                | package manager        |
| `iree-compile`                 | compiles the _model_ to Cortex-M kernels         | `pip` (pinned version) |
| `toolchains/*.cmake`           | NOT compilers: per-CPU flag presets (M4F/M7/M33) | this repo              |

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
2. the chip's device HAL (e.g. `embassy-nrf` here, `embassy-stm32` elsewhere)
   and its audio-capture peripheral (the Nano's mic is PDM, which the nRF52840
   decodes in hardware, simpler than the micro:bit's analog SAADC path),
3. a flash/log transport: an SWD probe for `probe-rs` + `defmt`, or the
   stock bootloader plus UART logging,
4. arena and buffer sizes for the RAM budget (256 KB on the nRF52840 is
   comfortable).

A different CPU class (M0/M7/M33) additionally needs an IREE runtime
cross-build and model recompile for that target triple, both mechanical.

## Licence

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT licence](LICENSE-MIT) at your option, matching the Rust and IREE
ecosystems.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 licence, shall be
dual licensed as above, without any additional terms or conditions.
