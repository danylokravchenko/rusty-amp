use std::f32::consts::PI;

/// Auto-wah — a resonant bandpass whose peak is swept by an envelope follower, so
/// the filter "opens" as you dig in and closes as the note decays. With SENS at 0
/// it's a static "cocked wah" parked at the FREQ position; turn SENS up and it
/// becomes a dynamic, vocal auto-wah that tracks your picking.
///
/// A real wah is a treadle-swept bandpass, but a terminal has no expression pedal,
/// so the envelope follower stands in for the foot: the louder the input, the higher
/// the peak sweeps. It sits early in the chain — after the whammy, *before* the
/// compressor and the drives — so it responds to raw pick dynamics (a compressor
/// ahead of it would flatten the very envelope it rides) and the vocal quack is what
/// the distortion then saturates, the classic wah-into-fuzz voice.
///
/// The swept peak is a TPT (topology-preserving transform) state-variable filter:
/// unlike a biquad, its centre frequency can be modulated every sample from a single
/// `tan`, with no coefficient-rebuild cost — exactly what a continuously swept filter
/// needs.
///
/// Knob ranges (all normalised 0–1):
///   FREQ → base peak position / heel-down point (~300 Hz–1.5 kHz, exponential).
///   SENS → how far the envelope sweeps the peak up (0 = fixed cocked wah).
///   Q    → resonance/sharpness of the peak — the "quack".
///   MIX  → dry/wet blend; a real wah is fully wet, but a blend keeps body.
pub struct Wah {
    sr: f32,
    // Envelope follower state and its attack/release smoothing coefficients.
    env: f32,
    att: f32,
    rel: f32,
    // TPT state-variable filter integrator states.
    ic1: f32,
    ic2: f32,
}

/// Lowest base peak frequency (FREQ at 0) — the heel-down position.
const F_MIN: f32 = 300.0;
/// Highest base peak frequency (FREQ at 1).
const F_MAX: f32 = 1500.0;
/// How many octaves the envelope can sweep the peak above the base at full SENS.
const SWEEP_OCT: f32 = 3.5;
/// A little resonant boost at the peak so the wah quacks rather than just filters.
const WAH_BOOST: f32 = 1.6;

impl Wah {
    pub fn new(sr: f32) -> Self {
        Self {
            sr,
            env: 0.0,
            // Fast attack (~3 ms) so the wah opens on the pick transient; slower
            // release (~80 ms) so it closes smoothly as the note decays.
            att: 1.0 - (-1.0 / (0.003 * sr)).exp(),
            rel: 1.0 - (-1.0 / (0.080 * sr)).exp(),
            ic1: 0.0,
            ic2: 0.0,
        }
    }

    /// `freq` 0–1 (base position), `sens` 0–1 (auto sweep), `q` 0–1, `mix` 0–1.
    #[inline]
    pub fn process(&mut self, x: f32, freq: f32, sens: f32, q: f32, mix: f32) -> f32 {
        // Envelope follower: rectify, then smooth with fast-attack / slow-release so
        // the peak tracks the pick transient and eases back down on the decay.
        let rect = x.abs();
        let coeff = if rect > self.env { self.att } else { self.rel };
        self.env += coeff * (rect - self.env);

        // Base peak position (exponential over the knob), swept up by the envelope.
        let base = F_MIN * (F_MAX / F_MIN).powf(freq.clamp(0.0, 1.0));
        let sweep = (self.env * sens.clamp(0.0, 1.0) * SWEEP_OCT).exp2();
        let fc = (base * sweep).clamp(120.0, self.sr * 0.45);

        // Resonance: Q 1.5–6.5. Higher = a narrower, sharper quack.
        let q_val = 1.5 + q.clamp(0.0, 1.0) * 5.0;
        let k = 1.0 / q_val;

        // TPT state-variable filter (Zavalishin/Cytomic). `g = tan(π·fc/sr)` is the
        // only trig per sample; the bandpass tap is `v1`.
        let g = (PI * fc / self.sr).tan();
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let v3 = x - self.ic2;
        let v1 = a1 * self.ic1 + a2 * v3;
        let v2 = self.ic2 + a2 * self.ic1 + a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;

        // The raw bandpass (`v1`) peaks at gain Q; `* k` normalises the peak to unity,
        // then a small fixed boost gives the resonant quack.
        let wet = v1 * k * WAH_BOOST;

        let mix = mix.clamp(0.0, 1.0);
        x * (1.0 - mix) + wet * mix
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const SR: f32 = 48_000.0;

    fn rms(v: &[f32]) -> f64 {
        (v.iter().map(|s| (*s as f64) * (*s as f64)).sum::<f64>() / v.len() as f64).sqrt()
    }

    /// Render a steady sine through the wah and return the settled tail, asserting
    /// finiteness along the way.
    fn render(f0: f32, amp: f32, freq: f32, sens: f32, q: f32, mix: f32) -> Vec<f32> {
        let mut w = Wah::new(SR);
        let n = SR as usize;
        let warmup = n / 4;
        let mut out = Vec::with_capacity(n - warmup);
        for i in 0..n {
            let x = (2.0 * PI * f0 * i as f32 / SR).sin() * amp;
            let y = w.process(x, freq, sens, q, mix);
            assert!(y.is_finite(), "wah non-finite at {i}");
            if i >= warmup {
                out.push(y);
            }
        }
        out
    }

    /// `mix = 0` must pass the dry signal through untouched.
    #[test]
    fn fully_dry_is_passthrough() {
        let mut w = Wah::new(SR);
        for n in 0..4000 {
            let x = (n as f32 * 0.021).sin() * 0.6;
            let y = w.process(x, 0.5, 0.5, 0.5, 0.0);
            assert!((y - x).abs() < 1e-6, "dry mix altered the signal at {n}");
        }
    }

    /// Extreme settings must stay finite and bounded — the resonant filter must not
    /// blow up even at max Q and a hot input.
    #[test]
    fn finite_and_bounded_under_extremes() {
        let mut w = Wah::new(SR);
        let mut max_abs = 0.0f32;
        for n in 0..(SR as usize) {
            let x = (2.0 * PI * 500.0 * n as f32 / SR).sin() * 0.9;
            let y = w.process(x, 0.8, 1.0, 1.0, 1.0);
            assert!(y.is_finite(), "non-finite at {n}");
            max_abs = max_abs.max(y.abs());
        }
        assert!(max_abs < 4.0, "wah output out of bounds: {max_abs}");
    }

    /// The FREQ knob must move the passband: as a static cocked wah (SENS 0), a high
    /// setting passes a high note far better than a low setting does, and vice versa
    /// for a low note — proving the resonant peak actually tracks the knob.
    #[test]
    fn freq_knob_moves_the_passband() {
        // A 1.6 kHz note: the peak is up near it at FREQ high, far below at FREQ low.
        let hi_note_lo = rms(&render(1600.0, 0.5, 0.05, 0.0, 0.5, 1.0));
        let hi_note_hi = rms(&render(1600.0, 0.5, 0.95, 0.0, 0.5, 1.0));
        assert!(
            hi_note_hi > hi_note_lo * 1.5,
            "FREQ up did not pass the high note more ({hi_note_hi:.4} vs {hi_note_lo:.4})"
        );
        // A 300 Hz note: the opposite — the low FREQ peak passes it far better.
        let lo_note_lo = rms(&render(300.0, 0.5, 0.05, 0.0, 0.5, 1.0));
        let lo_note_hi = rms(&render(300.0, 0.5, 0.95, 0.0, 0.5, 1.0));
        assert!(
            lo_note_lo > lo_note_hi * 1.5,
            "FREQ down did not pass the low note more ({lo_note_lo:.4} vs {lo_note_hi:.4})"
        );
    }

    /// The Q knob must sharpen the peak: a note an octave above the parked peak is
    /// attenuated more at high Q (narrower band) than at low Q.
    #[test]
    fn q_knob_sharpens_the_peak() {
        // Park the peak (SENS 0) low and probe an off-centre note above it.
        let off_centre_low_q = rms(&render(1400.0, 0.5, 0.2, 0.0, 0.1, 1.0));
        let off_centre_high_q = rms(&render(1400.0, 0.5, 0.2, 0.0, 0.95, 1.0));
        assert!(
            off_centre_low_q > off_centre_high_q * 1.3,
            "Q up did not narrow the band ({off_centre_high_q:.4} vs {off_centre_low_q:.4})"
        );
    }

    /// SENS turns it into an auto-wah: with sensitivity up, a loud input sweeps the
    /// peak up toward a high note (passing it well), while a quiet input leaves the
    /// peak parked low (attenuating it). Compared as output/input ratios so the raw
    /// level difference cancels out.
    #[test]
    fn sens_sweeps_the_peak_with_the_envelope() {
        let ratio = |amp: f32| {
            let out = render(1200.0, amp, 0.1, 1.0, 0.5, 1.0);
            rms(&out) / amp as f64 // input rms ∝ amp; normalise it out
        };
        let loud = ratio(0.9);
        let quiet = ratio(0.03);
        assert!(
            loud > quiet * 1.5,
            "SENS did not sweep the peak with the envelope (loud {loud:.4} vs quiet {quiet:.4})"
        );
    }
}
