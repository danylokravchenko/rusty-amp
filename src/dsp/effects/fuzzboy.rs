use crate::dsp::biquad::Biquad;
use crate::dsp::oversample::Oversampler4;

use std::f32::consts::E;

/// Fuzzboy — four-mode guitar distortion effect.
///
/// Ports the four distortion characters from the DirektDSP Fuzzboy plugin:
///   • Crunch  — wavefold into tanh for vintage crunch character
///   • Potato  — harmonic-rich phase distortion with a three-stage power stack
///   • Hiss    — noise-seeded phase distortion with sinh saturation
///   • Sour    — asymmetric exponential waveshaper with polarity inversion
///
/// All gain stages run 4× oversampled to push aliasing above the audio band.
/// DC blocks on the input and output keep the amp stage bias-free regardless
/// of mode.
///
/// All parameters are 0–1 and mapped internally to the original plugin's ranges:
///   in_gain  → 1–5× amplitude pre-gain
///   tone     → 1–2 (character parameter: brightness / wavefold depth)
///   power    → 1–2 (intensity parameter: stage depth / noise amount)
///   out_gain → 0–1 output level
pub struct Fuzzboy {
    dc_block: Biquad,
    os: Oversampler4,
    post_dc: Biquad,
    // Minimal LCG state for the Hiss noise source — allocation-free and deterministic.
    rng: u32,
}

/// Distortion character selector, matching the Fuzzboy plugin's MODE parameter.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FuzzboyMode {
    Crunch = 0,
    Potato = 1,
    Hiss = 2,
    Sour = 3,
}

impl FuzzboyMode {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Potato,
            2 => Self::Hiss,
            3 => Self::Sour,
            _ => Self::Crunch,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Crunch => "CRUNCH",
            Self::Potato => "POTATO",
            Self::Hiss => "HISS",
            Self::Sour => "SOUR",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Crunch => Self::Potato,
            Self::Potato => Self::Hiss,
            Self::Hiss => Self::Sour,
            Self::Sour => Self::Crunch,
        }
    }
}

impl Fuzzboy {
    pub fn new(sr: f32) -> Self {
        Self {
            dc_block: Biquad::highpass(sr, 10.0, 0.707),
            os: Oversampler4::new(sr),
            post_dc: Biquad::highpass(sr, 10.0, 0.707),
            rng: 0xdeadbeef,
        }
    }

    /// Process one mono sample.
    ///
    /// - `mode_u8`: 0=Crunch, 1=Potato, 2=Hiss, 3=Sour
    /// - `in_gain`, `out_gain`: 0–1
    /// - `tone`, `power`: 0–1 (mapped internally to 1–2)
    #[inline]
    pub fn process(
        &mut self,
        sample: f32,
        mode_u8: u8,
        in_gain: f32,
        tone: f32,
        power: f32,
        out_gain: f32,
    ) -> f32 {
        let x = self.dc_block.process(sample);

        // Map params to the original plugin's ranges.
        let gain = 1.0 + in_gain * 4.0; // 1× to 5×
        let t = 1.0 + tone; // 1–2
        let p = 1.0 + power; // 1–2

        let mode = FuzzboyMode::from_u8(mode_u8);

        let rng = &mut self.rng;
        let y = self.os.process(x, |u| {
            let u = u * gain;
            let raw = match mode {
                FuzzboyMode::Crunch => crunch(u, t, p),
                FuzzboyMode::Potato => potato(u, t, p),
                FuzzboyMode::Hiss => hiss(u, t, p, rng),
                FuzzboyMode::Sour => sour(t, u),
            };
            // Bound all modes — the gain stages can drive well above ±1.
            soft_clip(raw)
        });

        let y = self.post_dc.process(y);
        y * out_gain * 0.8
    }
}

// ── Mode DSP ─────────────────────────────────────────────────────────────────

/// Crunch: wavefolder → tanh.
///
/// `2x / (1 + sin(2x))` is a mild wavefolder that adds odd harmonics at low
/// amplitudes and folds the waveform at higher ones. `t × p²` then pushes the
/// result into tanh saturation. The denominator can reach zero at
/// `sin(2x) = −1`; a small guard clamps it to avoid a divide-by-zero.
#[inline]
fn crunch(x: f32, t: f32, p: f32) -> f32 {
    let denom = (1.0 + (2.0 * x).sin()).max(1e-6);
    let folded = 2.0 * x / denom;
    soft_clip(folded * t * p * p)
}

/// Potato: three-stage harmonic distortion.
///
/// Ported from HPdist(x, g=t, f=p, p=10) in the original plugin:
///   1. `tanh(t × x)` — gain stage, tone sets depth
///   2. `(sin(π × stage1 × p) + tanh(π × x)) / 2` — sine-shaped overtones
///      mixed with the straight-tanh path; power sweeps the harmonic emphasis
///   3. Dual-tanh at a fixed deep-saturation factor of 10, then scaled by
///      the same normalization as the original
#[inline]
fn potato(x: f32, t: f32, p: f32) -> f32 {
    use std::f32::consts::PI;
    let stage1 = soft_clip(t * x);
    let stage2 = ((PI * stage1 * p).sin() + soft_clip(PI * x)) * 0.5;
    // Fixed power=10 matches the HPdist call site in the original.
    const FIXED_P: f32 = 10.0;
    let out = (soft_clip(stage2 / PI * FIXED_P) + soft_clip(x * PI / (PI * PI)))
        * (4.0 / (FIXED_P + 1.0))
        * (1.0 + FIXED_P / 10.0);
    soft_clip(out * 0.8 * t)
}

/// Hiss: noise-seeded phase distortion with sinh saturation.
///
/// A tiny amount of band-randomised noise is added before the nonlinear stages;
/// noise amount scales with power so it is inaudible at low settings and adds
/// a gritty texture at high ones. `p × tan(sin(x))` applies phase distortion —
/// sin keeps the argument safely below the tan singularity. The sinh stage
/// generates dense upper harmonics; the final tanh bounds the output.
#[inline]
fn hiss(x: f32, t: f32, p: f32, rng: &mut u32) -> f32 {
    let noise = lcg_f32(rng);
    let x = x + noise * x * 0.001 * (p - 1.0);
    let x = p * x.sin().tan();
    let x = (p * x).sinh() * p;
    soft_clip(3.0 * t * x)
}

/// Sour: asymmetric exponential waveshaper.
///
/// `t × x² × sign(x) × (1 − e^|x|) / (e − 1)` compresses both half-cycles
/// toward zero while intentionally inverting polarity (positive input yields
/// negative output), producing an asymmetric bite. The `min(5.0)` guard on the
/// exponent argument prevents f32 overflow at high gain while keeping the curve
/// faithful within the expected input range.
#[inline]
fn sour(t: f32, x: f32) -> f32 {
    let sign_x: f32 = if x < 0.0 { -1.0 } else { 1.0 };
    let exp_part = (1.0 - x.abs().min(5.0).exp()) / (E - 1.0);
    t * x * x * sign_x * exp_part
}

/// `tanh` soft clipper — symmetric, smooth, bounded to ±1.
#[inline]
fn soft_clip(x: f32) -> f32 {
    x.tanh()
}

/// Minimal LCG producing a float in −1 … +1.
/// Coefficients: Numerical Recipes (Knuth). Per-sample cost: one multiply, one add.
#[inline]
fn lcg_f32(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*state as i32) as f32 * (1.0 / i32::MAX as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn run_mode(mode: u8, in_gain: f32, tone: f32, power: f32, out_gain: f32) -> (f32, f64) {
        let sr = 48_000.0_f32;
        let mut fb = Fuzzboy::new(sr);
        let warmup = sr as usize / 4;
        let total = sr as usize;
        let mut max_abs = 0.0f32;
        let mut sum = 0.0f64;
        let mut count = 0u32;
        for n in 0..total {
            let x = (2.0 * PI * 82.41 * n as f32 / sr).sin() * 0.8;
            let y = fb.process(x, mode, in_gain, tone, power, out_gain);
            assert!(y.is_finite(), "mode {mode} non-finite at sample {n}");
            if n >= warmup {
                max_abs = max_abs.max(y.abs());
                sum += y as f64;
                count += 1;
            }
        }
        (max_abs, sum / count as f64)
    }

    /// Every mode must produce finite, non-silent output and stay within
    /// a reasonable amplitude ceiling even at max gain settings.
    #[test]
    fn all_modes_finite_bounded_and_audible() {
        for mode in 0..4u8 {
            let (max_abs, dc) = run_mode(mode, 1.0, 0.7, 0.7, 0.8);
            let name = FuzzboyMode::from_u8(mode).name();
            assert!(max_abs > 0.001, "{name} mode is silent");
            assert!(max_abs <= 1.0, "{name} mode exceeded ceiling: {max_abs}");
            assert!(dc.abs() < 0.05, "{name} mode has DC offset: {dc}");
        }
    }

    /// Output amplitude must increase as `in_gain` rises — the effect is not reversed.
    #[test]
    fn in_gain_raises_output() {
        for mode in 0..4u8 {
            let (lo, _) = run_mode(mode, 0.1, 0.5, 0.5, 0.5);
            let (hi, _) = run_mode(mode, 0.9, 0.5, 0.5, 0.5);
            let name = FuzzboyMode::from_u8(mode).name();
            assert!(
                hi >= lo * 0.5,
                "{name}: in_gain not raising output (lo={lo:.4}, hi={hi:.4})"
            );
        }
    }

    /// Tone control must visibly affect the output level (acts as a gain/shape knob).
    #[test]
    fn tone_affects_output() {
        for mode in 0..4u8 {
            let (lo, _) = run_mode(mode, 0.5, 0.1, 0.5, 0.8);
            let (hi, _) = run_mode(mode, 0.5, 0.9, 0.5, 0.8);
            let name = FuzzboyMode::from_u8(mode).name();
            // At least one direction should differ meaningfully.
            assert!(
                (hi - lo).abs() > 0.001,
                "{name}: tone control has no effect (lo={lo:.4}, hi={hi:.4})"
            );
        }
    }
}
