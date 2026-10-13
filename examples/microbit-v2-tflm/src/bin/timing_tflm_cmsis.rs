//! Timing firmware for the TFLite-Micro baseline with the CMSIS-NN kernels.
//! Same source as `timing_tflm`. Build it with `--features cmsis-nn`.
#![no_std]
#![no_main]
#![forbid(unsafe_code)]

include!("../timing_main.rs");
