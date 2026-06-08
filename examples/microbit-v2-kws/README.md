# micro:bit v2 IREE firmware

On-device IREE inference on the BBC micro:bit v2 (nRF52833, Cortex-M4F).

## Working today: `simple_mul` smoke firmware

`src/main.rs` runs the `simple_mul` model (4 x 2 = 8) through the IREE runtime
on the board and prints the result over RTT. With the board plugged in:

```sh
cd examples/microbit-v2-kws
cargo run          # builds, flashes via probe-rs, streams defmt/RTT
```

Expected output: `simple_mul result = [8.0, 8.0, 8.0, 8.0]`.

Footprint: ~373 KB flash / ~69 KB RAM (incl. a 64 KB arena), within the
512 KB / 128 KB budget. This proves the IREE runtime executes on real hardware.

## Planned: live keyword spotting

Live "yes"/"no" keyword spotting using a TFLite-Micro `micro_speech` model and
the onboard PDM microphone (see the design doc). The pipeline below documents
that target workflow.

## Hardware

- BBC micro:bit v2 (Nordic nRF52833, Cortex-M4F, 128 KB RAM)
- Onboard PDM microphone
- A USB cable (the onboard DAPLink debugger handles flashing and logging, no
  external probe needed)

## Prerequisites (host, one-time)

- Rust target: `rustup target add thumbv7em-none-eabihf`
- [`probe-rs`](https://probe.rs/) installed
- IREE host tools on `PATH`: `iree-import-tflite`, `iree-compile`
- The IREE runtime cross-built for the board: `just build-runtime-mcu`
- The model: `micro_speech.tflite` from
  [`tensorflow/tflite-micro`](https://github.com/tensorflow/tflite-micro)
  (`tensorflow/lite/micro/examples/micro_speech`)

## Two-phase workflow

### Phase 1: package the model (occasional, host)

Turn a `.tflite` into linkable artefacts. Run once per model.

```sh
# 1. import: TFLite flatbuffer -> TOSA MLIR
iree-import-tflite micro_speech.tflite -o model.mlir

# 2. compile: -> static-library kernels (.o) + VM program (.vmfb)
iree-compile \
  --iree-hal-target-device=local \
  --iree-hal-local-target-device-backends=llvm-cpu \
  --iree-llvmcpu-target-triple=thumbv7em-none-eabihf \
  --iree-llvmcpu-link-embedded=false \
  --iree-llvmcpu-static-library-output-path=model.o \
  model.mlir -o model.vmfb
```

This produces `model.o`, `model.h`, and `model.vmfb`, which live in this folder
and are committed so the demo builds out of the box. `model.vmfb` is embedded
with `include_bytes!`; `model.o` is linked into the firmware.

The compile target triple **must match** the firmware target: the kernels in
`model.o` are real Cortex-M machine code.

### Phase 2: deploy (your inner loop)

```sh
cargo run --release
```

The crate's `.cargo/config.toml` sets the target and a probe-rs runner, so this
one command:

1. builds the firmware for `thumbv7em-none-eabihf` (linking the safe crate, the
   prebuilt IREE runtime, `model.o`, and the embedded `model.vmfb`),
2. flashes it to the nRF52833 over USB (`probe-rs run --chip nRF52833_xxAA`),
3. attaches and streams `defmt` logs back to your terminal.

Say "yes" or "no" near the microphone; detections print over RTT and light an
LED on the display.

## On-device test

```sh
cargo test
```

probe-rs auto-detects the `embedded-test` binary and runs it on the board. The
`simple_mul` smoke test asserts the runtime returns `[8, 8, 8, 8]` before the
heavier model is trusted.

## How it works

The model is compiled to a self-contained artefact on the host; the board only
loads it and executes it through IREE's local-sync runtime, wrapped by the safe
`iree-embedded` API. Each inference, raw PDM samples are turned into the audio
spectrogram the model expects (the on-device feature pipeline) before being
handed to `invoke`. See the design doc for the full architecture.
