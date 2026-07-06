use super::{OnePoleLp, param_changed};
use std::f32::consts::TAU;

/// Pitch shifter / Whammy — a time-domain, allocation-free pitch transposer.
///
/// Sits early in the mono chain (right after the gate, before the drives) so it
/// works like a real whammy pedal: the *shifted* note is what the amp's gain stage
/// then clips. That placement is the whole point — octave-down for a fake-baritone
/// heaviness, octave-up shrieks and dive-bombs, all feeding raw into the distortion.
///
/// Algorithm: the classic two-tap crossfaded varispeed delay line ("H910"-style
/// granular shifter). Input is written to a ring buffer; two read taps sweep through
/// it at a rate set by the pitch ratio, kept half a grain apart. A raised-cosine
/// window fades each tap to silence exactly at the point where it wraps past the
/// grain boundary, so the two taps crossfade seamlessly and the wrap discontinuity
/// is inaudible. It is not phase-vocoder clean — a little warble is inherent — but
/// that roughness is part of the whammy character, and it costs almost nothing.
///
/// Knob ranges (all normalised 0–1):
///   PITCH → transpose interval: 0 = −12 st (octave down), 0.5 = unison, 1 = +12 st.
///   MIX   → dry/wet blend; 0 = dry, 1 = fully wet (full dive-bomb).
///   TONE  → low-pass on the wet path, taming the granular fizz; 0 = dark, 1 = open.
pub struct Pitch {
    buf: Vec<f32>,
    write: usize,
    /// Grain phase in [0, 1). `phase * GRAIN` is the delay of the first read tap;
    /// the second tap reads half a grain away. Advances by `(1 − ratio)/GRAIN`.
    phase: f32,
    tone: OnePoleLp,
    last_tone: f32,
    sr: f32,
}

/// Grain length in samples. ~43 ms at 48 kHz — long enough to hide the wrap under
/// the window, short enough that transposed transients don't smear badly.
const GRAIN: f32 = 2048.0;
/// Ring buffer length (power of two ≥ `GRAIN` + interpolation headroom).
const BUF_LEN: usize = 4096;

impl Pitch {
    pub fn new(sr: f32) -> Self {
        let mut p = Self {
            buf: vec![0.0; BUF_LEN],
            write: 0,
            phase: 0.0,
            tone: OnePoleLp::new(),
            last_tone: -1.0, // force first update
            sr,
        };
        p.set_tone(0.7);
        p
    }

    fn set_tone(&mut self, tone: f32) {
        // tone 0 → ~800 Hz (dark, artifacts rolled off), tone 1 → ~10 kHz (open)
        let freq = 800.0 * (10_000.0_f32 / 800.0).powf(tone.clamp(0.0, 1.0));
        self.tone.set_cutoff(freq, self.sr);
        self.last_tone = tone;
    }

    /// Linear-interpolated read `delay` samples behind the write head.
    #[inline]
    fn read(buf: &[f32], write: usize, delay: f32) -> f32 {
        let len = buf.len();
        let d = delay.clamp(0.0, (len - 2) as f32);
        let i0 = d.floor() as usize;
        let frac = d - i0 as f32;
        let a = (write + len - i0) % len;
        let b = (write + len - i0 - 1) % len;
        buf[a] * (1.0 - frac) + buf[b] * frac
    }

    /// `pitch` 0–1 (0.5 = unison), `mix` 0–1, `tone` 0–1.
    #[inline]
    pub fn process(&mut self, x: f32, pitch: f32, mix: f32, tone: f32) -> f32 {
        if param_changed(tone, self.last_tone) {
            self.set_tone(tone);
        }

        // Write the incoming sample first, so a zero-delay tap reads exactly `x`.
        self.buf[self.write] = x;

        // Map the knob to a transpose ratio: ±12 semitones around unison.
        let semis = (pitch.clamp(0.0, 1.0) - 0.5) * 24.0;
        let ratio = (semis / 12.0).exp2();

        // The read taps must advance `ratio` samples per output sample while the
        // write head advances one, so the tap *delay* drifts by `(1 − ratio)` per
        // sample; dividing by GRAIN expresses that drift in grain-phase units.
        self.phase += (1.0 - ratio) / GRAIN;
        self.phase -= self.phase.floor(); // wrap into [0, 1)
        let phase_b = {
            let p = self.phase + 0.5;
            p - p.floor()
        };

        let wet = {
            let tap_a = self.phase * GRAIN;
            let tap_b = phase_b * GRAIN;
            // Raised-cosine grain windows: each fades to zero at its wrap point
            // (phase 0/1) and peaks mid-grain; being half a grain apart, their sum
            // is unity, so the two taps crossfade at constant gain.
            let win_a = 0.5 - 0.5 * (self.phase * TAU).cos();
            let win_b = 0.5 - 0.5 * (phase_b * TAU).cos();
            win_a * Self::read(&self.buf, self.write, tap_a)
                + win_b * Self::read(&self.buf, self.write, tap_b)
        };

        self.write = (self.write + 1) % BUF_LEN;

        let wet = self.tone.process(wet);
        let mix = mix.clamp(0.0, 1.0);
        x * (1.0 - mix) + wet * mix
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const SR: f32 = 48_000.0;

    /// Single-bin amplitude estimate via Goertzel (f64 accumulators: the recurrence
    /// sits on the unit circle and loses small bins in f32 over long windows).
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

    /// Render a steady sine through the shifter and return the settled tail (past the
    /// initial buffer fill), asserting finiteness along the way.
    fn render(f0: f32, pitch: f32, mix: f32, tone: f32) -> Vec<f32> {
        let mut p = Pitch::new(SR);
        let n = SR as usize;
        let warmup = n / 4;
        let mut out = Vec::with_capacity(n - warmup);
        for i in 0..n {
            let x = (2.0 * PI * f0 * i as f32 / SR).sin() * 0.6;
            let y = p.process(x, pitch, mix, tone);
            assert!(y.is_finite(), "pitch non-finite at {i}");
            if i >= warmup {
                out.push(y);
            }
        }
        out
    }

    /// Center pitch does not matter for `mix = 0`: fully dry must pass through the
    /// input untouched, so the pedal is transparent when blended out.
    #[test]
    fn fully_dry_is_passthrough() {
        let mut p = Pitch::new(SR);
        for n in 0..4000 {
            let x = (n as f32 * 0.017).sin() * 0.5;
            let y = p.process(x, 1.0, 0.0, 0.5);
            assert!((y - x).abs() < 1e-6, "dry mix altered the signal at {n}");
        }
    }

    /// Extreme settings must stay finite and bounded — the unity-sum windows mean the
    /// wet tap can never exceed the buffered peak, so the blend is always well behaved.
    #[test]
    fn finite_and_bounded_under_extremes() {
        let mut p = Pitch::new(SR);
        let mut max_abs = 0.0f32;
        for n in 0..(SR as usize) {
            let x = (2.0 * PI * 110.0 * n as f32 / SR).sin() * 0.9;
            let y = p.process(x, 0.0, 1.0, 1.0);
            assert!(y.is_finite(), "non-finite at {n}");
            max_abs = max_abs.max(y.abs());
        }
        assert!(max_abs < 2.0, "pitch output out of bounds: {max_abs}");
    }

    /// Pitch fully up (+12 st) must transpose a note up an octave: the shifted
    /// fundamental at 2·f0 must dominate the original f0 in the wet output.
    #[test]
    fn octave_up_moves_the_fundamental_up() {
        let f0 = 500.0;
        let out = render(f0, 1.0, 1.0, 1.0);
        let shifted = goertzel(&out, 2.0 * f0, SR);
        let original = goertzel(&out, f0, SR);
        assert!(
            shifted > 2.0 * original,
            "octave-up weak: 2f0 {shifted:.4} vs f0 {original:.4}"
        );
    }

    /// Pitch fully down (−12 st) must transpose a note down an octave: the shifted
    /// fundamental at f0/2 must dominate the original f0.
    #[test]
    fn octave_down_moves_the_fundamental_down() {
        let f0 = 1000.0;
        let out = render(f0, 0.0, 1.0, 1.0);
        let shifted = goertzel(&out, f0 / 2.0, SR);
        let original = goertzel(&out, f0, SR);
        assert!(
            shifted > 2.0 * original,
            "octave-down weak: f0/2 {shifted:.4} vs f0 {original:.4}"
        );
    }

    /// The tone knob must roll off the wet path's brightness: shifting a bright note
    /// up an octave and opening the tone must pass more high-frequency energy than a
    /// dark setting does.
    #[test]
    fn tone_controls_wet_brightness() {
        let energy = |tone: f32| {
            let out = render(3000.0, 1.0, 1.0, tone);
            out.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>()
        };
        assert!(
            energy(0.9) > energy(0.1) * 1.5,
            "tone knob did not control wet brightness"
        );
    }
}
