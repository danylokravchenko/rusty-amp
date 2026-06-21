use super::biquad::Biquad;
use crate::dsp::oversample::Oversampler4;
use std::f32::consts::PI;

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
    tone_z: f32,
    tone_coeff: f32,
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
            tone_z: 0.0,
            tone_coeff: 0.0,
            last_tone: -1.0, // force first update
        };
        fz.set_tone(0.5);
        fz
    }

    fn set_tone(&mut self, tone: f32) {
        // tone 0 → ~400 Hz (dark/woolly), tone 1 → ~6 kHz (bright/buzzy)
        let freq = 400.0 * (6000.0_f32 / 400.0).powf(tone);
        self.tone_coeff = 1.0 - (-2.0 * PI * freq / self.sr).exp();
        self.last_tone = tone;
    }

    /// `fuzz` 0–1 (sustain/gain), `tone` 0–1, `level` 0–1
    #[inline]
    pub fn process(&mut self, x: f32, fuzz: f32, tone: f32, level: f32) -> f32 {
        if (tone - self.last_tone).abs() > 0.001 {
            self.set_tone(tone);
        }

        let x = self.dc_block.process(x);
        let x = self.input_hp.process(x);

        // Enormous gain into the cascaded clippers — this is what makes it a fuzz
        // rather than an overdrive.
        let gain = 1.0 + fuzz * 120.0;

        // ── 4× oversampled two-stage clip ─────────────────────────────────────
        let up = self.os.upsample(x);
        let mut down = [0.0f32; 4];
        for (o, &u) in down.iter_mut().zip(up.iter()) {
            let s1 = fuzz_clip(u * gain);
            let s2 = fuzz_clip(s1 * 2.5);
            *o = s2;
        }
        let x = self.os.downsample(down);
        // ── end oversampled section ───────────────────────────────────────────

        let x = self.post_dc.process(x);
        let x = self.scoop.process(x);

        // Tone: variable 1-pole low-pass.
        self.tone_z += self.tone_coeff * (x - self.tone_z);

        self.tone_z * level * 0.5
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
