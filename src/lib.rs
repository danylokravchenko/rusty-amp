//! rusty-amp — a guitar amplifier and effects simulator.
//!
//! The crate is split into a library (this file) and a thin binary (`main.rs`) so
//! the DSP, preset, recording, and UI modules can be unit-tested and reused
//! without going through the executable entry point.

pub mod audio;
pub mod dsp;
/// CLAP plugin hosting (behind the `clap` feature). Lets third-party CLAP effect
/// plugins be loaded as a stereo insert in the signal chain.
#[cfg(feature = "clap")]
pub mod host;
pub mod preset;
pub mod recording;
pub mod ui;
