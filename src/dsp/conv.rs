//! Direct-form FIR convolution engine for cabinet impulse responses.
//!
//! A real miked guitar cabinet has an impulse response hundreds of taps long,
//! full of dense comb filtering and sharp cone/cabinet resonances that a chain
//! of a few biquads cannot reproduce. Convolving the amp signal with such an IR
//! is what every studio-grade amp sim (Guitar Rig, Helix, Neural DSP) does, and
//! it is the single biggest contributor to a "real cab in a room" sound versus
//! the flat, boxy character of a parametric-EQ cab.
//!
//! Implementation notes:
//!   • Direct time-domain convolution. For our IR length (≤ 1024 taps) at 48 kHz
//!     this is well within budget and is fully real-time safe: no allocation and
//!     no unbounded work in `process`.
//!   • The coefficient buffer can be swapped (`load`) without clearing history,
//!     so changing mic position / cabinet does not click — the delay line stays
//!     continuous and only the taps change.

/// A fixed-capacity FIR convolver with a circular history buffer.
pub struct FirConvolver {
    /// Impulse-response taps (length == `len`, padded to `capacity`).
    taps: Vec<f32>,
    /// Circular buffer of past input samples, same capacity as `taps`.
    history: Vec<f32>,
    /// Current number of active taps.
    len: usize,
    /// Write cursor into `history`.
    pos: usize,
}

impl FirConvolver {
    /// Create a convolver sized for up to `capacity` taps, initialised to a unit
    /// impulse (pass-through) so it is safe to run before a real IR is loaded.
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(1);
        let mut taps = vec![0.0; cap];
        taps[0] = 1.0;
        Self {
            taps,
            history: vec![0.0; cap],
            len: 1,
            pos: 0,
        }
    }

    /// Replace the active taps. `ir.len()` must not exceed the capacity; longer
    /// IRs are truncated. History is preserved for click-free swaps.
    pub fn load(&mut self, ir: &[f32]) {
        let n = ir.len().min(self.taps.len());
        self.taps[..n].copy_from_slice(&ir[..n]);
        for t in &mut self.taps[n..] {
            *t = 0.0;
        }
        self.len = n.max(1);
    }

    /// Convolve one input sample, returning one output sample.
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        // Newest sample at `pos`, walking backward in time.
        self.history[self.pos] = x;
        let cap = self.history.len();

        let mut acc = 0.0f32;
        let mut h = self.pos;
        // taps[0] multiplies the newest sample, taps[k] the sample k steps back.
        for &t in &self.taps[..self.len] {
            acc += t * self.history[h];
            h = if h == 0 { cap - 1 } else { h - 1 };
        }

        self.pos = if self.pos + 1 == cap { 0 } else { self.pos + 1 };
        acc
    }
}
