use std::f32::consts::TAU;

/// Stereo chorus: a set of LFO-swept delay taps mixed back with the dry signal.
/// Where the flanger uses a *short* (sub-millisecond) delay so its comb notches
/// land in the audible band, the chorus runs a much *longer* base delay (~8–20 ms)
/// and no feedback, so instead of a metallic sweep you get gentle pitch-shimmer —
/// the sound of several slightly detuned, slightly delayed copies of the note. The
/// classic lush, watery thickening.
///
/// Sits in the stereo rack after the cab, parametric EQ and flanger, before the
/// delay — modulation belongs on the finished tone, ahead of the ambience. The two
/// channels share one LFO but read it half a cycle apart, so the shimmer drifts
/// across the stereo field and widens the image.
///
/// Knob ranges (all normalised 0–1):
///   RATE  → LFO speed, 0.05–5 Hz (exponential).
///   DEPTH → sweep width; the delay swings from `MIN_MS` up to +`SWEEP_MS`.
///   MIX   → dry/wet blend; 0 = dry, 0.5 = classic chorus, 1 = fully wet (vibrato).
pub struct Chorus {
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    write: usize,
    phase: f32,
    sr: f32,
}

/// Shortest delay in the sweep. A long base delay (vs. the flanger's fraction of a
/// millisecond) is what turns the moving comb into audible pitch-shimmer.
const MIN_MS: f32 = 8.0;
/// Longest additional delay the DEPTH knob can add on top of `MIN_MS`.
const SWEEP_MS: f32 = 12.0;
/// Buffer headroom above the deepest possible delay (`MIN_MS + SWEEP_MS`).
const MAX_MS: f32 = 22.0;

impl Chorus {
    pub fn new(sr: f32) -> Self {
        let len = (sr * MAX_MS / 1000.0) as usize + 2;
        Self {
            buf_l: vec![0.0; len],
            buf_r: vec![0.0; len],
            write: 0,
            phase: 0.0,
            sr,
        }
    }

    /// Linear-interpolated read `delay` samples behind the write head.
    #[inline]
    fn read(buf: &[f32], write: usize, delay: f32) -> f32 {
        let len = buf.len();
        let d = delay.clamp(1.0, (len - 2) as f32);
        let i0 = d.floor() as usize;
        let frac = d - i0 as f32;
        let a = (write + len - i0) % len;
        let b = (write + len - i0 - 1) % len;
        buf[a] * (1.0 - frac) + buf[b] * frac
    }

    #[inline]
    pub fn process(&mut self, l: f32, r: f32, rate: f32, depth: f32, mix: f32) -> (f32, f32) {
        // LFO advances once per sample; exponential map spreads the slow, musical
        // rates across most of the knob's travel.
        let rate_hz = 0.05 * 100.0_f32.powf(rate.clamp(0.0, 1.0));
        self.phase = (self.phase + rate_hz / self.sr).fract();

        // Right channel reads the sweep half a cycle out of phase for stereo drift.
        let span = SWEEP_MS * depth.clamp(0.0, 1.0);
        let lfo = |ph: f32| 0.5 - 0.5 * (ph.fract() * TAU).cos();
        let del_l = (MIN_MS + span * lfo(self.phase)) * self.sr / 1000.0;
        let del_r = (MIN_MS + span * lfo(self.phase + 0.5)) * self.sr / 1000.0;

        let wet_l = Self::read(&self.buf_l, self.write, del_l);
        let wet_r = Self::read(&self.buf_r, self.write, del_r);

        // No feedback: the chorus writes only the dry signal, so the swept taps stay
        // clean shimmer rather than the flanger's resonant, regenerating comb.
        self.buf_l[self.write] = l;
        self.buf_r[self.write] = r;
        let len = self.buf_l.len();
        self.write = (self.write + 1) % len;

        let mix = mix.clamp(0.0, 1.0);
        (l * (1.0 - mix) + wet_l * mix, r * (1.0 - mix) + wet_r * mix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const SR: f32 = 48_000.0;

    /// `mix = 0` must pass the dry signal through untouched on both channels.
    #[test]
    fn fully_dry_is_passthrough() {
        let mut c = Chorus::new(SR);
        for n in 0..2000 {
            let x = (n as f32 * 0.03).sin();
            let (l, r) = c.process(x, x * 0.7, 0.4, 0.8, 0.0);
            assert!((l - x).abs() < 1e-6 && (r - x * 0.7).abs() < 1e-6);
        }
    }

    /// Extreme settings (fast, deep) must stay finite and bounded — with no feedback
    /// the wet output can never exceed the dry peak, so the mix stays well behaved.
    #[test]
    fn finite_and_bounded_under_extreme_settings() {
        let mut c = Chorus::new(SR);
        let mut max_abs = 0.0f32;
        for n in 0..(SR as usize) {
            let x = (2.0 * PI * 220.0 * n as f32 / SR).sin() * 0.9;
            let (l, r) = c.process(x, x, 1.0, 1.0, 0.5);
            assert!(l.is_finite() && r.is_finite(), "non-finite at {n}");
            max_abs = max_abs.max(l.abs()).max(r.abs());
        }
        assert!(max_abs < 2.0, "chorus output out of bounds: {max_abs}");
    }

    /// With a wet mix the swept delay must actually modulate the tone — a steady
    /// sine should come out with a time-varying envelope, not a constant.
    #[test]
    fn wet_output_sweeps_over_time() {
        let mut c = Chorus::new(SR);
        let mut min_e = f32::INFINITY;
        let mut max_e = 0.0f32;
        // Skip the first sweep so the buffer has filled.
        for n in 0..(SR as usize * 3) {
            let x = (2.0 * PI * 1500.0 * n as f32 / SR).sin();
            let (l, _r) = c.process(x, x, 0.6, 1.0, 0.5);
            if n > SR as usize {
                min_e = min_e.min(l.abs());
                max_e = max_e.max(l.abs());
            }
        }
        assert!(
            max_e - min_e > 0.1,
            "wet output not modulated (env {min_e:.3}..{max_e:.3})"
        );
    }

    /// Deeper DEPTH must sweep the delay across a wider range. With no feedback a
    /// fully-wet tap is just a delayed sine (constant amplitude), so the sweep shows
    /// up as dry/wet interference: at a 50/50 mix the moving comb notch travels
    /// further under a deep sweep, modulating the tone's envelope more. (Peak
    /// amplitude, not RMS: RMS averages the modulation away.)
    #[test]
    fn deeper_depth_sweeps_further() {
        let envelope_range = |depth: f32| {
            let mut c = Chorus::new(SR);
            let mut window_peaks = Vec::new();
            let mut peak = 0.0f32;
            for n in 0..(SR as usize * 2) {
                let x = (2.0 * PI * 300.0 * n as f32 / SR).sin();
                // A 50/50 mix lets dry and the swept wet tap interfere into a comb.
                let (l, _r) = c.process(x, x, 0.6, depth, 0.5);
                if n > SR as usize {
                    peak = peak.max(l.abs());
                    if n % 200 == 0 {
                        window_peaks.push(peak);
                        peak = 0.0;
                    }
                }
            }
            let hi = window_peaks.iter().cloned().fold(0.0f32, f32::max);
            let lo = window_peaks.iter().cloned().fold(f32::INFINITY, f32::min);
            hi - lo
        };
        assert!(
            envelope_range(1.0) > envelope_range(0.05) * 1.5,
            "depth knob does not widen the sweep"
        );
    }
}
