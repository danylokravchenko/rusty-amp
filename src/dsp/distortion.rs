use super::biquad::Biquad;

/// Boss DS-1 Distortion simulation.
///
/// Signal path:
///   DC block → input HP (100 Hz) → hard-clip gain stage → tone control → level
///
/// DS-1 character:
///   • Hard clipping via asymmetric op-amp + diode stack → more aggressive than TS soft-clip
///   • Wide input bandwidth (100 Hz HP) → full-range saturation
///   • Active tone stack: LP + HP blend controlled by a single knob
///     - 0 = dark (LP dominates)  /  10 = bright (HP dominates)
pub struct Distortion {
    sr: f32,
    dc_block:  Biquad,
    input_hp:  Biquad,
    tone_lp:   Biquad,
    tone_hp:   Biquad,
    last_tone: f32,
}

impl Distortion {
    pub fn new(sr: f32) -> Self {
        let mut d = Self {
            sr,
            dc_block: Biquad::highpass(sr, 10.0,  0.707),
            input_hp: Biquad::highpass(sr, 100.0, 0.707),
            tone_lp:  Biquad::lowpass (sr, 500.0, 0.707),
            tone_hp:  Biquad::highpass(sr, 2000.0, 0.707),
            last_tone: -1.0,
        };
        d.update_tone(0.5);
        d
    }

    fn update_tone(&mut self, tone: f32) {
        // DS-1 tone stack: active shelf network.
        // Simulate with a fixed LP + HP pair, blended by the tone knob.
        // LP centred at 500 Hz, HP at 2 kHz — crossover creates the scooped/bright shape.
        let lp_fc = 200.0 + tone * 2800.0;   // 200 Hz (dark) → 3 kHz (bright)
        let hp_fc = 300.0 + tone * 4700.0;   // 300 Hz (dark) → 5 kHz (bright)
        self.tone_lp  = Biquad::lowpass (self.sr, lp_fc, 0.5);
        self.tone_hp  = Biquad::highpass(self.sr, hp_fc, 0.5);
        self.last_tone = tone;
    }

    /// All knobs 0–1.
    #[inline]
    pub fn process(&mut self, sample: f32, drive: f32, tone: f32, level: f32) -> f32 {
        if (tone - self.last_tone).abs() > 0.001 {
            self.update_tone(tone);
        }

        let x = self.dc_block.process(sample);
        let x = self.input_hp.process(x);

        // Hard-clip gain stage — op-amp driven into diode clipping.
        // Asymmetric: positive rail clips harder (one diode), negative softer (two in series).
        let gain = 1.0 + drive * 60.0;  // 1× – 61×, more aggressive than TS (51×)
        let x = ds1_clip(x * gain) / gain.sqrt();

        // DS-1 active tone: blend LP (dark) and HP (bright) outputs
        // tone=0 → full LP, tone=1 → full HP, mid gives a scooped character
        let dark   = self.tone_lp.process(x);
        let bright = self.tone_hp.process(x);
        let x = dark * (1.0 - tone) + bright * tone;

        x * level * 0.6
    }
}

/// DS-1 diode clipper: hard asymmetric clipping.
/// Positive side — single silicon diode forward voltage (clips earlier).
/// Negative side — two diodes in series (clips later, asymmetry adds odd harmonics).
#[inline]
fn ds1_clip(x: f32) -> f32 {
    if x >= 0.0 {
        // Positive: clips at ~0.7 V equivalent (hard)
        x.min(0.7) + ((x - 0.7).max(0.0) * 0.05)  // soft knee above threshold
    } else {
        // Negative: clips at ~1.4 V equivalent (two diodes — allows more swing)
        x.max(-1.4) + ((x + 1.4).min(0.0) * 0.05)
    }
}
