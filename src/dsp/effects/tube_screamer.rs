use super::{OnePoleLp, param_changed};
use crate::dsp::biquad::Biquad;
use crate::dsp::oversample::Oversampler4;

/// Ibanez TS-808 Tube Screamer simulation.
///
/// Signal path:
///   DC block → input coupling HP (~60 Hz) → 720 Hz mid-peak boost → asymmetric diode clipper → variable tone LP → level
///
/// The real TS-808's 720 Hz characteristic comes from a frequency-dependent feedback network
/// inside the clipping op-amp: it gives more gain in the mids/highs relative to the bass,
/// but does NOT block the guitar fundamental from entering the stage. The input coupling cap
/// only cuts below ~60 Hz. Modeling the 720 Hz as an input HP (as many simulations do)
/// strips the fundamental from lower notes and causes intermodulation artifacts ("sitar" sound).
/// We instead model it as a peak boost at 720 Hz before the clipper.
pub struct TubeScreamer {
    sr: f32,
    dc_block: Biquad,
    input_hp: Biquad,
    mid_peak: Biquad, // 720 Hz feedback network peak — TS mid-push character
    os: Oversampler4, // 4× oversample the soft-clip stage to suppress aliasing
    // (the TS is a mild tanh soft-clip — 4× already keeps fold-back well down,
    // unlike the amp's harsher cascaded stages which earn 8×)
    tone: OnePoleLp, // variable 1-pole LP tone control
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
        // single diode: softer, saturates earlier
        1.0_f32.min(x.tanh())
    } else {
        // two diodes in series: higher threshold, asymmetric saturation
        let t = 1.5_f32;
        -(t * (x / t).tanh())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

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
}
