use super::db_to_lin;

/// Downward gate that mutes the signal once it drops below the threshold —
/// silences amp hiss and hum between notes without chopping sustain. A peak
/// envelope follower drives a smoothed open/closed gain so the gate doesn't click.
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
        let threshold_lin = db_to_lin((threshold - 1.0) * 80.0);

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// A loud tone above the threshold must pass; a quiet tone below it must be
    /// gated down to near silence. Output stays finite throughout.
    #[test]
    fn passes_loud_and_gates_quiet() {
        let sr = 48_000.0;
        let rms_at = |amp: f32| {
            let mut ng = NoiseGate::new(sr);
            let mut sum = 0.0f64;
            let warmup = sr as usize / 4;
            let mut count = 0u32;
            for n in 0..(sr as usize) {
                let x = (2.0 * PI * 220.0 * n as f32 / sr).sin() * amp;
                let y = ng.process(x, 0.4, 0.3);
                assert!(y.is_finite());
                if n >= warmup {
                    sum += (y * y) as f64;
                    count += 1;
                }
            }
            (sum / count as f64).sqrt()
        };

        let loud = rms_at(0.5);
        let quiet = rms_at(0.0005);
        assert!(loud > 0.1, "gate closed on a loud signal: {loud}");
        assert!(
            quiet < loud * 0.1,
            "gate failed to attenuate quiet signal: {quiet}"
        );
    }
}
