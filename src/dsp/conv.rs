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
//!   • The history is stored in a **mirrored** (doubled) buffer: every incoming
//!     sample is written both at `pos` and at `pos + cap`. This lets `process`
//!     read the relevant window as a single *contiguous* slice — the inner dot
//!     product has no per-tap wrap-around branch, so the compiler can
//!     auto-vectorise it (SIMD). For a ~1100-tap cab IR that is the difference
//!     between a scalar MAC loop and a vectorised one — several× throughput.

/// A fixed-capacity FIR convolver with a mirrored history buffer.
pub struct FirConvolver {
    /// Impulse-response taps (length == `len`, padded to `cap`).
    taps: Vec<f32>,
    /// Mirrored history of past input samples, length `2 * cap`: the logical
    /// circular content of length `cap` is duplicated so any `cap`-long window
    /// starting at `pos` is contiguous (`history[j] == history[j + cap]`).
    history: Vec<f32>,
    /// Tap capacity (one logical copy of the history).
    cap: usize,
    /// Current number of active taps.
    len: usize,
    /// Write cursor into the first half of `history`, in `[0, cap)`.
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
            history: vec![0.0; cap * 2],
            cap,
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
        // Step the cursor backward, then write the newest sample into both halves
        // of the mirrored buffer so the window read below is always contiguous.
        self.pos = if self.pos == 0 { self.cap - 1 } else { self.pos - 1 };
        self.history[self.pos] = x;
        self.history[self.pos + self.cap] = x;

        // taps[0] multiplies the newest sample (at `pos`), taps[k] the sample k
        // steps back (at `pos + k`). Because the buffer is mirrored, this window
        // never wraps — a branch-free dot product the compiler can vectorise.
        let window = &self.history[self.pos..self.pos + self.len];
        let mut acc = 0.0f32;
        for (&t, &h) in self.taps[..self.len].iter().zip(window) {
            acc += t * h;
        }
        acc
    }
}
