pub mod amp;
pub mod biquad;
pub mod cab;
pub mod conv;
pub mod delay;
pub mod distortion;
pub mod noise_gate;
pub mod oversample;
pub mod parametric_eq;
pub mod reverb;
pub mod tube_screamer;

use atomic_float::AtomicF32;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering::Relaxed};

use amp::AmpBank;
use cab::CabBank;
use delay::Delay;
use distortion::Distortion;
use noise_gate::NoiseGate;
use parametric_eq::ParametricEq;
use reverb::Reverb;
use tube_screamer::TubeScreamer;

// ── Amp model ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AmpModel {
    Marshall = 0,
    Mesa = 1,
    Randall = 2,
}

impl AmpModel {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Mesa,
            2 => Self::Randall,
            _ => Self::Marshall,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Marshall => "Marshall JCM800",
            Self::Mesa => "Mesa Dual Rectifier",
            Self::Randall => "Randall Warhead",
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            Self::Marshall => "JCM800",
            Self::Mesa => "DUAL RECT",
            Self::Randall => "RANDALL",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Marshall => Self::Mesa,
            Self::Mesa => Self::Randall,
            Self::Randall => Self::Marshall,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Marshall => Self::Randall,
            Self::Mesa => Self::Marshall,
            Self::Randall => Self::Mesa,
        }
    }
}

// ── Cabinet model ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CabModel {
    Mesa = 0,
    Marshall = 1,
}

impl CabModel {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Marshall,
            _ => Self::Mesa,
        }
    }

    #[allow(dead_code)]
    pub fn name(self) -> &'static str {
        match self {
            Self::Mesa => "Mesa 4×12 (V30)",
            Self::Marshall => "Marshall 4×12 (GB)",
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            Self::Mesa => "MESA V30",
            Self::Marshall => "MARSH GB",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Mesa => Self::Marshall,
            Self::Marshall => Self::Mesa,
        }
    }
}

// ── Shared parameters (written by UI thread, read by audio thread) ────────────

pub struct Params {
    // Amp model selector
    pub amp_model: Arc<AtomicU8>,

    // Cabinet model selector
    pub cab_model: Arc<AtomicU8>,

    // Mic position (0 = edge/dark, 1 = center/bright)
    pub mic_pos: Arc<AtomicF32>,

    // Noise gate
    pub ng_enabled: Arc<AtomicBool>,
    pub ng_threshold: Arc<AtomicF32>,
    pub ng_release: Arc<AtomicF32>,

    // TS-808
    pub ts_enabled: Arc<AtomicBool>,
    pub ts_drive: Arc<AtomicF32>,
    pub ts_tone: Arc<AtomicF32>,
    pub ts_level: Arc<AtomicF32>,

    // Boss DS-1 Distortion
    pub ds_enabled: Arc<AtomicBool>,
    pub ds_drive: Arc<AtomicF32>,
    pub ds_tone: Arc<AtomicF32>,
    pub ds_level: Arc<AtomicF32>,

    // Reverb
    pub rev_enabled: Arc<AtomicBool>,
    pub rev_room: Arc<AtomicF32>,
    pub rev_damp: Arc<AtomicF32>,
    pub rev_mix: Arc<AtomicF32>,

    // Parametric EQ
    pub eq_enabled: Arc<AtomicBool>,
    pub eq_low: Arc<AtomicF32>,
    pub eq_mid: Arc<AtomicF32>,
    pub eq_high: Arc<AtomicF32>,

    // Delay
    pub delay_enabled: Arc<AtomicBool>,
    pub delay_time: Arc<AtomicF32>,
    pub delay_feedback: Arc<AtomicF32>,
    pub delay_mix: Arc<AtomicF32>,

    // Amp (shared by all models)
    pub amp_gain: Arc<AtomicF32>,
    pub amp_bass: Arc<AtomicF32>,
    pub amp_mid: Arc<AtomicF32>,
    pub amp_treble: Arc<AtomicF32>,
    pub amp_presence: Arc<AtomicF32>,
    pub amp_master: Arc<AtomicF32>,
}

impl Params {
    pub fn new() -> Self {
        macro_rules! p {
            ($v:expr) => {
                Arc::new(AtomicF32::new($v))
            };
        }
        macro_rules! b {
            ($v:expr) => {
                Arc::new(AtomicBool::new($v))
            };
        }
        Self {
            amp_model: Arc::new(AtomicU8::new(AmpModel::Marshall as u8)),
            cab_model: Arc::new(AtomicU8::new(CabModel::Mesa as u8)),
            mic_pos: p!(0.5),

            ng_enabled: b!(true),
            ng_threshold: p!(0.20),
            ng_release: p!(0.30),

            ts_enabled: b!(true),
            ts_drive: p!(0.45),
            ts_tone: p!(0.60),
            ts_level: p!(0.70),

            ds_enabled: b!(false),
            ds_drive: p!(0.40),
            ds_tone: p!(0.50),
            ds_level: p!(0.65),

            rev_enabled: b!(true),
            rev_room: p!(0.55),
            rev_damp: p!(0.40),
            rev_mix: p!(0.25),

            eq_enabled: b!(false),
            eq_low: p!(0.50),
            eq_mid: p!(0.50),
            eq_high: p!(0.50),

            delay_enabled: b!(false),
            delay_time: p!(0.30),
            delay_feedback: p!(0.40),
            delay_mix: p!(0.30),

            amp_gain: p!(0.65),
            amp_bass: p!(0.50),
            amp_mid: p!(0.45),
            amp_treble: p!(0.65),
            amp_presence: p!(0.50),
            amp_master: p!(0.55),
        }
    }

    pub fn amp_model(&self) -> AmpModel {
        AmpModel::from_u8(self.amp_model.load(Relaxed))
    }

    pub fn cab_model(&self) -> CabModel {
        CabModel::from_u8(self.cab_model.load(Relaxed))
    }
}

// ── Signal levels (written by audio thread, read by UI) ───────────────────────

pub struct Levels {
    pub input: Arc<AtomicF32>,
    pub output: Arc<AtomicF32>,
}

impl Levels {
    pub fn new() -> Self {
        Self {
            input: Arc::new(AtomicF32::new(0.0)),
            output: Arc::new(AtomicF32::new(0.0)),
        }
    }
}

// ── DSP chain (owned by audio thread, never shared) ───────────────────────────

pub struct DspChain {
    ng: NoiseGate,
    ts: TubeScreamer,
    ds: Distortion,
    amp: AmpBank,
    cab: CabBank,
    eq: ParametricEq,
    delay: Delay,
    reverb: Reverb,
    params: Arc<Params>,
}

impl DspChain {
    pub fn new(sr: f32, params: Arc<Params>) -> Self {
        Self {
            ng: NoiseGate::new(sr),
            ts: TubeScreamer::new(sr),
            ds: Distortion::new(sr),
            amp: AmpBank::new(sr),
            cab: CabBank::new(sr),
            eq: ParametricEq::new(sr),
            delay: Delay::new(sr),
            reverb: Reverb::new(sr),
            params,
        }
    }

    /// Process one mono input sample, returning a stereo (L, R) pair.
    ///
    /// The pre-amp signal path (gate → pedals → amp) is mono; the signal becomes
    /// stereo at the cabinet (dual-mic convolution) and stays stereo through the
    /// EQ, ping-pong delay and stereo reverb for studio-grade width and depth.
    #[inline]
    pub fn process(&mut self, sample: f32) -> (f32, f32) {
        let p = &self.params;

        // Noise gate
        let x = if p.ng_enabled.load(Relaxed) {
            self.ng.process(
                sample,
                p.ng_threshold.load(Relaxed),
                p.ng_release.load(Relaxed),
            )
        } else {
            sample
        };

        // Pedal chain
        let x = if p.ts_enabled.load(Relaxed) {
            self.ts.process(
                x,
                p.ts_drive.load(Relaxed),
                p.ts_tone.load(Relaxed),
                p.ts_level.load(Relaxed),
            )
        } else {
            x
        };

        let x = if p.ds_enabled.load(Relaxed) {
            self.ds.process(
                x,
                p.ds_drive.load(Relaxed),
                p.ds_tone.load(Relaxed),
                p.ds_level.load(Relaxed),
            )
        } else {
            x
        };

        // Amp
        let x = self.amp.process(
            p.amp_model(),
            x,
            p.amp_gain.load(Relaxed),
            p.amp_bass.load(Relaxed),
            p.amp_mid.load(Relaxed),
            p.amp_treble.load(Relaxed),
            p.amp_presence.load(Relaxed),
            p.amp_master.load(Relaxed),
        );

        // Cabinet simulation — mono in, stereo (dual-mic) out
        let (l, r) = self.cab.process(p.cab_model(), x, p.mic_pos.load(Relaxed));

        // Parametric EQ
        let (l, r) = if p.eq_enabled.load(Relaxed) {
            self.eq.process(
                l,
                r,
                p.eq_low.load(Relaxed),
                p.eq_mid.load(Relaxed),
                p.eq_high.load(Relaxed),
            )
        } else {
            (l, r)
        };

        // Delay (ping-pong stereo)
        let (l, r) = if p.delay_enabled.load(Relaxed) {
            self.delay.process(
                l,
                r,
                p.delay_time.load(Relaxed),
                p.delay_feedback.load(Relaxed),
                p.delay_mix.load(Relaxed),
            )
        } else {
            (l, r)
        };

        // Reverb (stereo)
        let (l, r) = if p.rev_enabled.load(Relaxed) {
            self.reverb.process(
                l,
                r,
                p.rev_room.load(Relaxed),
                p.rev_damp.load(Relaxed),
                p.rev_mix.load(Relaxed),
            )
        } else {
            (l, r)
        };

        (soft_limit(l), soft_limit(r))
    }
}

/// Transparent soft limiter: unity for |x| < 0.95, gentle knee above.
/// Replaces the old x.tanh() which colored the signal even at normal levels.
#[inline]
fn soft_limit(x: f32) -> f32 {
    let a = x.abs();
    if a < 0.95 {
        x
    } else {
        let excess = a - 0.95;
        x.signum() * (0.95 + excess / (1.0 + excess * 5.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// Drive a loud sine through the full chain and confirm the output stays
    /// finite, within the limiter's bounds, and is genuinely stereo (the cab's
    /// dual-mic convolution should decorrelate L and R).
    #[test]
    fn full_chain_is_finite_bounded_and_stereo() {
        let sr = 48_000.0;
        let params = Arc::new(Params::new());
        let mut chain = DspChain::new(sr, params);

        let mut max_abs = 0.0f32;
        let mut channel_diff = 0.0f32;
        let f = 110.0; // low A — exercises the bass/clip interaction
        for n in 0..(sr as usize) {
            let x = (2.0 * PI * f * n as f32 / sr).sin() * 0.8;
            let (l, r) = chain.process(x);
            assert!(l.is_finite() && r.is_finite(), "non-finite output at {n}");
            max_abs = max_abs.max(l.abs()).max(r.abs());
            channel_diff += (l - r).abs();
        }

        // Soft limiter ceiling is ~1.0; allow a hair of headroom.
        assert!(
            max_abs <= 1.05,
            "output exceeded limiter ceiling: {max_abs}"
        );
        // L and R must differ once the reverb/cab decorrelation has filled in.
        assert!(
            channel_diff > 1.0,
            "output is effectively mono: {channel_diff}"
        );
    }

    /// Every amp model should be stable (no NaN/blowup) at full gain.
    #[test]
    fn all_amps_stable_at_max_gain() {
        let sr = 48_000.0;
        for model in [AmpModel::Marshall, AmpModel::Mesa, AmpModel::Randall] {
            let mut bank = amp::AmpBank::new(sr);
            let mut max_abs = 0.0f32;
            for n in 0..(sr as usize / 2) {
                let x = (2.0 * PI * 82.0 * n as f32 / sr).sin();
                let y = bank.process(model, x, 1.0, 0.5, 0.5, 0.7, 0.5, 0.7);
                assert!(y.is_finite(), "{} produced non-finite output", model.name());
                max_abs = max_abs.max(y.abs());
            }
            assert!(max_abs < 4.0, "{} runaway: {max_abs}", model.name());
        }
    }
}
