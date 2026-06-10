# micro:bit v2 keyword spotting with IREE

On-device keyword spotting on the BBC micro:bit v2 (nRF52833, Cortex-M4F): the
full pipeline runs on the chip.

## What it does (works today)

`src/main.rs` runs the **complete live KWS pipeline on-device**:

1. at boot, a self-test classifies an embedded 1-second "yes" recording;
2. then the loop captures **live audio from the onboard analog mic** (SAADC on
   P0.05/AIN3, 16 kHz via EasyDMA, see `src/mic.rs`);
3. the **`kws-frontend` crate** (a pure-Rust, `no_std` port of the TFLite-Micro
   audio front end, byte-exact against the C reference) turns each 1 s window
   into a 49x40 spectrogram;
4. the **micro_speech** model runs via the IREE runtime (kernels statically
   linked, executing from flash);
5. the predicted keyword is printed over RTT, once per second.

Everything above is **pure Rust**. The only non-Rust artefacts are the IREE
runtime (a vendored C library this project provides safe bindings over) and the
model itself (`model.o`/`model.vmfb`, the ahead-of-time output of
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
`GAIN` in `src/mic.rs`. Labels are `[silence, unknown, yes, no]` and logits
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

## Memory layout

Actual addresses and sizes from the built ELF (`arm-none-eabi-nm` / `size`):

```
nRF52833 RAM map: 0x20000000 .. 0x20020000 (128 KiB total)

+------------+----------------------------------------+-----------+------------------------------------------+
| Address    | Region                                 | Size      | What lives there                         |
+------------+----------------------------------------+-----------+------------------------------------------+
| 0x20020000 | _stack_start (top of RAM)              |           |                                          |
|     ...    |   STACK (grows downward)               | ~39 KiB   | main + IREE call frames + the 8 KiB VM   |
|            |                                        |           | stack alloca'd per invoke + kernel       |
|            |                                        |           | frames. NO guard below: overflow walks   |
|            |                                        |           | straight into the RTT buffer and bss.    |
| 0x200167B4 | __sheap (dead: _sbrk always fails)     |     0     | newlib heap permanently disabled         |
+------------+----------------------------------------+-----------+------------------------------------------+
| .bss       | (zeroed at boot)                       | 87.1 KiB  |                                          |
| 0x200129C0 |   FRONTEND (kws_frontend::Frontend)    | 14.2 KiB  | all front-end state inline: FFT twiddles,|
|            |                                        |           | filterbank weights, rolling spectrogram  |
| 0x2000EB40 |   MIC_BUF  (2 x 4000 x i16)            | 15.6 KiB  | SAADC EasyDMA double buffer              |
| 0x20000B40 |   HEAP     (IREE arena, talc)          |  56 KiB   | EVERY IREE allocation: instance, device, |
|            |                                        |           | context, tensors, transient blocks       |
|            |   (defmt RTT ring buffer also in bss)  |   1 KiB   | log bytes read out by probe-rs           |
+------------+----------------------------------------+-----------+------------------------------------------+
| .data      | (copied from flash at boot)            |  2.8 KiB  | _SEGGER_RTT control block @ 0x20000008,  |
| 0x20000000 |                                        |           | newlib tables (impure_data, locale)      |
+------------+----------------------------------------+-----------+------------------------------------------+

FLASH (512 KiB): ~425 KiB used. Model weights AND kernel machine code execute in place from here: zero RAM.
```

Note the split: the model's *static* parts (trained weights and compiled
kernels, both read-only) stay in flash and cost no RAM at all. The RAM map
above is the *working state* of running an inference: input/output tensors and
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
inline (no heap, no `unsafe`), and `MIC_BUF` is the audio capture ring.

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
"firmware went quiet" rather than a fault. Keep static buffers lean; this is
why the 1 s model window is stored as features (1960 bytes inside `FRONTEND`)
rather than 32 KiB of audio.
