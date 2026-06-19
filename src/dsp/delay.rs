/// Tempo-free digital delay with feedback and dry/wet mix.
/// TIME 0–1 maps to 0–500 ms, FEEDBACK 0–1 maps to 0–85% to prevent runaway.
pub struct Delay {
    buffer: Vec<f32>,
    write: usize,
    sr: f32,
}

impl Delay {
    pub fn new(sr: f32) -> Self {
        let max_samples = (sr * 0.5) as usize + 1; // 500 ms max
        Self {
            buffer: vec![0.0; max_samples],
            write: 0,
            sr,
        }
    }

    #[inline]
    pub fn process(&mut self, sample: f32, time: f32, feedback: f32, mix: f32) -> f32 {
        let delay_samples = (time * self.sr * 0.5) as usize; // time 0–1 → 0–500 ms
        let delay_samples = delay_samples.clamp(1, self.buffer.len() - 1);

        let read = (self.write + self.buffer.len() - delay_samples) % self.buffer.len();
        let delayed = self.buffer[read];

        self.buffer[self.write] = sample + delayed * (feedback * 0.85);
        self.write = (self.write + 1) % self.buffer.len();

        sample * (1.0 - mix) + delayed * mix
    }
}
