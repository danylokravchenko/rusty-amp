pub mod amp;
pub mod biquad;
pub mod cab;
pub mod conv;
pub mod delay;
pub mod distortion;
pub mod fuzz;
pub mod noise_gate;
pub mod oversample;
pub mod parametric_eq;
pub mod reverb;
pub mod tonestack;
pub mod tube_screamer;

use atomic_float::AtomicF32;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering::Relaxed};

use amp::AmpBank;
use cab::CabBank;
use delay::Delay;
use distortion::Distortion;
use fuzz::Fuzz;
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
    Orange = 2,
}

impl CabModel {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Marshall,
            2 => Self::Orange,
            _ => Self::Mesa,
        }
    }

    #[allow(dead_code)]
    pub fn name(self) -> &'static str {
        match self {
            Self::Mesa => "Mesa 4×12 (V30)",
            Self::Marshall => "Marshall 4×12 (GB)",
            Self::Orange => "Orange PPC412 (V30)",
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            Self::Mesa => "MESA V30",
            Self::Marshall => "MARSH GB",
            Self::Orange => "ORANGE",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Mesa => Self::Marshall,
            Self::Marshall => Self::Orange,
            Self::Orange => Self::Mesa,
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
    // Mic blend (0 = close SM57 dynamic, 1 = R121 ribbon)
    pub mic_blend: Arc<AtomicF32>,
    // Room mic amount (0 = dry close mic only, 1 = full ambient room)
    pub mic_room: Arc<AtomicF32>,

    // Noise gate
    pub ng_enabled: Arc<AtomicBool>,
    pub ng_threshold: Arc<AtomicF32>,
    pub ng_release: Arc<AtomicF32>,

    // Fuzz (Big Muff style)
    pub fz_enabled: Arc<AtomicBool>,
    pub fz_fuzz: Arc<AtomicF32>,
    pub fz_tone: Arc<AtomicF32>,
    pub fz_level: Arc<AtomicF32>,

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
            mic_blend: p!(0.15),
            mic_room: p!(0.15),

            ng_enabled: b!(true),
            ng_threshold: p!(0.20),
            ng_release: p!(0.30),

            fz_enabled: b!(false),
            fz_fuzz: p!(0.70),
            fz_tone: p!(0.50),
            fz_level: p!(0.60),

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
    fz: Fuzz,
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
            fz: Fuzz::new(sr),
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
    /// stereo at the cabinet (multi-mic blend convolution) and stays stereo through
    /// the EQ, ping-pong delay and stereo reverb for studio-grade width and depth.
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

        // Pedal chain — fuzz first, so it sees the rawest signal
        let x = if p.fz_enabled.load(Relaxed) {
            self.fz.process(
                x,
                p.fz_fuzz.load(Relaxed),
                p.fz_tone.load(Relaxed),
                p.fz_level.load(Relaxed),
            )
        } else {
            x
        };

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

        // Cabinet simulation — mono in, stereo (multi-mic blend) out
        let (l, r) = self.cab.process(
            p.cab_model(),
            x,
            p.mic_pos.load(Relaxed),
            p.mic_blend.load(Relaxed),
            p.mic_room.load(Relaxed),
        );

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

        // Master-bus stereo widening — push the cab/reverb decorrelation out for a
        // wider, deeper image without losing mono punch (the mid is untouched).
        let (l, r) = widen(l, r, 1.3);

        (soft_limit(l), soft_limit(r))
    }
}

/// Mid/side stereo widener. `width` 1.0 = unchanged, > 1.0 spreads the sides.
/// The mono (mid) component is preserved exactly, so the center stays solid and
/// the result folds down to mono cleanly.
#[inline]
fn widen(l: f32, r: f32, width: f32) -> (f32, f32) {
    let mid = (l + r) * 0.5;
    let side = (l - r) * 0.5 * width;
    (mid + side, mid - side)
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

    /// The DS-1's symmetric clipper must not inject a DC offset on a sustained
    /// low note — a wandering DC bias was the source of the "farty" blocking
    /// distortion. Measure the mean of the (settled) output on a low-E sine.
    #[test]
    fn distortion_has_no_dc_offset_on_low_e() {
        let sr = 48_000.0;
        let mut ds = distortion::Distortion::new(sr);
        let mut sum = 0.0f64;
        let mut count = 0u32;
        let warmup = sr as usize / 4; // let filters settle
        let total = sr as usize;
        for n in 0..total {
            let x = (2.0 * PI * 82.41 * n as f32 / sr).sin() * 0.7;
            let y = ds.process(x, 0.8, 0.5, 0.7);
            if n >= warmup {
                sum += y as f64;
                count += 1;
            }
        }
        let dc = (sum / count as f64).abs();
        assert!(dc < 0.01, "distortion has DC offset (fart risk): {dc}");
    }

    fn goertzel(samples: &[f32], f: f32, sr: f32) -> f32 {
        let w = 2.0 * PI * f / sr;
        let coeff = 2.0 * w.cos();
        let (mut s1, mut s2) = (0.0f32, 0.0f32);
        for &x in samples {
            let s0 = x + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        let real = s1 - s2 * w.cos();
        let imag = s2 * w.sin();
        (real * real + imag * imag).sqrt() / (samples.len() as f32 / 2.0)
    }

    /// Diagnostic: measure the low-frequency intermodulation ("fart") content of
    /// the DS-1 on a power chord. The difference tone E2↔B2 lands at ~41 Hz — a
    /// sub-fundamental blat that is the classic distortion fart.
    /// The DS-1 must stay tight, not blubbery: the "woof" energy at/below the
    /// low-E fundamental should be a small fraction of the body harmonics, and the
    /// pedal must not pump a hot level into the amp. Guards against regressing to
    /// the loose, bass-heavy voicing.
    #[test]
    fn distortion_low_end_balance() {
        let sr = 48_000.0;
        let mut ds = distortion::Distortion::new(sr);
        // Guitar-ish low E: fundamental + a few harmonics at a realistic pickup level.
        let e2 = 82.41;
        let n = sr as usize;
        let warmup = sr as usize / 4;
        let mut inp = 0.0f64;
        let mut out = Vec::with_capacity(n - warmup);
        let mut in_buf = Vec::with_capacity(n - warmup);
        for i in 0..n {
            let t = i as f32 / sr;
            let x = 0.15
                * ((2.0 * PI * e2 * t).sin()
                    + 0.5 * (2.0 * PI * 2.0 * e2 * t).sin()
                    + 0.3 * (2.0 * PI * 3.0 * e2 * t).sin());
            let y = ds.process(x, 0.5, 0.5, 0.65);
            if i >= warmup {
                out.push(y);
                in_buf.push(x);
                inp += (x * x) as f64;
            }
        }
        let rms =
            |v: &[f32]| (v.iter().map(|s| (s * s) as f64).sum::<f64>() / v.len() as f64).sqrt();
        let m = |v: &[f32], f| goertzel(v, f, sr);
        // "Woof" = energy at/below the low-E fundamental region; the blubber.
        let woof = m(&out, 55.0) + m(&out, 82.41) + m(&out, 110.0);
        let body = m(&out, 165.0) + m(&out, 247.0) + m(&out, 330.0);
        let through = rms(&out) / rms(&in_buf);
        let ratio = woof / body.max(1e-9);
        println!("DS-1 woof/body={ratio:.2}  through-level={through:.2}x");
        assert!(
            ratio < 0.5,
            "DS-1 low end is blubbery: woof/body = {ratio:.2}"
        );
        assert!(
            through < 1.0,
            "DS-1 output too hot, will slam the amp: {through:.2}x"
        );
        let _ = inp;
    }

    /// The fuzz must stay finite and bounded even at maximum sustain on a hot,
    /// low note — its two cascaded clippers run at enormous gain, so any
    /// instability or runaway DC would show up here.
    #[test]
    fn fuzz_is_finite_bounded_and_saturates() {
        let sr = 48_000.0;
        let mut fz = fuzz::Fuzz::new(sr);
        let mut max_abs = 0.0f32;
        let mut sum = 0.0f64;
        let warmup = sr as usize / 4;
        let total = sr as usize;
        let mut count = 0u32;
        for n in 0..total {
            let x = (2.0 * PI * 82.41 * n as f32 / sr).sin() * 0.8;
            let y = fz.process(x, 1.0, 0.5, 0.7);
            assert!(y.is_finite(), "fuzz produced non-finite output at {n}");
            if n >= warmup {
                max_abs = max_abs.max(y.abs());
                sum += y as f64;
                count += 1;
            }
        }
        assert!(max_abs <= 1.0, "fuzz output exceeded bounds: {max_abs}");
        // Heavy clipping should still produce a healthy signal, not silence.
        assert!(max_abs > 0.05, "fuzz output too quiet: {max_abs}");
        // Asymmetric clipping is fine, but the post-DC block must keep the mean
        // near zero so the fuzz doesn't push DC into the amp.
        let dc = (sum / count as f64).abs();
        assert!(dc < 0.02, "fuzz has DC offset: {dc}");
    }

    /// The passive FMV tone stack must be stable, peak-bounded (it only cuts), and
    /// its controls must move the right bands: turning a knob up should raise that
    /// band's output. Guards the hand-transcribed analog→digital coefficients.
    #[test]
    fn tonestack_is_stable_and_controls_work() {
        use super::tonestack::{Components, ToneStack};
        let sr = 48_000.0;

        // Measure a band's steady-state level for given (bass, mid, treble).
        let level = |b: f32, m: f32, t: f32, f: f32| {
            let mut ts = ToneStack::new(sr, Components::MARSHALL);
            ts.update(b, m, t);
            let mut out = Vec::with_capacity(sr as usize / 2);
            for n in 0..(sr as usize / 2) {
                let x = (2.0 * PI * f * n as f32 / sr).sin();
                let y = ts.process(x);
                assert!(y.is_finite(), "tonestack non-finite");
                // ignore the first quarter (settling)
                if n >= sr as usize / 8 {
                    out.push(y);
                }
            }
            goertzel(&out, f, sr)
        };

        // Peak-normalised: no setting should pass more than unity (a hair of slack).
        for &(b, m, t) in &[(0.5, 0.5, 0.5), (1.0, 0.0, 1.0), (0.0, 1.0, 0.0)] {
            for &f in &[100.0, 800.0, 4000.0] {
                assert!(level(b, m, t, f) <= 1.2, "tonestack boosts above unity");
            }
        }

        // Bass up → more lows; treble up → more highs; mid up → more mids.
        assert!(
            level(0.9, 0.5, 0.5, 100.0) > level(0.1, 0.5, 0.5, 100.0),
            "bass control inverted/dead at 100 Hz"
        );
        assert!(
            level(0.5, 0.5, 0.9, 4000.0) > level(0.5, 0.5, 0.1, 4000.0),
            "treble control inverted/dead at 4 kHz"
        );
        assert!(
            level(0.5, 0.9, 0.5, 800.0) > level(0.5, 0.1, 0.5, 800.0),
            "mid control inverted/dead at 800 Hz"
        );
    }

    /// A low-E power chord through a high-gain, bass-heavy, mid-scooped rig (the
    /// "Pantera rhythm" worst case) must not turn into sub-bass mush: the inaudible
    /// difference-tone / rumble energy below the low-E fundamental must stay a small
    /// fraction of the musical body harmonics, and the three amp models must be
    /// roughly level-matched so switching models doesn't jump the volume.
    #[test]
    fn power_chord_low_end_is_tight_and_amps_level_matched() {
        let sr = 48_000.0;
        // E2 power chord: root + fifth + octave, like a palm-muted metal chord.
        let chord = [82.41f32, 123.47, 164.81];
        let run = |model: AmpModel| {
            let params = Arc::new(Params::new());
            params.amp_model.store(model as u8, Relaxed);
            params.ts_enabled.store(false, Relaxed);
            params.ds_enabled.store(true, Relaxed);
            params.ds_drive.store(0.72, Relaxed);
            params.ds_tone.store(0.68, Relaxed);
            params.ds_level.store(0.80, Relaxed);
            params.rev_enabled.store(false, Relaxed);
            params.ng_enabled.store(false, Relaxed);
            params.amp_gain.store(0.93, Relaxed);
            params.amp_bass.store(0.82, Relaxed);
            params.amp_mid.store(0.12, Relaxed);
            params.amp_treble.store(0.86, Relaxed);
            params.amp_presence.store(0.73, Relaxed);
            params.amp_master.store(0.65, Relaxed);
            let mut chain = DspChain::new(sr, params);
            let n = sr as usize;
            let warmup = sr as usize / 3;
            let mut out = Vec::with_capacity(n - warmup);
            for i in 0..n {
                let t = i as f32 / sr;
                let x: f32 = chord.iter().map(|&f| (2.0 * PI * f * t).sin()).sum::<f32>() * 0.18;
                let (l, _r) = chain.process(x);
                if i >= warmup {
                    out.push(l);
                }
            }
            let rms = (out.iter().map(|s| (s * s) as f64).sum::<f64>() / out.len() as f64).sqrt();
            let m = |f| goertzel(&out, f, sr) as f64;
            let sub = m(41.0) + m(55.0); // sub / difference-tone fart
            let body = m(164.81) + m(247.0) + m(330.0); // musical body harmonics
            (rms, sub / body.max(1e-9))
        };

        let mut rms = Vec::new();
        for model in [AmpModel::Marshall, AmpModel::Mesa, AmpModel::Randall] {
            let (r, sub_body) = run(model);
            assert!(
                sub_body < 0.45,
                "{} low end is farty: sub/body = {sub_body:.2}",
                model.name()
            );
            rms.push(r);
        }
        // Loudness match: the quietest amp must be within ~6 dB of the loudest, so
        // switching models doesn't produce the old 4–7× volume jump.
        let lo = rms.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = rms.iter().cloned().fold(0.0, f64::max);
        assert!(
            hi / lo < 2.0,
            "amps not level-matched: rms spread {hi:.4}/{lo:.4} = {:.2}x",
            hi / lo
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
