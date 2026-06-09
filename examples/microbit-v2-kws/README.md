# micro:bit v2 keyword spotting with IREE

On-device keyword spotting on the BBC micro:bit v2 (nRF52833, Cortex-M4F): the
full pipeline runs on the chip.

## What it does (works today)

`src/main.rs` runs the **complete KWS pipeline on-device**:

1. a real 1-second "yes" recording is embedded in flash;
2. the **TFLite-Micro audio front end** (vendored C, runs on-device) turns it
   into a 49x40 spectrogram;
3. the **micro_speech** model runs via the IREE runtime;
4. the predicted keyword is printed over RTT.

```sh
cd examples/microbit-v2-kws
cargo run          # builds, flashes via probe-rs, streams defmt/RTT
```

Expected output: `KWS prediction = yes (expected 'yes')`.

Every stage is verified on the host: the on-device front end produces features
**byte-identical** to the reference TensorFlow front end, and the IREE model
matches the reference TFLite interpreter, so the deterministic on-device run
predicts "yes". Footprint ~439 KB flash / ~80 KB RAM (64 KB IREE arena + 12 KB
front-end pool), within the 512 KB / 128 KB budget.

A `simple_mul` smoke firmware (proving just the runtime) is preserved in git
history if you want the minimal check first.

## Live microphone (remaining board-side step)

The only part not yet wired is sourcing audio from the live mic instead of the
embedded clip. Note a correction to the original plan: the micro:bit v2
microphone is **analog**, not PDM. The Knowles SPU0410LR5H is read via the
**SAADC on P0.05 (AIN3)**, with the mic powered by driving **P0.20 high**, and a
~1.65 V DC bias (use gain ~1/4, subtract the mean, scale to int16). Feed those
samples to `kws_features` exactly as the embedded clip is today; the front end
and model are identical. This stage needs the physical board to calibrate
gain/offset, so it is left as the documented next step.

## Hardware

- BBC micro:bit v2 (Nordic nRF52833, Cortex-M4F, 128 KB RAM)
- Onboard analog MEMS microphone (SAADC on P0.05; enable P0.20)
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
