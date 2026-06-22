use super::{db_to_lin, lin_to_db};

/// Front-of-chain feed-forward compressor — evens out picking dynamics and adds
/// sustain, the single biggest "studio" upgrade for clean and edge-of-breakup
/// tones. A peak-follower detector drives a hard-knee gain computer in the dB
/// domain; the gain itself is then smoothed with the same attack/release feel so
/// it never zippers or clicks. Auto makeup compensates for the threshold pull-down
/// so turning up Sustain doesn't drop the level.
pub struct Compressor {
    sr: f32,
    env: f32,  // peak-follower envelope (linear amplitude)
    gain: f32, // smoothed gain-reduction (linear)
}

impl Compressor {
    pub fn new(sr: f32) -> Self {
        Self {
            sr,
            env: 0.0,
            gain: 1.0,
        }
    }

    /// `sustain` 0–1 = compression amount (lower threshold + higher ratio),
    /// `attack` 0–1 = 0.5–50 ms attack, `level` 0–1 = output makeup (≈0–2×).
    #[inline]
    pub fn process(&mut self, x: f32, sustain: f32, attack: f32, level: f32) -> f32 {
        // Peak-follower detector: fast attack chases transients, slower release.
        let a = x.abs();
        let atk_ms = 0.5 + attack * 49.5;
        let rel_ms = 150.0; // musical fixed auto-release
        let atk = (-1.0 / (atk_ms * 0.001 * self.sr)).exp();
        let rel = (-1.0 / (rel_ms * 0.001 * self.sr)).exp();
        let det_coeff = if a > self.env { atk } else { rel };
        self.env = det_coeff * self.env + (1.0 - det_coeff) * a;

        // Gain computer (dB domain, hard knee).
        let thresh_db = -6.0 - sustain * 34.0; // 0 → −6 dB, 1 → −40 dB
        let ratio = 2.0 + sustain * 8.0; // 2:1 … 10:1
        let over = lin_to_db(self.env) - thresh_db;
        let target = if over > 0.0 {
            db_to_lin(-over * (1.0 - 1.0 / ratio))
        } else {
            1.0
        };

        // Smooth the applied gain: pull down fast (attack), recover slow (release).
        let g_coeff = if target < self.gain { atk } else { rel };
        self.gain = g_coeff * self.gain + (1.0 - g_coeff) * target;

        // Auto makeup (half the max reduction) plus the Level knob (0–2×).
        let auto_makeup = db_to_lin(-thresh_db * (1.0 - 1.0 / ratio) * 0.5);
        x * self.gain * auto_makeup * (level * 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// The compressor must stay finite and actually reduce dynamic range: a loud
    /// passage should come out closer in level to a quiet passage than it went in.
    #[test]
    fn reduces_dynamic_range_and_is_finite() {
        let sr = 48_000.0;
        let f = 220.0;
        // Heavy compression settings.
        let (sustain, attack, level) = (0.9, 0.2, 0.5);

        let rms_at = |amp: f32| {
            let mut c = Compressor::new(sr);
            let mut sum = 0.0f64;
            let mut count = 0u32;
            let warmup = sr as usize / 4;
            for n in 0..(sr as usize) {
                let x = (2.0 * PI * f * n as f32 / sr).sin() * amp;
                let y = c.process(x, sustain, attack, level);
                assert!(y.is_finite(), "non-finite output");
                if n >= warmup {
                    sum += (y * y) as f64;
                    count += 1;
                }
            }
            (sum / count as f64).sqrt()
        };

        let loud_in = 0.8f64;
        let quiet_in = 0.1f64;
        let loud_out = rms_at(0.8);
        let quiet_out = rms_at(0.1);

        let in_ratio = loud_in / quiet_in; // 8×
        let out_ratio = loud_out / quiet_out;
        assert!(
            out_ratio < in_ratio,
            "compressor did not reduce dynamic range: in {in_ratio:.2}× out {out_ratio:.2}×"
        );
    }
}
