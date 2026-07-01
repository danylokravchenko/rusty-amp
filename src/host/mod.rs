//! Plugin hosting — load third-party effect plugins into the signal chain.
//!
//! Two formats are supported, each behind its own feature and mapped to the DSP
//! chain's [`StereoInsert`](crate::dsp::StereoInsert) trait:
//!
//! * [`clap_host`] (`clap` feature) — cross-platform CLAP effects, hosted as a
//!   post-rack stereo insert. Its API is re-exported at this module's root
//!   (`host::scan`, `host::load`, `host::LoadedPlugin`, …) for backwards compatibility.
//! * [`au`] (`au` feature, macOS only) — Audio Unit effects, used as an amp-position
//!   override. Kept namespaced (`host::au::…`) because its types mirror the CLAP ones.

#[cfg(feature = "clap")]
mod clap_host;
#[cfg(feature = "clap")]
pub use clap_host::*;

// AU hosting needs the CoreAudio frameworks, so it only exists on macOS even when the
// `au` feature is on (the dependency itself is target-gated to macOS in Cargo.toml).
#[cfg(all(feature = "au", target_os = "macos"))]
pub mod au;
