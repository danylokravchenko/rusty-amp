use super::{OnePoleLp, param_changed};
use crate::dsp::biquad::Biquad;
use crate::dsp::oversample::Oversampler4;

/// Big Muff–style fuzz simulation.
///
/// Signal path:
///   DC block → input HP (~70 Hz) → [4× OS: two cascaded asymmetric soft-clip
///   stages] → DC block → mid scoop → variable tone LP → level
///
/// Fuzz character & authenticity:
///   • A fuzz is far more saturated than an overdrive/distortion: it slams the
///     signal into near-square clipping for long, singing sustain. We model the
///     classic two–transistor-stage gain structure as **two cascaded soft-clip
///     stages** inside the oversampler — one stage alone stays too "polite".
///   • The clipping is mildly **asymmetric**, which is what gives a fuzz its
///     spitty, gated edge and a touch of octave texture on the top.
///   • The Big Muff's voice is **mid-scooped** — a fixed dip around 700 Hz gives
///     the scooped, wall-of-sound timbre without depending on the tone knob.
///   • The tone control is a simple dark→bright low-pass sweep, like the passive
///     tone stage feeding the output buffer.
///   • 4× oversampling is essential here: square-ish clipping is extremely rich
///     in harmonics, so the alias products must be pushed well above the band.
pub struct Fuzz {
    sr: f32,
    dc_block: Biquad,
    input_hp: Biquad,
    os: Oversampler4,
    // Removes the DC the asymmetric clipper injects before it reaches the amp.
    post_dc: Biquad,
    // Fixed mid scoop — the Big Muff "smiley" voicing.
    scoop: Biquad,
    // Variable 1-pole low-pass for the tone control.
    tone: OnePoleLp,
    last_tone: f32,
}

impl Fuzz {
    pub fn new(sr: f32) -> Self {
        let mut fz = Self {
            sr,
            dc_block: Biquad::highpass(sr, 10.0, 0.707),
            // Tighten the very low end before the huge gain so the fuzz doesn't
            // turn to mud, but keep the guitar fundamental intact.
            input_hp: Biquad::highpass(sr, 70.0, 0.707),
            os: Oversampler4::new(sr),
            post_dc: Biquad::highpass(sr, 45.0, 0.707),
            // −9 dB dip at 700 Hz: the scooped Muff midrange.
            scoop: Biquad::peak_eq(sr, 700.0, 0.7, -9.0),
            tone: OnePoleLp::new(),
            last_tone: -1.0, // force first update
        };
        fz.set_tone(0.5);
        fz
    }

    fn set_tone(&mut self, tone: f32) {
        // tone 0 → ~400 Hz (dark/woolly), tone 1 → ~6 kHz (bright/buzzy)
        let freq = 400.0 * (6000.0_f32 / 400.0).powf(tone);
        self.tone.set_cutoff(freq, self.sr);
        self.last_tone = tone;
    }

    /// `fuzz` 0–1 (sustain/gain), `tone` 0–1, `level` 0–1
    #[inline]
    pub fn process(&mut self, x: f32, fuzz: f32, tone: f32, level: f32) -> f32 {
        if param_changed(tone, self.last_tone) {
            self.set_tone(tone);
        }

        let x = self.dc_block.process(x);
        let x = self.input_hp.process(x);

        // Enormous gain into the cascaded clippers — this is what makes it a fuzz
        // rather than an overdrive.
        let gain = 1.0 + fuzz * 120.0;

        // 4× oversampled two-stage clip.
        let x = self.os.process(x, |u| {
            let s1 = fuzz_clip(u * gain);
            fuzz_clip(s1 * 2.5)
        });

        let x = self.post_dc.process(x);
        let x = self.scoop.process(x);

        self.tone.process(x) * level * 0.5
    }
}

/// Asymmetric soft clipper for the fuzz gain stages.
///
/// The positive half saturates a touch harder than the negative half. Cascading
/// two of these drives the waveform toward a gated square wave (long sustain),
/// and the asymmetry seeds the even-harmonic "octave" shimmer fuzz is known for.
#[inline]
fn fuzz_clip(x: f32) -> f32 {
    if x >= 0.0 {
        x.tanh()
    } else {
        0.85 * (x / 0.85).tanh()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// The fuzz must stay finite and bounded even at maximum sustain on a hot, low
    /// note — its two cascaded clippers run at enormous gain, so any instability or
    /// runaway DC would show up here.
    #[test]
    fn finite_bounded_and_saturates() {
        let sr = 48_000.0;
        let mut fz = Fuzz::new(sr);
        let mut max_abs = 0.0f32;
        let mut sum = 0.0f64;
        let warmup = sr as usize / 4;
        let total = sr as usize;
        let mut count = 0u32;
        for n in 0..total {
            let x = (2.0 * PI * 82.41 * n as f32 / sr).sin() * 0.8;
            let y = fz.process(x, 1.0, 0.5, 0.7);
            assert!(y.is_finite(), "fuzz produced non-finite output at {n}");
            if n >= warmup {
                max_abs = max_abs.max(y.abs());
                sum += y as f64;
                count += 1;
            }
        }
        assert!(max_abs <= 1.0, "fuzz output exceeded bounds: {max_abs}");
        // Heavy clipping should still produce a healthy signal, not silence.
        assert!(max_abs > 0.05, "fuzz output too quiet: {max_abs}");
        // Asymmetric clipping is fine, but the post-DC block must keep the mean
        // near zero so the fuzz doesn't push DC into the amp.
        let dc = (sum / count as f64).abs();
        assert!(dc < 0.02, "fuzz has DC offset: {dc}");
    }
}
