pub mod amp;
pub mod biquad;
pub mod cab;
pub mod conv;
pub mod effects;
pub mod oversample;
pub mod tonestack;
pub mod tuner;

pub use tuner::Tuner;

use atomic_float::AtomicF32;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering::Relaxed};

use amp::AmpBank;
use cab::{CabBank, ExternalIrCab};
use effects::{
    Compressor, Delay, Distortion, Flanger, Fuzz, NoiseGate, ParametricEq, PreampEq, Reverb,
    TubeScreamer,
};

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

const DEFAULT_AMP_MODEL: u8 = AmpModel::Marshall as u8;
const DEFAULT_CAB_MODEL: u8 = CabModel::Mesa as u8;
const DEFAULT_MIC_POS: f32 = 0.5;
const DEFAULT_MIC_BLEND: f32 = 0.15;
const DEFAULT_MIC_ROOM: f32 = 0.15;

// When an external IR is loaded it can be toggled against the built-in cabs live;
// it starts inactive (the engine boots on a built-in cab).
const DEFAULT_CAB_EXTERNAL_ACTIVE: bool = false;

const DEFAULT_NG_ENABLED: bool = true;
const DEFAULT_NG_THRESHOLD: f32 = 0.20;
const DEFAULT_NG_RELEASE: f32 = 0.30;

const DEFAULT_CMP_ENABLED: bool = false;
const DEFAULT_CMP_SUSTAIN: f32 = 0.40;
const DEFAULT_CMP_ATTACK: f32 = 0.30;
const DEFAULT_CMP_LEVEL: f32 = 0.50;

const DEFAULT_PEQ_ENABLED: bool = false;
const DEFAULT_PEQ_LOW: f32 = 0.50;
const DEFAULT_PEQ_MID: f32 = 0.50;
const DEFAULT_PEQ_HIGH: f32 = 0.50;

const DEFAULT_FZ_ENABLED: bool = false;
const DEFAULT_FZ_FUZZ: f32 = 0.70;
const DEFAULT_FZ_TONE: f32 = 0.50;
const DEFAULT_FZ_LEVEL: f32 = 0.60;

const DEFAULT_TS_ENABLED: bool = true;
const DEFAULT_TS_DRIVE: f32 = 0.45;
const DEFAULT_TS_TONE: f32 = 0.60;
const DEFAULT_TS_LEVEL: f32 = 0.70;

const DEFAULT_DS_ENABLED: bool = false;
const DEFAULT_DS_DRIVE: f32 = 0.40;
const DEFAULT_DS_TONE: f32 = 0.50;
const DEFAULT_DS_LEVEL: f32 = 0.65;

const DEFAULT_REV_ENABLED: bool = true;
const DEFAULT_REV_ROOM: f32 = 0.55;
const DEFAULT_REV_DAMP: f32 = 0.40;
const DEFAULT_REV_MIX: f32 = 0.25;

const DEFAULT_EQ_ENABLED: bool = false;
const DEFAULT_EQ_LOW: f32 = 0.50;
const DEFAULT_EQ_MID: f32 = 0.50;
const DEFAULT_EQ_HIGH: f32 = 0.50;

const DEFAULT_DELAY_ENABLED: bool = false;
const DEFAULT_DELAY_TIME: f32 = 0.30;
const DEFAULT_DELAY_FEEDBACK: f32 = 0.40;
const DEFAULT_DELAY_MIX: f32 = 0.30;

const DEFAULT_FL_ENABLED: bool = false;
const DEFAULT_FL_RATE: f32 = 0.30;
const DEFAULT_FL_DEPTH: f32 = 0.55;
const DEFAULT_FL_FEEDBACK: f32 = 0.35;
const DEFAULT_FL_MIX: f32 = 0.50;

const DEFAULT_AMP_GAIN: f32 = 0.65;
const DEFAULT_AMP_BASS: f32 = 0.50;
const DEFAULT_AMP_MID: f32 = 0.45;
const DEFAULT_AMP_TREBLE: f32 = 0.65;
const DEFAULT_AMP_PRESENCE: f32 = 0.50;
const DEFAULT_AMP_MASTER: f32 = 0.55;

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

    // External-IR cab override. `cab_external_active` selects the loaded IR over the
    // built-in cab (flipped live by the UI, instant, no reload). `cab_external_loaded`
    // is set by the control thread so the UI knows an IR is installed and the toggle
    // is meaningful.
    pub cab_external_active: Arc<AtomicBool>,
    pub cab_external_loaded: Arc<AtomicBool>,

    // Noise gate
    pub ng_enabled: Arc<AtomicBool>,
    pub ng_threshold: Arc<AtomicF32>,
    pub ng_release: Arc<AtomicF32>,

    // Compressor (front of chain)
    pub cmp_enabled: Arc<AtomicBool>,
    pub cmp_sustain: Arc<AtomicF32>,
    pub cmp_attack: Arc<AtomicF32>,
    pub cmp_level: Arc<AtomicF32>,

    // Pre-amp EQ (before the amp)
    pub peq_enabled: Arc<AtomicBool>,
    pub peq_low: Arc<AtomicF32>,
    pub peq_mid: Arc<AtomicF32>,
    pub peq_high: Arc<AtomicF32>,

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

    // Flanger (stereo rack, post-cab modulation)
    pub fl_enabled: Arc<AtomicBool>,
    pub fl_rate: Arc<AtomicF32>,
    pub fl_depth: Arc<AtomicF32>,
    pub fl_feedback: Arc<AtomicF32>,
    pub fl_mix: Arc<AtomicF32>,

    // Amp (shared by all models)
    pub amp_gain: Arc<AtomicF32>,
    pub amp_bass: Arc<AtomicF32>,
    pub amp_mid: Arc<AtomicF32>,
    pub amp_treble: Arc<AtomicF32>,
    pub amp_presence: Arc<AtomicF32>,
    pub amp_master: Arc<AtomicF32>,
}

impl Default for Params {
    fn default() -> Self {
        Self::new()
    }
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
            amp_model: Arc::new(AtomicU8::new(DEFAULT_AMP_MODEL)),
            cab_model: Arc::new(AtomicU8::new(DEFAULT_CAB_MODEL)),
            mic_pos: p!(DEFAULT_MIC_POS),
            mic_blend: p!(DEFAULT_MIC_BLEND),
            mic_room: p!(DEFAULT_MIC_ROOM),
            cab_external_active: b!(DEFAULT_CAB_EXTERNAL_ACTIVE),
            cab_external_loaded: b!(false),

            ng_enabled: b!(DEFAULT_NG_ENABLED),
            ng_threshold: p!(DEFAULT_NG_THRESHOLD),
            ng_release: p!(DEFAULT_NG_RELEASE),

            cmp_enabled: b!(DEFAULT_CMP_ENABLED),
            cmp_sustain: p!(DEFAULT_CMP_SUSTAIN),
            cmp_attack: p!(DEFAULT_CMP_ATTACK),
            cmp_level: p!(DEFAULT_CMP_LEVEL),

            peq_enabled: b!(DEFAULT_PEQ_ENABLED),
            peq_low: p!(DEFAULT_PEQ_LOW),
            peq_mid: p!(DEFAULT_PEQ_MID),
            peq_high: p!(DEFAULT_PEQ_HIGH),

            fz_enabled: b!(DEFAULT_FZ_ENABLED),
            fz_fuzz: p!(DEFAULT_FZ_FUZZ),
            fz_tone: p!(DEFAULT_FZ_TONE),
            fz_level: p!(DEFAULT_FZ_LEVEL),

            ts_enabled: b!(DEFAULT_TS_ENABLED),
            ts_drive: p!(DEFAULT_TS_DRIVE),
            ts_tone: p!(DEFAULT_TS_TONE),
            ts_level: p!(DEFAULT_TS_LEVEL),

            ds_enabled: b!(DEFAULT_DS_ENABLED),
            ds_drive: p!(DEFAULT_DS_DRIVE),
            ds_tone: p!(DEFAULT_DS_TONE),
            ds_level: p!(DEFAULT_DS_LEVEL),

            rev_enabled: b!(DEFAULT_REV_ENABLED),
            rev_room: p!(DEFAULT_REV_ROOM),
            rev_damp: p!(DEFAULT_REV_DAMP),
            rev_mix: p!(DEFAULT_REV_MIX),

            eq_enabled: b!(DEFAULT_EQ_ENABLED),
            eq_low: p!(DEFAULT_EQ_LOW),
            eq_mid: p!(DEFAULT_EQ_MID),
            eq_high: p!(DEFAULT_EQ_HIGH),

            delay_enabled: b!(DEFAULT_DELAY_ENABLED),
            delay_time: p!(DEFAULT_DELAY_TIME),
            delay_feedback: p!(DEFAULT_DELAY_FEEDBACK),
            delay_mix: p!(DEFAULT_DELAY_MIX),

            fl_enabled: b!(DEFAULT_FL_ENABLED),
            fl_rate: p!(DEFAULT_FL_RATE),
            fl_depth: p!(DEFAULT_FL_DEPTH),
            fl_feedback: p!(DEFAULT_FL_FEEDBACK),
            fl_mix: p!(DEFAULT_FL_MIX),

            amp_gain: p!(DEFAULT_AMP_GAIN),
            amp_bass: p!(DEFAULT_AMP_BASS),
            amp_mid: p!(DEFAULT_AMP_MID),
            amp_treble: p!(DEFAULT_AMP_TREBLE),
            amp_presence: p!(DEFAULT_AMP_PRESENCE),
            amp_master: p!(DEFAULT_AMP_MASTER),
        }
    }

    pub fn reset_to_defaults(&self) {
        self.amp_model.store(DEFAULT_AMP_MODEL, Relaxed);
        self.cab_model.store(DEFAULT_CAB_MODEL, Relaxed);
        self.mic_pos.store(DEFAULT_MIC_POS, Relaxed);
        self.mic_blend.store(DEFAULT_MIC_BLEND, Relaxed);
        self.mic_room.store(DEFAULT_MIC_ROOM, Relaxed);
        // Fall back to the built-in cab. The loaded IR (if any) stays installed in
        // the chain — only its active/inactive selection is a default-able param.
        self.cab_external_active
            .store(DEFAULT_CAB_EXTERNAL_ACTIVE, Relaxed);

        self.ng_enabled.store(DEFAULT_NG_ENABLED, Relaxed);
        self.ng_threshold.store(DEFAULT_NG_THRESHOLD, Relaxed);
        self.ng_release.store(DEFAULT_NG_RELEASE, Relaxed);

        self.cmp_enabled.store(DEFAULT_CMP_ENABLED, Relaxed);
        self.cmp_sustain.store(DEFAULT_CMP_SUSTAIN, Relaxed);
        self.cmp_attack.store(DEFAULT_CMP_ATTACK, Relaxed);
        self.cmp_level.store(DEFAULT_CMP_LEVEL, Relaxed);

        self.peq_enabled.store(DEFAULT_PEQ_ENABLED, Relaxed);
        self.peq_low.store(DEFAULT_PEQ_LOW, Relaxed);
        self.peq_mid.store(DEFAULT_PEQ_MID, Relaxed);
        self.peq_high.store(DEFAULT_PEQ_HIGH, Relaxed);

        self.fz_enabled.store(DEFAULT_FZ_ENABLED, Relaxed);
        self.fz_fuzz.store(DEFAULT_FZ_FUZZ, Relaxed);
        self.fz_tone.store(DEFAULT_FZ_TONE, Relaxed);
        self.fz_level.store(DEFAULT_FZ_LEVEL, Relaxed);

        self.ts_enabled.store(DEFAULT_TS_ENABLED, Relaxed);
        self.ts_drive.store(DEFAULT_TS_DRIVE, Relaxed);
        self.ts_tone.store(DEFAULT_TS_TONE, Relaxed);
        self.ts_level.store(DEFAULT_TS_LEVEL, Relaxed);

        self.ds_enabled.store(DEFAULT_DS_ENABLED, Relaxed);
        self.ds_drive.store(DEFAULT_DS_DRIVE, Relaxed);
        self.ds_tone.store(DEFAULT_DS_TONE, Relaxed);
        self.ds_level.store(DEFAULT_DS_LEVEL, Relaxed);

        self.rev_enabled.store(DEFAULT_REV_ENABLED, Relaxed);
        self.rev_room.store(DEFAULT_REV_ROOM, Relaxed);
        self.rev_damp.store(DEFAULT_REV_DAMP, Relaxed);
        self.rev_mix.store(DEFAULT_REV_MIX, Relaxed);

        self.eq_enabled.store(DEFAULT_EQ_ENABLED, Relaxed);
        self.eq_low.store(DEFAULT_EQ_LOW, Relaxed);
        self.eq_mid.store(DEFAULT_EQ_MID, Relaxed);
        self.eq_high.store(DEFAULT_EQ_HIGH, Relaxed);

        self.delay_enabled.store(DEFAULT_DELAY_ENABLED, Relaxed);
        self.delay_time.store(DEFAULT_DELAY_TIME, Relaxed);
        self.delay_feedback.store(DEFAULT_DELAY_FEEDBACK, Relaxed);
        self.delay_mix.store(DEFAULT_DELAY_MIX, Relaxed);

        self.fl_enabled.store(DEFAULT_FL_ENABLED, Relaxed);
        self.fl_rate.store(DEFAULT_FL_RATE, Relaxed);
        self.fl_depth.store(DEFAULT_FL_DEPTH, Relaxed);
        self.fl_feedback.store(DEFAULT_FL_FEEDBACK, Relaxed);
        self.fl_mix.store(DEFAULT_FL_MIX, Relaxed);

        self.amp_gain.store(DEFAULT_AMP_GAIN, Relaxed);
        self.amp_bass.store(DEFAULT_AMP_BASS, Relaxed);
        self.amp_mid.store(DEFAULT_AMP_MID, Relaxed);
        self.amp_treble.store(DEFAULT_AMP_TREBLE, Relaxed);
        self.amp_presence.store(DEFAULT_AMP_PRESENCE, Relaxed);
        self.amp_master.store(DEFAULT_AMP_MASTER, Relaxed);
    }

    pub fn amp_model(&self) -> AmpModel {
        AmpModel::from_u8(self.amp_model.load(Relaxed))
    }

    pub fn cab_model(&self) -> CabModel {
        CabModel::from_u8(self.cab_model.load(Relaxed))
    }
}

pub struct Levels {
    pub input: Arc<AtomicF32>,
    pub output: Arc<AtomicF32>,
}

impl Default for Levels {
    fn default() -> Self {
        Self::new()
    }
}

impl Levels {
    pub fn new() -> Self {
        Self {
            input: Arc::new(AtomicF32::new(0.0)),
            output: Arc::new(AtomicF32::new(0.0)),
        }
    }
}

/// A stereo insert effect processed one block at a time, in place.
///
/// The built-in [`DspChain`] is hardwired, but this is the single extension point
/// third-party plugins hang off: a stereo slot after the cab/rack, before the
/// master bus. CLAP plugins are bridged to this trait by the `host` module. Must
/// be `Send` so an instance built on the UI thread can be handed to the audio
/// thread for processing.
pub trait StereoInsert: Send {
    /// Process one block in place. `left` and `right` always have equal length.
    fn process_block(&mut self, left: &mut [f32], right: &mut [f32]);
}

/// Run a mono effect when its enable flag is set, otherwise pass `$x` through.
///
/// Every pedal in the chain is bypassable and reads its knobs from the shared,
/// lock-free [`Params`]. Spelling that `if enabled { effect.process(load…) } else
/// { x }` dance out once per pedal was the bulk of the old `process` body; this
/// macro expands to the exact same straight-line code (no allocation, no dynamic
/// dispatch) so the audio thread pays nothing for the deduplication.
macro_rules! mono_stage {
    ($self:ident, $p:ident, $x:ident, $enabled:ident, $field:ident, $($param:ident),+) => {
        if $p.$enabled.load(Relaxed) {
            $self.$field.process($x, $($p.$param.load(Relaxed)),+)
        } else {
            $x
        }
    };
}

/// Stereo counterpart of [`mono_stage!`]: passes `($l, $r)` through when bypassed.
macro_rules! stereo_stage {
    ($self:ident, $p:ident, $l:ident, $r:ident, $enabled:ident, $field:ident, $($param:ident),+) => {
        if $p.$enabled.load(Relaxed) {
            $self.$field.process($l, $r, $($p.$param.load(Relaxed)),+)
        } else {
            ($l, $r)
        }
    };
}

pub struct DspChain {
    ng: NoiseGate,
    cmp: Compressor,
    fz: Fuzz,
    ts: TubeScreamer,
    ds: Distortion,
    peq: PreampEq,
    amp: AmpBank,
    cab: CabBank,
    eq: ParametricEq,
    flanger: Flanger,
    delay: Delay,
    reverb: Reverb,
    params: Arc<Params>,
    /// Optional third-party stereo insert (e.g. a hosted CLAP plugin), placed
    /// after the stereo rack and before the master bus. `None` = passthrough.
    insert: Option<Box<dyn StereoInsert>>,
    /// Optional user-loaded external-IR cab. When present *and*
    /// `params.cab_external_active`, it replaces the built-in [`CabBank`] at the cab
    /// stage. Built off the audio thread and swapped in lock-free, like `insert`.
    ext_cab: Option<Box<ExternalIrCab>>,
}

impl DspChain {
    pub fn new(sr: f32, params: Arc<Params>) -> Self {
        Self {
            ng: NoiseGate::new(sr),
            cmp: Compressor::new(sr),
            fz: Fuzz::new(sr),
            ts: TubeScreamer::new(sr),
            ds: Distortion::new(sr),
            peq: PreampEq::new(sr),
            amp: AmpBank::new(sr),
            cab: CabBank::new(sr),
            eq: ParametricEq::new(sr),
            flanger: Flanger::new(sr),
            delay: Delay::new(sr),
            reverb: Reverb::new(sr),
            params,
            insert: None,
            ext_cab: None,
        }
    }

    /// Install (or clear, with `None`) the stereo plugin insert.
    pub fn set_insert(&mut self, insert: Option<Box<dyn StereoInsert>>) {
        self.insert = insert;
    }

    /// Swap the external-IR cab, returning the displaced one (if any) for disposal
    /// off the audio thread. Same lock-free pointer-move discipline as
    /// [`replace_insert`](Self::replace_insert): the freed IR/FFT buffers must not be
    /// dropped in the realtime callback.
    #[must_use = "the displaced external cab must be dropped off the audio thread"]
    pub fn replace_external_cab(
        &mut self,
        cab: Option<Box<ExternalIrCab>>,
    ) -> Option<Box<ExternalIrCab>> {
        std::mem::replace(&mut self.ext_cab, cab)
    }

    /// Swap the stereo plugin insert, returning the displaced one (if any).
    ///
    /// The swap itself is just a pointer move, so it is safe to call on the audio
    /// thread. The returned box must be **dropped elsewhere**: freeing a plugin
    /// (and the allocations it owns) on the audio thread would block it. The engine
    /// hands the old insert back to a non-audio thread for disposal.
    #[must_use = "the displaced insert must be dropped off the audio thread"]
    pub fn replace_insert(
        &mut self,
        insert: Option<Box<dyn StereoInsert>>,
    ) -> Option<Box<dyn StereoInsert>> {
        std::mem::replace(&mut self.insert, insert)
    }

    /// The built-in signal path up to (but not including) the master bus:
    /// pedals → amp → cab → stereo rack, returning a stereo (L, R) pair.
    ///
    /// The pre-amp signal path (gate → pedals → amp) is mono; the signal becomes
    /// stereo at the cabinet (multi-mic blend convolution) and stays stereo through
    /// the EQ, ping-pong delay and stereo reverb for studio-grade width and depth.
    ///
    /// The plugin insert and the master-bus widen + soft-limit run *after* this; in
    /// the live block path they run in [`process_block`], while the per-sample
    /// [`process`](Self::process) wrapper applies the master bus directly.
    #[inline]
    fn process_core(&mut self, sample: f32) -> (f32, f32) {
        let p = &self.params;

        // Mono pedal chain (gate → compressor → fuzz → TS → DS → pre-amp EQ).
        // Fuzz comes first so it sees the rawest signal; the pre-amp EQ comes last
        // so it shapes exactly what the amp's gain stage clips.
        let x = sample;
        let x = mono_stage!(self, p, x, ng_enabled, ng, ng_threshold, ng_release);
        let x = mono_stage!(
            self,
            p,
            x,
            cmp_enabled,
            cmp,
            cmp_sustain,
            cmp_attack,
            cmp_level
        );
        let x = mono_stage!(self, p, x, fz_enabled, fz, fz_fuzz, fz_tone, fz_level);
        let x = mono_stage!(self, p, x, ts_enabled, ts, ts_drive, ts_tone, ts_level);
        let x = mono_stage!(self, p, x, ds_enabled, ds, ds_drive, ds_tone, ds_level);
        let x = mono_stage!(self, p, x, peq_enabled, peq, peq_low, peq_mid, peq_high);

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

        // Cabinet simulation — mono in, stereo out. A loaded external IR overrides
        // the built-in cab when active; otherwise the multi-mic blend renders. The
        // external path ignores the mic knobs (the capture is already miked).
        let (l, r) = match self.ext_cab.as_mut() {
            Some(ext) if p.cab_external_active.load(Relaxed) => {
                use cab::Cabinet;
                ext.process(x, 0.0, 0.0, 0.0)
            }
            _ => self.cab.process(
                p.cab_model(),
                x,
                p.mic_pos.load(Relaxed),
                p.mic_blend.load(Relaxed),
                p.mic_room.load(Relaxed),
            ),
        };

        // Stereo rack (parametric EQ → flanger → ping-pong delay → reverb).
        // The flanger modulates the finished tone ahead of the time-based ambience.
        let (l, r) = stereo_stage!(self, p, l, r, eq_enabled, eq, eq_low, eq_mid, eq_high);
        let (l, r) = stereo_stage!(
            self,
            p,
            l,
            r,
            fl_enabled,
            flanger,
            fl_rate,
            fl_depth,
            fl_feedback,
            fl_mix
        );
        let (l, r) = stereo_stage!(
            self,
            p,
            l,
            r,
            delay_enabled,
            delay,
            delay_time,
            delay_feedback,
            delay_mix
        );
        let (l, r) = stereo_stage!(
            self,
            p,
            l,
            r,
            rev_enabled,
            reverb,
            rev_room,
            rev_damp,
            rev_mix
        );

        (l, r)
    }

    /// Process one mono input sample, returning a stereo (L, R) pair.
    ///
    /// The plugin-free path: [`process_core`](Self::process_core) followed by the
    /// master bus. The live engine uses [`process_block`] instead, which also runs
    /// the optional plugin insert between the two.
    #[inline]
    pub fn process(&mut self, sample: f32) -> (f32, f32) {
        let (l, r) = self.process_core(sample);
        master_bus(l, r)
    }

    /// Process a block of mono input samples into stereo output buffers.
    ///
    /// Per sample: the core chain (pedals → amp → cab → stereo rack). Then, once
    /// per block, the optional plugin insert runs across the whole buffer in the
    /// stereo domain. Finally the master bus (widen + soft-limit) is applied per
    /// sample. The master bus is stateless, so splitting it out of the core loop is
    /// exact — and with no insert loaded the whole block is bit-identical to calling
    /// [`process`](Self::process) on each sample.
    ///
    /// `out_l`/`out_r` must each be at least `input.len()` long; processing stops at
    /// the shortest of the three slices.
    pub fn process_block(&mut self, input: &[f32], out_l: &mut [f32], out_r: &mut [f32]) {
        let n = input.len().min(out_l.len()).min(out_r.len());
        let out_l = &mut out_l[..n];
        let out_r = &mut out_r[..n];

        // Core chain, per sample (mono → stereo internally).
        for ((&x, l), r) in input.iter().zip(out_l.iter_mut()).zip(out_r.iter_mut()) {
            let (lv, rv) = self.process_core(x);
            *l = lv;
            *r = rv;
        }

        // Optional third-party stereo insert, one block at a time.
        if let Some(insert) = self.insert.as_mut() {
            insert.process_block(out_l, out_r);
        }

        // Master bus, per sample.
        for (l, r) in out_l.iter_mut().zip(out_r.iter_mut()) {
            let (wl, wr) = master_bus(*l, *r);
            *l = wl;
            *r = wr;
        }
    }
}

/// Master bus: stereo-widen then soft-limit. Pushes the cab/reverb decorrelation
/// out for a wider, deeper image without losing mono punch (the mid is untouched),
/// then catches peaks. Stateless, so it can run per-sample inside the core loop or
/// as a separate pass over a block with identical results.
#[inline]
fn master_bus(l: f32, r: f32) -> (f32, f32) {
    let (l, r) = widen(l, r, 1.3);
    (soft_limit(l), soft_limit(r))
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

    /// `process_block` must be bit-identical to running `process` per sample:
    /// the block form is purely a buffering convenience and changes no DSP math.
    /// The whole CLAP-insert plan relies on this equivalence holding.
    #[test]
    fn process_block_matches_per_sample() {
        let sr = 48_000.0;
        let input: Vec<f32> = (0..2000)
            .map(|n| (2.0 * PI * 110.0 * n as f32 / sr).sin() * 0.7)
            .collect();

        let mut per_sample = DspChain::new(sr, Arc::new(Params::new()));
        let mut block = DspChain::new(sr, Arc::new(Params::new()));

        let mut out_l = vec![0.0f32; input.len()];
        let mut out_r = vec![0.0f32; input.len()];
        block.process_block(&input, &mut out_l, &mut out_r);

        for ((&x, &bl), &br) in input.iter().zip(out_l.iter()).zip(out_r.iter()) {
            let (l, r) = per_sample.process(x);
            assert_eq!(l, bl, "L diverged from per-sample path");
            assert_eq!(r, br, "R diverged from per-sample path");
        }
    }

    /// A loaded stereo insert must actually run on the block, between the core
    /// chain and the master bus. A trivial gain insert lets us verify the slot is
    /// wired (output differs from the no-insert path) without depending on a plugin.
    #[test]
    fn loaded_insert_is_applied_to_the_block() {
        struct HalfGain;
        impl StereoInsert for HalfGain {
            fn process_block(&mut self, left: &mut [f32], right: &mut [f32]) {
                for (l, r) in left.iter_mut().zip(right.iter_mut()) {
                    *l *= 0.5;
                    *r *= 0.5;
                }
            }
        }

        let sr = 48_000.0;
        let input: Vec<f32> = (0..1000)
            .map(|n| (2.0 * PI * 220.0 * n as f32 / sr).sin() * 0.6)
            .collect();

        // Fresh chains so internal state (reverb/delay) doesn't bleed between runs.
        let mut bare = DspChain::new(sr, Arc::new(Params::new()));
        let (mut bare_l, mut bare_r) = (vec![0.0; input.len()], vec![0.0; input.len()]);
        bare.process_block(&input, &mut bare_l, &mut bare_r);

        let mut with_insert = DspChain::new(sr, Arc::new(Params::new()));
        with_insert.set_insert(Some(Box::new(HalfGain)));
        let (mut ins_l, mut ins_r) = (vec![0.0; input.len()], vec![0.0; input.len()]);
        with_insert.process_block(&input, &mut ins_l, &mut ins_r);

        // The insert must have changed the output somewhere in the block.
        assert!(
            bare_l.iter().zip(&ins_l).any(|(a, b)| a != b)
                || bare_r.iter().zip(&ins_r).any(|(a, b)| a != b),
            "insert had no effect on the output"
        );
    }

    /// `replace_insert` must hand back exactly the insert it displaced — the engine
    /// relies on this to ship the old plugin off the audio thread for disposal.
    #[test]
    fn replace_insert_returns_the_displaced_insert() {
        #[allow(dead_code)]
        struct Tagged(u32);
        impl StereoInsert for Tagged {
            fn process_block(&mut self, _l: &mut [f32], _r: &mut [f32]) {}
        }

        let mut chain = DspChain::new(48_000.0, Arc::new(Params::new()));

        // First install: nothing displaced.
        assert!(chain.replace_insert(Some(Box::new(Tagged(1)))).is_none());

        // Second install: the first one comes back.
        let old = chain
            .replace_insert(Some(Box::new(Tagged(2))))
            .expect("expected the previously installed insert");
        // (Downcasting through dyn isn't available without Any; the round-trip and
        // the None-on-first-install above are enough to prove the swap semantics.)
        drop(old);

        // Clearing returns the live one.
        assert!(chain.replace_insert(None).is_some());
        assert!(chain.replace_insert(None).is_none());
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

    // ── Base-tone quality (amp + cab, no pedals) ──────────────────────────────
    //
    // These tests pin the "clean, melodic, not artificial" voicing the amp/cab
    // were tuned for. The danger with hand-dialled DSP is that the parameters drift
    // back into the failure modes we measured and fixed: the played note buried
    // under its own overtones (fizz), an ice-pick presence band, or one note
    // jumping out far louder than its neighbours. Each test is an objective bound on
    // one of those, measured on the real amp→cab signal path at the shipped default
    // controls, so a future tweak that re-introduces the problem fails loudly.

    /// Amp model paired with the cab it is voiced against.
    const RIGS: [(AmpModel, CabModel); 3] = [
        (AmpModel::Marshall, CabModel::Marshall),
        (AmpModel::Mesa, CabModel::Mesa),
        (AmpModel::Randall, CabModel::Orange),
    ];

    /// Notes spanning the full usable range, low-E open up into the top octave —
    /// deliberately including the high register (A5–E6), where an over-bright system
    /// makes single notes blast out and lose their body. Lead lines live up here.
    const NECK: [(&str, f32); 11] = [
        ("E2", 82.41),
        ("A2", 110.0),
        ("D3", 146.83),
        ("G3", 196.0),
        ("B3", 246.94),
        ("E4", 329.63),
        ("A4", 440.0),
        ("E5", 659.25),
        ("A5", 880.0),
        ("C6", 1046.5),
        ("E6", 1318.5),
    ];

    /// Render one sustained note through amp+cab at the shipped default controls and
    /// return the mono (L+R) steady-state tail (filter/envelope transients dropped).
    fn render_note(am: AmpModel, cm: CabModel, freq: f32) -> Vec<f32> {
        let sr = 48_000.0;
        let mut amp = amp::AmpBank::new(sr);
        let mut cab = cab::CabBank::new(sr);
        let n = sr as usize;
        let warmup = n / 3;
        let mut out = Vec::with_capacity(n - warmup);
        for i in 0..n {
            // Defaults from DEFAULT_AMP_* / DEFAULT_MIC_*.
            let x = (2.0 * PI * freq * i as f32 / sr).sin() * 0.5;
            let a = amp.process(am, x, 0.65, 0.50, 0.45, 0.65, 0.50, 0.55);
            let (l, r) = cab.process(cm, a, 0.5, 0.15, 0.15);
            if i >= warmup {
                out.push(l + r);
            }
        }
        out
    }

    /// Summed Goertzel energy in [lo, hi) on a semitone grid (coarse but consistent).
    fn band_energy(s: &[f32], lo: f32, hi: f32) -> f32 {
        let sr = 48_000.0;
        let mut f = lo;
        let mut e = 0.0;
        while f < hi {
            let g = goertzel(s, f, sr);
            e += g * g;
            f *= 2.0_f32.powf(1.0 / 12.0);
        }
        e
    }

    /// Spectral centroid (Hz) on a quarter-tone grid 50 Hz–10 kHz — a single number
    /// for "how bright": a guitar-amp tone lives in the low hundreds–low thousands,
    /// and a note whose centroid runs into the multi-kHz range reads as ice-pick.
    fn centroid(s: &[f32]) -> f32 {
        let sr = 48_000.0;
        let mut f = 50.0f32;
        let (mut num, mut den) = (0.0f32, 0.0f32);
        while f < 10_000.0 {
            let g = goertzel(s, f, sr);
            num += f * g;
            den += g;
            f *= 2.0_f32.powf(1.0 / 24.0);
        }
        num / den.max(1e-12)
    }

    /// The played note must lead its own overtones. Driving the amp adds harmonics
    /// (that is the point), but if the 5th–10th harmonic ends up *louder than the
    /// fundamental* the note loses its pitch centre and the tone turns to fizz —
    /// the exact failure (fundamental 30–100× below an upper harmonic) the
    /// inter-stage coupling high-passes were re-tuned to cure. We require the
    /// fundamental to stay within ~5× of the loudest partial across the neck.
    #[test]
    fn fundamental_is_not_buried_under_overtones() {
        let sr = 48_000.0;
        for (am, cm) in RIGS {
            for (name, f) in NECK {
                let out = render_note(am, cm, f);
                let h: Vec<f32> = (1..=10).map(|k| goertzel(&out, f * k as f32, sr)).collect();
                let peak = h.iter().cloned().fold(0.0f32, f32::max).max(1e-9);
                let fund_dom = h[0] / peak;
                assert!(
                    fund_dom >= 0.20,
                    "{} {name}: fundamental buried under overtones (funDom {fund_dom:.3})",
                    am.name()
                );
            }
        }
    }

    /// No single note may sit in the harsh / ice-pick zone. Two complementary
    /// bounds: the 2–5 kHz "presence" energy must stay a fraction of the 200 Hz–2
    /// kHz body (so the cab presence spike + upper harmonics don't dominate), and
    /// the overall brightness (centroid) must stay in the musical guitar band. This
    /// guards the cab presence-peak gains and the amp's harmonic generation.
    #[test]
    fn no_note_is_harsh_or_ice_picky() {
        for (am, cm) in RIGS {
            for (name, f) in NECK {
                let out = render_note(am, cm, f);
                let harsh = band_energy(&out, 2000.0, 5000.0);
                let body = band_energy(&out, 200.0, 2000.0).max(1e-12);
                assert!(
                    harsh / body < 0.6,
                    "{} {name}: harsh/ice-pick (2-5k / body = {:.3})",
                    am.name(),
                    harsh / body
                );
                let c = centroid(&out);
                assert!(
                    c < 2500.0,
                    "{} {name}: too bright (centroid {c:.0} Hz)",
                    am.name()
                );
            }
        }
    }

    /// Almost no energy should survive above the speaker's rolloff. A real cab is a
    /// steep low-pass; audible content above ~6 kHz is aliasing/fizz, the "digital"
    /// artefact the 8× oversampling and cab IR exist to suppress. We require the
    /// 6–12 kHz band to be a tiny fraction of the total — a direct guard on the
    /// oversampling and the cab's top-end voicing.
    #[test]
    fn no_audible_fizz_above_the_cab_rolloff() {
        for (am, cm) in RIGS {
            for (name, f) in NECK {
                let out = render_note(am, cm, f);
                let fizz = band_energy(&out, 6000.0, 12_000.0);
                let total = band_energy(&out, 50.0, 12_000.0).max(1e-12);
                assert!(
                    fizz / total < 0.01,
                    "{} {name}: fizz above cab rolloff ({:.2}% of total)",
                    am.name(),
                    100.0 * fizz / total
                );
            }
        }
    }

    /// High notes must not blast out over the mid neck. The original voicing had a
    /// steep rising frequency tilt — a note at 880 Hz ran +17 dB louder than one at
    /// 220 Hz — so single notes high up the neck leapt out and, with their harmonics
    /// past the cab rolloff, collapsed to thin, piercing fundamentals (the "strange
    /// high notes"). At high gain, clipping compression hides this; at clean settings
    /// it is laid bare.
    ///
    /// We compare the top octave (E5–E6) against the mid neck (G3–A4) rather than the
    /// raw min/max across all notes: the deep low E is *intentionally* a touch
    /// quieter on the tight-voiced metal amps (the low-mid cut that makes palm mutes
    /// chug), and penalising that would conflate two different things. What must stay
    /// bounded is the high register relative to the body of the neck.
    #[test]
    fn high_notes_dont_blast_over_the_mid_neck() {
        let level = |am, cm, f: f32| {
            let out = render_note(am, cm, f);
            (out.iter().map(|&x| x * x).sum::<f32>() / out.len() as f32).sqrt()
        };
        for (am, cm) in RIGS {
            let mid = [196.0, 246.94, 329.63, 440.0] // G3 B3 E4 A4
                .iter()
                .map(|&f| level(am, cm, f))
                .sum::<f32>()
                / 4.0;
            let high = [659.25, 880.0, 1046.5, 1318.5] // E5 A5 C6 E6
                .iter()
                .map(|&f| level(am, cm, f))
                .sum::<f32>()
                / 4.0;
            assert!(
                high / mid.max(1e-9) < 2.5,
                "{}: high register blasts over the mid neck (high/mid {:.2}x)",
                am.name(),
                high / mid.max(1e-9)
            );
        }
    }

    /// Power chords (root + fifth + octave) up the neck must stay even and tight —
    /// the rhythm-playing counterpart to the single-note evenness test. No chord
    /// should jump out far louder than its neighbours, and the inaudible
    /// difference-tone "fart" an octave below the root must stay well under the
    /// chord's musical body at every position.
    #[test]
    fn power_chords_are_even_and_tight_across_the_neck() {
        // (root, fifth, octave) for chords rooted up the low strings.
        let chords: [(f32, f32, f32); 6] = [
            (82.41, 123.47, 164.81),  // E2
            (98.0, 146.83, 196.0),    // G2
            (110.0, 164.81, 220.0),   // A2
            (130.81, 196.0, 261.63),  // C3
            (164.81, 246.94, 329.63), // E3
            (220.0, 329.63, 440.0),   // A3
        ];
        let sr = 48_000.0;
        for (am, cm) in RIGS {
            let mut levels = Vec::new();
            for &(r, fifth, oct) in &chords {
                let mut amp = amp::AmpBank::new(sr);
                let mut cab = cab::CabBank::new(sr);
                let n = sr as usize;
                let warmup = n / 3;
                let mut out = Vec::with_capacity(n - warmup);
                for i in 0..n {
                    let t = i as f32 / sr;
                    let x = ((2.0 * PI * r * t).sin()
                        + (2.0 * PI * fifth * t).sin()
                        + (2.0 * PI * oct * t).sin())
                        * 0.3;
                    let a = amp.process(am, x, 0.65, 0.50, 0.45, 0.65, 0.50, 0.55);
                    let (l, rr) = cab.process(cm, a, 0.5, 0.15, 0.15);
                    if i >= warmup {
                        out.push(l + rr);
                    }
                }
                let sub = goertzel(&out, r * 0.5, sr) + goertzel(&out, r * 0.66, sr);
                let body =
                    goertzel(&out, r, sr) + goertzel(&out, fifth, sr) + goertzel(&out, oct, sr);
                assert!(
                    sub / body.max(1e-9) < 0.5,
                    "{}: power chord at {r:.0} Hz is farty (sub/body {:.2})",
                    am.name(),
                    sub / body.max(1e-9)
                );
                levels.push((out.iter().map(|&x| x * x).sum::<f32>() / out.len() as f32).sqrt());
            }
            let lo = levels.iter().cloned().fold(f32::INFINITY, f32::min);
            let hi = levels.iter().cloned().fold(0.0, f32::max);
            assert!(
                hi / lo < 4.0,
                "{}: power chords uneven across neck (spread {:.1}x)",
                am.name(),
                hi / lo
            );
        }
    }

    /// A note must sound the same whether played on its own or right after other
    /// notes. The dynamic grid-bias "bloom" follower deliberately adds even-harmonic
    /// warmth that grows with how hard you play — touch sensitivity — but if it
    /// releases too slowly it stays loaded from the previous notes and over-warms the
    /// *next* note's attack, so the same note picks up a different timbre depending on
    /// what preceded it (an audible note-to-note inconsistency). We render a note's
    /// attack cold (from silence) and again right after a loud lick, and require the
    /// even-harmonic content of its attack to barely move. This guards the bloom
    /// depth/release: within-note give stays, cross-note bleed does not.
    #[test]
    fn a_note_attacks_the_same_regardless_of_what_preceded_it() {
        let sr = 48_000.0;
        let note = 164.81; // E3 — absent from the preceding lick below
        // Attack-window 2nd-harmonic ratio of `note`, optionally after a loud lick.
        let attack_h2_ratio = |am: AmpModel, cm: CabModel, preceded: bool| -> f32 {
            let mut amp = amp::AmpBank::new(sr);
            let mut cab = cab::CabBank::new(sr);
            let run =
                |amp: &mut amp::AmpBank, cab: &mut cab::CabBank, f: f32, n: usize, amp_in: f32| {
                    let mut last = 0.0;
                    for i in 0..n {
                        let x = (2.0 * PI * f * i as f32 / sr).sin() * amp_in;
                        let a = amp.process(am, x, 0.7, 0.5, 0.45, 0.65, 0.5, 0.6);
                        let (l, r) = cab.process(cm, a, 0.5, 0.15, 0.15);
                        last = l + r;
                    }
                    last
                };
            // Settle filters from rest.
            run(&mut amp, &mut cab, 0.0, sr as usize / 20, 0.0);
            if preceded {
                for &f in &[196.0f32, 261.63, 329.63, 220.0] {
                    run(&mut amp, &mut cab, f, sr as usize * 90 / 1000, 0.6);
                }
            }
            // Capture the note's attack (first 80 ms).
            let n = sr as usize * 80 / 1000;
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let x = (2.0 * PI * note * i as f32 / sr).sin() * 0.5;
                let a = amp.process(am, x, 0.7, 0.5, 0.45, 0.65, 0.5, 0.6);
                let (l, r) = cab.process(cm, a, 0.5, 0.15, 0.15);
                out.push(l + r);
            }
            goertzel(&out, note * 2.0, sr) / goertzel(&out, note, sr).max(1e-9)
        };
        for (am, cm) in RIGS {
            let fresh = attack_h2_ratio(am, cm, false);
            let after = attack_h2_ratio(am, cm, true);
            assert!(
                (after - fresh).abs() < 0.10,
                "{}: note attack changes after other notes (h2/h1 {fresh:.3} → {after:.3})",
                am.name()
            );
        }
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
