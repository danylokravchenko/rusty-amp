//! rusty-amp — a guitar amplifier and effects simulator.
//!
//! The crate is split into a library (this file) and a thin binary (`main.rs`) so
//! the DSP, preset, recording, and UI modules can be unit-tested and reused
//! without going through the executable entry point.

pub mod audio;
pub mod dsp;
/// Third-party plugin hosting: CLAP effects as a stereo insert (behind the `clap`
/// feature) and macOS Audio Units as an amp-position override (behind `au`).
#[cfg(any(feature = "clap", feature = "au"))]
pub mod host;
pub mod preset;
pub mod recording;
pub mod ui;
