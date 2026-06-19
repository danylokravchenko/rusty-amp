use super::Amplifier;
use crate::dsp::biquad::Biquad;

/// Mesa/Boogie Dual Rectifier — Modern channel simulation.
///
/// Signal path:
///   DC block → input HP (60 Hz) → three preamp gain stages → passive tone stack
///   → silicon-rectifier power amp (tight sag, fast attack)
///
/// Key differences from the Marshall JCM800:
///   • Three gain stages instead of two — more compression, less dynamics
///   • Tone stack centred higher (mid at 750 Hz, treble at 3.3 kHz vs 400 / 2.5 kHz)
///   • Silicon rectifier sag: 10× faster attack, 3× faster release → tighter, punchier
///   • Third stage uses a harder exponential waveshaper (rectifier character)
pub struct Mesa {
    sr: f32,
    dc_block: Biquad,
    input_hp: Biquad,
    bass_shelf: Biquad,
    mid_peak: Biquad,
    treble_shelf: Biquad,
    last_bass: f32,
    last_mid: f32,
    last_treble: f32,
    envelope: f32,
}

impl Mesa {
    pub fn new(sr: f32) -> Self {
        let mut m = Self {
            sr,
            dc_block: Biquad::highpass(sr, 10.0, 0.707),
            input_hp: Biquad::highpass(sr, 60.0, 0.707),
            bass_shelf: Biquad::low_shelf(sr, 100.0, 0.0),
            mid_peak: Biquad::peak_eq(sr, 750.0, 0.5, 0.0),
            treble_shelf: Biquad::high_shelf(sr, 3300.0, 0.0),
            last_bass: -1.0,
            last_mid: -1.0,
            last_treble: -1.0,
            envelope: 0.0,
        };
        m.update_tone_stack(0.5, 0.45, 0.65);
        m
    }

    fn update_tone_stack(&mut self, bass: f32, mid: f32, treble: f32) {
        // ±15 dB for bass / treble, ±12 dB for mid (same range as Marshall)
        self.bass_shelf = Biquad::low_shelf(self.sr, 100.0, (bass - 0.5) * 30.0);
        self.mid_peak = Biquad::peak_eq(self.sr, 750.0, 0.5, (mid - 0.5) * 24.0);
        self.treble_shelf = Biquad::high_shelf(self.sr, 3300.0, (treble - 0.5) * 30.0);
        self.last_bass = bass;
        self.last_mid = mid;
        self.last_treble = treble;
    }

    /// Silicon rectifier sag: much tighter than a tube rectifier.
    /// Attack ~0.5 ms (vs ~5 ms), release ~80 ms (vs ~200 ms).
    #[inline]
    fn power_amp(&mut self, x: f32) -> f32 {
        let abs_x = x.abs();
        let coeff = if abs_x > self.envelope {
            1.0 - (-1.0 / (0.0005 * self.sr)).exp() // 0.5 ms attack
        } else {
            1.0 - (-1.0 / (0.080 * self.sr)).exp() // 80 ms release
        };
        self.envelope += coeff * (abs_x - self.envelope);

        // Less sag headroom than the Marshall (silicon is stiffer than a tube rectifier)
        let sag = 1.0 / (1.0 + self.envelope * 0.35);
        silicon_clip(x * sag * 2.5) * 0.4
    }
}

impl Amplifier for Mesa {
    #[inline]
    fn process(
        &mut self,
        sample: f32,
        gain: f32,
        bass: f32,
        mid: f32,
        treble: f32,
        master: f32,
    ) -> f32 {
        if (bass - self.last_bass).abs() > 0.001
            || (mid - self.last_mid).abs() > 0.001
            || (treble - self.last_treble).abs() > 0.001
        {
            self.update_tone_stack(bass, mid, treble);
        }

        let x = self.dc_block.process(sample);
        let x = self.input_hp.process(x);

        // Stage 1: first 12AX7 — moderate gain, soft atan clip
        let pregain = 1.0 + gain * 35.0; // 1× – 36×
        let x = tube_clip(x * pregain) / pregain.sqrt();

        // Stage 2: second 12AX7 — fixed extra compression
        let x = tube_clip(x * 5.0) / 5.0_f32.sqrt();

        // Stage 3: silicon-diode character — harder exponential clip
        // This is what gives the Rectifier its aggressive, compressed feel
        let x = silicon_clip(x * 3.0) / 3.0_f32.sqrt();

        // Tone stack
        let x = self.bass_shelf.process(x);
        let x = self.mid_peak.process(x);
        let x = self.treble_shelf.process(x);

        // Silicon rectifier power amp
        let x = self.power_amp(x);

        x * master * 0.8
    }
}

/// 12AX7 triode soft clip (arctangent).
#[inline]
fn tube_clip(x: f32) -> f32 {
    use std::f32::consts::FRAC_2_PI;
    FRAC_2_PI * x.atan()
}

/// Silicon diode / exponential waveshaper.
/// f(x) = sign(x) * (1 - exp(-|x|))  →  approaches ±1 faster than atan.
#[inline]
fn silicon_clip(x: f32) -> f32 {
    x.signum() * (1.0 - (-x.abs()).exp())
}
