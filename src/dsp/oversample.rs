//! 4× oversampling helper for nonlinear (clipping) stages.
//!
//! Stacking several saturating stages at high gain generates harmonics far above
//! the base Nyquist. At the base rate (or even 2×) those fold back as inharmonic
//! aliasing — the harsh, "digital" edge that makes high-gain amp sims sound
//! artificial. Running the nonlinearities at 4× pushes the alias products much
//! higher so the decimation low-pass can remove them, leaving a smoother,
//! more analog-sounding distortion.
//!
//! Usage from a gain stage:
//! ```ignore
//! let up = self.os.upsample(x);          // [f32; 4]
//! let mut down = [0.0f32; 4];
//! for i in 0..4 {
//!     down[i] = self.nonlinear(up[i]);   // internal stage filters run at 4× rate
//! }
//! let y = self.os.downsample(down);
//! ```

use crate::dsp::biquad::Biquad;

/// Polyphase-free 4× oversampler built from cascaded Butterworth low-pass
/// sections at the base-rate Nyquist, running at `sr * 4`.
pub struct Oversampler4 {
    up_a: Biquad,
    up_b: Biquad,
    dn_a: Biquad,
    dn_b: Biquad,
}

impl Oversampler4 {
    pub fn new(sr: f32) -> Self {
        let sr4 = sr * 4.0;
        // Cutoff just under the base Nyquist; 4th-order Butterworth (two sections).
        let fc = sr * 0.5 * 0.9;
        Self {
            up_a: Biquad::lowpass(sr4, fc, 0.5412),
            up_b: Biquad::lowpass(sr4, fc, 1.3066),
            dn_a: Biquad::lowpass(sr4, fc, 0.5412),
            dn_b: Biquad::lowpass(sr4, fc, 1.3066),
        }
    }

    /// Zero-stuff by 4 and interpolate. The ×4 gain compensates for the energy
    /// lost to the inserted zeros so unity is preserved through the round trip.
    #[inline]
    pub fn upsample(&mut self, x: f32) -> [f32; 4] {
        let mut out = [0.0f32; 4];
        for (i, o) in out.iter_mut().enumerate() {
            let s = if i == 0 { x * 4.0 } else { 0.0 };
            *o = self.up_b.process(self.up_a.process(s));
        }
        out
    }

    /// Band-limit the 4× stream and decimate back to the base rate.
    #[inline]
    pub fn downsample(&mut self, block: [f32; 4]) -> f32 {
        let mut y = 0.0f32;
        for &s in &block {
            y = self.dn_b.process(self.dn_a.process(s));
        }
        y
    }
}
