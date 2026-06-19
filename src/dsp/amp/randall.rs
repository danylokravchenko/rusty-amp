use super::Amplifier;
use crate::dsp::biquad::Biquad;

/// Randall Warhead solid-state amp simulation.
///
/// Dimebag Darrell's amp of choice — a 300W solid-state head known for
/// its crushing tightness and completely different character from tube amps.
///
/// Signal path:
///   DC block → input HP (60 Hz) → FET stage → BJT stage → op-amp rail clipper
///   → active tone stack → stiff solid-state power section
///
/// Key differences from tube amps:
///   • Three progressively harder clipping stages (FET → BJT → rail clip)
///   • No power-supply sag — solid-state rails are stiff and consistent
///   • Higher treble shelf at 4.5 kHz ("glassy" top end) vs JCM800 at 2.5 kHz
///   • Mid peak at 500 Hz — when scooped, the hollow rests below the Mesa's 750 Hz
///   • Fixed presence shelf (+3 dB at 5 kHz) built into the output section
///   • Asymmetric headroom: very tight on bass, explosive on treble transients
pub struct Randall {
    sr: f32,
    dc_block: Biquad,
    input_hp: Biquad,
    bass_shelf: Biquad,
    mid_peak: Biquad,
    treble_shelf: Biquad,
    presence_shelf: Biquad,
    last_bass: f32,
    last_mid: f32,
    last_treble: f32,
}

impl Randall {
    pub fn new(sr: f32) -> Self {
        let mut r = Self {
            sr,
            dc_block: Biquad::highpass(sr, 10.0, 0.707),
            input_hp: Biquad::highpass(sr, 60.0, 0.707),
            bass_shelf: Biquad::low_shelf(sr, 80.0, 0.0),
            mid_peak: Biquad::peak_eq(sr, 500.0, 0.4, 0.0),
            treble_shelf: Biquad::high_shelf(sr, 4500.0, 0.0),
            presence_shelf: Biquad::high_shelf(sr, 5000.0, 3.0), // fixed presence boost
            last_bass: -1.0,
            last_mid: -1.0,
            last_treble: -1.0,
        };
        r.update_tone_stack(0.5, 0.3, 0.75); // Dime default: scooped mids, cranked treble
        r
    }

    fn update_tone_stack(&mut self, bass: f32, mid: f32, treble: f32) {
        // Wider Q on mid (0.4 vs 0.5) → deeper scoop achievable at lower mid settings
        self.bass_shelf = Biquad::low_shelf(self.sr, 80.0, (bass - 0.5) * 30.0);
        self.mid_peak = Biquad::peak_eq(self.sr, 500.0, 0.4, (mid - 0.5) * 24.0);
        self.treble_shelf = Biquad::high_shelf(self.sr, 4500.0, (treble - 0.5) * 30.0);
        self.last_bass = bass;
        self.last_mid = mid;
        self.last_treble = treble;
    }
}

impl Amplifier for Randall {
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

        // Stage 1 — FET preamp: high gain, smooth asymptotic saturation.
        // x / sqrt(1 + x²) approaches ±1 faster than atan, softer than tanh.
        let pregain = 1.0 + gain * 45.0; // 1× – 46×
        let x = fet_clip(x * pregain) / pregain.sqrt();

        // Stage 2 — BJT driver: tanh-based, harder knee than FET
        let x = bjt_clip(x * 6.0) / 6.0_f32.sqrt();

        // Stage 3 — op-amp rail clipper: asymmetric, very hard approach to ±1
        let x = rail_clip(x * 3.0) / 3.0_f32.sqrt();

        // Active tone stack
        let x = self.bass_shelf.process(x);
        let x = self.mid_peak.process(x);
        let x = self.treble_shelf.process(x);
        let x = self.presence_shelf.process(x);

        // Solid-state power section — no sag, stiff output
        let x = (x * 2.0).tanh() * 0.5;

        x * master * 0.8
    }
}

/// FET soft saturation — smooth approach to ±1, softer knee than tanh.
/// f(x) = x / sqrt(1 + x²)
#[inline]
fn fet_clip(x: f32) -> f32 {
    x / (1.0 + x * x).sqrt()
}

/// BJT transistor clip — standard tanh, harder knee than the FET stage.
#[inline]
fn bjt_clip(x: f32) -> f32 {
    x.tanh()
}

/// Op-amp rail limiter — hard clip with a brief soft knee above 0.85.
/// Produces the tight, unforgiving character of a solid-state output stage.
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
