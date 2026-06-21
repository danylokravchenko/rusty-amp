use super::{Amplifier, Bloom, SpeakerLoad};
use crate::dsp::biquad::Biquad;
use crate::dsp::oversample::Oversampler8;
use crate::dsp::tonestack::{Components, ToneStack};

/// Mesa/Boogie Dual Rectifier — Modern channel simulation.
///
/// Signal path:
///   DC block → input HP → [4× OS: stage-1 + HP + stage-2 + HP + stage-3 silicon] → tone stack → power amp → presence
///
/// Character:
///   • 4× oversampling through all three nonlinear stages keeps aliasing inaudible
///   • Asymmetric waveshapers on tube stages add even-harmonic warmth
///   • Dynamic grid-bias bloom adds touch sensitivity under hard playing
///   • Two inter-stage HPs (680 Hz and 1 kHz) prevent bass accumulation across stages
///   • Presence shelf at 4 kHz — Recto's presence is brighter/tighter than the JCM800
pub struct Mesa {
    sr: f32,
    dc_block: Biquad,
    input_hp: Biquad,
    os: Oversampler8,
    // Pre-clip HP at 8× rate — cuts sub-bass before the first gain stage
    pre_clip_hp: Biquad,
    // Two inter-stage coupling HPs at 8× rate
    stage_hp_1: Biquad,
    stage_hp_2: Biquad,
    bloom: Bloom,
    // Passive FMV tone stack (base rate) — Fender-type values for the Recto's
    // thicker low end and gentler scoop.
    tone: ToneStack,
    last_bass: f32,
    last_mid: f32,
    last_treble: f32,
    // Presence (base rate)
    presence_shelf: Biquad,
    last_presence: f32,
    // Silicon-rectifier sag envelope
    envelope: f32,
    // Power-amp ↔ speaker impedance interaction.
    speaker: SpeakerLoad,
}

impl Mesa {
    pub fn new(sr: f32) -> Self {
        let sr8 = sr * 8.0;
        let mut m = Self {
            sr,
            dc_block: Biquad::highpass(sr, 10.0, 0.707),
            input_hp: Biquad::highpass(sr, 60.0, 0.707),
            os: Oversampler8::new(sr),
            // Recto input coupling HP at ~40 Hz — sub-rumble only, still preserves
            // the 82 Hz low-E fundamental going into the gain stages.
            pre_clip_hp: Biquad::highpass(sr8, 40.0, 0.707),
            // Between stage 1 and 2: ~680 Hz (Recto coupling cap characteristic)
            stage_hp_1: Biquad::highpass(sr8, 680.0, 0.707),
            // Between stage 2 and 3: ~1 kHz (silicon stage compresses harder so HP is tighter)
            stage_hp_2: Biquad::highpass(sr8, 1000.0, 0.707),
            bloom: Bloom::new(sr, 8.0, 120.0),
            tone: ToneStack::new(sr, Components::FENDER),
            last_bass: -1.0,
            last_mid: -1.0,
            last_treble: -1.0,
            presence_shelf: Biquad::high_shelf(sr, 4000.0, 0.0),
            last_presence: -1.0,
            envelope: 0.0,
            // Recto 4×12 resonance ~100 Hz; silicon supply sags less than a tube
            // rectifier, so a slightly tighter dynamic bloom than the JCM800.
            speaker: SpeakerLoad::new(sr, 100.0, 1.0, 0.06, 0.45, 0.8),
        };
        m.update_tone_stack(0.5, 0.45, 0.65);
        m.update_presence(0.5);
        m
    }

    fn update_tone_stack(&mut self, bass: f32, mid: f32, treble: f32) {
        self.tone.update(bass, mid, treble);
        self.last_bass = bass;
        self.last_mid = mid;
        self.last_treble = treble;
    }

    fn update_presence(&mut self, presence: f32) {
        // Recto presence: 4 kHz (brighter/tighter than JCM800 3.5 kHz), ±6 dB
        self.presence_shelf = Biquad::high_shelf(self.sr, 4000.0, (presence - 0.5) * 12.0);
        self.last_presence = presence;
    }

    /// Silicon rectifier sag: tight attack (0.5 ms), moderate release (80 ms).
    #[inline]
    fn power_amp(&mut self, x: f32) -> f32 {
        let abs_x = x.abs();
        let coeff = if abs_x > self.envelope {
            1.0 - (-1.0 / (0.0005 * self.sr)).exp()
        } else {
            1.0 - (-1.0 / (0.080 * self.sr)).exp()
        };
        self.envelope += coeff * (abs_x - self.envelope);
        let sag = 1.0 / (1.0 + self.envelope * 0.35);
        silicon_clip_asym(x * sag * 2.5) * 0.4
    }
}

impl Amplifier for Mesa {
    #[allow(clippy::too_many_arguments)]
    #[inline]
    fn process(
        &mut self,
        sample: f32,
        gain: f32,
        bass: f32,
        mid: f32,
        treble: f32,
        presence: f32,
        master: f32,
    ) -> f32 {
        if (bass - self.last_bass).abs() > 0.001
            || (mid - self.last_mid).abs() > 0.001
            || (treble - self.last_treble).abs() > 0.001
        {
            self.update_tone_stack(bass, mid, treble);
        }
        if (presence - self.last_presence).abs() > 0.001 {
            self.update_presence(presence);
        }

        let x = self.dc_block.process(sample);
        let x = self.input_hp.process(x);

        let pregain = 1.0 + gain * 35.0;
        let bias = self.bloom.follow(x) * 0.12;

        // ── 4× oversampled nonlinear section ──────────────────────────────────
        let up = self.os.upsample(x);
        let mut down = [0.0f32; 8];
        for (o, &u) in down.iter_mut().zip(up.iter()) {
            let u = self.pre_clip_hp.process(u); // cut sub-bass before clipping
            let s = tube_clip_asym((u + bias) * pregain) / pregain.sqrt();
            let s = self.stage_hp_1.process(s);
            let s = tube_clip_asym(s * 5.0) / 5.0_f32.sqrt();
            let s = self.stage_hp_2.process(s);
            *o = silicon_clip_asym(s * 3.0) / 3.0_f32.sqrt();
        }
        let x = self.os.downsample(down);
        // ── end oversampled section ───────────────────────────────────────────

        let x = self.tone.process(x);

        let x = self.power_amp(x);
        let x = self.speaker.process(x, self.envelope);
        let x = self.presence_shelf.process(x);

        x * master * 0.8
    }
}

/// Asymmetric 12AX7 triode waveshaper (see marshall.rs for rationale).
#[inline]
fn tube_clip_asym(x: f32) -> f32 {
    use std::f32::consts::FRAC_2_PI;
    if x >= 0.0 {
        FRAC_2_PI * x.atan()
    } else {
        FRAC_2_PI * (x * 1.1).atan()
    }
}

/// Asymmetric silicon diode clipper.
///
/// Positive half: 1 - e^{-x} (forward-biased exponential).
/// Negative half: atan-based with 1.1× input scale — models the different
/// reverse-bias characteristic of real junction diodes (harder knee on neg swing).
#[inline]
fn silicon_clip_asym(x: f32) -> f32 {
    use std::f32::consts::FRAC_2_PI;
    if x >= 0.0 {
        1.0 - (-x).exp()
    } else {
        // Reverse direction: slightly faster saturation, still → -1 asymptotically
        FRAC_2_PI * (x * 1.1).atan()
    }
}
