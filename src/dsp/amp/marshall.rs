use crate::dsp::biquad::Biquad;
use super::Amplifier;

/// Marshall JCM800 amplifier simulation.
///
/// Signal path:
///   DC block → stage-1 tube gain+clip → passive tone stack → stage-2 gain+clip → power amp sag
///
/// Tone stack approximates the classic passive Marshall network (Treble 250 kΩ, Bass 1 MΩ,
/// Middle 25 kΩ) as three cascaded biquad filters with parametric interaction.
pub struct Marshall {
    sr: f32,
    dc_block:     Biquad,
    input_hp:     Biquad,
    // Tone stack filters — rebuilt only when knob values change
    bass_shelf:   Biquad,
    mid_peak:     Biquad,
    treble_shelf: Biquad,
    last_bass:    f32,
    last_mid:     f32,
    last_treble:  f32,
    // Power amp envelope follower (sag simulation)
    envelope:     f32,
}

impl Marshall {
    pub fn new(sr: f32) -> Self {
        let mut m = Self {
            sr,
            dc_block:     Biquad::highpass(sr, 10.0,   0.707),
            input_hp:     Biquad::highpass(sr, 60.0,   0.707),
            bass_shelf:   Biquad::low_shelf (sr, 80.0,   0.0),
            mid_peak:     Biquad::peak_eq   (sr, 400.0, 0.7, 0.0),
            treble_shelf: Biquad::high_shelf(sr, 2500.0, 0.0),
            last_bass:   -1.0,
            last_mid:    -1.0,
            last_treble: -1.0,
            envelope:     0.0,
        };
        m.update_tone_stack(0.5, 0.45, 0.65);
        m
    }

    fn update_tone_stack(&mut self, bass: f32, mid: f32, treble: f32) {
        // Marshall characteristic: bass and treble ±15 dB, mid ±12 dB with slight default scoop
        self.bass_shelf   = Biquad::low_shelf (self.sr, 80.0,   (bass   - 0.5) * 30.0);
        self.mid_peak     = Biquad::peak_eq   (self.sr, 400.0, 0.7, (mid - 0.5) * 24.0);
        self.treble_shelf = Biquad::high_shelf(self.sr, 2500.0, (treble - 0.5) * 30.0);
        self.last_bass   = bass;
        self.last_mid    = mid;
        self.last_treble = treble;
    }

    /// Simulates output transformer saturation and power-supply sag.
    /// An envelope follower tracks RMS level; high levels compress the gain
    /// (the "sag" that makes power-amp crunch feel responsive and dynamic).
    #[inline]
    fn power_amp(&mut self, x: f32) -> f32 {
        let abs_x = x.abs();
        // Attack ~5 ms, release ~200 ms at typical sample rates
        let coeff = if abs_x > self.envelope {
            1.0 - (-220.0 / self.sr).exp()
        } else {
            1.0 - (-5.0 / self.sr).exp()
        };
        self.envelope += coeff * (abs_x - self.envelope);

        // Sag: gain drops as envelope rises (power supply can't keep up)
        let sag = 1.0 / (1.0 + self.envelope * 0.6);
        tube_clip(x * sag * 2.5) * 0.4
    }
}

impl Amplifier for Marshall {
    #[inline]
    fn process(
        &mut self,
        sample: f32,
        gain:   f32,
        bass:   f32,
        mid:    f32,
        treble: f32,
        master: f32,
    ) -> f32 {
        if (bass   - self.last_bass).abs()   > 0.001
        || (mid    - self.last_mid).abs()    > 0.001
        || (treble - self.last_treble).abs() > 0.001
        {
            self.update_tone_stack(bass, mid, treble);
        }

        let x = self.dc_block.process(sample);
        let x = self.input_hp.process(x);

        // Stage 1: 12AX7 triode gain + atan soft-clip (1× – 40×)
        let pregain = 1.0 + gain * 39.0;
        let x = tube_clip(x * pregain) / pregain.sqrt();

        // Stage 2: second triode (fixed mild gain, always contributing harmonic character)
        let x = tube_clip(x * 4.0) / 4.0_f32.sqrt();

        // Passive tone stack
        let x = self.bass_shelf.process(x);
        let x = self.mid_peak.process(x);
        let x = self.treble_shelf.process(x);

        // Power amp: sag + light saturation
        let x = self.power_amp(x);

        x * master * 0.8
    }
}

/// 12AX7 triode approximation using arc-tangent transfer function.
/// Produces predominantly odd harmonics with gentle, asymptotically bounded output.
#[inline]
fn tube_clip(x: f32) -> f32 {
    use std::f32::consts::FRAC_2_PI;
    FRAC_2_PI * x.atan()
}
