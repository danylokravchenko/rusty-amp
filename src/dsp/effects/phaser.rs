use std::f32::consts::{PI, TAU};

/// Stereo phaser: a cascade of LFO-swept first-order all-pass filters summed back
/// with the dry signal. Where the flanger sweeps a *delay* (a comb with evenly
/// spaced notches) and the chorus sweeps a *long* delay (pitch shimmer), the phaser
/// sweeps the break frequencies of an all-pass chain: the dry-plus-wet sum notches
/// out wherever the all-pass phase hits 180°, and those few notches glide up and
/// down the spectrum for the hollow, vocal "whoosh" — Phase 90 / Small Stone
/// territory, and the third sibling to the flanger and chorus.
///
/// Sits in the stereo rack after the flanger and chorus, before the delay — all the
/// modulation shaping the finished tone ahead of the ambience. The two channels
/// share one LFO but read it a quarter-cycle apart, so the sweep drifts across the
/// stereo field instead of moving in lockstep.
///
/// An all-pass cascade has flat magnitude on its own, so the notches only exist in
/// the dry+wet *sum*: the effect is deepest at an equal blend and vanishes toward
/// fully wet. Feedback (regeneration) routes the wet output back into the chain,
/// sharpening the notches into resonant peaks — the classic "throaty" phaser voice.
///
/// Knob ranges (all normalised 0–1):
///   RATE     → LFO speed, 0.05–5 Hz (exponential).
///   DEPTH    → sweep width; how far the all-pass break frequency glides.
///   FEEDBACK → regeneration, 0–90%; higher = sharper, resonant notches.
///   MIX      → dry/wet blend; 0 = dry, 0.5 = deepest phase, 1 = fully wet.
pub struct Phaser {
    /// Per-channel all-pass state (one delay element per stage).
    z_l: [f32; N_STAGES],
    z_r: [f32; N_STAGES],
    /// Per-channel wet feedback memory.
    fb_l: f32,
    fb_r: f32,
    phase: f32,
    sr: f32,
}

/// Number of cascaded all-pass stages. Four stages give two sweeping notches — the
/// classic four-stage phaser voicing.
const N_STAGES: usize = 4;
/// Lowest all-pass break frequency the sweep reaches (bottom of the whoosh).
const F_MIN: f32 = 200.0;
/// Highest break frequency at full depth (top of the whoosh).
const F_MAX: f32 = 1600.0;

impl Phaser {
    pub fn new(sr: f32) -> Self {
        Self {
            z_l: [0.0; N_STAGES],
            z_r: [0.0; N_STAGES],
            fb_l: 0.0,
            fb_r: 0.0,
            phase: 0.0,
            sr,
        }
    }

    /// Run one channel's all-pass cascade with feedback, returning the wet (all-pass)
    /// output. The dry/wet blend happens in `process`; the returned value is what
    /// feeds the regeneration loop.
    #[inline]
    fn run_channel(
        z: &mut [f32; N_STAGES],
        fb_state: &mut f32,
        x: f32,
        a: f32,
        feedback: f32,
    ) -> f32 {
        // Feed the previous wet output back in for resonance.
        let mut s = x + *fb_state * feedback;
        // First-order all-pass per stage: y = a·s + z, z = s − a·y. The chain has
        // unit magnitude at every frequency but a frequency-dependent phase, so the
        // sweep lives entirely in the phase the dry sum then cancels against.
        for zi in z.iter_mut() {
            let y = a * s + *zi;
            *zi = s - a * y;
            s = y;
        }
        *fb_state = s;
        s
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
        // rates across most of the knob's travel (mirrors the flanger/chorus).
        let rate_hz = 0.05 * 100.0_f32.powf(rate.clamp(0.0, 1.0));
        self.phase = (self.phase + rate_hz / self.sr).fract();

        // Right channel reads the sweep a quarter-cycle ahead for stereo drift.
        let depth = depth.clamp(0.0, 1.0);
        let lfo = |ph: f32| 0.5 - 0.5 * (ph.fract() * TAU).cos();
        let coeff = |lfo_val: f32| {
            // Exponential sweep of the all-pass break frequency; depth scales how far
            // it travels above F_MIN. A first-order all-pass at break frequency `fc`
            // has coefficient a = (tan(π·fc/sr) − 1) / (tan(π·fc/sr) + 1); the sign
            // puts the pole near z = +1 for a low `fc`, so the −90° phase point lands
            // at `fc` (not up near Nyquist) and the swept notches fall in-band.
            let fc = F_MIN * (F_MAX / F_MIN).powf(depth * lfo_val);
            let t = (PI * fc / self.sr).tan();
            (t - 1.0) / (t + 1.0)
        };
        let a_l = coeff(lfo(self.phase));
        let a_r = coeff(lfo(self.phase + 0.25));

        // Regeneration capped below unity so the resonant loop never runs away
        // (the all-pass chain has unit gain, so feedback ≥ 1 would sustain forever).
        let fb = feedback.clamp(0.0, 1.0) * 0.9;
        let wet_l = Self::run_channel(&mut self.z_l, &mut self.fb_l, l, a_l, fb);
        let wet_r = Self::run_channel(&mut self.z_r, &mut self.fb_r, r, a_r, fb);

        let mix = mix.clamp(0.0, 1.0);
        (l * (1.0 - mix) + wet_l * mix, r * (1.0 - mix) + wet_r * mix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const SR: f32 = 48_000.0;

    /// Windowed peak-amplitude envelope of a channel, sampled every `win` samples
    /// past a warm-up. Used to watch the sweeping notch modulate a steady carrier.
    fn window_peaks<F: FnMut(usize) -> f32>(
        n: usize,
        warmup: usize,
        win: usize,
        mut f: F,
    ) -> Vec<f32> {
        let mut peaks = Vec::new();
        let mut peak = 0.0f32;
        for i in 0..n {
            let y = f(i);
            assert!(y.is_finite(), "phaser non-finite at {i}");
            if i >= warmup {
                peak = peak.max(y.abs());
                if (i - warmup) % win == win - 1 {
                    peaks.push(peak);
                    peak = 0.0;
                }
            }
        }
        peaks
    }

    /// `mix = 0` must pass the dry signal through untouched on both channels.
    #[test]
    fn fully_dry_is_passthrough() {
        let mut p = Phaser::new(SR);
        for n in 0..2000 {
            let x = (n as f32 * 0.03).sin();
            let (l, r) = p.process(x, x * 0.7, 0.4, 0.8, 0.6, 0.0);
            assert!((l - x).abs() < 1e-6 && (r - x * 0.7).abs() < 1e-6);
        }
    }

    /// Extreme settings (fast, deep, near-max feedback) must stay finite and bounded
    /// — the resonant all-pass loop must not blow up.
    #[test]
    fn finite_and_bounded_under_extreme_settings() {
        let mut p = Phaser::new(SR);
        let mut max_abs = 0.0f32;
        for n in 0..(SR as usize) {
            let x = (2.0 * PI * 300.0 * n as f32 / SR).sin() * 0.9;
            let (l, r) = p.process(x, x, 1.0, 1.0, 1.0, 0.5);
            assert!(l.is_finite() && r.is_finite(), "non-finite at {n}");
            max_abs = max_abs.max(l.abs()).max(r.abs());
        }
        assert!(max_abs < 12.0, "resonant loop ran away: {max_abs}");
    }

    /// With a wet mix the sweeping notch must actually modulate the tone — a steady
    /// sine in the sweep band should come out with a time-varying envelope. Kept at
    /// low feedback so the moving notch is exposed rather than masked by the resonant
    /// peak that heavy feedback parks on the carrier.
    #[test]
    fn wet_output_sweeps_over_time() {
        let mut p = Phaser::new(SR);
        let peaks = window_peaks(SR as usize * 3, SR as usize, 256, |n| {
            let x = (2.0 * PI * 600.0 * n as f32 / SR).sin();
            p.process(x, x, 0.6, 1.0, 0.1, 0.5).0
        });
        let hi = peaks.iter().cloned().fold(0.0f32, f32::max);
        let lo = peaks.iter().cloned().fold(f32::INFINITY, f32::min);
        assert!(
            hi - lo > 0.1,
            "wet output not modulated (env {lo:.3}..{hi:.3})"
        );
    }

    /// Deeper DEPTH must glide the break frequency across a wider band, so the notch
    /// travels further and modulates the carrier's envelope more.
    #[test]
    fn deeper_depth_sweeps_further() {
        let envelope_range = |depth: f32| {
            let mut p = Phaser::new(SR);
            let peaks = window_peaks(SR as usize * 2, SR as usize, 256, |n| {
                let x = (2.0 * PI * 600.0 * n as f32 / SR).sin();
                p.process(x, x, 0.6, depth, 0.4, 0.5).0
            });
            let hi = peaks.iter().cloned().fold(0.0f32, f32::max);
            let lo = peaks.iter().cloned().fold(f32::INFINITY, f32::min);
            hi - lo
        };
        assert!(
            envelope_range(1.0) > envelope_range(0.05) * 1.5,
            "depth knob does not widen the sweep"
        );
    }

    /// A faster RATE must move the notch through more sweep cycles in a fixed window:
    /// the centred envelope crosses its mean more often.
    #[test]
    fn faster_rate_modulates_more_often() {
        let mean_crossings = |rate: f32| {
            let mut p = Phaser::new(SR);
            let peaks = window_peaks(SR as usize * 2, SR as usize / 2, 256, |n| {
                let x = (2.0 * PI * 600.0 * n as f32 / SR).sin();
                p.process(x, x, rate, 1.0, 0.5, 0.5).0
            });
            let mean = peaks.iter().sum::<f32>() / peaks.len().max(1) as f32;
            peaks
                .windows(2)
                .filter(|w| (w[0] - mean).signum() != (w[1] - mean).signum())
                .count()
        };
        assert!(
            mean_crossings(0.95) > mean_crossings(0.5),
            "rate knob does not speed the sweep"
        );
    }

    /// FEEDBACK is regeneration: it sharpens the notches into resonant peaks, so the
    /// swept carrier's envelope swings more widely with feedback up than at zero.
    #[test]
    fn feedback_deepens_resonance() {
        let envelope_range = |fb: f32| {
            let mut p = Phaser::new(SR);
            let peaks = window_peaks(SR as usize * 2, SR as usize, 256, |n| {
                let x = (2.0 * PI * 500.0 * n as f32 / SR).sin();
                p.process(x, x, 0.6, 1.0, fb, 0.5).0
            });
            let hi = peaks.iter().cloned().fold(0.0f32, f32::max);
            let lo = peaks.iter().cloned().fold(f32::INFINITY, f32::min);
            hi - lo
        };
        assert!(
            envelope_range(0.85) > envelope_range(0.0) * 1.2,
            "feedback does not deepen the resonance"
        );
    }
}
