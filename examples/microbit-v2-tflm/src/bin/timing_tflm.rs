//! Timing firmware for the TFLite-Micro baseline: cycles per model invoke.
//!
//! Same method as `timing_int8.rs` of `microbit-v2-kws`: 10 warm-up invokes,
//! then 1000 timed invokes of the model only, with the instruction cache off
//! and then on. The input is the fixed upstream feature file. The label and
//! the four int8 logits go out over defmt RTT after each pass.
#![no_std]
#![no_main]
#![forbid(unsafe_code)]

include!("../timing_main.rs");
