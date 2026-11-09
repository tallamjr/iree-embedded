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

## Size and timing

`scripts/size-report.sh` builds the release firmware and prints a markdown
table for each binary: `.text`, `.rodata`, `.data`, `.bss`, the flash total,
`.uninit` and the static RAM. It gives the share of the nRF52833 (512 KB flash,
128 KB RAM). Pass an ELF path to report one file without a build. The tools
`arm-none-eabi-size` and `jq` must be on the PATH.

```sh
scripts/size-report.sh
```

`src/bin/timing.rs` is a second firmware that times the model on the device
under test (DUT). It computes the features once from the embedded "yes" clip.
It runs 10 warm-up invokes, then times 1000 invokes of the model alone with
the DWT cycle counter. It does this twice: once with the instruction cache
off (the reset state) and once with it on. It prints min, median, p99 and
max, in cycles and in microseconds at 64 MHz, with `defmt`. The statistics
come from the `cycle-stats` crate, which has host tests:
`cargo test -p cycle-stats --target aarch64-apple-darwin`.

To run the hardware test, connect the board and run:

```sh
cargo run --release --bin timing
```

The probe-rs runner flashes the timing firmware and prints the results over
RTT. The top-left LED of the matrix turns on when the run is done. A fast blink
of that LED means an error. Plain `cargo run --release` still starts the main
firmware.

Size report of the release build (static RAM includes the 56 KB arena):

| Binary | Flash | Static RAM |
|---|---:|---:|
| microbit-v2-kws | 437372 B (83.4%) | 94936 B (72.4%) |
| timing | 437460 B (83.4%) | 80472 B (61.4%) |

The timing firmware leaves about 49 KB of RAM for the stack.

### Measured on a micro:bit v2

First hardware run, 2026-10-08, nRF52833 at 64 MHz, embedded "yes" clip,
1000 timed invokes of the model alone:

| Instruction cache | Cycles per invoke | Time per invoke |
|---|---:|---:|
| off | 5,822,985 (min = median = p99 = max) | 90.98 ms |
| on | 4,824,981 (min = median = p99; max +6) | 75.39 ms |

The model labelled the clip "yes" in both runs.

With the cache off, the cycle count depends on where the code sits in flash.
Four builds of the same model gave 5.82 to 6.44 million cycles. With the cache
on, two builds gave 4,824,981 and 4,825,526 cycles. Compare builds only with
the cache on.

### How the counter was checked

- A host timer measured a full run of an earlier build (6,439,009 cycles per
  invoke, cache off). 1,010 invokes took 102.3 s, including set-up. The counter
  predicts 101.6 s. The error is at most 0.7%, because the set-up time is
  included.
- `src/bin/validate.rs` times known delays and three model inputs (the "yes"
  clip, all zeros and random bytes). Observed: `asm::delay(N)` counts about
  1.5 N + 20 cycles.
- With the cache off, the three inputs differ by only 0.2%. With the cache on,
  the "yes" clip and all zeros differ by 0.2% (the random-byte run was not
  captured). The counter responds to the input, but this check
  is weak evidence on its own. The host timer is the stronger check.
- The firmware has no live audio in this test. The input is the recorded clip
  inside the firmware, so nobody needs to speak.

## Int8 model

This section adds the upstream int8 `micro_speech_quantized.tflite` from
tflite-micro. It is the model for a size and speed comparison with
TFLite-Micro (issue #11). Both runtimes run the same weights, the same cut
before the softmax and the same fixed feature file.

A micro:bit v2 has run this firmware (2026-10-13). The logits were correct
and the label was yes. The four logits were -5.870843, -1.6511745, 9.815315
and -1.6511745. They are within one quantisation step of the reference.

| Instruction cache | Cycles (min = median = p99) | Time at 64 MHz | Max |
|---|---:|---:|---:|
| off | 7,279,930 | 113.75 ms | equals min |
| on | 6,233,349 | 97.40 ms | min + 6 |

The run did 10 warm-up invokes and 1000 timed invokes per setting. Three runs gave the same numbers. The raw logs are in `results/` (`timing_int8-run1.log` to `-run3.log`). The table with TFLite-Micro is in [`../microbit-v2-tflm/results/README.md`](../microbit-v2-tflm/results/README.md).

The flash figure depends on which IREE runtime library is linked. The prebuilt v0.1.1 download gives 386,648 bytes. A local `.iree/build/mcu` build gives 391,368 bytes. The three runs above used the prebuilt download: the worktree has no `.iree` folder, no `IREE_RUNTIME_DIR` or `IREE_SRC_DIR` was set, and the build unpacked the download into its output folder. The log header of any later run names the source.

- Files: `models/int8/` (see its `README.md` for the source, the commit and
  the SHA-256 values) and the compiled `models/micro_speech_int8.{o,h,vmfb}`.
- Input: int8, shape `[1,1960]`, from the fixed file
  `models/int8/yes_features_int8.bin`. The upstream front end made it. The
  `kws-frontend` crate does not make int8 values.
- Rebuild the compiled files: `scripts/compile-model.sh int8` (add
  `--from-tflite` to import the model again). The default `uint8` is the
  original flow and does not touch the int8 files.
- Build the firmware: `cargo build --release --features int8 --bin
  timing_int8`. The feature adds a binary. The default binaries do not change.
- Size: `scripts/size-report.sh $CARGO_TARGET_DIR/thumbv7em-none-eabihf/release/timing_int8`.
- Symbol split (runtime, model kernels, model data, HAL, Rust app, C
  library): `scripts/symbol-split.sh $CARGO_TARGET_DIR/thumbv7em-none-eabihf/release/timing_int8`.
  The TFLite-Micro example uses the same script, so the runtime shares can be
  compared. See "Symbol split" below.
- Run: the firmware times the invoke in the same way as `timing`. After each
  pass it prints the label and the four logits. The reference logits are
  -5.8708, -1.6512, 9.8153 and -1.6512 (label yes).

Sizes of `timing_int8` (release build, `scripts/size-report.sh`): flash
386,648 bytes, static RAM 65,912 bytes (including the 56 KB arena). The
compiled files are 18,176 bytes (`.o`) and 23,689 bytes (`.vmfb`).

Board timing, arena size and the comparison with TFLite-Micro: [`../microbit-v2-tflm/results/README.md`](../microbit-v2-tflm/results/README.md). The logs of `timing_int8` are in `results/`.

## Symbol split

`scripts/symbol-split.sh <elf>` sums the sizes of the symbols that
`arm-none-eabi-nm -S -C` lists, in groups. The first matching rule wins. An
alias at the same address counts once. Flash is text, rodata and data. Static
RAM is data and bss. The rest of each section (string literals, padding,
tables without a symbol size) is shown as "not covered by symbols".

| Group | Rule (symbol name) |
|---|---|
| model data | `microbit_v2_kws::int8::VMFB` or `microbit_v2_tflm::MODEL` (the model bytes) |
| model kernels | `tf2onnx$*` and `micro_speech_int8_nosm_linked_library_query*` (IREE compiled code); `arm_*`, `reference_integer_ops::`, and TFLite-Micro depthwise, fully connected and reshape kernel functions |
| runtime | `iree_*`; `tflite::`, `flatbuffers::`, `tflm_*` |
| hal | embassy, nRF PAC, cortex-m, defmt, panic-probe, static-cell symbols |
| rust | other Rust symbols (app code, `core`, `compiler_builtins`, the arena and cycle buffers) |
| libc | everything else: newlib, libm, libgcc, interrupt vectors, C glue |

The runtime-only share is the runtime group divided by the flash total. Limits
of the method: it uses names, not object files. TFLite-Micro helper functions
that the kernels call (for example the quantisation set-up) count as runtime.

Result for `timing_int8` (flash 386,648 bytes, static RAM 65,912 bytes):

| Group | Flash bytes | Flash share | Static RAM bytes |
|---|---:|---:|---:|
| runtime | 222,612 | 57.6% | 116 |
| model kernels | 3,756 | 1.0% | 240 |
| model data | 23,744 | 6.1% | 0 |
| hal | 1,212 | 0.3% | 1,478 |
| rust | 12,774 | 3.3% | 61,370 |
| libc | 84,438 | 21.8% | 2,597 |
| not covered by symbols | 38,112 | 9.9% | 111 |

The IREE runtime includes the bytecode verifier and disassembler
(`iree_vm_bytecode_function_verify`, `iree_vm_bytecode_disassemble_op_impl`,
95 KB together). The libc group holds newlib printf, strtod and libm code
plus the rest of newlib.

Cause (linker map of the local `.iree/build/mcu` build, issue
https://github.com/tallamjr/iree-embedded/issues/19): the IREE runtime was
built without `-ffunction-sections`. Each object file was one section, so the
linker kept every function of an object that the firmware used. The map shows
these chains:

- `disassembler.c.obj`, `allocator.c.obj` and `buffer_view_util.c.obj` call
  `fprintf`. That pulls in newlib `_vfprintf_r`, `_dtoa_r` and `strtod`.
  Nothing in the firmware calls the `fprintf` users
  (`iree_vm_bytecode_trace_disassembly`, `iree_hal_allocator_statistics_fprint`,
  `iree_hal_buffer_view_fprint`).
- `string_view.c.obj` calls `strtod`, `strtof` and `strtol`.
- `dispatch.c.obj` (the bytecode interpreter) calls `pow`, `powf`, `erf`,
  `expm1`, `log1p` and `atan` for the f32 and f64 extension opcodes. That pulls
  in the libm code.

`toolchains/arm-*.cmake` now compile the runtime with `-ffunction-sections
-fdata-sections`, so the Rust linker drops these functions. Measured with a
local runtime build from the same IREE commit (`timing_int8`, `--features int8`):

| Runtime library | Flash bytes | Static RAM bytes | runtime group | libc group |
|---|---:|---:|---:|---:|
| prebuilt v0.1.1 download | 386,648 | 65,912 | 222,612 | 84,438 |
| local build, old flags | 391,368 | 65,912 | 227,130 | 84,634 |
| local build, new flags | 264,480 | 65,544 | 144,564 | 50,849 |

The new flags apply to a new prebuilt runtime release only. The timing and the
logits of the new build are not measured on the board yet.

Host checks:

- `iree-run-module` on the committed feature file gives the same four logits as
  the TFLite reference interpreter (0 quantisation steps apart).
- `cargo test -p iree-embedded` runs the int8 model on the host through
  `Tensor::from_i8`.
- The `timing_int8` binary has no audio front end. Its RAM and flash figures
  are not comparable with `timing`, which includes the front end.

## Smoke check

To prove just the runtime before trusting the heavier model, point `main.rs` at
the committed `simple_mul` artefacts (`models/simple_mul.{vmfb,o,h}`, query
symbol `simple_mul_dispatch_0_library_query`, entry `module.simple_mul`) and
`cargo run`; it must print `[8.0, 8.0, 8.0, 8.0]`.

## Layout

This directory is self-contained. It is its own Cargo workspace: the firmware
binary plus the bundled `kws-frontend/` crate (the pure-Rust audio front end)
and the `cycle-stats/` crate.
The only path dependency that points outside is `iree-embedded`, the reusable
runtime library this example exists to demonstrate.

```
examples/microbit-v2-kws/
  src/            firmware: main.rs, bin/timing.rs, shared lib.rs
  cycle-stats/    cycle statistics for the timing firmware (host-testable)
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
