use std::sync::Arc;

use atomic_float::AtomicF32;
use std::sync::atomic::Ordering::Relaxed;

use crate::dsp::Params;

pub(super) struct Knob {
    pub(super) label: &'static str,
    pub(super) param: fn(&Params) -> &Arc<AtomicF32>,
}

pub(super) type SectionDef = (
    fn(&Params) -> String,
    usize,
    usize,
    fn(&Params) -> Option<bool>,
    u32, // relative width weight
);

// Knob index range boundaries
pub(super) const TS_START: usize = 0;
pub(super) const TS_END: usize = 3;
pub(super) const DS_START: usize = 3;
pub(super) const DS_END: usize = 6;
pub(super) const REV_START: usize = 6;
pub(super) const REV_END: usize = 9;
pub(super) const DELAY_START: usize = 9;
pub(super) const DELAY_END: usize = 12;
pub(super) const NG_START: usize = 12;
pub(super) const NG_END: usize = 14;
pub(super) const AMP_START: usize = 14;
pub(super) const AMP_END: usize = 20;
pub(super) const EQ_START: usize = 20;
pub(super) const EQ_END: usize = 23;

pub(super) const KNOBS: &[Knob] = &[
    // 0–2: TS-808
    Knob {
        label: "DRIVE",
        param: |p| &p.ts_drive,
    },
    Knob {
        label: "TONE",
        param: |p| &p.ts_tone,
    },
    Knob {
        label: "LEVEL",
        param: |p| &p.ts_level,
    },
    // 3–5: DS-1
    Knob {
        label: "DRIVE",
        param: |p| &p.ds_drive,
    },
    Knob {
        label: "TONE",
        param: |p| &p.ds_tone,
    },
    Knob {
        label: "LEVEL",
        param: |p| &p.ds_level,
    },
    // 6–8: Reverb
    Knob {
        label: "ROOM",
        param: |p| &p.rev_room,
    },
    Knob {
        label: "DAMP",
        param: |p| &p.rev_damp,
    },
    Knob {
        label: "MIX",
        param: |p| &p.rev_mix,
    },
    // 9–11: Delay
    Knob {
        label: "TIME",
        param: |p| &p.delay_time,
    },
    Knob {
        label: "FEEDBACK",
        param: |p| &p.delay_feedback,
    },
    Knob {
        label: "MIX",
        param: |p| &p.delay_mix,
    },
    // 12–13: Noise Gate
    Knob {
        label: "THRESH",
        param: |p| &p.ng_threshold,
    },
    Knob {
        label: "RELEASE",
        param: |p| &p.ng_release,
    },
    // 14–19: Amp
    Knob {
        label: "GAIN",
        param: |p| &p.amp_gain,
    },
    Knob {
        label: "BASS",
        param: |p| &p.amp_bass,
    },
    Knob {
        label: "MID",
        param: |p| &p.amp_mid,
    },
    Knob {
        label: "TREBLE",
        param: |p| &p.amp_treble,
    },
    Knob {
        label: "PRESENCE",
        param: |p| &p.amp_presence,
    },
    Knob {
        label: "MASTER",
        param: |p| &p.amp_master,
    },
    // 20–22: Parametric EQ
    Knob {
        label: "LOW",
        param: |p| &p.eq_low,
    },
    Knob {
        label: "MID",
        param: |p| &p.eq_mid,
    },
    Knob {
        label: "HIGH",
        param: |p| &p.eq_high,
    },
];

// Row 1: TS-808, DS-1, Reverb, Delay, Noise Gate
pub(super) const PEDAL_SECTIONS: &[SectionDef] = &[
    (
        |_| "⚡ TS-808".into(),
        TS_START,
        TS_END,
        |p| Some(p.ts_enabled.load(Relaxed)),
        3,
    ),
    (
        |_| "⚡ DS-1 DISTORTION".into(),
        DS_START,
        DS_END,
        |p| Some(p.ds_enabled.load(Relaxed)),
        3,
    ),
    (
        |_| "⚡ SPRING REVERB".into(),
        REV_START,
        REV_END,
        |p| Some(p.rev_enabled.load(Relaxed)),
        3,
    ),
    (
        |_| "⏱ DELAY".into(),
        DELAY_START,
        DELAY_END,
        |p| Some(p.delay_enabled.load(Relaxed)),
        3,
    ),
    (
        |_| "🔇 NOISE GATE".into(),
        NG_START,
        NG_END,
        |p| Some(p.ng_enabled.load(Relaxed)),
        2,
    ),
];

// Row 2: Amp (wider), Parametric EQ
pub(super) const AMP_SECTIONS: &[SectionDef] = &[
    (
        |p| format!("⚡ {}", p.amp_model().name()),
        AMP_START,
        AMP_END,
        |_| None,
        2,
    ),
    (
        |_| "🎛 PARAMETRIC EQ".into(),
        EQ_START,
        EQ_END,
        |p| Some(p.eq_enabled.load(Relaxed)),
        1,
    ),
];

// Tab order: None (selectors) → TS → DS → Rev → Delay → NG → Amp → EQ
pub(super) const SECTION_STARTS: &[Option<usize>] = &[
    None,
    Some(TS_START),
    Some(DS_START),
    Some(REV_START),
    Some(DELAY_START),
    Some(NG_START),
    Some(AMP_START),
    Some(EQ_START),
];
