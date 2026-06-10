# micro:bit v2 keyword spotting with IREE

On-device keyword spotting on the BBC micro:bit v2 (nRF52833, Cortex-M4F): the
full pipeline runs on the chip.

## What it does (works today)

`src/main.rs` runs the **complete live KWS pipeline on-device**:

1. at boot, a self-test classifies an embedded 1-second "yes" recording;
2. then the loop captures **live audio from the onboard analog mic** (SAADC on
   P0.05/AIN3, 16 kHz via EasyDMA, see `src/mic.rs`);
3. the **TFLite-Micro audio front end** (vendored C, runs on-device) turns
   each 1 s window into a 49x40 spectrogram;
4. the **micro_speech** model runs via the IREE runtime (kernels statically
   linked, executing from flash);
5. the predicted keyword is printed over RTT, once per second.

```sh
cd examples/microbit-v2-kws
cargo run          # builds, flashes via probe-rs, streams defmt/RTT
```

Expected output over RTT (then say "yes" or "no" near the mic):

```
INFO self-test on embedded clip: yes (expected 'yes'), logits = [-12.69, -3.90, 9.94, -3.28]
INFO listening: say 'yes' or 'no' near the mic (window = 1 s)
INFO heard: silence | level = 31 | logits = [-0.53, -3.81, -2.66, -6.30]
INFO heard: yes | level = ... | logits = [...]
```

`level` is the post-gain mean absolute sample value: ~96-112 in a quiet room,
200-900 for speech near the mic (a `==> DETECTED` line additionally requires
`level >= 160` and a logit margin of 2). If speech barely moves it, raise
`GAIN` in `src/mic.rs`. Labels are `[silence, unknown, yes, no]` and logits
print in that order.

Every stage also runs on the host: the on-device front end produces features
**byte-identical** to the reference TensorFlow front end, and the IREE model
matches the reference TFLite interpreter, so the deterministic on-device run
predicts "yes". The IREE arena is 64 KB plus a 12 KB front-end pool, within
the 512 KB flash / 128 KB RAM budget. Note `.cargo/config.toml` sets
`DEFMT_LOG=info`; defmt filters at compile time and without it only `error!`
output survives.

A `simple_mul` smoke firmware (proving just the runtime) is preserved in git
history if you want the minimal check first.

## Live microphone

Implemented in `src/mic.rs`. The micro:bit v2 microphone is **analog**, not
PDM: the Knowles SPU0410LR5H is read via the **SAADC on P0.05 (AIN3)**. Three
hardware details matter (all verified against the V2.00 schematic):

- **P0.20 (RUN_MIC) is the mic's power supply**, not an enable line: it feeds
  the mic through 105R and the mic LED through 68R (~19 mA), so the GPIO must
  be **high-drive** or the rail collapses.
- The mic output is **AC-coupled (1 uF) and re-biased to ~97 mV** by a
  33k/1k divider, so the signal at the pin is millivolts.
- The SAADC must therefore measure 0..150 mV: **gain 4 against the internal
  0.6 V reference** (37 uV/LSB), the same operating point CODAL uses. A wider
  range buries speech below one LSB, which looks like a dead mic.

Sampling is 16 kHz single-ended (SAADC local timer), DMA'd straight into RAM;
the firmware subtracts the per-chunk mean and applies a x16 digital gain
before the front end.

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
  --iree-llvmcpu-target-cpu=cortex-m4 \
  --iree-llvmcpu-target-float-abi=hard \
  --iree-llvmcpu-link-embedded=false \
  --iree-llvmcpu-link-static \
  --iree-llvmcpu-static-library-output-path=model.o \
  --iree-vm-target-index-bits=32 \
  model.mlir -o model.vmfb
```

This produces `model.o`, `model.h`, and `model.vmfb`, which live in `models/`
and are committed so the demo builds out of the box. `model.vmfb` is embedded
with `include_vmfb!`; `model.o` is archived by `build.rs` and linked into the
firmware, so the kernels execute in place from flash (the embedded-ELF loader
would instead copy them into RAM, which the nRF52833 cannot spare; its
64 KiB-aligned segments alone need 192 KiB). The firmware registers the
`*_library_query` symbol from `model.h` with
`Device::local_sync_static`.

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

After the boot self-test on the embedded "yes" clip, the firmware classifies
one second of live microphone audio per loop and prints the verdict over RTT.

## Smoke check

To prove just the runtime before trusting the heavier model, point `main.rs` at
the committed `simple_mul` artefacts (`models/simple_mul.{vmfb,o,h}`, query
symbol `simple_mul_dispatch_0_library_query`, entry `module.simple_mul`) and
`cargo run`; it must print `[8.0, 8.0, 8.0, 8.0]`.

## How it works

The model is compiled to a self-contained artefact on the host; the board only
loads it and executes it through IREE's local-sync runtime, wrapped by the safe
`iree-embedded` API. Each inference, raw int16 audio samples are turned into
the audio spectrogram the model expects (the on-device feature pipeline) before
being handed to `invoke`. See the design doc for the full architecture.
