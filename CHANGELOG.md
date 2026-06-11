# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Safe `no_std` API (`iree-embedded`) over IREE's bare-metal runtime: RAII
  `Arena`, `Instance`, `Device`, `Context`, `Tensor`, `Error`.
- Static-library executable loader (`Device::local_sync_static`) so model
  kernels execute in place from flash.
- `kws-frontend`: a pure-Rust, byte-exact port of the TFLite-Micro audio front
  end (bundled in the micro:bit example).
- Live keyword-spotting firmware example for the BBC micro:bit v2, using the
  `embassy-nrf` SAADC HAL, verified end to end on hardware.
- CI that builds the IREE runtime per target (host, cortex-m4f/m7/m33) and
  publishes prebuilt artefacts on tagged releases.
- Dual MIT / Apache-2.0 licensing and crates.io metadata.

[Unreleased]: https://github.com/tallamjr/iree-embedded/commits/master
