//! N× oversampling helper for nonlinear (clipping) stages.
//!
//! Stacking several saturating stages at high gain generates harmonics far above
//! the base Nyquist. At the base rate (or even 2×) those fold back as inharmonic
//! aliasing — the harsh, "digital" edge that makes high-gain amp sims sound
//! artificial. Running the nonlinearities at a high oversampling factor pushes the
//! alias products much higher so the decimation low-pass can remove them, leaving a
//! smoother, more analog-sounding distortion.
//!
//! Two quality choices matter as much as the factor itself:
//!   • **Factor.** Each cascaded clipper roughly doubles the bandwidth of the
//!     harmonic series, so three high-gain stages need a lot of headroom above
//!     Nyquist. 8× (see [`Oversampler8`]) keeps the worst foldback products
//!     ~60 dB down for our gain structure — the difference between "fizzy" and
//!     "creamy" on a saturated power chord.
//!   • **Filter steepness.** The anti-image / anti-alias low-pass is an 8th-order
//!     Butterworth (four cascaded biquad sections). A gentle 2nd-order filter
//!     leaves images poking through just under Nyquist; the steep transition here
//!     is what actually cashes in the extra sample rate.
//!
//! Usage from a gain stage (factor inferred from the array length):
//! ```ignore
//! let up = self.os.upsample(x);          // [f32; N]
//! let mut down = [0.0f32; N];
//! for i in 0..N {
//!     down[i] = self.nonlinear(up[i]);   // internal stage filters run at N× rate
//! }
//! let y = self.os.downsample(down);
//! ```

use crate::dsp::biquad::Biquad;

/// Section Qs for an 8th-order Butterworth low-pass (four cascaded biquads).
/// Q_k = 1 / (2·cos((2k+1)·π/16)), k = 0..3.
const BW8_Q: [f32; 4] = [0.50980, 0.60134, 0.89998, 2.56292];

/// `N`× oversampler built from cascaded 8th-order Butterworth low-pass sections at
/// the base-rate Nyquist, running at `sr * N`.
pub struct Oversampler<const N: usize> {
    up: [Biquad; 4],
    dn: [Biquad; 4],
}

impl<const N: usize> Oversampler<N> {
    pub fn new(sr: f32) -> Self {
        let srn = sr * N as f32;
        // Cutoff just under the base Nyquist; 8th-order Butterworth (four sections)
        // gives a steep, flat-passband transition so images/aliases just above the
        // base Nyquist are strongly rejected.
        let fc = sr * 0.5 * 0.9;
        let mk = || {
            [
                Biquad::lowpass(srn, fc, BW8_Q[0]),
                Biquad::lowpass(srn, fc, BW8_Q[1]),
                Biquad::lowpass(srn, fc, BW8_Q[2]),
                Biquad::lowpass(srn, fc, BW8_Q[3]),
            ]
        };
        Self {
            up: mk(),
            dn: mk(),
        }
    }

    /// Zero-stuff by `N` and interpolate. The ×N gain compensates for the energy
    /// lost to the inserted zeros so unity is preserved through the round trip.
    #[inline]
    pub fn upsample(&mut self, x: f32) -> [f32; N] {
        let mut out = [0.0f32; N];
        for (i, o) in out.iter_mut().enumerate() {
            let s = if i == 0 { x * N as f32 } else { 0.0 };
            let mut v = s;
            for b in &mut self.up {
                v = b.process(v);
            }
            *o = v;
        }
        out
    }

    /// Band-limit the N× stream and decimate back to the base rate.
    #[inline]
    pub fn downsample(&mut self, block: [f32; N]) -> f32 {
        let mut y = 0.0f32;
        for &s in &block {
            let mut v = s;
            for b in &mut self.dn {
                v = b.process(v);
            }
            y = v;
        }
        y
    }
}

/// 4× oversampler (kept for lighter stages / reference).
#[allow(dead_code)]
pub type Oversampler4 = Oversampler<4>;

/// 8× oversampler — used by the high-gain amp models for creamy, alias-free drive.
pub type Oversampler8 = Oversampler<8>;
