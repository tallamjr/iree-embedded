# Results: IREE int8 against TFLite-Micro on the micro:bit v2

Every number below comes from a log file in this folder or in [`../../microbit-v2-kws/results/`](../../microbit-v2-kws/results/). The file name is in the table.

## Set-up

- Device under test (DUT): BBC micro:bit v2, nRF52833, Cortex-M4F, 64 MHz. One board.
- Date: 2026-10-13 (UTC). Commit of the runs: `7db74e333b7a16e7a2c3e702115322e68274ce58`.
- C++ compiler: `arm-none-eabi-g++` 10.3.1 (GNU Arm Embedded Toolchain 10.3-2021.10). Rust: `rustc 1.98.0-nightly (54333ff07 2026-05-22)`.
- IREE: `iree-compile` 3.11.0 (`IREE_PIP_VERSION` in `scripts/iree-version.env`); IREE runtime commit `e4a3b0405d7d23554da26403658d0e8c3c5ecf25`.
- TFLite-Micro: commit `5328e0d852c420f9494a53135529677229dfcdd2`. CMSIS-NN: commit `4ab83cc3cc98fb85ed6dafb55e8ca02f1628dcae`.
- Model: `micro_speech_int8_nosm.tflite`, SHA-256 `34f10a8ea3f5b75919d2c65b4a2990179cc7c04962b77e0094fd69617cb9196a` (the upstream int8 model without softmax). Both runtimes use the same file.
- Feature file: `yes_features_int8.bin`, SHA-256 `83077a32cd04b7bc28bb923faef883207fa9fad6513d2bcb526c9c3660a96c8d`. Both examples hold identical copies.
- Method: 10 warm-up invokes, then 1000 timed invokes of the model only. The DWT cycle counter wraps the invoke call. No logging in the timed region. The instruction cache is off, then on.
- IREE runtime library: the prebuilt v0.1.2 download (`iree-embedded-sys` at commit `4ad21f9`, release tag `v0.1.2`, one section per function). The build used no `IREE_RUNTIME_DIR` or `IREE_SRC_DIR`, and the log header of each run has no runtime path. The earlier runs (`timing_int8-run1.log` to `-run3.log`, 2026-10-13) used the prebuilt v0.1.1 download: 386,648 bytes of flash and 6,233,349 cycles with the cache on. The v0.1.2 runs replace them in the table. The arena bisection ran on the v0.1.1 build. The arena size does not depend on the runtime build.
- Check: every run printed correct logits. TFLM must equal `[-50, -4, 121, -4]` exactly. IREE must be within one step (0.0917) of the reference.

## Results

Cycles are the median of the 1000 invokes of one run. The three runs are in the "runs" column as min to max of the three run medians. Milliseconds are cycles divided by 64,000.

| Item | IREE int8 (`timing_int8`) | TFLM reference (`timing_tflm`) | TFLM CMSIS-NN (`timing_tflm_cmsis`) |
|---|---:|---:|---:|
| Flash total, bytes | 260,344 | 57,280 | 74,756 |
| Runtime-only flash, bytes (share) | 140,548 (54.0%) | 12,704 (22.2%) | 12,436 (16.6%) |
| Static RAM at the default arena, bytes | 65,544 (arena 57,344) | 23,304 (arena 16,384) | 23,304 (arena 16,384) |
| Arena that TFLM reports using, bytes | not reported | 6,692 | 6,724 |
| Smallest working arena, bytes | 52,028 | 6,700 | 6,736 |
| Static RAM at the smallest arena, bytes | about 60,230 (computed) | 13,624 | 13,656 |
| Cache off, median cycles | 7,285,466 | 22,729,049 | 4,398,523 |
| Cache off, milliseconds | 113.83 | 355.14 | 68.73 |
| Cache off, runs min to max | 7,285,466 to 7,285,466 | 22,729,049 to 22,729,049 | 4,398,523 to 4,398,523 |
| Cache on, median cycles | 6,231,589 | 18,457,094 | 3,515,389 |
| Cache on, milliseconds | 97.37 | 288.39 | 54.93 |
| Cache on, runs min to max | 6,231,589 to 6,231,589 | 18,457,094 to 18,457,094 | 3,515,389 to 3,515,389 |
| Cache on, worst single invoke (max over runs) | 6,231,613 | 18,457,096 | 3,515,389 |

All runs of one firmware gave the same median. The spread inside a run is at most 6 cycles.

Source files:

| Number | File |
|---|---|
| IREE cycles | `../../microbit-v2-kws/results/timing_int8-runv012-2.log`, `-runv012-3.log`, `-runv012-4.log` (the first attempt, `-runv012-1`, failed because the probe was missing) |
| TFLM reference cycles | `timing_tflm-run1.log`, `-run2.log`, `-run3.log` |
| TFLM CMSIS-NN cycles | `timing_tflm_cmsis-run1.log`, `-run2.log`, `-run3.log` |
| TFLM arena used (6,692 and 6,724) | the `arena used by TFLite-Micro` line in the `timing_tflm*` logs |
| Smallest arena, TFLM reference | `arena-bisect-arena_tflm-default.log` |
| Smallest arena, TFLM CMSIS-NN | `arena-bisect-arena_tflm-cmsis-nn.log` |
| Smallest arena, IREE | `../../microbit-v2-kws/results/arena-bisect-arena_int8-int8.log` |
| Flash, static RAM, symbol split | `size-report.txt` here (TFLM) and `../../microbit-v2-kws/results/size-report-v0.1.2.txt` (IREE; the older `size-report.txt` there is the v0.1.1 build) |

The smallest arena is the least size, to the byte, that gives correct logits after one init and one invoke. It comes from a bisection on the board with `arena_tflm` and `arena_int8`. The size below fails and the size itself passes (6,699 fails and 6,700 passes; 6,735 and 6,736; 52,027 and 52,028). The IREE arena of 16,384 bytes fails with `OutOfMemory`; the largest failed allocation was 16,824 bytes at that size.

Caveat: "Arena that TFLM reports using" is not a minimum. It changes with the arena size (16-byte alignment of the head and tail, cause not confirmed), and CMSIS-NN reports 6,724 at every size while an arena of 6,724 bytes fails. The smallest working arena is the bisected value.

Reading the numbers (cache on): TFLM reference takes 2.96 times the cycles of IREE. IREE takes 1.77 times the cycles of TFLM with CMSIS-NN. IREE uses 4.5 times the flash of TFLM reference and 7.8 times the smallest arena.

## What is not equal

- IREE kernels come from LLVM. TFLM kernels come from GCC 10.3.1. The compilers differ, so the comparison includes the compiler.
- IREE outputs f32 logits. TFLM outputs int8 logits. The check is exact for TFLM and within one step for IREE.
- There is one input clip ("yes"), one board and one build of each firmware.
- The IREE timed region is `ctx.invoke`. It includes the IREE call and output handling that the TFLM `invoke` does not have.
- The IREE bytecode verifier is linked and adds flash (about 95 KB, see "Symbol split" in the kws README).
- The IREE bytecode and runtime are linked with newlib `printf` and `libm`. The `libc` group of `timing_int8` is 50,705 bytes with the v0.1.2 runtime. It was 84,438 bytes with v0.1.1, where the runtime had one section per object file. The cause is in "Symbol split" in the kws README.
- TFLM CMSIS-NN does not use SIMD code in the depthwise layer. The model has 1 input channel and a channel multiplier of 8. The wrapper then calls the generic `arm_depthwise_conv_s8`, which is scalar C. Only the fully connected layer uses the DSP code.
- The TFLM runtime-only share counts `tflite::`, `flatbuffers::` and `tflm_*` symbols that are not kernels. Grouping rules: `../../microbit-v2-kws/scripts/symbol-split.sh`.
- Static RAM at the smallest arena is for a build of the timing firmware with that arena. The timing runs themselves used the default arenas (16,384 and 57,344 bytes).

## Reproduce

    scripts/board-run.sh <example-dir> <bin> <run-number> [features]
    scripts/arena-bisect.sh <example-dir> <bin> <env-var> <fail-bytes> <pass-bytes> [features]

See the header of each script. `board-run.sh` fails when a log lacks the two `cycles:` lines or the two passing result lines.
