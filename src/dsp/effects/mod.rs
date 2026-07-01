//! Pedals and rack effects — the stompbox/processor chain that sits around the
//! amp and cabinet core. Each effect is a self-contained `struct` with a
//! `process` method; the [`DspChain`](super::DspChain) wires them together.
//!
//! Logic shared between several effects lives here so the individual files stay
//! focused on their *voicing* (which frequencies, how much gain) rather than
//! re-implementing the same plumbing:
//!   • [`OnePoleLp`] — the variable low-pass used by passive tone controls.
//!   • [`ThreeBandEq`] — low-shelf / mid-peak / high-shelf trio shared by the
//!     pre-amp and post-cab equalizers.
//!   • [`db_to_lin`] / [`lin_to_db`] — decibel conversions for dynamics stages.
//!   • [`param_changed`] — the "did this knob move enough to rebuild?" test.

use crate::dsp::biquad::Biquad;
use std::f32::consts::PI;

pub mod compressor;
pub mod delay;
pub mod distortion;
pub mod flanger;
pub mod fuzz;
pub mod noise_gate;
pub mod parametric_eq;
pub mod preamp_eq;
pub mod reverb;
pub mod tube_screamer;

pub use compressor::Compressor;
pub use delay::Delay;
pub use distortion::Distortion;
pub use flanger::Flanger;
pub use fuzz::Fuzz;
pub use noise_gate::NoiseGate;
pub use parametric_eq::ParametricEq;
pub use preamp_eq::PreampEq;
pub use reverb::Reverb;
pub use tube_screamer::TubeScreamer;

/// A knob is "dirty" — worth rebuilding filter coefficients for — only once it has
/// moved by more than this. Filter rebuilds aren't free, so we skip them while a
/// control sits still (the common case on the audio thread).
const PARAM_EPSILON: f32 = 0.001;

/// Has a control moved far enough since we last acted on it to justify a rebuild?
#[inline]
pub fn param_changed(new: f32, last: f32) -> bool {
    (new - last).abs() > PARAM_EPSILON
}

/// Convert decibels to a linear amplitude ratio.
#[inline]
pub fn db_to_lin(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// Convert a linear amplitude (floored to avoid `-inf`) to decibels.
#[inline]
pub fn lin_to_db(x: f32) -> f32 {
    20.0 * x.max(1e-6).log10()
}

/// One-pole low-pass filter — the workhorse behind passive "tone" controls.
///
/// A guitar pedal's tone knob is, electrically, a variable RC low-pass: turn it
/// down and the corner frequency drops, rolling off the highs. Both the TS-808 and
/// the Big Muff model their tone stage exactly this way, so they share this filter
/// and only differ in the frequency range they sweep it across.
pub struct OnePoleLp {
    z: f32,
    coeff: f32,
}

impl OnePoleLp {
    pub fn new() -> Self {
        Self { z: 0.0, coeff: 0.0 }
    }

    /// Set the −3 dB corner to `freq` Hz at sample rate `sr`.
    #[inline]
    pub fn set_cutoff(&mut self, freq: f32, sr: f32) {
        self.coeff = 1.0 - (-2.0 * PI * freq / sr).exp();
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        self.z += self.coeff * (x - self.z);
        self.z
    }
}

impl Default for OnePoleLp {
    fn default() -> Self {
        Self::new()
    }
}

/// Mono three-band equalizer: low shelf → mid peak → high shelf in series.
///
/// Both the pre-amp EQ and the post-cab parametric EQ are this exact topology;
/// they differ only in their centre frequencies, mid Q and gain range. Factoring
/// the filter trio out here means the [`ParametricEq`] (stereo = two of these) and
/// [`PreampEq`] (mono = one) wrappers only own their voicing and the dirty-check,
/// not three biquads' worth of duplicated rebuild code.
pub struct ThreeBandEq {
    sr: f32,
    low_freq: f32,
    mid_freq: f32,
    mid_q: f32,
    high_freq: f32,
    low: Biquad,
    mid: Biquad,
    high: Biquad,
}

impl ThreeBandEq {
    /// Build a flat (0 dB) EQ with the given band layout.
    pub fn new(sr: f32, low_freq: f32, mid_freq: f32, mid_q: f32, high_freq: f32) -> Self {
        Self {
            sr,
            low_freq,
            mid_freq,
            mid_q,
            high_freq,
            low: Biquad::low_shelf(sr, low_freq, 0.0),
            mid: Biquad::peak_eq(sr, mid_freq, mid_q, 0.0),
            high: Biquad::high_shelf(sr, high_freq, 0.0),
        }
    }

    /// Recompute the three biquads for the given per-band gains (in dB).
    pub fn set_gains_db(&mut self, low_db: f32, mid_db: f32, high_db: f32) {
        self.low = Biquad::low_shelf(self.sr, self.low_freq, low_db);
        self.mid = Biquad::peak_eq(self.sr, self.mid_freq, self.mid_q, mid_db);
        self.high = Biquad::high_shelf(self.sr, self.high_freq, high_db);
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        self.high.process(self.mid.process(self.low.process(x)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_changed_respects_epsilon() {
        assert!(!param_changed(0.5, 0.5));
        assert!(!param_changed(0.5, 0.5005));
        assert!(param_changed(0.5, 0.51));
    }

    #[test]
    fn db_lin_round_trip() {
        for &db in &[-40.0, -6.0, 0.0, 6.0, 12.0] {
            let back = lin_to_db(db_to_lin(db));
            assert!(
                (back - db).abs() < 1e-3,
                "db round trip off: {db} -> {back}"
            );
        }
        assert!((db_to_lin(0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn one_pole_lp_passes_dc_and_rolls_off_highs() {
        let sr = 48_000.0;
        let mut lp = OnePoleLp::new();
        lp.set_cutoff(1000.0, sr);

        // DC settles to unity.
        let mut y = 0.0;
        for _ in 0..2000 {
            y = lp.process(1.0);
        }
        assert!((y - 1.0).abs() < 1e-3, "one-pole LP DC gain off: {y}");

        // Energy at a high frequency (well above the corner) is attenuated.
        let rms = |freq: f32| {
            let mut lp = OnePoleLp::new();
            lp.set_cutoff(1000.0, sr);
            let mut sum = 0.0f64;
            for n in 0..(sr as usize) {
                let x = (2.0 * PI * freq * n as f32 / sr).sin();
                let y = lp.process(x);
                if n >= sr as usize / 2 {
                    sum += (y * y) as f64;
                }
            }
            sum.sqrt()
        };
        assert!(
            rms(12_000.0) < rms(200.0) * 0.5,
            "LP did not roll off highs"
        );
    }

    #[test]
    fn three_band_eq_boost_and_cut_track_their_bands() {
        let sr = 48_000.0;
        let band_rms = |gains: (f32, f32, f32), freq: f32| {
            let mut eq = ThreeBandEq::new(sr, 120.0, 800.0, 1.5, 5000.0);
            eq.set_gains_db(gains.0, gains.1, gains.2);
            let mut sum = 0.0f64;
            for n in 0..(sr as usize) {
                let x = (2.0 * PI * freq * n as f32 / sr).sin();
                let y = eq.process(x);
                assert!(y.is_finite());
                if n >= sr as usize / 2 {
                    sum += (y * y) as f64;
                }
            }
            sum.sqrt()
        };

        // Boosting the low shelf raises 60 Hz; boosting the high shelf raises 10 kHz.
        assert!(band_rms((12.0, 0.0, 0.0), 60.0) > band_rms((-12.0, 0.0, 0.0), 60.0));
        assert!(band_rms((0.0, 0.0, 12.0), 10_000.0) > band_rms((0.0, 0.0, -12.0), 10_000.0));
        // Boosting the mid peak raises its centre.
        assert!(band_rms((0.0, 12.0, 0.0), 800.0) > band_rms((0.0, -12.0, 0.0), 800.0));
    }
}
