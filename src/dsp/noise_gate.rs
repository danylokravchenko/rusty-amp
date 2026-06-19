pub struct NoiseGate {
    envelope: f32,
    gain: f32,
    attack_coeff: f32,
    release_coeff: f32,
}

impl NoiseGate {
    pub fn new(sr: f32) -> Self {
        Self {
            envelope: 0.0,
            gain: 1.0,
            // ~1 ms attack, ~100 ms release at default
            attack_coeff: (-1.0 / (0.001 * sr)).exp(),
            release_coeff: (-1.0 / (0.100 * sr)).exp(),
        }
    }

    #[inline]
    pub fn process(&mut self, sample: f32, threshold: f32, release: f32) -> f32 {
        let abs = sample.abs();
        let coeff = if abs > self.envelope {
            self.attack_coeff
        } else {
            self.release_coeff
        };
        self.envelope = coeff * self.envelope + (1.0 - coeff) * abs;

        // threshold 0.0–1.0 maps to -80 dB–0 dB
        let threshold_lin = 10.0_f32.powf((threshold - 1.0) * 80.0 / 20.0);

        // release 0.0–1.0 maps to 10 ms–500 ms hold
        let release_ms = 10.0 + release * 490.0;
        let _ = release_ms; // used for future hold extension; release shapes gain smoothing

        let target = if self.envelope < threshold_lin {
            0.0_f32
        } else {
            1.0_f32
        };

        // Smooth gain changes to avoid clicks
        let gain_coeff = if target > self.gain {
            0.9
        } else {
            0.999 - release * 0.009
        };
        self.gain = gain_coeff * self.gain + (1.0 - gain_coeff) * target;

        sample * self.gain
    }
}
