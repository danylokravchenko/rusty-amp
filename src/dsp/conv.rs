//! Uniformly-partitioned FFT convolution engine for cabinet impulse responses.
//!
//! A real miked guitar cabinet has an impulse response hundreds of taps long,
//! full of dense comb filtering and sharp cone/cabinet resonances that a chain
//! of a few biquads cannot reproduce. Convolving the amp signal with such an IR
//! is what every studio-grade amp sim (Guitar Rig, Helix, Neural DSP) does, and
//! it is the single biggest contributor to a "real cab in a room" sound versus
//! the flat, boxy character of a parametric-EQ cab.
//!
//! Why FFT instead of a direct MAC loop:
//!   • Our IR is ~1100 taps at 48 kHz, run on two channels — comfortably past the
//!     point (~64–256 taps) where frequency-domain convolution wins. Direct form
//!     costs O(N) multiplies per sample; partitioned FFT convolution costs
//!     ~O(log N) for several× less CPU on the single hottest DSP stage. The output
//!     is the exact same *linear* convolution, so the sound is unchanged.
//!
//! Implementation — uniformly-partitioned overlap-save (UPOLS):
//!   • The IR is split into `K` partitions of `P` taps. Each partition is
//!     zero-padded to `N = 2P` and forward-FFT'd once (in [`load`]).
//!   • Input is collected `P` samples at a time. Each block, the 2P-sample window
//!     `[previous P | current P]` is FFT'd and pushed into a frequency-domain
//!     delay line. The output spectrum is the accumulated product of the IR
//!     partition spectra with the matching (delayed) input spectra; one inverse
//!     FFT yields the block, of which the last `P` samples are the alias-free
//!     linear-convolution output.
//!   • Latency is exactly `P` samples (~2.7 ms at 48 kHz) — both cab channels
//!     share it so the stereo image stays aligned, and nothing downstream mixes a
//!     dry copy that could phase-cancel.
//!   • `process` never allocates and does no unbounded work. The IR spectra
//!     ([`load`]) can be swapped without touching the input delay line, so
//!     changing mic position / cabinet stays click-free.

use std::f32::consts::PI;

/// Partition (block) size in samples. Power of two; sets the FFT size (`2 * P`)
/// and the convolution latency. 128 keeps latency ~2.7 ms at 48 kHz while
/// amortising the FFT cost well across the ~9 partitions of a cab IR.
const P: usize = 128;

/// An FFT partitioned-convolution FIR filter with a frequency-domain delay line.
pub struct FftConvolver {
    /// Partition size (== `P`).
    p: usize,
    /// FFT size (`2 * p`).
    n: usize,
    /// Number of IR partitions.
    k: usize,
    /// IR partition spectra, `k` blocks of `n` complex bins (`h_re`/`h_im`).
    h_re: Vec<f32>,
    h_im: Vec<f32>,
    /// Frequency-domain delay line of past input spectra: a ring of `k` blocks of
    /// `n` complex bins. `fdl_pos` is the slot holding the newest spectrum.
    x_re: Vec<f32>,
    x_im: Vec<f32>,
    fdl_pos: usize,
    /// Time-domain overlap buffer, length `n`: `[previous P | current P]`.
    in_buf: Vec<f32>,
    /// Number of current-block input samples collected so far, in `[0, p)`.
    fill: usize,
    /// Output block to emit sample-by-sample, length `p`.
    out_buf: Vec<f32>,
    /// FFT scratch (length `n`) and accumulator spectrum (length `n`).
    sre: Vec<f32>,
    sim: Vec<f32>,
    acc_re: Vec<f32>,
    acc_im: Vec<f32>,
    /// Precomputed bit-reversal permutation and twiddle factors for size `n`.
    bitrev: Vec<usize>,
    wre: Vec<f32>,
    wim: Vec<f32>,
}

impl FftConvolver {
    /// Create a convolver sized for up to `capacity` taps, initialised to a unit
    /// impulse (pass-through) so it is safe to run before a real IR is loaded.
    pub fn new(capacity: usize) -> Self {
        let p = P;
        let n = p * 2;
        let k = capacity.max(1).div_ceil(p);

        // Bit-reversal table for size `n`.
        let bits = n.trailing_zeros();
        let bitrev = (0..n).map(|i| bit_reverse(i, bits)).collect();

        // Forward twiddles W_n^j = exp(-2πi j / n); stored as cos/sin of the
        // positive angle and sign-flipped per direction in `fft`.
        let mut wre = vec![0.0f32; n / 2];
        let mut wim = vec![0.0f32; n / 2];
        for j in 0..n / 2 {
            let ang = 2.0 * PI * j as f32 / n as f32;
            wre[j] = ang.cos();
            wim[j] = ang.sin();
        }

        let mut c = Self {
            p,
            n,
            k,
            h_re: vec![0.0; k * n],
            h_im: vec![0.0; k * n],
            x_re: vec![0.0; k * n],
            x_im: vec![0.0; k * n],
            fdl_pos: 0,
            in_buf: vec![0.0; n],
            fill: 0,
            out_buf: vec![0.0; p],
            sre: vec![0.0; n],
            sim: vec![0.0; n],
            acc_re: vec![0.0; n],
            acc_im: vec![0.0; n],
            bitrev,
            wre,
            wim,
        };
        // Start as pass-through until a real cab IR is loaded.
        c.load(&[1.0]);
        c
    }

    /// Replace the active IR. `ir.len()` beyond the capacity is truncated. The
    /// input delay line is preserved for click-free swaps. Real-time safe: only
    /// fixed, preallocated buffers are touched (a handful of FFTs, no allocation).
    pub fn load(&mut self, ir: &[f32]) {
        let n_taps = ir.len().min(self.k * self.p);
        for kk in 0..self.k {
            let start = kk * self.p;
            // Partition `kk` of the IR, zero-padded to the FFT size.
            for i in 0..self.n {
                let t = start + i;
                self.sre[i] = if i < self.p && t < n_taps { ir[t] } else { 0.0 };
                self.sim[i] = 0.0;
            }
            Self::fft(
                &self.bitrev,
                &self.wre,
                &self.wim,
                &mut self.sre,
                &mut self.sim,
                false,
            );
            let dst = kk * self.n;
            self.h_re[dst..dst + self.n].copy_from_slice(&self.sre);
            self.h_im[dst..dst + self.n].copy_from_slice(&self.sim);
        }
    }

    /// Convolve one input sample, returning one output sample (delayed by `P`).
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        // Emit the precomputed output for this position, then stash the input into
        // the current half of the overlap buffer.
        let y = self.out_buf[self.fill];
        self.in_buf[self.p + self.fill] = x;
        self.fill += 1;
        if self.fill == self.p {
            self.transform_block();
            self.fill = 0;
        }
        y
    }

    /// Run one block of overlap-save: FFT the 2P input window, accumulate the
    /// frequency-domain delay line against the IR spectra, inverse-FFT, and keep
    /// the alias-free second half as the next output block.
    fn transform_block(&mut self) {
        let (n, p) = (self.n, self.p);

        // Forward-transform the current 2P window `[prev P | cur P]`.
        self.sre[..n].copy_from_slice(&self.in_buf);
        for v in &mut self.sim[..n] {
            *v = 0.0;
        }
        Self::fft(
            &self.bitrev,
            &self.wre,
            &self.wim,
            &mut self.sre,
            &mut self.sim,
            false,
        );

        // Store it as the newest spectrum in the delay line.
        let cur = self.fdl_pos * n;
        self.x_re[cur..cur + n].copy_from_slice(&self.sre);
        self.x_im[cur..cur + n].copy_from_slice(&self.sim);

        // ACC = Σ_kk  H_kk · X_{n-kk}  (partition kk pairs with the input block
        // it is kk steps old). Complex multiply-accumulate, bin by bin.
        for v in &mut self.acc_re {
            *v = 0.0;
        }
        for v in &mut self.acc_im {
            *v = 0.0;
        }
        for kk in 0..self.k {
            let slot = (self.fdl_pos + self.k - kk) % self.k;
            let hb = kk * n;
            let xb = slot * n;
            for bin in 0..n {
                let hr = self.h_re[hb + bin];
                let hi = self.h_im[hb + bin];
                let xr = self.x_re[xb + bin];
                let xi = self.x_im[xb + bin];
                self.acc_re[bin] += hr * xr - hi * xi;
                self.acc_im[bin] += hr * xi + hi * xr;
            }
        }

        // Back to time domain; the last P samples are the valid linear output.
        Self::fft(
            &self.bitrev,
            &self.wre,
            &self.wim,
            &mut self.acc_re,
            &mut self.acc_im,
            true,
        );
        self.out_buf.copy_from_slice(&self.acc_re[p..n]);

        // Current block becomes the previous block for the next window, and the
        // delay line advances.
        self.in_buf.copy_within(p..n, 0);
        self.fdl_pos = (self.fdl_pos + 1) % self.k;
    }

    /// In-place iterative radix-2 FFT (decimation-in-time). `inverse` selects the
    /// transform sign and applies the `1/n` scaling. No allocation, no
    /// transcendental calls in the loop (twiddles are precomputed).
    fn fft(
        bitrev: &[usize],
        wre: &[f32],
        wim: &[f32],
        re: &mut [f32],
        im: &mut [f32],
        inverse: bool,
    ) {
        let n = re.len();
        for (i, &j) in bitrev.iter().enumerate() {
            if j > i {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let mut len = 2usize;
        while len <= n {
            let half = len / 2;
            let step = n / len;
            let mut base = 0;
            while base < n {
                for kk in 0..half {
                    let tw = kk * step;
                    let wr = wre[tw];
                    // Forward twiddle is exp(-iθ) → -sin; inverse flips the sign.
                    let wi = if inverse { wim[tw] } else { -wim[tw] };
                    let i0 = base + kk;
                    let i1 = i0 + half;
                    let xr = re[i1];
                    let xi = im[i1];
                    let tr = wr * xr - wi * xi;
                    let ti = wr * xi + wi * xr;
                    re[i1] = re[i0] - tr;
                    im[i1] = im[i0] - ti;
                    re[i0] += tr;
                    im[i0] += ti;
                }
                base += len;
            }
            len <<= 1;
        }
        if inverse {
            let inv = 1.0 / n as f32;
            for v in re.iter_mut() {
                *v *= inv;
            }
            for v in im.iter_mut() {
                *v *= inv;
            }
        }
    }
}

/// Reverse the low `bits` bits of `x` (index permutation for the in-place FFT).
fn bit_reverse(x: usize, bits: u32) -> usize {
    let mut r = 0usize;
    let mut v = x;
    for _ in 0..bits {
        r = (r << 1) | (v & 1);
        v >>= 1;
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference direct-form convolution to validate the FFT engine against.
    fn direct_conv(ir: &[f32], x: &[f32]) -> Vec<f32> {
        let mut y = vec![0.0f32; x.len()];
        for (n, yn) in y.iter_mut().enumerate() {
            let mut acc = 0.0f32;
            for (k, &h) in ir.iter().enumerate() {
                if n >= k {
                    acc += h * x[n - k];
                }
            }
            *yn = acc;
        }
        y
    }

    /// The FFT convolver must reproduce direct convolution exactly (to float
    /// tolerance), accounting for its `P`-sample latency.
    #[test]
    fn fft_matches_direct_convolution() {
        // An IR long enough to span several partitions.
        let ir: Vec<f32> = (0..300)
            .map(|i| (i as f32 * 0.07).sin() * 0.5f32.powf(i as f32 / 80.0))
            .collect();
        let len = 4000;
        let x: Vec<f32> = (0..len)
            .map(|i| (i as f32 * 0.3).sin() + (i as f32 * 0.013).cos())
            .collect();

        let reference = direct_conv(&ir, &x);

        let mut conv = FftConvolver::new(ir.len() + 1);
        conv.load(&ir);
        let out: Vec<f32> = x.iter().map(|&s| conv.process(s)).collect();

        // Compare ignoring the P-sample startup latency.
        let lat = P;
        let mut max_err = 0.0f32;
        for i in 0..(len - lat) {
            max_err = max_err.max((out[i + lat] - reference[i]).abs());
        }
        assert!(
            max_err < 1e-3,
            "FFT conv diverges from direct: max_err = {max_err}"
        );
    }

    /// A freshly constructed convolver is a (delayed) pass-through.
    #[test]
    fn defaults_to_passthrough() {
        let mut conv = FftConvolver::new(1200);
        let x: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.1).sin()).collect();
        let out: Vec<f32> = x.iter().map(|&s| conv.process(s)).collect();
        for i in 0..(1000 - P) {
            assert!((out[i + P] - x[i]).abs() < 1e-4, "not pass-through at {i}");
        }
    }
}
