use super::{OnePoleLp, param_changed};
use crate::dsp::biquad::Biquad;
use crate::dsp::oversample::Oversampler4;

/// Ibanez TS-808 Tube Screamer simulation.
///
/// Signal path:
///   DC block → input coupling HP (~60 Hz) → 720 Hz mid-peak boost → asymmetric
///   diode clipper → output coupling cap (DC block) → variable tone LP → level
///
/// The real TS-808's 720 Hz characteristic comes from a frequency-dependent feedback network
/// inside the clipping op-amp: it gives more gain in the mids/highs relative to the bass,
/// but does NOT block the guitar fundamental from entering the stage. The input coupling cap
/// only cuts below ~60 Hz. Modeling the 720 Hz as an input HP (as many simulations do)
/// strips the fundamental from lower notes and causes intermodulation artifacts ("sitar" sound).
/// We instead model it as a peak boost at 720 Hz before the clipper.
///
/// Authenticity — why the output coupling cap matters:
///   The asymmetric diode pair (one diode one way, two the other) is what gives the
///   TS its warm, vocal **even-harmonic** character — but asymmetric clipping of a
///   symmetric input also leaves a *static DC bias* on the output. The real pedal's
///   output coupling cap strips that DC while leaving the 2nd harmonic (0 Hz ≠ the
///   harmonic) untouched. Without it, that bias rides into the amp's gain stage,
///   shifts its operating point, and turns the clean even-harmonic warmth into a
///   lopsided, intermodulating "growl" — the electronic/artificial tell. We model
///   the cap as a post-clip high-pass so the warmth stays but the bias does not.
pub struct TubeScreamer {
    sr: f32,
    dc_block: Biquad,
    input_hp: Biquad,
    mid_peak: Biquad, // 720 Hz feedback network peak — TS mid-push character
    os: Oversampler4, // 4× oversample the soft-clip stage to suppress aliasing
    // (the TS is a mild tanh soft-clip — 4× already keeps fold-back well down,
    // unlike the amp's harsher cascaded stages which earn 8×)
    out_dc_block: Biquad, // output coupling cap — strips the asymmetric clip's DC bias
    tone: OnePoleLp,      // variable 1-pole LP tone control
    last_tone: f32,
}

impl TubeScreamer {
    pub fn new(sr: f32) -> Self {
        let mut ts = Self {
            sr,
            dc_block: Biquad::highpass(sr, 10.0, 0.707),
            // TS-808 input coupling cap: 0.047µF into 10kΩ → f = 1/(2π×RC) ≈ 340 Hz.
            // This cuts the sub-bass before the clipper without stripping guitar fundamentals
            // as aggressively as the 720 Hz feedback-network frequency would.
            input_hp: Biquad::highpass(sr, 340.0, 0.707),
            // Models the TS-808 feedback network resonance: mid-push centered at 720 Hz
            mid_peak: Biquad::peak_eq(sr, 720.0, 0.7, 6.0),
            os: Oversampler4::new(sr),
            // Output coupling cap: a gentle ~20 Hz high-pass removes the static DC
            // the asymmetric clipper leaves on the signal (see struct docs) without
            // touching anything in the guitar's range.
            out_dc_block: Biquad::highpass(sr, 20.0, 0.707),
            tone: OnePoleLp::new(),
            last_tone: -1.0, // force first update
        };
        ts.set_tone(0.6);
        ts
    }

    fn set_tone(&mut self, tone: f32) {
        // tone 0 → ~500 Hz (dark), tone 1 → ~7 kHz (bright)
        let freq = 500.0 * (7000.0_f32 / 500.0).powf(tone);
        self.tone.set_cutoff(freq, self.sr);
        self.last_tone = tone;
    }

    /// `drive` 0–1, `tone` 0–1, `level` 0–1
    #[inline]
    pub fn process(&mut self, x: f32, drive: f32, tone: f32, level: f32) -> f32 {
        if param_changed(tone, self.last_tone) {
            self.set_tone(tone);
        }

        let x = self.dc_block.process(x);
        let x = self.input_hp.process(x);
        let x = self.mid_peak.process(x);

        // Drive: 10 kΩ fixed + up to 500 kΩ pot → gain ratio 1×–51×
        let gain = 1.0 + drive * 50.0;

        // 4× oversampled soft-clip stage
        let x = self
            .os
            .process(x, |u| asymmetric_clip(u * gain) / gain.sqrt());

        // Output coupling cap: strip the asymmetric clip's DC bias before tone.
        let x = self.out_dc_block.process(x);

        self.tone.process(x) * level * 0.5
    }
}

/// Asymmetric diode clipping (TS808 feedback network).
///
/// Positive half: one 1N914 silicon diode → clips at ~0.7 V (threshold = 1.0 normalised)
/// Negative half: two diodes in series → clips at ~1.4 V (threshold = 1.5 normalised)
///
/// The asymmetry introduces even harmonics (2nd harmonic) giving the warm, vocal TS tone.
#[inline]
fn asymmetric_clip(x: f32) -> f32 {
    if x >= 0.0 {
        // single diode: softer, saturates earlier. `tanh` already asymptotes to
        // 1.0, so it never exceeds the diode threshold — no extra clamp needed.
        x.tanh()
    } else {
        // Two diodes in series: higher threshold, saturating toward −1.5. `x` is
        // negative here, so `t·tanh(x/t)` is already negative and approaches −t —
        // do NOT negate it again, or the negative half flips positive and the
        // stage becomes a full-wave rectifier (octave-up ghosting, no fundamental).
        let t = 1.5_f32;
        t * (x / t).tanh()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const SR: f32 = 48_000.0;

    /// Single-bin amplitude estimate via the Goertzel algorithm (a unit sine reads
    /// ~1.0 over integer cycles). Computed in `f64`: the recurrence's poles sit on
    /// the unit circle, so over the long windows these tests use, an `f32`
    /// accumulator loses the small bins (the fundamental can read as noise).
    fn goertzel(samples: &[f32], f: f32, sr: f32) -> f32 {
        let w = 2.0 * std::f64::consts::PI * f as f64 / sr as f64;
        let coeff = 2.0 * w.cos();
        let (mut s1, mut s2) = (0.0f64, 0.0f64);
        for &x in samples {
            let s0 = x as f64 + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        let real = s1 - s2 * w.cos();
        let imag = s2 * w.sin();
        ((real * real + imag * imag).sqrt() / (samples.len() as f64 / 2.0)) as f32
    }

    fn rms(v: &[f32]) -> f64 {
        (v.iter().map(|s| (*s as f64) * (*s as f64)).sum::<f64>() / v.len() as f64).sqrt()
    }

    /// Render a steady sine through the TS at the given controls and return the
    /// settled tail. `f0` should divide the window into whole cycles so the Goertzel
    /// bins land exactly on the harmonics (no spectral leakage).
    fn render(f0: f32, amp: f32, drive: f32, tone: f32, level: f32) -> Vec<f32> {
        let mut ts = TubeScreamer::new(SR);
        let n = SR as usize; // 1.0 s
        let warmup = n / 4; // 12 000 — a whole number of periods for the test tones
        let mut out = Vec::with_capacity(n - warmup);
        for i in 0..n {
            let x = (2.0 * PI * f0 * i as f32 / SR).sin() * amp;
            let y = ts.process(x, drive, tone, level);
            assert!(y.is_finite(), "TS non-finite at {i}");
            if i >= warmup {
                out.push(y);
            }
        }
        out
    }

    /// The TS must stay finite and bounded driving a hot low note, and its drive
    /// knob must actually add saturation harmonics (more drive → hotter output).
    #[test]
    fn finite_bounded_and_drive_adds_gain() {
        let sr = 48_000.0;
        let rms_at = |drive: f32| {
            let mut ts = TubeScreamer::new(sr);
            let mut sum = 0.0f64;
            let warmup = sr as usize / 4;
            let mut count = 0u32;
            for n in 0..(sr as usize) {
                let x = (2.0 * PI * 110.0 * n as f32 / sr).sin() * 0.5;
                let y = ts.process(x, drive, 0.6, 0.7);
                assert!(y.is_finite(), "TS produced non-finite output");
                assert!(y.abs() < 2.0, "TS output unbounded: {y}");
                if n >= warmup {
                    sum += (y * y) as f64;
                    count += 1;
                }
            }
            (sum / count as f64).sqrt()
        };
        assert!(rms_at(0.9) > rms_at(0.1), "drive knob did not add level");
    }

    /// Turning the tone knob up must brighten the output: more high-frequency
    /// energy passes through the variable low-pass.
    #[test]
    fn tone_controls_brightness() {
        let sr = 48_000.0;
        let high_energy = |tone: f32| {
            let mut ts = TubeScreamer::new(sr);
            let mut sum = 0.0f64;
            let warmup = sr as usize / 4;
            for n in 0..(sr as usize) {
                let x = (2.0 * PI * 4000.0 * n as f32 / sr).sin() * 0.4;
                let y = ts.process(x, 0.5, tone, 0.7);
                if n >= warmup {
                    sum += (y * y) as f64;
                }
            }
            sum
        };
        assert!(
            high_energy(0.9) > high_energy(0.1),
            "tone knob did not control brightness"
        );
    }

    // ── Clip transfer (unit tests on the diode model itself) ──────────────────

    /// The clipper must be a real bipolar saturator: sign-preserving, bounded by the
    /// two diode thresholds, and monotonic. The sign check is a direct guard on the
    /// rectifier bug — negating the negative half once turned the stage into a
    /// full-wave rectifier (frequency-doubling octave ghost).
    #[test]
    fn clip_preserves_sign_is_bounded_and_monotonic() {
        let mut prev = f32::NEG_INFINITY;
        let mut x = -8.0f32;
        while x <= 8.0 {
            let y = asymmetric_clip(x);
            assert!(y.is_finite(), "clip non-finite at {x}");
            assert!(y > -1.5001 && y < 1.0001, "clip out of bounds at {x}: {y}");
            if x > 0.01 {
                assert!(
                    y > 0.0,
                    "positive input {x} produced non-positive {y} (rectifying!)"
                );
            }
            if x < -0.01 {
                assert!(
                    y < 0.0,
                    "negative input {x} produced non-negative {y} (rectifying!)"
                );
            }
            assert!(y >= prev - 1e-6, "clip not monotonic at {x}: {y} < {prev}");
            prev = y;
            x += 0.01;
        }
        // Diode thresholds: +1.0 (single) and −1.5 (two in series).
        assert!(
            (asymmetric_clip(20.0) - 1.0).abs() < 1e-3,
            "positive threshold off"
        );
        assert!(
            (asymmetric_clip(-20.0) + 1.5).abs() < 1e-2,
            "negative threshold off"
        );
    }

    /// The diode asymmetry (one diode up, two down) is the source of the warm 2nd
    /// harmonic: the negative half must saturate deeper than the positive half. A
    /// symmetric clipper would make `|f(a)| == |f(-a)|` and sound clinical.
    #[test]
    fn clip_is_asymmetric_for_even_harmonic_warmth() {
        for a in [0.8f32, 1.2, 2.0, 4.0] {
            let pos = asymmetric_clip(a);
            let neg = asymmetric_clip(-a);
            assert!(
                neg.abs() > pos.abs() * 1.05,
                "clip not asymmetric at ±{a}: +{pos:.4} / {neg:.4}"
            );
        }
    }

    // ── Full-stage spectral behaviour ─────────────────────────────────────────

    /// Regression for the rectifier bug: the negative half was negated, turning the
    /// clipper into a full-wave rectifier, so a 440 Hz note emerged as a 880 Hz
    /// octave ghost — the most blatant "electronic/artificial" failure. The played
    /// fundamental must dominate its octave by a wide margin.
    #[test]
    fn negative_half_is_not_rectified() {
        for f0 in [440.0f32, 587.33, 659.25] {
            let out = render(f0, 0.3, 0.5, 0.6, 0.7);
            let fund = goertzel(&out, f0, SR);
            let octave = goertzel(&out, 2.0 * f0, SR);
            assert!(
                fund > 4.0 * octave,
                "{f0} Hz: octave ghost (fund {fund:.4}, 2f {octave:.4}) — rectifying?"
            );
        }
    }

    /// The output coupling cap must strip the static DC bias the asymmetric clipper
    /// leaves behind, so it doesn't shift the amp's downstream bias point.
    #[test]
    fn output_has_no_dc_offset() {
        for f0 in [110.0f32, 440.0, 880.0] {
            let out = render(f0, 0.6, 0.8, 0.6, 0.7);
            let mean = out.iter().map(|&x| x as f64).sum::<f64>() / out.len() as f64;
            assert!(mean.abs() < 1e-3, "{f0} Hz: DC offset {mean:.6}");
        }
    }

    /// A properly oversampled clipper puts essentially all of its energy on the
    /// exact harmonics of the input. Aliasing — the harsh digital "fizz" — would
    /// scatter energy onto inharmonic bins and pull this fraction down.
    #[test]
    fn output_is_harmonic_not_aliased() {
        for (f0, drive, tone) in [
            (440.0f32, 0.6f32, 0.6f32),
            (660.0, 0.9, 0.8),
            (2000.0, 0.8, 1.0),
        ] {
            let out = render(f0, 0.4, drive, tone, 0.7);
            let harm_pow: f64 = (1..=10)
                .map(|k| (goertzel(&out, f0 * k as f32, SR) as f64).powi(2) / 2.0)
                .sum();
            let frac = harm_pow / rms(&out).powi(2);
            assert!(
                frac > 0.95,
                "{f0} Hz: only {:.1}% of energy is harmonic — aliasing present",
                100.0 * frac
            );
        }
    }

    /// A mild TS overdrive adds harmonics but must not bury the played note: the
    /// fundamental stays the loudest partial across the usable range (a buried
    /// fundamental is the loss-of-pitch "fizz" we are guarding against).
    #[test]
    fn fundamental_leads_the_spectrum() {
        for f0 in [440.0f32, 523.25, 659.25, 880.0] {
            let out = render(f0, 0.3, 0.45, 0.6, 0.7);
            let h: Vec<f32> = (1..=8).map(|k| goertzel(&out, f0 * k as f32, SR)).collect();
            let peak = h.iter().cloned().fold(0.0f32, f32::max);
            assert!(
                h[0] >= peak * 0.99,
                "{f0} Hz: fundamental ({:.4}) is not the loudest partial ({peak:.4})",
                h[0]
            );
        }
    }

    /// Turning the drive up must add saturation harmonics: total harmonic content
    /// relative to the fundamental rises monotonically with the drive knob.
    #[test]
    fn drive_increases_harmonic_distortion() {
        let thd = |drive: f32| {
            let out = render(523.25, 0.1, drive, 0.8, 0.7);
            let fund = goertzel(&out, 523.25, SR);
            let upper: f32 = (2..=8).map(|k| goertzel(&out, 523.25 * k as f32, SR)).sum();
            upper / fund.max(1e-9)
        };
        let (lo, mid, hi) = (thd(0.05), thd(0.4), thd(0.9));
        assert!(
            hi > mid && mid > lo,
            "drive did not add harmonics monotonically: {lo:.3} -> {mid:.3} -> {hi:.3}"
        );
    }
}
