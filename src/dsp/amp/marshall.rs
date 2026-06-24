use super::{Amplifier, Bloom, SpeakerLoad};
use crate::dsp::biquad::Biquad;
use crate::dsp::oversample::Oversampler8;
use crate::dsp::tonestack::{Components, ToneStack};

/// Marshall JCM800 amplifier simulation.
///
/// Signal path:
///   DC block → input HP → [8× OS: stage-1 tube + inter-stage HP + stage-2 tube] → tone stack → power amp sag → presence
///
/// Character:
///   • 8× oversampling through the nonlinear gain stages keeps aliasing well above
///     the audible band, removing the harsh "digital" edge of stacked clippers
///   • Asymmetric 12AX7 waveshaper generates even harmonics (2nd, 4th) for warmth
///   • Dynamic grid-bias bloom adds touch sensitivity under hard playing
///   • Inter-stage coupling HP at ~720 Hz (JCM800 22 nF coupling cap) tightens low-end
///   • Presence shelf in the power-amp NFB loop adds air and cut at 3.5 kHz
pub struct Marshall {
    sr: f32,
    // Pre-gain linear filters (base rate)
    dc_block: Biquad,
    input_hp: Biquad,
    // 8× oversampling for the nonlinear section
    os: Oversampler8,
    // Bass cut before the first gain stage at 8× rate — prevents sub-bass from
    // entering the clipper and generating low-frequency IM products ("fart").
    pre_clip_hp: Biquad,
    // Inter-stage coupling HP between tube stages (at 8× rate)
    stage_hp: Biquad,
    // Dynamic preamp bloom
    bloom: Bloom,
    // Passive FMV tone stack (base rate) — bass/mid/treble interact like the real
    // JCM800 network, with the characteristic mid scoop.
    tone: ToneStack,
    last_bass: f32,
    last_mid: f32,
    last_treble: f32,
    // Presence — power-amp NFB characteristic (base rate)
    presence_shelf: Biquad,
    last_presence: f32,
    // Power amp envelope follower (sag simulation)
    envelope: f32,
    // Power-amp ↔ speaker impedance interaction (dynamic low-end bloom).
    speaker: SpeakerLoad,
}

impl Marshall {
    pub fn new(sr: f32) -> Self {
        let sr8 = sr * 8.0;
        let mut m = Self {
            sr,
            dc_block: Biquad::highpass(sr, 10.0, 0.707),
            input_hp: Biquad::highpass(sr, 60.0, 0.707),
            os: Oversampler8::new(sr),
            // JCM800 input coupling cap → sub-rumble cut at ~35 Hz, kept below the
            // 82 Hz low-E fundamental so the distorted bass string stays intact.
            pre_clip_hp: Biquad::highpass(sr8, 35.0, 0.707),
            // JCM800 22 nF inter-stage coupling cap → HP at ~720 Hz
            stage_hp: Biquad::highpass(sr8, 720.0, 0.707),
            bloom: Bloom::new(sr, 8.0, 120.0),
            tone: ToneStack::new(sr, Components::MARSHALL),
            last_bass: -1.0,
            last_mid: -1.0,
            last_treble: -1.0,
            presence_shelf: Biquad::high_shelf(sr, 3500.0, 0.0),
            last_presence: -1.0,
            envelope: 0.0,
            // 8×12 resonance ~95 Hz; tube amp has moderate damping, so a healthy
            // dynamic bloom under sag and a gentle inductive top lift.
            speaker: SpeakerLoad::new(sr, 95.0, 1.0, 0.06, 0.55, 0.8),
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
        // Presence models the JCM800 output-transformer NFB loop: shelf at 3.5 kHz, ±6 dB
        self.presence_shelf = Biquad::high_shelf(self.sr, 3500.0, (presence - 0.5) * 12.0);
        self.last_presence = presence;
    }

    #[inline]
    fn power_amp(&mut self, x: f32) -> f32 {
        let abs_x = x.abs();
        let coeff = if abs_x > self.envelope {
            1.0 - (-220.0 / self.sr).exp()
        } else {
            1.0 - (-5.0 / self.sr).exp()
        };
        self.envelope += coeff * (abs_x - self.envelope);
        let sag = 1.0 / (1.0 + self.envelope * 0.6);
        tube_clip_asym(x * sag * 2.5) * 0.4
    }
}

impl Amplifier for Marshall {
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

        let pregain = 1.0 + gain * 39.0;
        // Dynamic grid-bias offset (removed downstream by the inter-stage HP).
        let bias = self.bloom.follow(x) * 0.12;

        // ── 8× oversampled nonlinear section ──────────────────────────────────
        let up = self.os.upsample(x);
        let mut down = [0.0f32; 8];
        for (o, &u) in down.iter_mut().zip(up.iter()) {
            let u = self.pre_clip_hp.process(u); // cut sub-bass before clipping
            let s = tube_clip_asym((u + bias) * pregain) / pregain.sqrt();
            let s = self.stage_hp.process(s);
            *o = tube_clip_asym(s * 4.0) / 2.0;
        }
        let x = self.os.downsample(down);
        // ── end oversampled section ───────────────────────────────────────────

        // Passive FMV tone stack (base rate — no aliasing risk)
        let x = self.tone.process(x);

        // Power amp: transformer sag + light saturation
        let x = self.power_amp(x);

        // Speaker impedance interaction — dynamic low-end bloom driven by sag.
        let x = self.speaker.process(x, self.envelope);

        // Presence: output transformer NFB shelf
        let x = self.presence_shelf.process(x);

        // Output trim: the tube power stage runs at a conservative level; this
        // makeup brings the JCM800 up to the same loudness as the (much hotter)
        // solid-state Randall so switching models doesn't jump in volume.
        x * master * 3.6
    }
}

/// Asymmetric 12AX7 triode waveshaper.
///
/// Positive half: atan soft-clip (triode toward cutoff — gentle knee).
/// Negative half: atan with 1.1× input scale (toward plate saturation — clips sooner).
/// The asymmetry produces 2nd-harmonic content that gives tube amps their warmth.
#[inline]
fn tube_clip_asym(x: f32) -> f32 {
    use std::f32::consts::FRAC_2_PI;
    if x >= 0.0 {
        FRAC_2_PI * x.atan()
    } else {
        // Negative half saturates faster; still asymptotically approaches -1
        FRAC_2_PI * (x * 1.1).atan()
    }
}
