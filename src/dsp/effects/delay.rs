/// Tempo-free stereo ping-pong delay with feedback and dry/wet mix.
/// TIME 0–1 maps to 0–500 ms, FEEDBACK 0–1 maps to 0–85% to prevent runaway.
/// Feedback cross-feeds L↔R so repeats bounce across the stereo field.
pub struct Delay {
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    write: usize,
    sr: f32,
}

impl Delay {
    pub fn new(sr: f32) -> Self {
        let max_samples = (sr * 0.5) as usize + 1; // 500 ms max
        Self {
            buf_l: vec![0.0; max_samples],
            buf_r: vec![0.0; max_samples],
            write: 0,
            sr,
        }
    }

    #[inline]
    pub fn process(&mut self, l: f32, r: f32, time: f32, feedback: f32, mix: f32) -> (f32, f32) {
        let len = self.buf_l.len();
        let delay_samples = ((time * self.sr * 0.5) as usize).clamp(1, len - 1);
        let read = (self.write + len - delay_samples) % len;

        let delayed_l = self.buf_l[read];
        let delayed_r = self.buf_r[read];

        let fb = feedback * 0.85;
        // Cross-fed feedback → ping-pong.
        self.buf_l[self.write] = l + delayed_r * fb;
        self.buf_r[self.write] = r + delayed_l * fb;
        self.write = (self.write + 1) % len;

        let out_l = l * (1.0 - mix) + delayed_l * mix;
        let out_r = r * (1.0 - mix) + delayed_r * mix;
        (out_l, out_r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `mix = 0` must pass the dry signal through unchanged.
    #[test]
    fn fully_dry_is_passthrough() {
        let mut d = Delay::new(48_000.0);
        for n in 0..1000 {
            let x = (n as f32 * 0.02).sin();
            let (l, r) = d.process(x, x, 0.3, 0.5, 0.0);
            assert!((l - x).abs() < 1e-6 && (r - x).abs() < 1e-6);
        }
    }

    /// A single impulse fed only to the left must re-emerge on the *left* one delay
    /// later (the direct tap), then bounce to the *right* after a second delay — the
    /// cross-fed feedback that makes the echoes ping-pong across the stereo field.
    #[test]
    fn impulse_pings_across_channels() {
        let sr = 48_000.0;
        let mut d = Delay::new(sr);
        let time = 0.2; // → 0.2 * sr * 0.5 = 4800 samples
        let delay_samples = (time * sr * 0.5) as usize;

        // Impulse on the left only, then silence; fully wet with feedback.
        d.process(1.0, 0.0, time, 0.6, 1.0);
        // `peak` finds the loudest sample index in `[lo, hi)` for one channel.
        let mut left = (0.0f32, 0usize);
        let mut right = (0.0f32, 0usize);
        for n in 1..(delay_samples * 3) {
            let (l, r) = d.process(0.0, 0.0, time, 0.6, 1.0);
            assert!(l.is_finite() && r.is_finite());
            if l.abs() > left.0 {
                left = (l.abs(), n);
            }
            if r.abs() > right.0 {
                right = (r.abs(), n);
            }
        }
        // First repeat lands on the left at ~one delay; the bounce lands on the
        // right at ~two delays.
        assert!(left.0 > 0.5, "left echo missing");
        assert!(right.0 > 0.1, "ping-pong bounce to right missing");
        assert!(
            (left.1 as i64 - delay_samples as i64).abs() <= 2,
            "left echo at {}, expected ~{delay_samples}",
            left.1
        );
        assert!(
            (right.1 as i64 - 2 * delay_samples as i64).abs() <= 2,
            "right bounce at {}, expected ~{}",
            right.1,
            2 * delay_samples
        );
    }
}
