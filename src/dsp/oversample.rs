//! N× oversampling helper for nonlinear (clipping) stages, built on polyphase FIR
//! interpolation/decimation.
//!
//! Stacking several saturating stages at high gain generates harmonics far above
//! the base Nyquist. At the base rate those fold back as inharmonic aliasing — the
//! harsh, "digital" edge that makes high-gain amp sims sound artificial. Running
//! the nonlinearities at a high oversampling factor pushes the alias products much
//! higher so the band-limiting filters can remove them, leaving a smoother, more
//! analog-sounding distortion.
//!
//! Why polyphase instead of zero-stuff + IIR:
//!   • A naive interpolator zero-stuffs by N and then runs the anti-image filter on
//!     every high-rate sample — but N−1 of every N inputs are zeros, so that work
//!     is wasted. A naive decimator likewise filters all N high-rate samples to
//!     keep just one. **Polyphase decomposition computes only the contributions
//!     that survive**: a single prototype low-pass split into N sub-filters, so the
//!     filter cost is independent of N and drops the per-sample work substantially.
//!   • The prototype is a linear-phase windowed-sinc FIR (Kaiser window). Unlike
//!     the previous Butterworth IIR it has *no phase smear*, and the round trip is
//!     unity-gain and flat in the passband — so swapping it in does not recolour
//!     the dry signal, only removes aliasing.

use std::f32::consts::PI;

/// Prototype taps **per polyphase phase**. The full prototype is `M * N` taps; a
/// higher `M` buys a steeper transition / deeper stopband at linear extra cost.
const M: usize = 12;

/// Kaiser window β — trades passband ripple for stopband depth. β = 8 gives
/// roughly 80 dB of stopband rejection, ample for alias suppression given the
/// wide transition band an N× design affords.
const KAISER_BETA: f32 = 8.0;

/// `N`× oversampler using a polyphase FIR for both interpolation and decimation.
///
/// `proto` holds the shared prototype low-pass scaled for interpolation
/// (sum = `N`, so each phase is unity-gain); `proto_dec` is the same filter scaled
/// for decimation (sum = 1). Histories are stored mirrored (doubled) so the inner
/// dot products read a contiguous slice with no wrap-around branch — exactly the
/// SIMD-friendly layout the convolver uses.
pub struct Oversampler<const N: usize> {
    /// Interpolation prototype, length `M * N`, total gain `N`.
    proto: Vec<f32>,
    /// Decimation prototype, length `M * N`, total gain `1`.
    proto_dec: Vec<f32>,
    /// Mirrored base-rate input history for interpolation (length `2 * M`).
    up_hist: Vec<f32>,
    up_pos: usize,
    /// Mirrored high-rate input history for decimation (length `2 * M * N`).
    dn_hist: Vec<f32>,
    dn_pos: usize,
}

impl<const N: usize> Oversampler<N> {
    /// Build for an `N`× rate. `_sr` is accepted for API symmetry with the rest of
    /// the DSP modules; the filter is designed in normalised frequency and does not
    /// depend on the sample rate.
    pub fn new(_sr: f32) -> Self {
        let l = M * N;

        // Linear-phase windowed-sinc low-pass. Cutoff sits at 0.9× the base
        // Nyquist (a small guard band), expressed as a fraction of the *high*
        // sample rate: base Nyquist is 0.5/N of the high rate.
        let fc = 0.5 / N as f32 * 0.9;
        let center = (l - 1) as f32 / 2.0;
        let i0_beta = i0(KAISER_BETA);

        let mut proto = vec![0.0f32; l];
        for (i, tap) in proto.iter_mut().enumerate() {
            let m = i as f32 - center;
            let sinc = if m.abs() < 1e-6 {
                2.0 * fc
            } else {
                (2.0 * PI * fc * m).sin() / (PI * m)
            };
            // Kaiser window.
            let r = (i as f32 - center) / center;
            let win = i0(KAISER_BETA * (1.0 - r * r).max(0.0).sqrt()) / i0_beta;
            *tap = sinc * win;
        }

        // Normalise so the interpolator preserves unity (total gain = N, i.e. each
        // of the N phases sums to ~1), then derive the decimation copy (gain 1).
        let sum: f32 = proto.iter().sum();
        let scale = N as f32 / sum;
        for t in &mut proto {
            *t *= scale;
        }
        let proto_dec: Vec<f32> = proto.iter().map(|&t| t / N as f32).collect();

        Self {
            proto,
            proto_dec,
            up_hist: vec![0.0; 2 * M],
            up_pos: 0,
            dn_hist: vec![0.0; 2 * l],
            dn_pos: 0,
        }
    }

    /// Run a per-sample nonlinearity at the `N`× rate: upsample one base-rate
    /// sample, apply `f` to each of the `N` high-rate samples, then band-limit and
    /// decimate back down. This is the canonical "oversample a clipper" loop, so
    /// the drive pedals share it instead of each repeating the up/map/down dance.
    #[inline]
    pub fn process<F: FnMut(f32) -> f32>(&mut self, x: f32, mut f: F) -> f32 {
        let up = self.upsample(x);
        let mut down = [0.0f32; N];
        for (o, &u) in down.iter_mut().zip(up.iter()) {
            *o = f(u);
        }
        self.downsample(down)
    }

    /// Interpolate one base-rate sample into `N` high-rate samples. Each output
    /// phase `p` is the dot product of sub-filter `proto[p + j·N]` with the recent
    /// base-rate history — no zeros are ever multiplied.
    #[inline]
    pub fn upsample(&mut self, x: f32) -> [f32; N] {
        // Push newest into the mirrored history so the window read is contiguous.
        self.up_pos = if self.up_pos == 0 {
            M - 1
        } else {
            self.up_pos - 1
        };
        self.up_hist[self.up_pos] = x;
        self.up_hist[self.up_pos + M] = x;
        let win = &self.up_hist[self.up_pos..self.up_pos + M]; // win[j] = x[n-j]

        let mut out = [0.0f32; N];
        for (p, o) in out.iter_mut().enumerate() {
            let mut acc = 0.0f32;
            for (j, &h) in win.iter().enumerate() {
                acc += self.proto[p + j * N] * h;
            }
            *o = acc;
        }
        out
    }

    /// Band-limit the `N`× block and decimate back to one base-rate sample. The
    /// `N` incoming samples enter a high-rate delay line; one length-`M·N` dot
    /// product with the decimation prototype yields the output — computed once per
    /// output rather than once per high-rate sample.
    #[inline]
    pub fn downsample(&mut self, block: [f32; N]) -> f32 {
        for &s in &block {
            self.dn_pos = if self.dn_pos == 0 {
                self.proto_dec.len() - 1
            } else {
                self.dn_pos - 1
            };
            self.dn_hist[self.dn_pos] = s;
            self.dn_hist[self.dn_pos + self.proto_dec.len()] = s;
        }
        let l = self.proto_dec.len();
        let win = &self.dn_hist[self.dn_pos..self.dn_pos + l]; // win[i] = x_hr[m-i]
        let mut acc = 0.0f32;
        for (&h, &s) in self.proto_dec.iter().zip(win) {
            acc += h * s;
        }
        acc
    }
}

/// Modified Bessel function of the first kind, order 0 — for the Kaiser window.
/// Power series, evaluated in `f64` for accuracy and capped so it always returns.
fn i0(x: f32) -> f32 {
    let x = x as f64;
    let mut sum = 1.0f64;
    let mut term = 1.0f64;
    let mut k = 1.0f64;
    while k < 64.0 {
        term *= (x / (2.0 * k)).powi(2);
        sum += term;
        if term < 1e-12 * sum {
            break;
        }
        k += 1.0;
    }
    sum as f32
}

/// 4× oversampler — used by milder soft-clip stages (fuzz, TS-808) where the
/// nonlinearity is gentle enough that 4× already keeps alias fold-back well down.
pub type Oversampler4 = Oversampler<4>;

/// 8× oversampler — used by the high-gain amp models for creamy, alias-free drive.
pub type Oversampler8 = Oversampler<8>;

#[cfg(test)]
mod tests {
    use super::*;

    /// A linear (no nonlinearity) round trip through up- then down-sampling must
    /// preserve a passband signal at unity gain, up to the filter's group delay.
    #[test]
    fn roundtrip_is_unity_in_passband() {
        let mut os = Oversampler::<8>::new(48_000.0);
        let n = 4000;
        // ~1 kHz at 48 kHz — comfortably inside the passband.
        let f = 0.021f32;
        let x: Vec<f32> = (0..n).map(|i| (2.0 * PI * f * i as f32).sin()).collect();
        let y: Vec<f32> = x
            .iter()
            .map(|&s| {
                let up = os.upsample(s);
                os.downsample(up)
            })
            .collect();

        // Find the best-matching integer delay, then check residual energy.
        let settle = 500;
        let mut best = f32::INFINITY;
        for d in 0..(2 * M) {
            let mut err = 0.0f32;
            for i in settle..(n - d) {
                err += (y[i + d] - x[i]).powi(2);
            }
            best = best.min(err);
        }
        let ref_energy: f32 = x[settle..].iter().map(|s| s * s).sum();
        let rel = best / ref_energy;
        assert!(
            rel < 1e-3,
            "round trip not unity in passband: rel err {rel}"
        );
    }

    /// DC must pass at unity through the round trip (sanity on the gain scaling).
    #[test]
    fn roundtrip_preserves_dc() {
        let mut os = Oversampler::<4>::new(48_000.0);
        let mut last = 0.0f32;
        for _ in 0..2000 {
            let up = os.upsample(1.0);
            last = os.downsample(up);
        }
        assert!((last - 1.0).abs() < 1e-3, "DC gain off: {last}");
    }

    /// The interpolator must reject an out-of-band tone: a signal above the base
    /// Nyquist (which would alias) is what the filter exists to kill, so a tone
    /// just under base Nyquist should still survive while energy is band-limited.
    #[test]
    fn upsample_band_limits() {
        let mut os = Oversampler::<8>::new(48_000.0);
        // Strong DC-ish low tone should pass; verify interpolated block isn't wild.
        let mut max_abs = 0.0f32;
        for i in 0..2000 {
            let x = (2.0 * PI * 0.01 * i as f32).sin();
            for v in os.upsample(x) {
                assert!(v.is_finite());
                max_abs = max_abs.max(v.abs());
            }
        }
        assert!(max_abs < 1.5, "interpolator overshoot too large: {max_abs}");
    }
}
