# microbit-v2-tflm: TFLite-Micro timing baseline

This example runs the int8 `micro_speech` model on TFLite-Micro with the reference kernels. It measures cycles per model invoke on the BBC micro:bit v2 (nRF52833, Cortex-M4F, 64 MHz).

It is the baseline for the IREE firmware `timing_int8` in [`../microbit-v2-kws`](../microbit-v2-kws/README.md). Both firmware images run the same model weights on the same fixed feature file. This example does not use IREE.

**No board has run this firmware yet.** The numbers below are build sizes and a host test. There are no cycle counts.

## What it runs

- Model: `models/micro_speech_int8_nosm.tflite`, the upstream int8 model without its softmax. SHA-256 `34f10a8ea3f5b75919d2c65b4a2990179cc7c04962b77e0094fd69617cb9196a`. See [`../microbit-v2-kws/models/int8/README.md`](../microbit-v2-kws/models/int8/README.md) for its source.
- Input: `models/yes_features_int8.bin`, 49 frames of 40 int8 values from the upstream "yes" clip.
- Operators: Reshape, DepthwiseConv2D and FullyConnected. All use the reference kernels. CMSIS-NN is not used yet.
- Expected int8 logits (silence, unknown, yes, no): `[-50, -4, 121, -4]`. The TFLite reference interpreter gives these. TFLite-Micro must match them exactly.

## Fetch and build

You need `git`, GNU make 3.82 or later as `make`, Python 3, network access and `arm-none-eabi-g++` on `PATH`. On macOS, `brew install make` and put its `gnubin` first on `PATH`. The Rust target is `thumbv7em-none-eabihf`.

    scripts/fetch-tflm.sh
    cargo build --release

`scripts/fetch-tflm.sh` does four things:

1. It clones TFLite-Micro at commit `5328e0d852c420f9494a53135529677229dfcdd2` into `tflm-src/`, and checks the commit.
2. It checks the SHA-256 of the upstream model and of the local no-softmax model.
3. It runs `tensorflow/lite/micro/tools/project_generation/create_tflm_tree.py` to make `tflm-tree/` (about 5 MB).
4. It stops if `tflm-tree/` exists. Delete the folder to generate it again.

`tflm-src/` and `tflm-tree/` are git-ignored. I used the tree script, not a full `make` build, because it gives plain source files that `cargo` can compile. It lists all kernels. `scripts/tflm-sources.txt` selects the files that this model needs. Both `build.rs` and the host test read that list.

## Test on the host

    scripts/host-check.sh

The script compiles the same TFLite-Micro files and `src/shim.cc` with the host compiler. It runs `tests/host_check.cc` on the fixed features. It checks that the logits equal the reference exactly. It also checks that a second init and a wrong input size fail.

Result on macOS (Apple clang through `c++`, arm64): logits `-50 -4 121 -4`, equal to the reference. The arena used is 7152 bytes. This host figure uses 8-byte pointers. The Cortex-M figure comes from the board run.

## Test the firmware code in an emulator

    scripts/qemu-check.sh

The script runs `qemu_tflm` on an emulated Cortex-M4F (qemu `mps2-an386`, needs `qemu-system-arm`). It uses the same C++ objects, newlib and libgcc as the firmware. It sets the input before each of 20 invokes and checks the logits. It also shows what a second invoke without a new input gives: `[27, 15, 10, 6]`. Emulator cycle counts mean nothing.

## Set the input before every invoke

The model overwrites its input tensor while it runs, because TFLite-Micro reuses arena memory. A second invoke without a new input reads wrong data. The timing firmware once did this and printed `[14, 14, 14, 14]` on the board. The firmware now calls `set_input` before each invoke, outside the timed region. `scripts/host-check.sh` and `qemu_tflm` test this.

`diag_tflm` prints the status codes, the first bytes of the model, features and input tensor, and the output of one run. Run it with `cargo run --release --bin diag_tflm`.

## Run on a board

Connect the micro:bit v2 with USB. Then run:

    cargo run --release --bin timing_tflm

The probe-rs runner flashes the board and prints the results over defmt RTT. The firmware does 10 warm-up invokes and 1000 timed invokes. It does this twice: first with the instruction cache off, then on. It switches the cache with `nrf_pac::NVMC.icachecnf()`, as `timing_int8` does. The DWT cycle counter times the `invoke()` call only. The output is min, median, p99 and max cycles, the label, the four logits and whether they equal the reference. It also prints the arena that TFLite-Micro uses.

The top-left LED is solid when the run finishes and blinks fast when it fails.

## Build size

`scripts/size-report.sh` prints the size of the release build.

| Item | Bytes |
|---|---:|
| Flash (`.text` + `.rodata` + `.data`) | 57,232 |
| Static RAM (`.data` + `.bss` + `.uninit`) | 23,304 |

The flash figure contains the 18,648 byte model. The static RAM figure contains the 16 KiB arena, the 4,000 byte cycle buffer and the 1,024 byte RTT buffer. The 16 KiB arena is not minimal. For the comparison, count the smallest working arena. The host run needs 7152 bytes. Read the Cortex-M value from the firmware output.

## Build choices

- C++ compiler: `arm-none-eabi-g++` 10.3.1 (GNU Arm Embedded Toolchain 10.3-2021.10). Rust: 1.98.0-nightly, LLVM 22.1.6. TFLite-Micro kernels come from GCC. IREE kernels come from LLVM. The two sets of kernels are not the same code, so the comparison includes the compiler.
- C++ flags: `-std=c++17 -mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 -fno-rtti -fno-exceptions -fno-threadsafe-statics -fno-pic -fno-pie -Os` and `-ffunction-sections -fdata-sections`. The IREE runtime build uses the same CPU flags and `-Os` (MinSizeRel). The linker removes unused sections.
- Defines: `TF_LITE_STRIP_ERROR_STRINGS` and `NDEBUG`, as the upstream release build sets. There is no log output, so `DebugLog` and `MicroPrintf` are not linked. A failure returns a status code, and the firmware prints it.
- Tick functions: `src/platform.cc` returns 0 from `GetCurrentTimeTicks()`. TFLite-Micro does not time itself here. The Rust code times with the DWT counter.
- Start-up: the shim builds the resolver and interpreter with placement new in static storage. The firmware has no `.init_array` section, so no C++ static initialiser needs to run. The link has no `operator new`, `__cxa_*` or `malloc` symbols, and does not use libstdc++.
- The libc heap never grows. The firmware needs no `_sbrk`.
- Unsafe code: only `src/ffi.rs` has it, as the calls into the C++ shim. The firmware `src/bin/timing_tflm.rs` uses `#![forbid(unsafe_code)]`.
- `cycle-stats/` is a copy of the crate in `microbit-v2-kws`, so that this example is a self-contained cargo workspace.

## Open points

- CMSIS-NN kernels are not enabled. That is the next step.
- The arena is a fixed 16 KiB. The smallest working arena on the Cortex-M is not measured.
- Cycle counts are not measured.
