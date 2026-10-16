# Arduino Nano 33 BLE Sense keyword spotting with IREE

Live "yes"/"no" keyword spotting on the Arduino Nano 33 BLE Sense (Nordic
nRF52840, Cortex-M4F): the complete pipeline runs on the chip. It is the same
IREE pipeline as the [micro:bit v2 example](../microbit-v2-kws/README.md):
the same model artefacts, the same bundled pure-Rust audio front end, and the
same safe `iree-embedded` runtime API. The only differences are the board, the
microphone (a PDM mic here, analog on the micro:bit), and how it is flashed:
this firmware loads over USB through the stock Arduino bootloader, **no debug
probe required**. The firmware is `#![forbid(unsafe_code)]` and has been
verified on hardware.

## What you need

- An Arduino Nano 33 BLE Sense and a USB cable (no debug probe).
- The `arm-none-eabi` toolchain (`arm-none-eabi-objcopy` is used to make the
  flashable binary).
- Rust with the `thumbv7em-none-eabihf` target:
  `rustup target add thumbv7em-none-eabihf`.
- `bossac` (the SAM-BA flasher the bootloader speaks), from either:
  - `brew install bossa`, or
  - the Arduino IDE / `arduino-cli core install arduino:mbed_nano` core, which
    bundles `bossac 1.9.1-arduino2`. The flash script finds the bundled copy at
    `~/Library/Arduino15/...` automatically, so it need not be on `PATH`.

Linux works too: the board's serial port appears as `/dev/ttyACM*` and the flash
script probes for it automatically, with `bossac` coming from your distribution's
package or the Arduino tools.

## Flash and run

From this directory:

```sh
cargo run --release
```

The crate's `.cargo/config.toml` sets the target and a `scripts/flash.sh`
runner, so this one command builds the firmware, converts the ELF to a raw
binary with `objcopy`, and writes it over USB with `bossac`.

**The first flash from a board still running an Arduino sketch works
automatically.** The runner sends the standard 1200-baud touch, which asks the
sketch to reboot into the bootloader, then flashes.

**Every later flash needs the double-tap reset dance.** Once this firmware is
running, the board does **not** appear as a USB device: it has no USB stack, so
there is no port to send the 1200-baud touch to. This is intentional and the
board is **not** dead. To re-flash, double-tap the small reset button: the
orange LED slowly fades in and out, which means the bootloader is running and
the USB port is back. Then re-run `cargo run --release`.

## LED legend

The Nano has no probe to print logs to, so the LEDs are the interface:

| LED                            | Meaning                                   |
| ------------------------------ | ----------------------------------------- |
| Two slow orange blinks at boot | Self-test passed; live loop starting      |
| Rapid orange blink, forever    | Fatal error or failed self-test           |
| Centre RGB green, ~0.5 s       | Detected "yes"                            |
| Centre RGB red, ~0.5 s         | Detected "no"                             |
| No LEDs at all                 | Hard fault: double-tap reset and re-flash |

The always-on green **power** LED next to the USB connector is not a signal;
ignore it. The detection feedback comes from the centre RGB LED.

## How it works

Capture uses the onboard **MP34DT05/06 PDM microphone** through `embassy-nrf`'s
typed `Pdm` driver (no raw register pokes, so the capture path is safe Rust).
Three pins matter: data on **P0.25**, clock on **P0.26**, and the mic's power
rail on **P0.17** (driven high-drive, given ~100 ms to settle before capture).
The PDM clock is `1.280 MHz` and the decimation ratio is `80`, giving exactly
`16 kHz` mono PCM.

`Pdm::run_task_sampler` streams continuously into a **250 ms double buffer**:
one buffer is classified in the callback (~130 ms compute, inside the budget)
while the next fills, so capture never pauses. The very first chunk is
discarded to let the PDM decimation filter settle. Each buffer feeds the
**bundled `kws-frontend` crate** (a pure-Rust, `no_std` port of the TFLite-Micro
audio front end, byte-exact against the C reference), which maintains a rolling
1 s spectrogram. That spectrogram is classified by the **micro_speech** model,
run through IREE's static-library loader with the compiled kernels executing in
place from flash.

Two tunables live in `src/main.rs`:

- **`SPEECH_LEVEL`**: the energy gate a window must clear to count as speech.
  Raise it if ambient noise false-triggers detections.
- **`GAIN`**: the digital boost applied after DC removal. Lower `SPEECH_LEVEL`
  or raise `GAIN` if near-mic speech does not trigger.

## Memory

The app is linked at **0x10000** (`memory.x`), behind the untouched 64 KB
Arduino bootloader, so flashing with `bossac` never disturbs the bootloader and
the board stays Arduino-compatible. The firmware uses roughly **424 KB of text**
out of the 960 KB available to the app, and about **93 KB of RAM** out of the
nRF52840's 256 KB. The model's weights and compiled kernels stay in flash and
cost no RAM; the RAM budget is the working state of an inference (the IREE
arena, the audio double buffer, the front-end state, and the stack).

## Size and timing

`scripts/size-report.sh` builds the release firmware and prints a markdown
table for each binary: `.text`, `.rodata`, `.data`, `.bss`, the flash total
and the static RAM, with the share of the nRF52840 (1 MB flash, 256 KB RAM).
Pass an ELF path to report one file without a build. The tool
`arm-none-eabi-size` and `jq` must be on the PATH.

```sh
scripts/size-report.sh
```

`src/bin/timing.rs` is a second firmware that times the model on the device
under test (DUT). It computes the features once from the embedded "yes"
clip. It runs 10 warm-up invokes, then times 1000 invokes of the model alone
with the DWT cycle counter. It does this twice: once with the instruction
cache off (the reset state) and once with it on. It prints min, median, p99
and max, in cycles and in microseconds at 64 MHz, with `defmt`. The
statistics come from the `cycle-stats` crate, which has host tests:
`cargo test -p cycle-stats --target aarch64-apple-darwin`.

With the cache off, the cycle count changes with the code layout in flash.
On the micro:bit v2 the same model took 5.8 to 6.4 million cycles in four
builds. Compare cycle counts only with the cache on, or within one build.

To run the hardware test, put the board in bootloader mode (double-tap reset)
and run:

```sh
cargo run --release --bin timing
```

The `flash.sh` runner flashes the timing firmware. The solid orange LED means the
run is done. The results go out over RTT, so you need a probe to read them
(see "Logs" below). Without a probe, use `timing_usb` (see the next section).
A fast orange blink means an error.

The timing numbers are not yet measured on this board. The USB serial route is untested on hardware. Only the micro:bit v2
has run the timing firmware (see its README).

## Read the timing result over USB serial

The Nano has no debug probe, so `timing_usb` prints the result over USB serial.
It is the same measurement as `timing`. The code is shared in `run_timing`
(`src/lib.rs`). The USB stack starts only after the timed loops end, so USB
interrupts cannot change the cycle counts. The firmware keeps the result text
in RAM. It prints the text every second while a host has the port open, so
a port opened late still gets the result.

The extra cost of the USB build, measured with `arm-none-eabi-size` on the
release ELF files:

| Binary       | text (flash) | data | bss (RAM) |
| ------------ | -----------: | ---: | --------: |
| `timing`     |      430,368 |  2,852 |    77,620 |
| `timing_usb` |      461,256 |  2,900 |    80,564 |

The USB build adds 30,888 bytes of flash and 2,992 bytes of RAM. The 56 KB arena is
the same size as in `timing`. Both fit: 461 KB of 960 KB flash, 83 KB of 256 KB RAM.

Run it with the board in bootloader mode (double-tap reset):

```sh
scripts/board-run.sh 1
```

The script builds `timing_usb` with `--features usb-serial` and flashes it with
`bossac`. It waits for the serial port, reads it with a time limit
(`BOARD_RUN_LIMIT_SECONDS`, default 900), and checks the result with
`scripts/validate-log.sh`. The timed run takes about four minutes, and the
port appears only after it ends. A valid log goes to
`results/timing_usb-run1.log`. An invalid log stays as
`results/timing_usb-run1.log.invalid`. The script stops if either file exists.

The log starts with `# ` header lines: UTC date, commit and tree state,
binary, features, compiler versions, board and serial port. Then come nine
lines of text from the firmware:

```text
iree-embedded KWS timing: 1000 timed invokes
instruction cache off: last invoke classified: yes (expected 'yes')
logits [a, b, c, d]
cycles: min .. median .. p99 .. max ..
microseconds at 64000000 Hz: min .. median .. p99 .. max ..
instruction cache on: ... (the same four lines again)
```

The validator fails when the label is not `yes`, when the third logit is not
the largest, when a cycle line is missing or out of order, or when the
firmware prints an error. The check on the logits is a label check. The
repository has no reference logit values for the float model.

Test the validator on the host with `scripts/test-validate-log.sh`. It builds
a good log from the real micro:bit v2 log
`../microbit-v2-kws/results/timing_int8-run1.log` and breaks it in nine ways
(test names start with `synthetic_`). Test the text formatter with
`cargo test -p cycle-stats --target aarch64-apple-darwin`.

Limits:

- This path has not run on a Nano. No Nano was attached when it was written
  (see https://github.com/tallamjr/iree-embedded/issues/18). The USB code
  builds, but its behaviour on the chip is untested.
- QEMU has no model of the nRF52840 USB device, so the USB path cannot run
  without a board.
- Once `timing_usb` runs, the board is a USB serial device. It does not
  reboot into the bootloader on the 1200 baud touch. Double-tap reset before
  each flash.
- The USB build uses the external crystal as HFCLK and the RTC1 time driver.
  The RTC1 time driver can raise a short interrupt during the timed loops. That can raise the max value.
- Use `timing_usb` as the Nano source for
  https://github.com/tallamjr/iree-embedded/issues/12. Commit the logs only
  after a real run.

## Logs (optional)

`defmt` over RTT is compiled into the firmware but **dormant**: nothing reads it
without a probe, and the Nano's SWD lines are on test pads rather than a header.
If you wire a probe (for example a Raspberry Pi Debug Probe) to the SWD test
pads, you can stream the same logs the binary already emits:

```sh
probe-rs attach --chip nRF52840_xxAA <elf>
```

Use **`attach`** plus `bossac` flashing rather than `probe-rs run`: attaching
leaves the Arduino bootloader intact, whereas flashing through `probe-rs` would
erase it and you would lose the no-probe USB workflow.

## Restoring Arduino

Nothing here is permanent. The bootloader is never touched, so an ordinary
Arduino IDE upload overwrites this firmware and returns the board to stock.

## Package the model (occasional, host)

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

Both boards are Cortex-M4F, so the statically linked kernels are identical to
the [micro:bit v2 example](../microbit-v2-kws/README.md)'s; the committed
`models/micro_speech.{vmfb,o,h}` here are equivalent artefacts (identical
machine code), regenerated independently by the same script.

### Model and test-vector provenance

- `models/micro_speech.tflite`: the original uint8 micro_speech keyword
  spotting model, from the TensorFlow micro_speech example bundle
  [`micro_speech_2020_04_13.zip`](https://storage.googleapis.com/download.tensorflow.org/models/tflite/micro/micro_speech_2020_04_13.zip)
  (member `micro_speech/models/model.tflite`, Apache-2.0). sha256
  `454779dcfea05290759256178162fa86eb17642e7c9cac0de8ea78e1693cee00`. Note
  the model in the tflite-micro repository is the later int8 variant, a
  different model; this uint8 one was only ever distributed in the bundle.
- `models/micro_speech_nosm.mlir`: the canonical intermediate produced by
  stage 1 ("nosm" = no softmax; see above). The cut is at `add_1_dequant`,
  the dequantised f32 input of the model's softmax.
- `models/yes_audio.bin`: the 16-bit mono 16 kHz PCM payload of
  [`yes_1000ms.wav`](https://github.com/tensorflow/tflite-micro/blob/main/tensorflow/lite/micro/examples/micro_speech/testdata/yes_1000ms.wav)
  from tflite-micro (Apache-2.0), byte for byte.
- `models/yes_features.bin`: the audio front end's expected output for
  `yes_audio.bin`; the `kws-frontend` golden tests prove the Rust port
  reproduces it byte-exactly.
