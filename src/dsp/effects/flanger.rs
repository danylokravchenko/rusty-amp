use std::f32::consts::TAU;

/// Stereo flanger: a short LFO-swept delay mixed back with the dry signal, the
/// moving comb-filter notches producing the classic "jet plane" sweep. Feedback
/// (regeneration) sharpens the notches into a resonant, metallic voice.
///
/// Sits in the stereo rack after the cab and parametric EQ, before the delay —
/// modulation belongs on the finished tone, ahead of the ambience. The two
/// channels share one LFO but read it a quarter-cycle apart, so the sweep drifts
/// across the stereo field instead of moving in lockstep.
///
/// Knob ranges (all normalised 0–1):
///   RATE     → LFO speed, 0.05–5 Hz (exponential).
///   DEPTH    → sweep width; the delay swings between `MIN_MS` and up to ~5 ms.
///   FEEDBACK → regeneration, 0–90%; higher = sharper, ringing notches.
///   MIX      → dry/wet blend; 0 = dry, 0.5 = deepest flange, 1 = fully wet.
pub struct Flanger {
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    write: usize,
    phase: f32,
    sr: f32,
}

/// Shortest delay in the sweep — a small floor keeps the interpolation reading
/// valid samples and avoids a through-zero click at the top of the sweep.
const MIN_MS: f32 = 0.5;
/// Longest additional delay the DEPTH knob can add on top of `MIN_MS`.
const SWEEP_MS: f32 = 4.5;
/// Buffer headroom above the deepest possible delay (`MIN_MS + SWEEP_MS`).
const MAX_MS: f32 = 6.0;

impl Flanger {
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
    pub fn process(
        &mut self,
        l: f32,
        r: f32,
        rate: f32,
        depth: f32,
        feedback: f32,
        mix: f32,
    ) -> (f32, f32) {
        // LFO advances once per sample; exponential map spreads the slow, musical
        // rates across most of the knob's travel.
        let rate_hz = 0.05 * 100.0_f32.powf(rate.clamp(0.0, 1.0));
        self.phase = (self.phase + rate_hz / self.sr).fract();

        // Right channel reads the sweep a quarter-cycle ahead for stereo drift.
        let span = MIN_MS + SWEEP_MS * depth.clamp(0.0, 1.0);
        let lfo = |ph: f32| 0.5 - 0.5 * (ph.fract() * TAU).cos();
        let del_l = (MIN_MS + span * lfo(self.phase)) * self.sr / 1000.0;
        let del_r = (MIN_MS + span * lfo(self.phase + 0.25)) * self.sr / 1000.0;

        let wet_l = Self::read(&self.buf_l, self.write, del_l);
        let wet_r = Self::read(&self.buf_r, self.write, del_r);

        // Regeneration feeds the swept tap back in; capped below unity so the comb
        // never runs away.
        let fb = feedback.clamp(0.0, 1.0) * 0.9;
        self.buf_l[self.write] = l + wet_l * fb;
        self.buf_r[self.write] = r + wet_r * fb;
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
        let mut f = Flanger::new(SR);
        for n in 0..2000 {
            let x = (n as f32 * 0.03).sin();
            let (l, r) = f.process(x, x * 0.7, 0.4, 0.8, 0.6, 0.0);
            assert!((l - x).abs() < 1e-6 && (r - x * 0.7).abs() < 1e-6);
        }
    }

    /// Extreme settings (fast, deep, near-max feedback) must stay finite and
    /// bounded — the feedback comb must not blow up.
    #[test]
    fn finite_and_bounded_under_extreme_settings() {
        let mut f = Flanger::new(SR);
        let mut max_abs = 0.0f32;
        for n in 0..(SR as usize) {
            let x = (2.0 * PI * 220.0 * n as f32 / SR).sin() * 0.9;
            let (l, r) = f.process(x, x, 1.0, 1.0, 1.0, 0.5);
            assert!(l.is_finite() && r.is_finite(), "non-finite at {n}");
            max_abs = max_abs.max(l.abs()).max(r.abs());
        }
        assert!(max_abs < 12.0, "feedback comb ran away: {max_abs}");
    }

    /// With a wet mix the moving notches must actually modulate the tone — a
    /// steady sine should come out with a time-varying envelope, not a constant.
    #[test]
    fn wet_output_sweeps_over_time() {
        let mut f = Flanger::new(SR);
        let mut min_e = f32::INFINITY;
        let mut max_e = 0.0f32;
        // Skip the first sweep so the buffer has filled.
        for n in 0..(SR as usize * 3) {
            let x = (2.0 * PI * 1500.0 * n as f32 / SR).sin();
            let (l, _r) = f.process(x, x, 0.6, 1.0, 0.5, 0.5);
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

    /// Deeper DEPTH must sweep the delay across a wider range, so the swept comb
    /// notch travels further and the wet tone's envelope is modulated more
    /// deeply. Measured at a low frequency, where the notch spacing in delay-time
    /// is wide enough that a shallow sweep only grazes it while a deep one crosses
    /// it fully. (Peak amplitude, not RMS: RMS averages the modulation away.)
    #[test]
    fn deeper_depth_sweeps_further() {
        let envelope_range = |depth: f32| {
            let mut f = Flanger::new(SR);
            // Peak amplitude within each short window, so the metric follows the
            // notch sweep rather than the 300 Hz carrier.
            let mut window_peaks = Vec::new();
            let mut peak = 0.0f32;
            for n in 0..(SR as usize * 2) {
                let x = (2.0 * PI * 300.0 * n as f32 / SR).sin();
                let (l, _r) = f.process(x, x, 0.6, depth, 0.4, 0.5);
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

    /// FEEDBACK is regeneration: an impulse must leave a longer-lived tail with
    /// feedback up than with it at zero.
    #[test]
    fn feedback_extends_the_tail() {
        let tail_energy = |fb: f32| {
            let mut f = Flanger::new(SR);
            f.process(1.0, 1.0, 0.2, 0.5, fb, 1.0);
            let mut e = 0.0f64;
            for n in 1..4000 {
                let (l, _r) = f.process(0.0, 0.0, 0.2, 0.5, fb, 1.0);
                if n > 500 {
                    e += (l * l) as f64;
                }
            }
            e
        };
        assert!(
            tail_energy(0.85) > tail_energy(0.0) * 2.0,
            "feedback does not lengthen the tail"
        );
    }
}
