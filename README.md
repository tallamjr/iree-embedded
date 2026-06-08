# iree-embedded

An embedded `no_std` Rust runtime for machine-learning inference on Cortex-M
microcontrollers, built on [IREE](https://iree.dev)'s bare-metal C runtime.

## What and why

IREE is an ahead-of-time compiler plus a small runtime: a model is compiled
once on a host into a self-contained artefact, and the device only loads and
executes it through a thin executor. `iree-embedded` provides safe, idiomatic
Rust bindings over the _runtime_ half of that split, for constrained `no_std`
embedded systems, a niche not currently served by mature Rust crates (`ort`,
`tract`, and `candle` are all host/`std`).

## v1 target

- **Board**: BBC micro:bit v2 (Nordic nRF52833, Cortex-M4F, 128 KB RAM).
- **Demo**: live keyword spotting ("yes"/"no") from the onboard PDM microphone.
- **Stack**: `embassy-nrf`, `defmt` over RTT, `probe-rs run`.

## Workspace

| Crate                      | Purpose                                                                   |
| -------------------------- | ------------------------------------------------------------------------- |
| `crates/iree-embedded-sys` | Raw `bindgen` FFI to the prebuilt IREE runtime (the only `unsafe` crate). |
| `crates/iree-embedded`     | Safe `no_std` public API.                                                 |
| `examples/microbit-v2-kws` | Keyword-spotting demo binary.                                             |

## Building (planned)

The IREE runtime is built out of band and linked as a static library:

```sh
just build-runtime-host   # native macOS arm64 build, for host unit tests
just build-runtime-mcu    # thumbv7em-none-eabihf cross-build, for the board
```

Models are compiled manually with `iree-compile` (documented per release) into
a static-library `.o` plus a `.vmfb`, which the firmware links and embeds.

## Licence

To be decided before first release (Apache-2.0 or MIT/Apache dual, to match the
Rust and IREE ecosystems).
