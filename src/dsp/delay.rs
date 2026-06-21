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
