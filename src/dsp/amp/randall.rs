use super::{Amplifier, Bloom, SpeakerLoad};
use crate::dsp::biquad::Biquad;
use crate::dsp::oversample::Oversampler8;

/// Randall Warhead solid-state amp simulation.
///
/// Signal path:
///   DC block → input HP → [4× OS: FET + HP + BJT + HP + rail clip] → active tone stack → presence → stiff power section
///
/// Character:
///   • 4× oversampling through all three gain stages keeps aliasing inaudible
///   • Asymmetric FET waveshaper adds subtle even harmonics
///   • A touch of dynamic bloom keeps the otherwise stiff solid-state feel responsive
///   • Two inter-stage HPs (500 Hz and 800 Hz) tighten the solid-state response
///   • Presence knob (user-adjustable shelf at 5 kHz)
pub struct Randall {
    sr: f32,
    dc_block: Biquad,
    input_hp: Biquad,
    os: Oversampler8,
    // Pre-clip HP at 8× rate — the Warhead's tight solid-state input coupling
    pre_clip_hp: Biquad,
    // Inter-stage HPs at 8× rate
    stage_hp_1: Biquad,
    stage_hp_2: Biquad,
    // Power section bass cut (base rate) — prevents the output tanh distorting bass
    power_hp: Biquad,
    bloom: Bloom,
    // Active tone stack (base rate)
    bass_shelf: Biquad,
    mid_peak: Biquad,
    treble_shelf: Biquad,
    last_bass: f32,
    last_mid: f32,
    last_treble: f32,
    // Presence (base rate)
    presence_shelf: Biquad,
    last_presence: f32,
    // Speaker impedance interaction (static — stiff rails, high damping factor).
    speaker: SpeakerLoad,
}

impl Randall {
    pub fn new(sr: f32) -> Self {
        let sr8 = sr * 8.0;
        let mut r = Self {
            sr,
            dc_block: Biquad::highpass(sr, 10.0, 0.707),
            // 75 Hz: tighter than tube amps (60 Hz) but doesn't cut the 82 Hz low-E
            input_hp: Biquad::highpass(sr, 75.0, 0.707),
            os: Oversampler8::new(sr),
            // Warhead pre-clip HP: 55 Hz — tighter than Marshall/Mesa but below 82 Hz
            pre_clip_hp: Biquad::highpass(sr8, 55.0, 0.707),
            // After FET stage: 500 Hz (Warhead input coupling)
            stage_hp_1: Biquad::highpass(sr8, 500.0, 0.707),
            // After BJT stage: 800 Hz (driver stage coupling)
            stage_hp_2: Biquad::highpass(sr8, 800.0, 0.707),
            // Output stage HP at 80 Hz: lets the fundamental through while blocking
            // sub-rumble from the tanh stage
            power_hp: Biquad::highpass(sr, 80.0, 0.707),
            bloom: Bloom::new(sr, 8.0, 100.0),
            bass_shelf: Biquad::low_shelf(sr, 80.0, 0.0),
            mid_peak: Biquad::peak_eq(sr, 500.0, 0.4, 0.0),
            treble_shelf: Biquad::high_shelf(sr, 4500.0, 0.0),
            presence_shelf: Biquad::high_shelf(sr, 5000.0, 3.0),
            last_bass: -1.0,
            last_mid: -1.0,
            last_treble: -1.0,
            last_presence: -1.0,
            // Tight 4×12 resonance ~90 Hz, modest and static (no rectifier sag).
            speaker: SpeakerLoad::new(sr, 90.0, 1.3, 0.10, 0.0, 1.0),
        };
        r.update_tone_stack(0.5, 0.3, 0.75);
        r.update_presence(0.5);
        r
    }

    fn update_tone_stack(&mut self, bass: f32, mid: f32, treble: f32) {
        self.bass_shelf = Biquad::low_shelf(self.sr, 80.0, (bass - 0.5) * 30.0);
        self.mid_peak = Biquad::peak_eq(self.sr, 500.0, 0.4, (mid - 0.5) * 24.0);
        self.treble_shelf = Biquad::high_shelf(self.sr, 4500.0, (treble - 0.5) * 30.0);
        self.last_bass = bass;
        self.last_mid = mid;
        self.last_treble = treble;
    }

    fn update_presence(&mut self, presence: f32) {
        // Randall presence at 5 kHz (glassy solid-state top end), +3 dB at noon → ±6 dB range
        let gain_db = 3.0 + (presence - 0.5) * 12.0;
        self.presence_shelf = Biquad::high_shelf(self.sr, 5000.0, gain_db);
        self.last_presence = presence;
    }
}

impl Amplifier for Randall {
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

        let pregain = 1.0 + gain * 45.0;
        let bias = self.bloom.follow(x) * 0.08;

        // ── 4× oversampled nonlinear section ──────────────────────────────────
        let up = self.os.upsample(x);
        let mut down = [0.0f32; 8];
        for (o, &u) in down.iter_mut().zip(up.iter()) {
            let u = self.pre_clip_hp.process(u); // cut sub-bass before FET stage
            let s = fet_clip_asym((u + bias) * pregain) / pregain.sqrt();
            let s = self.stage_hp_1.process(s);
            let s = bjt_clip(s * 6.0) / 6.0_f32.sqrt();
            let s = self.stage_hp_2.process(s);
            *o = rail_clip(s * 3.0) / 3.0_f32.sqrt();
        }
        let x = self.os.downsample(down);
        // ── end oversampled section ───────────────────────────────────────────

        let x = self.bass_shelf.process(x);
        let x = self.mid_peak.process(x);
        let x = self.treble_shelf.process(x);
        let x = self.presence_shelf.process(x);

        // Solid-state power section — stiff rails, no sag.
        // HP before tanh: prevents the output stage from distorting sub-bass.
        let x = self.power_hp.process(x);
        let x = (x * 2.0).tanh() * 0.5;
        let x = self.speaker.process(x, 0.0);

        x * master * 0.8
    }
}

/// Asymmetric FET saturation.
///
/// f(x) = x / sqrt(1 + x²) — smooth approach to ±1, softer than tanh.
/// Negative half uses 1.08× input scale to simulate FET pinch-off asymmetry.
#[inline]
fn fet_clip_asym(x: f32) -> f32 {
    if x >= 0.0 {
        x / (1.0 + x * x).sqrt()
    } else {
        let x2 = x * 1.08;
        x2 / (1.0 + x2 * x2).sqrt()
    }
}

/// BJT transistor clip — standard tanh, harder knee than FET.
#[inline]
fn bjt_clip(x: f32) -> f32 {
    x.tanh()
}

/// Op-amp rail limiter — hard clip with a brief soft knee above 0.85.
#[inline]
fn rail_clip(x: f32) -> f32 {
    let lim = 0.85_f32;
    let abs_x = x.abs();
    if abs_x <= lim {
        x
    } else {
        let excess = abs_x - lim;
        let knee = excess / (1.0 + excess * 8.0);
        x.signum() * (lim + knee * (1.0 - lim))
    }
}
