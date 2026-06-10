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

The IREE runtime is built out of band with CMake (host build for unit tests,
`thumbv7em-none-eabihf` cross-build via `toolchains/arm-cortex-m4f.cmake` for
the board) and linked as static libraries by `iree-embedded-sys`'s build
script (`IREE_RUNTIME_DIR` overrides the default `.iree/build/{host,mcu}`
location). Models are compiled with `iree-compile` into a static-library `.o`
plus a `.vmfb`, which the firmware links and embeds; the exact flags are
documented in the example README.

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
