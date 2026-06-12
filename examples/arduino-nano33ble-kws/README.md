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

## Model artefacts

The artefacts in `models/` (`micro_speech.vmfb`, `micro_speech.o`,
`micro_speech.h`) are **copied from
[`../microbit-v2-kws`](../microbit-v2-kws/)**: both boards are Cortex-M4F, so the
statically linked kernels are identical and need no recompilation. How those
artefacts are regenerated from the `.tflite` model is documented in that
example's README (phase 1).
