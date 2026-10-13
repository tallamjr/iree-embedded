# microbit-v2-tflm: TFLite-Micro timing baseline

This example runs the int8 `micro_speech` model on TFLite-Micro. It has two builds of the same sources: one with the reference kernels and one with the CMSIS-NN kernels. It measures cycles per model invoke on the BBC micro:bit v2 (nRF52833, Cortex-M4F, 64 MHz).

It is the baseline for the IREE firmware `timing_int8` in [`../microbit-v2-kws`](../microbit-v2-kws/README.md). Both firmware images run the same model weights on the same fixed feature file. This example does not use IREE.

Board results (cycles, smallest arena, flash and RAM) are in [`results/README.md`](results/README.md). The sections below describe the build, the host test and the emulator test.

A first board run found a bug: the firmware did not set the input before each invoke, and the model overwrites its input. It printed wrong logits. The fix is in commit `4e43aa9`. The timing of that first run is not valid.

## What it runs

- Model: `models/micro_speech_int8_nosm.tflite`, the upstream int8 model without its softmax. SHA-256 `34f10a8ea3f5b75919d2c65b4a2990179cc7c04962b77e0094fd69617cb9196a`. See [`../microbit-v2-kws/models/int8/README.md`](../microbit-v2-kws/models/int8/README.md) for its source.
- Input: `models/yes_features_int8.bin`, 49 frames of 40 int8 values from the upstream "yes" clip.
- Operators: Reshape, DepthwiseConv2D and FullyConnected. Reshape always uses the reference kernel. Depthwise and fully connected use the reference kernels in `timing_tflm` and the CMSIS-NN kernels in `timing_tflm_cmsis`.
- Expected int8 logits (silence, unknown, yes, no): `[-50, -4, 121, -4]`. The TFLite reference interpreter gives these. TFLite-Micro must match them exactly.

## Fetch and build

You need `git`, GNU make 3.82 or later as `make`, Python 3, network access and `arm-none-eabi-g++` on `PATH`. On macOS, `brew install make` and put its `gnubin` first on `PATH`. The Rust target is `thumbv7em-none-eabihf`.

    scripts/fetch-tflm.sh
    cargo build --release --bin timing_tflm
    cargo build --release --features cmsis-nn --bin timing_tflm_cmsis

The first build gives `timing_tflm` (reference kernels). The second gives `timing_tflm_cmsis` (CMSIS-NN kernels). The feature `cmsis-nn` picks the kernels, so a build with the feature must name `--bin timing_tflm_cmsis`. A build of `timing_tflm` with the feature fails on purpose, so the name always tells which kernels the file holds. `qemu_tflm` follows the feature and prints `kernels reference` or `kernels cmsis-nn`.

`scripts/fetch-tflm.sh` does these steps:

1. It clones TFLite-Micro at commit `5328e0d852c420f9494a53135529677229dfcdd2` into `tflm-src/`, and checks the commit.
2. It checks the SHA-256 of the upstream model and of the local no-softmax model.
3. It checks that the download scripts of that commit pin the CMSIS-NN and CMSIS-Core revisions below.
4. It runs `tensorflow/lite/micro/tools/project_generation/create_tflm_tree.py` twice. The first run makes `tflm-tree/` (reference kernels, about 5 MB). The second run adds `OPTIMIZED_KERNEL_DIR=cmsis_nn` and makes `tflm-tree-cmsis-nn/` (about 10 MB). The TFLite-Micro download scripts fetch the CMSIS zips and check their MD5 sums.
5. It skips a tree that exists. Delete the folder to generate it again.

Pinned revisions (from `tensorflow/lite/micro/tools/make/ext_libs/` at the TFLite-Micro commit above):

| Part | Repository | Commit | MD5 of the zip |
|---|---|---|---|
| CMSIS-NN | ARM-software/CMSIS-NN | `4ab83cc3cc98fb85ed6dafb55e8ca02f1628dcae` | `c960031e7d10cf31c477fb34ef460106` |
| CMSIS-Core (CMSIS 6) | ARM-software/CMSIS_6 | `5782d6f8057906d360f4b95ec08a2354afe5c9b9` | `563e7c6465f63bdc034359e9b536b366` |

`tflm-src/`, `tflm-tree/` and `tflm-tree-cmsis-nn/` are git-ignored. I used the tree script, not a full `make` build, because it gives plain source files that `cargo` can compile. It lists all kernels. These lists in `scripts/` select the files that this model needs. `build.rs` and the host test read them.

| List | Content |
|---|---|
| `tflm-sources.txt` | TFLite-Micro files in both builds |
| `tflm-kernels-reference.txt` | reference depthwise and fully connected kernels |
| `tflm-kernels-cmsis-nn.txt` | CMSIS-NN depthwise and fully connected kernels (they replace the reference ones) |
| `cmsis-nn-sources.txt` | 28 CMSIS-NN C files. I found them as the symbols that the two CMSIS-NN kernels need. The s4 and s16 files are on the list because the kernels call them |

## Test on the host

    scripts/host-check.sh              # reference kernels
    scripts/host-check.sh cmsis-nn     # CMSIS-NN kernels

The script compiles the same TFLite-Micro files and `src/shim.cc` with the host compiler. It runs `tests/host_check.cc` on the fixed features. It checks that the logits equal the reference exactly. It also checks that a second init and a wrong input size fail. The `cmsis-nn` variant also compiles the CMSIS-NN C files. The host has no Cortex-M SIMD instructions, so CMSIS-NN uses its plain C code.

Result on macOS (Apple clang through `c++` and `cc`, arm64), both variants: logits `-50 -4 121 -4`, equal to the reference. The arena used is 7152 bytes (reference) and 7200 bytes (CMSIS-NN). These host figures use 8-byte pointers. The Cortex-M figure comes from the board run.

Which CMSIS-NN functions the model calls (found with a debugger on the host check): the depthwise layer calls `arm_depthwise_conv_wrapper_s8`, which calls the generic `arm_depthwise_conv_s8`. The wrapper picks the 3x3 and `_opt` functions only when the channel multiplier is 1. This model has 1 input channel and a multiplier of 8, so it takes the generic function. That function has no DSP code for the Cortex-M4. The fully connected layer calls `arm_fully_connected_wrapper_s8`, then `arm_fully_connected_s8`. That path uses the DSP instructions.

## Test the firmware code in an emulator

    scripts/qemu-check.sh              # reference kernels
    scripts/qemu-check.sh cmsis-nn     # CMSIS-NN kernels

The script runs `qemu_tflm` on an emulated Cortex-M4F (qemu `mps2-an386`, needs `qemu-system-arm`). It uses the same C++ objects, newlib and libgcc as the firmware. It sets the input before each of 20 invokes and checks the logits. It also shows what a second invoke without a new input gives: `[27, 15, 10, 6]`. Emulator cycle counts mean nothing.

Result: both variants give logits `[-50, -4, 121, -4]`. The arena used is 6692 bytes (reference) and 6724 bytes (CMSIS-NN). These are 4-byte-pointer figures from the emulated Cortex-M4F. The `timing_tflm_cmsis` code has DSP instructions (for example `smlad` and `sxtb16`) and the emulator ran them with correct results. The reference ELF file has none. The emulator does not show whether the DSP code is fast on the real chip.

## Set the input before every invoke

The model overwrites its input tensor while it runs, because TFLite-Micro reuses arena memory. A second invoke without a new input reads wrong data. The timing firmware once did this and printed `[14, 14, 14, 14]` on the board. The firmware now calls `set_input` before each invoke, outside the timed region. `scripts/host-check.sh` and `qemu_tflm` test this.

`diag_tflm` prints the status codes, the first bytes of the model, features and input tensor, and the output of one run. Run it with `cargo run --release --bin diag_tflm`.

## Run on a board

Connect the micro:bit v2 with USB. Then run:

    cargo run --release --bin timing_tflm

The probe-rs runner flashes the board and prints the results over defmt RTT. The firmware does 10 warm-up invokes and 1000 timed invokes. It does this twice: first with the instruction cache off, then on. It switches the cache with `nrf_pac::NVMC.icachecnf()`, as `timing_int8` does. The DWT cycle counter times the `invoke()` call only. The output is min, median, p99 and max cycles, the label, the four logits and whether they equal the reference. It also prints the arena that TFLite-Micro uses.

The top-left LED is solid when the run finishes and blinks fast when it fails.

## Build size

`scripts/size-report.sh` builds both binaries (in `target/reference` and `target/cmsis-nn`) and prints the sizes and the symbol split of each.

| Item | `timing_tflm` (reference) | `timing_tflm_cmsis` |
|---|---:|---:|
| Flash (`.text` + `.rodata` + `.data`), bytes | 57,280 | 74,756 |
| Static RAM (`.data` + `.bss` + `.uninit`), bytes | 23,304 | 23,304 |

The flash figure contains the 18,648 byte model. The static RAM figure contains the 16 KiB arena, the 4,000 byte cycle buffer and the 1,024 byte RTT buffer. The 16 KiB arena is not minimal. The smallest working arena on the board is 6700 bytes (reference) and 6736 bytes (CMSIS-NN); see `results/README.md`.

### Symbol split

The split uses the same script as the IREE firmware: [`../microbit-v2-kws/scripts/symbol-split.sh`](../microbit-v2-kws/scripts/symbol-split.sh). The README of that example has the table of grouping rules. The runtime-only share is the "runtime" group divided by the flash total.

| Group | Reference flash | Share | CMSIS-NN flash | Share |
|---|---:|---:|---:|---:|
| runtime | 12,704 | 22.2% | 12,436 | 16.6% |
| model kernels | 6,354 | 11.1% | 23,098 | 30.9% |
| model data | 18,656 | 32.6% | 18,656 | 25.0% |
| hal | 1,168 | 2.0% | 1,168 | 1.6% |
| rust | 6,274 | 11.0% | 6,274 | 8.4% |
| libc | 8,914 | 15.6% | 9,898 | 13.2% |
| not covered by symbols | 3,210 | 5.6% | 3,226 | 4.3% |

The CMSIS-NN kernels are 16,744 bytes larger than the reference kernels. They include code for s4 and s16 types that this model does not use, because the kernel functions call them.

## Build choices

- C++ compiler: `arm-none-eabi-g++` 10.3.1 (GNU Arm Embedded Toolchain 10.3-2021.10). Rust: 1.98.0-nightly, LLVM 22.1.6. TFLite-Micro kernels come from GCC. IREE kernels come from LLVM. The two sets of kernels are not the same code, so the comparison includes the compiler.
- C++ flags: `-std=c++17 -mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 -fno-rtti -fno-exceptions -fno-threadsafe-statics -fno-pic -fno-pie -Os` and `-ffunction-sections -fdata-sections`. The IREE runtime build uses the same CPU flags and `-Os` (MinSizeRel). The linker removes unused sections.
- Defines: `TF_LITE_STRIP_ERROR_STRINGS` and `NDEBUG`, as the upstream release build sets. The CMSIS-NN build adds `CMSIS_NN` (it selects the CMSIS-NN registrations in the kernel headers, as the upstream Makefile does for `OPTIMIZED_KERNEL_DIR=cmsis_nn`), `ARM_NN_ENABLE_F16=0` and `ARM_NN_ENABLE_F32=0`. No define is needed for the DSP code: `-mcpu=cortex-m4` makes GCC define `__ARM_FEATURE_DSP`, and CMSIS-NN then defines `ARM_MATH_DSP`. The CMSIS-NN C files use the same CPU flags and `-Os` as the C++ files. There is no log output, so `DebugLog` and `MicroPrintf` are not linked. A failure returns a status code, and the firmware prints it.
- Tick functions: `src/platform.cc` returns 0 from `GetCurrentTimeTicks()`. TFLite-Micro does not time itself here. The Rust code times with the DWT counter.
- Start-up: the shim builds the resolver and interpreter with placement new in static storage. The firmware has no `.init_array` section, so no C++ static initialiser needs to run. The link has no `operator new`, `__cxa_*` or `malloc` symbols, and does not use libstdc++.
- The libc heap never grows. The firmware needs no `_sbrk`.
- Unsafe code: only `src/ffi.rs` has it, as the calls into the C++ shim. The firmware `src/bin/timing_tflm.rs` uses `#![forbid(unsafe_code)]`.
- `cycle-stats/` is a copy of the crate in `microbit-v2-kws`, so that this example is a self-contained cargo workspace.

## Results

Board logs, the results table and the arena bisection are in [`results/README.md`](results/README.md). `scripts/board-run.sh` makes a log. `scripts/arena-bisect.sh` finds the smallest arena. `TFLM_ARENA_BYTES` sets the arena size at build time.

## Open points

- One board and one input clip were measured. See `results/README.md` for what is not equal between the two runtimes.
- The CMSIS-NN depthwise layer has no SIMD code for this model, as described above.
