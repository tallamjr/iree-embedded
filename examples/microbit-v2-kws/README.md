# micro:bit v2 keyword spotting with IREE

On-device keyword spotting on the BBC micro:bit v2 (nRF52833, Cortex-M4F): the
full pipeline runs on the chip.

## What it does (works today)

`src/main.rs` runs the **complete live KWS pipeline on-device**:

1. at boot, a self-test classifies an embedded 1-second "yes" recording;
2. then the loop captures **live audio from the onboard analog mic** (SAADC on
   P0.05/AIN3, 16 kHz, via the `embassy-nrf` SAADC HAL);
3. the **`kws-frontend` crate** (a pure-Rust, `no_std` port of the TFLite-Micro
   audio front end, byte-exact against the C reference) turns each 1 s window
   into a 49x40 spectrogram;
4. the **micro_speech** model runs via the IREE runtime (kernels statically
   linked, executing from flash);
5. the predicted keyword is printed over RTT, once per second.

Everything above is **pure Rust**. The only non-Rust artefacts are the IREE
runtime (a vendored C library this project provides safe bindings over) and the
model itself (`micro_speech.o`/`micro_speech.vmfb`, the ahead-of-time output of
`iree-compile`, i.e. machine code, not source).

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
`GAIN` in `src/main.rs`. Labels are `[silence, unknown, yes, no]` and logits
print in that order.

Every stage also runs on the host: `kws-frontend`'s golden-vector tests assert
its output is **byte-identical** to the reference TensorFlow front end (so the
deterministic on-device run is provably the same as the hardware-validated C
version it replaced), and the IREE model matches the reference TFLite
interpreter. The IREE arena is 56 KB and the front end ~14 KB of inline state,
within the 512 KB flash / 128 KB RAM budget. Note `.cargo/config.toml` sets
`DEFMT_LOG=info`; defmt filters at compile time and without it only `error!`
output survives.

A `simple_mul` smoke firmware (proving just the runtime) is preserved in git
history if you want the minimal check first.

## Live microphone

Capture uses the **`embassy-nrf` device HAL** (not raw registers): its typed
`Saadc` driver replaces the manual SAADC/PPI/DMA register pokes a PAC needs,
so the capture path is safe Rust. The micro:bit v2 microphone is **analog**,
not PDM: the Knowles SPU0410LR5H is read via the **SAADC on P0.05 (AIN3)**.
Three hardware details matter (all verified against the V2.00 schematic):

- **P0.20 (RUN_MIC) is the mic's power supply**, not an enable line: it feeds
  the mic through 105R and the mic LED through 68R (~19 mA), so the GPIO must
  be **high-drive** (`OutputDrive::HighDrive`) or the rail collapses.
- The mic output is **AC-coupled (1 uF) and re-biased to ~97 mV** by a
  33k/1k divider, so the signal at the pin is millivolts.
- The SAADC must therefore measure 0..150 mV: **gain 4 against the internal
  0.6 V reference** (37 uV/LSB), the same operating point CODAL uses. A wider
  range buries speech below one LSB, which looks like a dead mic.

`Saadc::run_task_sampler` samples continuously at 16 kHz (internal timer,
16 MHz / 1000) into a double buffer; each 250 ms buffer is classified in the
callback (~130 ms compute, inside the budget) while the next one fills, so
capture never pauses. The firmware subtracts the per-chunk mean and applies a
x16 digital gain before the front end.

## Hardware

- BBC micro:bit v2 (Nordic nRF52833, Cortex-M4F, 128 KB RAM)
- Onboard analog MEMS microphone (SAADC on P0.05; powered via P0.20)
- A USB cable (the onboard DAPLink debugger handles flashing and logging, no
  external probe needed)

## Prerequisites (host, one-time)

- Rust target: `rustup target add thumbv7em-none-eabihf`
- [`probe-rs`](https://probe.rs/) installed
- The pinned `iree-compile` in `.venv` (stage 2 of Phase 1): `compile-model.sh`
  runs `$ROOT/.venv/bin/iree-compile`, so install it with
  `python3 -m venv .venv && .venv/bin/pip install "iree-base-compiler==3.11.0"`
  (see `scripts/iree-version.env`), or point `IREE_COMPILE` at an existing binary
- The IREE runtime cross-built for the board: `scripts/build-runtime-mcu.sh`
- The model is committed at `models/micro_speech.tflite` (provenance below), so
  nothing needs fetching

## Two-phase workflow

### Phase 1: package the model (occasional, host)

Everything is scripted; regenerate the committed artefacts with:

```sh
scripts/compile-model.sh                # MLIR -> .o/.h/.vmfb (stage 2)
scripts/compile-model.sh --from-tflite  # also tflite -> MLIR first (stage 1)
```

The pipeline behind it (`models/` carries every input):

```
micro_speech.tflite      original quantised model (see provenance below)
  -> tf2onnx 1.17.0      TFLite -> ONNX (opset 13), graph truncated at the
                         pre-softmax tensor: the firmware argmaxes raw
                         logits, so softmax is dead weight on an M4
  -> iree-import-onnx    ONNX -> MLIR (micro_speech_nosm.mlir, committed)
  -> iree-compile        -> micro_speech.o/.h (static kernels) + .vmfb (VM
                         program), with the flags in compile-model.sh
```

Stage 2 needs only the pinned compiler
(`pip install "iree-base-compiler==3.11.0"`, see `scripts/iree-version.env`);
stage 1 additionally needs the conversion toolchain described in
`scripts/requirements-model.txt` (Python 3.11 via uv; TensorFlow and tf2onnx).
A stage-1 rerun differs cosmetically from the committed MLIR (tf2onnx's
generated names are not run-stable) while compiling identically. CI runs
stage 2 on every push and rebuilds this firmware against the fresh artefacts.

The artefacts live in `models/` and are committed so the demo builds out of the
box. `micro_speech.vmfb` is embedded with `include_vmfb!`; `micro_speech.o` is
archived by `build.rs` and linked into the firmware, so the kernels execute in
place from flash (the embedded-ELF loader would instead copy them into RAM,
which the nRF52833 cannot spare; its 64 KiB-aligned segments alone need
192 KiB). The firmware registers the `*_library_query` symbol from
`micro_speech.h` with `Device::local_sync_static`.

The compile target triple **must match** the firmware target: the kernels in
`micro_speech.o` are real Cortex-M machine code.

### Model and test-vector provenance

- `models/micro_speech.tflite`: the original uint8 micro_speech keyword
  spotting model, from the TensorFlow micro_speech example bundle
  [`micro_speech_2020_04_13.zip`](https://storage.googleapis.com/download.tensorflow.org/models/tflite/micro/micro_speech_2020_04_13.zip)
  (member `micro_speech/models/model.tflite`, Apache-2.0). sha256
  `454779dcfea05290759256178162fa86eb17642e7c9cac0de8ea78e1693cee00`. Note
  the model in the tflite-micro repository is the later int8 variant, a
  different model; this uint8 one was only ever distributed in the bundle.
- `models/micro_speech_nosm.mlir`: the canonical intermediate produced by
  stage 1 ("nosm" = no softmax; see Phase 1). The cut is at `add_1_dequant`,
  the dequantised f32 input of the model's softmax.
- `models/simple_mul.mlir`: copied from the pinned IREE checkout
  (`runtime/src/iree/runtime/demo/simple_mul.mlir`, Apache-2.0).
- `models/yes_audio.bin`: the 16-bit mono 16 kHz PCM payload of
  [`yes_1000ms.wav`](https://github.com/tensorflow/tflite-micro/blob/main/tensorflow/lite/micro/examples/micro_speech/testdata/yes_1000ms.wav)
  from tflite-micro (Apache-2.0), byte for byte.
- `models/yes_features.bin`: the audio front end's expected output for
  `yes_audio.bin`; the `kws-frontend` golden tests prove the Rust port
  reproduces it byte-exactly.

### Phase 2: deploy (your inner loop)

```sh
cargo run --release
```

The crate's `.cargo/config.toml` sets the target and a probe-rs runner, so this
one command:

1. builds the firmware for `thumbv7em-none-eabihf` (linking the safe crate, the
   prebuilt IREE runtime, `micro_speech.o`, and the embedded `micro_speech.vmfb`),
2. flashes it to the nRF52833 over USB (`probe-rs run --chip nRF52833_xxAA`),
3. attaches and streams `defmt` logs back to your terminal.

After the boot self-test on the embedded "yes" clip, the firmware classifies
one second of live microphone audio per loop and prints the verdict over RTT.

## Smoke check

To prove just the runtime before trusting the heavier model, point `main.rs` at
the committed `simple_mul` artefacts (`models/simple_mul.{vmfb,o,h}`, query
symbol `simple_mul_dispatch_0_library_query`, entry `module.simple_mul`) and
`cargo run`; it must print `[8.0, 8.0, 8.0, 8.0]`.

## Layout

This directory is self-contained. It is its own Cargo workspace: the firmware
binary plus a bundled `kws-frontend/` crate (the pure-Rust audio front end).
The only path dependency that points outside is `iree-embedded`, the reusable
runtime library this example exists to demonstrate.

```
examples/microbit-v2-kws/
  src/            firmware (no_std, no_main, thumbv7em)
  kws-frontend/   bundled audio front end (no_std lib, host-testable)
  models/         iree-compile output + the embedded "yes" clip
```

The front-end golden-vector tests run on the host. Because `.cargo/config.toml`
pins the build to `thumbv7em-none-eabihf` for the firmware, name the host
target explicitly:

```sh
cargo test -p kws-frontend --target $(rustc -vV | sed -n 's/host: //p')
```

## How it works

The model is compiled to a self-contained artefact on the host; the board only
loads it and executes it through IREE's local-sync runtime, wrapped by the safe
`iree-embedded` API. Each inference, raw int16 audio samples are turned into
the audio spectrogram the model expects (`kws-frontend`) before being handed to
`invoke`. The Memory layout section below documents the on-device footprint.

## Memory layout

Actual addresses and sizes from the built ELF (`arm-none-eabi-nm` / `size`):

```
nRF52833 RAM map: 0x20000000 .. 0x20020000 (128 KiB total)

+------------+----------------------------------------+-----------+------------------------------------------+
| Address    | Region                                 | Size      | What lives there                         |
+------------+----------------------------------------+-----------+------------------------------------------+
| 0x20020000 | _stack_start (top of RAM)              |           |                                          |
|     ...    |   STACK (grows downward)               | ~35 KiB   | main + IREE call frames + the 8 KiB VM   |
|            |                                        |           | stack alloca'd per invoke + kernel       |
|            |                                        |           | frames. NO guard below: overflow walks   |
|            |                                        |           | straight into the RTT buffer and bss.    |
| 0x200172F4 | __sheap (dead: _sbrk always fails)     |     0     | newlib heap permanently disabled         |
+------------+----------------------------------------+-----------+------------------------------------------+
| .bss       | (zeroed at boot)                       | 89.9 KiB  |                                          |
| 0x200162B0 |   embassy + nRF HAL + defmt RTT ring   | ~4 KiB    | async executor state, log ring buffer    |
| 0x200129C0 |   FRONTEND (kws_frontend::Frontend)    | 14.2 KiB  | all front-end state inline: FFT twiddles,|
|            |                                        |           | filterbank weights, rolling spectrogram  |
| 0x2000EB40 |   SAADC_BUFS (2 x 4000 x i16)          | 15.6 KiB  | embassy-nrf EasyDMA double buffer        |
| 0x20000B40 |   HEAP     (IREE arena, talc)          |  56 KiB   | EVERY IREE allocation: instance, device, |
|            |                                        |           | context, tensors, transient blocks       |
+------------+----------------------------------------+-----------+------------------------------------------+
| .data      | (copied from flash at boot)            |  2.8 KiB  | _SEGGER_RTT control block @ 0x20000008,  |
| 0x20000000 |                                        |           | newlib tables (impure_data, locale)      |
+------------+----------------------------------------+-----------+------------------------------------------+

FLASH (512 KiB): ~425 KiB used. Model weights AND kernel machine code execute in place from here: zero RAM.
```

Note the split: the model's _static_ parts (trained weights and compiled
kernels, both read-only) stay in flash and cost no RAM at all. The RAM map
above is the _working state_ of running an inference: input/output tensors and
per-layer activation buffers (allocated from `HEAP`), audio capture, the
front-end state, and the stack. Mutable state cannot live in flash.

### Arenas, and where the "heap" went

There is **no system heap**: `_sbrk` always fails, so newlib's `malloc` can
never grow memory (the C front end that once needed a `malloc`/`calloc` shim
is gone). The one dynamic allocator is the IREE arena:

- **`HEAP` (56 KiB, the IREE arena).** Every IREE runtime allocation goes
  through the [talc](https://crates.io/crates/talc) allocator running inside
  this static array. It behaves like a heap (alloc and free; IREE's objects
  are refcounted), but its footprint is a compile-time constant.

Everything else is a plain static: `FRONTEND` holds the whole front-end state
inline (no heap, no `unsafe`), and `SAADC_BUFS` is the embassy-nrf capture
double buffer.

If you know TFLite-Micro's `tensor_arena` from Arduino sketches, `HEAP` is its
direct counterpart: a fixed buffer the runtime must live inside, sized
empirically. The difference is the machinery. TFLM's arena is a one-shot
planner (a fixed tensor layout computed at init, nothing freed) and reports
its peak via `arena_used_bytes()`; IREE needs a real general-purpose allocator
because buffers come and go per invoke, so undersizing shows up at runtime.
That is what `LAST_ALLOC_FAIL_SIZE` is for: an out-of-memory failure reports
the size of the allocation that did not fit.

The stack has **no guard**: it grows down from the top of RAM into bss. If it
overflows (deep IREE call chains plus the 8 KiB VM stack), it silently
corrupts the RTT buffer and whatever bss sits highest, which presents as
"firmware went quiet" rather than a fault. The embassy executor and nRF HAL
add ~3 KiB of bss, leaving ~35 KiB of stack, so keep the static buffers lean;
this is why the 1 s model window is stored as features (1960 bytes inside
`FRONTEND`) rather than 32 KiB of audio. The boot self-test is the canary: it
runs the same inference path as the live loop, so if `self-test on embedded
clip: yes` prints, the stack and arena are large enough. If the firmware goes
quiet before that line, shrink `HEAP` to give the stack more room.
