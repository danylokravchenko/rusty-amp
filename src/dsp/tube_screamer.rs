use super::biquad::Biquad;
use std::f32::consts::PI;

/// Ibanez TS-808 Tube Screamer simulation.
///
/// Signal path:
///   DC block → 720 Hz high-pass → asymmetric diode clipper → variable tone LP → level
///
/// The 720 Hz HP before clipping is the key character of the TS sound: it removes low-end
/// before the gain stage so the distortion stays "focused" rather than flabby.
pub struct TubeScreamer {
    sr: f32,
    dc_block: Biquad,
    input_hp: Biquad,
    tone_z: f32, // 1-pole LP state for variable tone control
    last_tone: f32,
    tone_coeff: f32,
}

impl TubeScreamer {
    pub fn new(sr: f32) -> Self {
        let mut ts = Self {
            sr,
            dc_block: Biquad::highpass(sr, 10.0, 0.707),
            input_hp: Biquad::highpass(sr, 720.0, 0.707),
            tone_z: 0.0,
            last_tone: -1.0, // force first update
            tone_coeff: 0.0,
        };
        ts.set_tone(0.6);
        ts
    }

    fn set_tone(&mut self, tone: f32) {
        // tone 0 → ~500 Hz (dark), tone 1 → ~7 kHz (bright)
        let freq = 500.0 * (7000.0_f32 / 500.0).powf(tone);
        self.tone_coeff = 1.0 - (-2.0 * PI * freq / self.sr).exp();
        self.last_tone = tone;
    }

    /// `drive` 0–1, `tone` 0–1, `level` 0–1
    #[inline]
    pub fn process(&mut self, x: f32, drive: f32, tone: f32, level: f32) -> f32 {
        if (tone - self.last_tone).abs() > 0.001 {
            self.set_tone(tone);
        }

        let x = self.dc_block.process(x);
        let x = self.input_hp.process(x);

        // Drive: 10 kΩ fixed + up to 500 kΩ pot → gain ratio 1×–51×
        let gain = 1.0 + drive * 50.0;
        let x = asymmetric_clip(x * gain) / gain.sqrt();

        // Tone: variable 1-pole low-pass
        self.tone_z += self.tone_coeff * (x - self.tone_z);

        self.tone_z * level * 0.5
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
        // single diode: softer, saturates earlier
        1.0_f32.min(x.tanh())
    } else {
        // two diodes in series: higher threshold, asymmetric saturation
        let t = 1.5_f32;
        -(t * (x / t).tanh())
    }
}
